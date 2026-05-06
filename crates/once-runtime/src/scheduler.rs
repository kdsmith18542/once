use crate::channel::Channel;
use crate::deadlock::DeadlockDetector;
use crate::group::TaskGroup;
use crate::task::{Task, TaskRegistry};
use crate::value::{
    clear_current_task, current_task_id, set_current_task, BackpressurePolicy, ChannelHandle,
    ChannelId, RuntimeError, TaskHandle, TaskId, TaskStatus, Value,
};
use crossbeam_deque::{Stealer, Worker};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct WorkerStats {
    pub tasks_completed: usize,
    pub tasks_stolen: usize,
}

pub struct SchedulerInner {
    pub tasks: HashMap<TaskId, Task>,
    pub channels: HashMap<ChannelId, Arc<Channel<Value>>>,
    pub group_notifiers: HashMap<usize, Arc<Mutex<TaskGroup>>>,
    pub task_group_memberships: HashMap<TaskId, Vec<Arc<Mutex<TaskGroup>>>>,
    pub next_task_id: TaskId,
    pub next_channel_id: ChannelId,
    pub is_running: bool,
    pub deadlock_detector: DeadlockDetector,
    pub blocked_tasks: HashMap<TaskId, Vec<ChannelId>>,
    pub ready_queue: VecDeque<TaskId>,
    pub worker_stats: Vec<WorkerStats>,
    pub deterministic: bool,
}

pub struct Scheduler {
    pub inner: Arc<Mutex<SchedulerInner>>,
    pub registry: Arc<Mutex<TaskRegistry>>,
    pub workers: Mutex<Vec<JoinHandle<()>>>,
    pub work_sender: Mutex<Option<crossbeam::channel::Sender<TaskId>>>,
    pub shutdown_sender: Mutex<Option<crossbeam::channel::Sender<()>>>,
    pub worker_count: usize,
    /// Per-worker deques for work-stealing (one Worker consumed per thread, stealers for others)
    pub worker_stealers: Mutex<Vec<Stealer<TaskId>>>,
    /// Current worker's deque for fast local injection
    pub local_worker: Mutex<Option<Worker<TaskId>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .max(2);

        Self {
            inner: Arc::new(Mutex::new(SchedulerInner {
                tasks: HashMap::new(),
                channels: HashMap::new(),
                group_notifiers: HashMap::new(),
                task_group_memberships: HashMap::new(),
                next_task_id: 0,
                next_channel_id: 0,
                is_running: false,
                deadlock_detector: DeadlockDetector::new(),
                blocked_tasks: HashMap::new(),
                ready_queue: VecDeque::new(),
                worker_stats: vec![WorkerStats::default(); worker_count],
                deterministic: false,
            })),
            registry: Arc::new(Mutex::new(TaskRegistry::new())),
            workers: Mutex::new(Vec::new()),
            work_sender: Mutex::new(None),
            shutdown_sender: Mutex::new(None),
            worker_count,
            worker_stealers: Mutex::new(Vec::new()),
            local_worker: Mutex::new(None),
        }
    }

    pub fn spawn_task(&self, function: String, args: Vec<Value>) -> TaskHandle {
        let mut inner = self.inner.lock().unwrap();
        let task_id = inner.next_task_id;
        inner.next_task_id += 1;

        let task = Task::new(task_id, function, args);
        inner.tasks.insert(task_id, task);

        inner
            .deadlock_detector
            .wait_for_graph
            .entry(task_id)
            .or_insert_with(Vec::new);

        inner.ready_queue.push_back(task_id);

        if inner.is_running {
            if let Some(ref sender) = *self.work_sender.lock().unwrap() {
                let _ = sender.send(task_id);
            }
        }

        TaskHandle {
            id: task_id,
            status: TaskStatus::Pending,
            result: None,
        }
    }

    pub fn register_handler(&self, name: &str, handler: crate::task::TaskHandler) {
        self.registry.lock().unwrap().register(name, handler);
    }

    pub fn await_task(&self, task_handle: TaskHandle) -> Result<Value, RuntimeError> {
        let task_id = task_handle.id;

        if let Some(current_id) = current_task_id() {
            if current_id != task_id {
                let mut inner = self.inner.lock().unwrap();
                inner.deadlock_detector.add_wait(current_id, task_id);
                drop(inner);
            }
        }

        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();

        let result = loop {
            if start.elapsed() >= timeout {
                break Err(RuntimeError::TaskError(
                    "Task execution timeout".to_string(),
                ));
            }

            let inner = self.inner.lock().unwrap();
            if let Some(task) = inner.tasks.get(&task_id) {
                match task.status {
                    TaskStatus::Completed => {
                        break task.result.clone().ok_or_else(|| {
                            RuntimeError::SchedulerError(
                                "Task completed but no result".to_string(),
                            )
                        });
                    }
                    TaskStatus::Failed => {
                        break Err(RuntimeError::SchedulerError("Task failed".to_string()));
                    }
                    TaskStatus::Cancelled => {
                        break Err(RuntimeError::SchedulerError(
                            "Task cancelled".to_string(),
                        ));
                    }
                    _ => {}
                }
            } else {
                break Err(RuntimeError::TaskError("Task not found".to_string()));
            }
            drop(inner);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };

        if let Some(current_id) = current_task_id() {
            if current_id != task_id {
                let mut inner = self.inner.lock().unwrap();
                inner.deadlock_detector.remove_wait(current_id, task_id);
            }
        }

        result
    }

    pub fn create_channel(
        &self,
        capacity: usize,
        backpressure_policy: BackpressurePolicy,
    ) -> ChannelHandle {
        let mut inner = self.inner.lock().unwrap();
        let channel_id = inner.next_channel_id;
        inner.next_channel_id += 1;

        let channel = Arc::new(Channel::new(
            channel_id,
            capacity,
            backpressure_policy.clone(),
        ));
        inner.channels.insert(channel_id, channel);

        inner
            .deadlock_detector
            .channel_waits
            .entry(channel_id)
            .or_insert_with(Vec::new);
        inner
            .deadlock_detector
            .channel_senders
            .entry(channel_id)
            .or_insert_with(Vec::new);

        ChannelHandle {
            id: channel_id,
            capacity,
            backpressure_policy,
        }
    }

    pub fn spawn_actor(
        &self,
        name: &str,
        handler: crate::task::TaskHandler,
    ) -> TaskHandle {
        // Register the handler under "actor:<name>" so the scheduler can dispatch
        // messages to it as regular tasks on the work-stealing pool.
        self.registry
            .lock()
            .unwrap()
            .register(&format!("actor:{}", name), handler);

        let mut inner = self.inner.lock().unwrap();
        let actor_task_id = inner.next_task_id;
        inner.next_task_id += 1;

        let task = Task::new(actor_task_id, format!("actor:{}", name), vec![]);
        inner.tasks.insert(actor_task_id, task);

        inner
            .deadlock_detector
            .wait_for_graph
            .entry(actor_task_id)
            .or_insert_with(Vec::new);

        let is_running_state = inner.is_running;
        drop(inner);

        {
            let mut guard = self.inner.lock().unwrap();
            if let Some(task) = guard.tasks.get_mut(&actor_task_id) {
                task.status = TaskStatus::Running;
            }
        }

        TaskHandle {
            id: actor_task_id,
            status: TaskStatus::Running,
            result: None,
        }
    }

    /// Send a message to an actor. Instead of queueing in a mailbox and processing
    /// on a dedicated thread, this directly spawns a task on the work-stealing pool
    /// that invokes the actor's handler with the message as arguments.
    fn send_to_actor(&self, actor_name: &str, value: Value) -> Result<(), RuntimeError> {
        let handle = self.spawn_task(format!("actor:{}", actor_name), vec![value]);
        // Mark the spawned task's thread as the current sender for deadlock tracking
        if let Some(task_id) = current_task_id() {
            self.deadlock_add_wait(task_id, handle.id);
        }
        Ok(())
    }

    pub fn spawn_actor_with_init(
        &self,
        name: &str,
        handler: crate::task::TaskHandler,
        init_args: Vec<Value>,
    ) -> TaskHandle {
        let mut inner = self.inner.lock().unwrap();

        let channel_id = inner.next_channel_id;
        inner.next_channel_id += 1;
        let mailbox_ch = Arc::new(Channel::new(
            channel_id,
            256,
            BackpressurePolicy::Blocking,
        ));
        inner.channels.insert(channel_id, mailbox_ch.clone());

        inner
            .deadlock_detector
            .channel_waits
            .entry(channel_id)
            .or_insert_with(Vec::new);
        inner
            .deadlock_detector
            .channel_senders
            .entry(channel_id)
            .or_insert_with(Vec::new);

        let task_id = inner.next_task_id;
        inner.next_task_id += 1;

        let task = Task::new(task_id, format!("actor:{}", name), init_args);
        inner.tasks.insert(task_id, task);
        inner
            .deadlock_detector
            .wait_for_graph
            .entry(task_id)
            .or_insert_with(Vec::new);

        drop(inner);

        self.registry
            .lock()
            .unwrap()
            .register(&format!("actor:{}", name), handler);

        let mailbox = mailbox_ch;
        let actor_name = name.to_string();
        let registry = Arc::clone(&self.registry);

        let handle = std::thread::spawn(move || {
            set_current_task(task_id);
            loop {
                match mailbox.recv() {
                    Ok(value) => {
                        let reg = registry.lock().unwrap();
                        match reg.execute(&format!("actor:{}", actor_name), &[value]) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Actor '{}' error: {:?}", actor_name, e);
                                break;
                            }
                        }
                    }
                    Err(RuntimeError::ChannelError(_)) => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Actor '{}' mailbox error: {:?}", actor_name, e);
                        break;
                    }
                }
            }
            clear_current_task();
        });

        {
            let mut workers = self.workers.lock().unwrap();
            workers.push(handle);
        }

        TaskHandle {
            id: task_id,
            status: TaskStatus::Running,
            result: None,
        }
    }

    pub fn block_task_on_channel(&self, task_id: TaskId, channel_id: ChannelId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .blocked_tasks
            .entry(task_id)
            .or_insert_with(Vec::new)
            .push(channel_id);
        if let Some(task) = inner.tasks.get_mut(&task_id) {
            task.status = TaskStatus::Pending;
        }
        inner.ready_queue.retain(|&id| id != task_id);
    }

    pub fn unblock_task(&self, task_id: TaskId) {
        let mut inner = self.inner.lock().unwrap();
        inner.blocked_tasks.remove(&task_id);
        if !inner.ready_queue.contains(&task_id) {
            inner.ready_queue.push_back(task_id);
        }
        if let Some(ref sender) = *self.work_sender.lock().unwrap() {
            let _ = sender.send(task_id);
        }
    }

    pub fn wake_blocked_on_channel(&self, channel_id: ChannelId) {
        let inner = self.inner.lock().unwrap();
        let blocked: Vec<TaskId> = inner
            .blocked_tasks
            .iter()
            .filter_map(|(tid, channels)| {
                if channels.contains(&channel_id) {
                    Some(*tid)
                } else {
                    None
                }
            })
            .collect();
        drop(inner);

        for task_id in blocked {
            self.unblock_task(task_id);
        }
    }

    pub fn send_to_channel(
        &self,
        channel_id: ChannelId,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(task_id) = current_task_id() {
            let mut inner = self.inner.lock().unwrap();
            inner
                .deadlock_detector
                .register_sender(task_id, channel_id);
            drop(inner);
        }

        let result = {
            let inner = self.inner.lock().unwrap();
            if let Some(channel) = inner.channels.get(&channel_id) {
                channel.send(value)
            } else {
                Err(RuntimeError::ChannelError("Channel not found".to_string()))
            }
        };

        if result.is_ok() {
            self.wake_blocked_on_channel(channel_id);
        }

        result
    }

    pub fn recv_from_channel(&self, channel_id: ChannelId) -> Result<Value, RuntimeError> {
        if let Some(task_id) = current_task_id() {
            let mut inner = self.inner.lock().unwrap();
            inner
                .deadlock_detector
                .add_channel_wait(task_id, channel_id);
            drop(inner);
        }

        let inner = self.inner.lock().unwrap();
        let result = if let Some(channel) = inner.channels.get(&channel_id) {
            channel.recv()
        } else {
            Err(RuntimeError::ChannelError("Channel not found".to_string()))
        };

        if let Some(task_id) = current_task_id() {
            let mut inner = self.inner.lock().unwrap();
            inner
                .deadlock_detector
                .remove_channel_wait(task_id, channel_id);
        }

        result
    }

    pub fn set_deterministic(&self, deterministic: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.deterministic = deterministic;
    }

    pub fn run(&self) -> Result<(), RuntimeError> {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.is_running = true;
        }

        {
            let inner = self.inner.lock().unwrap();
            if inner.deterministic {
                drop(inner);
                loop {
                    let guard = self.inner.lock().unwrap();
                    if !guard.is_running { break; }
                    if guard.ready_queue.is_empty() && Self::all_tasks_done(&guard.tasks) { break; }
                    if let Some(task_id) = guard.ready_queue.front().copied() {
                        drop(guard);
                        let (name, args) = {
                            let mut guard = self.inner.lock().unwrap();
                            guard.ready_queue.pop_front();
                            let task = match guard.tasks.get_mut(&task_id) {
                                Some(t) => t,
                                None => continue,
                            };
                            task.status = TaskStatus::Running;
                            task.started_at = Some(Instant::now());
                            (task.function.clone(), task.args.clone())
                        };

                        crate::value::set_current_task(task_id);

                        let result = {
                            let reg = self.registry.lock().unwrap();
                            reg.execute(&name, &args)
                        };

                        crate::value::clear_current_task();

                        let mut guard = self.inner.lock().unwrap();
                        let mut groups_to_notify = Vec::new();
                        if let Some(task) = guard.tasks.get_mut(&task_id) {
                            match result {
                                Ok(value) => {
                                    task.status = TaskStatus::Completed;
                                    task.result = Some(value);
                                }
                                Err(e) => {
                                    task.status = TaskStatus::Failed;
                                    eprintln!("Task '{}' failed: {}", name, e);
                                }
                            }
                            task.completed_at = Some(Instant::now());
                        }

                        {
                            let channel_ids: Vec<ChannelId> = guard
                                .deadlock_detector
                                .channel_senders
                                .iter()
                                .filter_map(|(ch_id, senders)| {
                                    if senders.iter().any(|&sid| sid == task_id) {
                                        Some(*ch_id)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            for channel_id in channel_ids {
                                guard
                                    .deadlock_detector
                                    .deregister_sender(task_id, channel_id);
                            }
                            guard.blocked_tasks.remove(&task_id);
                            groups_to_notify = guard
                                .task_group_memberships
                                .remove(&task_id)
                                .unwrap_or_default();
                        }
                        drop(guard);
                        for group in groups_to_notify {
                            if let Ok(group) = group.lock() {
                                group.notify_child_complete();
                            }
                        }
                    } else {
                        drop(guard);
                        thread::sleep(Duration::from_millis(1));
                    }
                }
                return Ok(());
            }
        }

        let (work_tx, work_rx) = crossbeam::channel::unbounded::<TaskId>();
        let (shutdown_tx, shutdown_rx) = crossbeam::channel::unbounded::<()>();

        *self.work_sender.lock().unwrap() = Some(work_tx.clone());
        *self.shutdown_sender.lock().unwrap() = Some(shutdown_tx);

        let inner = Arc::clone(&self.inner);
        let registry = Arc::clone(&self.registry);

        // Create per-worker deques for work-stealing
        let mut workers_list = Vec::new();
        let mut stealers_list = Vec::new();
        for _ in 0..self.worker_count {
            let worker = Worker::new_fifo();
            stealers_list.push(worker.stealer());
            workers_list.push(worker);
        }
        *self.worker_stealers.lock().unwrap() = stealers_list;

        {
            let guard = inner.lock().unwrap();
            for (&task_id, task) in &guard.tasks {
                if task.status == TaskStatus::Pending {
                    let _ = work_tx.send(task_id);
                }
            }
        }

        let mut handles = self.workers.lock().unwrap();
        for worker_idx in 0..self.worker_count {
            let work_rx = work_rx.clone();
            let shutdown_rx = shutdown_rx.clone();
            let inner = Arc::clone(&inner);
            let registry = Arc::clone(&registry);
            let local_worker = std::mem::replace(&mut workers_list[worker_idx], Worker::new_fifo());
            let stealers = Arc::clone(&Arc::new(
                self.worker_stealers.lock().unwrap().clone(),
            ));

            let handle = thread::spawn(move || {
                crate::worker::run_worker_loop(
                    worker_idx,
                    inner,
                    registry,
                    local_worker,
                    stealers,
                    work_rx,
                    shutdown_rx,
                );
            });
            handles.push(handle);
        }

        loop {
            let guard = self.inner.lock().unwrap();
            if !guard.is_running {
                drop(guard);
                break;
            }

            let tasks = guard.tasks.clone();
            drop(guard);
            if let Err(deadlock) = self.detect_deadlock(&tasks) {
                return Err(deadlock);
            }

            self.handle_completed_tasks();
            thread::sleep(Duration::from_millis(1));
        }

        if let Some(ref sender) = *self.shutdown_sender.lock().unwrap() {
            let _ = sender.send(());
        }
        for handle in handles.drain(..) {
            let _ = handle.join();
        }

        Ok(())
    }

    fn detect_deadlock(&self, tasks: &HashMap<TaskId, Task>) -> Result<(), RuntimeError> {
        let guard = self.inner.lock().unwrap();
        guard.deadlock_detector.detect_deadlock(tasks)
    }

    fn handle_completed_tasks(&self) {
        let guard = self.inner.lock().unwrap();
        for (task_id, task) in &guard.tasks {
            if task.status == TaskStatus::Completed {
                println!("Task {} completed successfully", task_id);
            } else if task.status == TaskStatus::Failed {
                println!("Task {} failed", task_id);
            }
        }
    }

    pub fn stop(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.is_running = false;

        let channel_ids: Vec<ChannelId> = inner.channels.keys().copied().collect();
        for channel_id in channel_ids {
            if let Some(channel) = inner.channels.get(&channel_id) {
                channel.close();
            }
        }
    }

    pub fn deadlock_detect_enhanced(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.deadlock_detector.detect_deadlock_enhanced()
    }

    pub fn deadlock_check_channel(&self) -> Result<(), RuntimeError> {
        let inner = self.inner.lock().unwrap();
        inner.deadlock_detector.check_channel_deadlock()
    }

    pub fn deadlock_add_wait(&self, task_id: TaskId, waiting_for: TaskId) {
        let mut inner = self.inner.lock().unwrap();
        inner.deadlock_detector.add_wait(task_id, waiting_for);
    }

    pub fn deadlock_add_channel_wait(&self, task_id: TaskId, channel_id: ChannelId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .deadlock_detector
            .add_channel_wait(task_id, channel_id);
    }

    pub fn deadlock_remove_channel_wait(&self, task_id: TaskId, channel_id: ChannelId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .deadlock_detector
            .remove_channel_wait(task_id, channel_id);
    }

    pub fn deadlock_get_info(&self) -> (bool, Vec<TaskId>) {
        let inner = self.inner.lock().unwrap();
        let trace = inner.deadlock_detector.get_deadlock_trace();
        (inner.deadlock_detector.cycle_detected, trace)
    }

    pub fn deadlock_insert_wait_graph(&self, task_id: TaskId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .deadlock_detector
            .wait_for_graph
            .insert(task_id, Vec::new());
    }

    pub fn deadlock_insert_channel_waits(&self, channel_id: ChannelId) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .deadlock_detector
            .channel_waits
            .insert(channel_id, Vec::new());
    }

    pub fn spawn_task_in_group(
        &self,
        group_id: usize,
        function: String,
        args: Vec<Value>,
    ) -> TaskHandle {
        let handle = self.spawn_task(function, args);
        self.register_group_child(group_id, handle.id);
        handle
    }

    pub fn register_group_child(&self, group_id: usize, task_id: TaskId) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(group) = inner.group_notifiers.get(&group_id).cloned() {
            inner
                .task_group_memberships
                .entry(task_id)
                .or_insert_with(Vec::new)
                .push(group);
        }
    }

    pub fn register_group_membership(
        &self,
        group_id: usize,
        group: Arc<Mutex<TaskGroup>>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .group_notifiers
            .insert(group_id, group);
    }

    fn all_tasks_done(tasks: &HashMap<TaskId, Task>) -> bool {
        tasks.values().all(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled))
    }

    pub fn await_group(
        &self,
        group: Arc<Mutex<TaskGroup>>,
        child_ids: &[TaskId],
    ) -> Result<Vec<Value>, RuntimeError> {
        let completion_condvar = {
            let group_guard = group.lock().unwrap();
            Arc::clone(&group_guard.completion_condvar)
        };
        let completed_count = {
            let group_guard = group.lock().unwrap();
            Arc::clone(&group_guard.completed_count)
        };

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);
        let mut observed_completions = 0usize;

        loop {
            let mut all_complete = true;
            let mut results = Vec::new();

            let mut inner = self.inner.lock().unwrap();
            for &child_id in child_ids {
                if let Some(task) = inner.tasks.get(&child_id) {
                    match task.status {
                        TaskStatus::Completed => {
                            if let Some(ref result) = task.result {
                                results.push(result.clone());
                            } else {
                                results.push(Value::Unit);
                            }
                        }
                        TaskStatus::Failed => {
                            for &remaining_id in child_ids.iter().filter(|&&id| id > child_id) {
                                if let Some(t) = inner.tasks.get_mut(&remaining_id) {
                                    t.status = TaskStatus::Cancelled;
                                }
                            }
                            return Err(RuntimeError::TaskError(format!(
                                "Task {} in group failed",
                                child_id
                            )));
                        }
                        TaskStatus::Cancelled => {
                            return Err(RuntimeError::TaskError(format!(
                                "Task {} in group was cancelled",
                                child_id
                            )));
                        }
                        _ => {
                            all_complete = false;
                        }
                    }
                }
            }
            drop(inner);

            if all_complete {
                return Ok(results);
            }

            if start.elapsed() > timeout {
                return Err(RuntimeError::TaskError("Group await timeout".to_string()));
            }

            let mut completion_guard = completed_count.lock().unwrap();
            while *completion_guard <= observed_completions {
                let elapsed = start.elapsed();
                if elapsed > timeout {
                    return Err(RuntimeError::TaskError("Group await timeout".to_string()));
                }
                let remaining = timeout
                    .checked_sub(elapsed)
                    .unwrap_or_else(|| std::time::Duration::from_millis(0));
                let (guard, wait_result) = completion_condvar
                    .wait_timeout(completion_guard, remaining)
                    .unwrap();
                completion_guard = guard;
                if wait_result.timed_out() && *completion_guard <= observed_completions {
                    return Err(RuntimeError::TaskError("Group await timeout".to_string()));
                }
            }
            observed_completions = *completion_guard;
        }
    }
}
