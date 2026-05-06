use crate::scheduler::SchedulerInner;
use crate::task::TaskRegistry;
use crate::value::{clear_current_task, set_current_task, ChannelId, TaskId, TaskStatus};
use crossbeam_deque::{Steal, Stealer, Worker};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn execute_task(
    task_id: TaskId,
    is_stolen: bool,
    worker_idx: usize,
    inner: &Arc<Mutex<SchedulerInner>>,
    registry: &Arc<Mutex<TaskRegistry>>,
) -> bool {
    let (name, args) = {
        let mut guard = inner.lock().unwrap();
        guard.ready_queue.retain(|&id| id != task_id);
        let task = match guard.tasks.get_mut(&task_id) {
            Some(t) => t,
            None => {
                thread::sleep(Duration::from_micros(100));
                return false;
            }
        };
        task.status = TaskStatus::Running;
        task.started_at = Some(Instant::now());
        (task.function.clone(), task.args.clone())
    };

    set_current_task(task_id);

    let result = {
        let reg = registry.lock().unwrap();
        reg.execute(&name, &args)
    };

    clear_current_task();

    let mut guard = inner.lock().unwrap();
    if worker_idx < guard.worker_stats.len() {
        guard.worker_stats[worker_idx].tasks_completed += 1;
        if is_stolen {
            guard.worker_stats[worker_idx].tasks_stolen += 1;
        }
    }
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

    let groups_to_notify = {
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
        guard
            .task_group_memberships
            .remove(&task_id)
            .unwrap_or_default()
    };
    drop(guard);

    for group in groups_to_notify {
        if let Ok(group) = group.lock() {
            group.notify_child_complete();
        }
    }

    true
}

pub fn run_worker_loop(
    worker_idx: usize,
    inner: Arc<Mutex<SchedulerInner>>,
    registry: Arc<Mutex<TaskRegistry>>,
    local_worker: Worker<TaskId>,
    stealers: Arc<Vec<Stealer<TaskId>>>,
    work_rx: crossbeam::channel::Receiver<TaskId>,
    shutdown_rx: crossbeam::channel::Receiver<()>,
) {
    loop {
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        let task_id;
        let is_stolen;
        match local_worker.pop() {
            Some(id) => {
                task_id = id;
                is_stolen = false;
            }
            None => {
                match work_rx.recv_timeout(Duration::from_millis(10)) {
                    Ok(id) => {
                        task_id = id;
                        is_stolen = false;
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                        let mut stolen_id = None;
                        for (si, stealer) in stealers.iter().enumerate() {
                            if si == worker_idx {
                                continue;
                            }
                            match stealer.steal() {
                                Steal::Success(id) => {
                                    stolen_id = Some(id);
                                    break;
                                }
                                Steal::Retry => {
                                    if let Steal::Success(id) = stealer.steal() {
                                        stolen_id = Some(id);
                                        break;
                                    }
                                }
                                Steal::Empty => {}
                            }
                        }
                        match stolen_id {
                            Some(id) => {
                                task_id = id;
                                is_stolen = true;
                            }
                            None => continue,
                        }
                    }
                    Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
        };

        if !execute_task(task_id, is_stolen, worker_idx, &inner, &registry) {
            continue;
        }
    }
}
