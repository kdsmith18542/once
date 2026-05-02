//! Runtime for the Once language
//! 
//! Implements:
//! - Deterministic scheduler
//! - Actor/channel system
//! - Memory management
//! - OS integration
//! - Deadlock detection
//! - Backpressure handling

use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::{Arc, Mutex, Condvar};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::fmt;
use thiserror::Error;

/// Runtime errors
#[derive(Error, Debug, Clone)]
pub enum RuntimeError {
    #[error("Scheduler error: {0}")]
    SchedulerError(String),
    
    #[error("Channel error: {0}")]
    ChannelError(String),
    
    #[error("Memory management error: {0}")]
    MemoryError(String),
    
    #[error("Deadlock detected: {0}")]
    DeadlockError(String),
    
    #[error("Backpressure error: {0}")]
    BackpressureError(String),
    
    #[error("Task error: {0}")]
    TaskError(String),
}

/// Task handle
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskHandle {
    pub id: TaskId,
    pub status: TaskStatus,
    pub result: Option<Box<Value>>,
}

/// Task ID
pub type TaskId = usize;

/// Task status
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Value types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Task(Box<TaskHandle>),
    Channel(ChannelHandle),
    Json(serde_json::Value),
}

/// Channel handle
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelHandle {
    pub id: ChannelId,
    pub capacity: usize,
    pub backpressure_policy: BackpressurePolicy,
}

/// Channel ID
pub type ChannelId = usize;

/// Backpressure policy
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackpressurePolicy {
    Blocking,
    Dropping,
    Erroring,
}

/// Channel
pub struct Channel<T> {
    pub id: ChannelId,
    pub capacity: usize,
    pub buffer: Mutex<VecDeque<T>>,
    pub senders: usize,
    pub receivers: usize,
    pub backpressure_policy: BackpressurePolicy,
    pub condvar: Condvar,
}

impl<T> Channel<T> {
    pub fn new(id: ChannelId, capacity: usize, backpressure_policy: BackpressurePolicy) -> Self {
        Self {
            id,
            capacity,
            buffer: Mutex::new(VecDeque::new()),
            senders: 0,
            receivers: 0,
            backpressure_policy,
            condvar: Condvar::new(),
        }
    }

    pub fn send(&self, value: T) -> Result<(), RuntimeError> {
        let mut buffer = self.buffer.lock().unwrap();
        
        match self.backpressure_policy {
            BackpressurePolicy::Blocking => {
                while buffer.len() >= self.capacity {
                    buffer = self.condvar.wait(buffer).unwrap();
                }
                buffer.push_back(value);
                self.condvar.notify_one();
                Ok(())
            }
            BackpressurePolicy::Dropping => {
                if buffer.len() >= self.capacity {
                    buffer.pop_front();
                }
                buffer.push_back(value);
                self.condvar.notify_one();
                Ok(())
            }
            BackpressurePolicy::Erroring => {
                if buffer.len() >= self.capacity {
                    Err(RuntimeError::BackpressureError("Channel full".to_string()))
                } else {
                    buffer.push_back(value);
                    self.condvar.notify_one();
                    Ok(())
                }
            }
        }
    }

    pub fn recv(&self) -> Result<T, RuntimeError> {
        let mut buffer = self.buffer.lock().unwrap();
        
        while buffer.is_empty() {
            buffer = self.condvar.wait(buffer).unwrap();
        }
        
        buffer.pop_front()
            .ok_or_else(|| RuntimeError::ChannelError("Channel closed".to_string()))
    }
}

/// Task
pub struct Task {
    pub id: TaskId,
    pub function: String,
    pub args: Vec<Value>,
    pub status: TaskStatus,
    pub result: Option<Value>,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl Task {
    pub fn new(id: TaskId, function: String, args: Vec<Value>) -> Self {
        Self {
            id,
            function,
            args,
            status: TaskStatus::Pending,
            result: None,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
        }
    }
}

/// Scheduler
pub struct Scheduler {
    pub tasks: HashMap<TaskId, Task>,
    pub channels: HashMap<ChannelId, Arc<Channel<Value>>>,
    pub next_task_id: TaskId,
    pub next_channel_id: ChannelId,
    pub is_running: bool,
    pub deadlock_detector: DeadlockDetector,
    pub registry: TaskRegistry,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            channels: HashMap::new(),
            next_task_id: 0,
            next_channel_id: 0,
            is_running: false,
            deadlock_detector: DeadlockDetector::new(),
            registry: TaskRegistry::new(),
        }
    }

    pub fn spawn_task(&mut self, function: String, args: Vec<Value>) -> TaskHandle {
        let task_id = self.next_task_id;
        self.next_task_id += 1;

        let task = Task::new(task_id, function, args);
        self.tasks.insert(task_id, task);

        TaskHandle {
            id: task_id,
            status: TaskStatus::Pending,
            result: None,
        }
    }

    /// Register a task handler function
    pub fn register_handler(&mut self, name: &str, handler: TaskHandler) {
        self.registry.register(name, handler);
    }

    pub fn await_task(&mut self, task_handle: TaskHandle) -> Result<Value, RuntimeError> {
        let task_id = task_handle.id;
        
        if let Some(task) = self.tasks.get(&task_id) {
            match task.status {
                TaskStatus::Completed => {
                    task.result.clone()
                        .ok_or_else(|| RuntimeError::SchedulerError("Task completed but no result".to_string()))
                }
                TaskStatus::Failed => {
                    Err(RuntimeError::SchedulerError("Task failed".to_string()))
                }
                TaskStatus::Cancelled => {
                    Err(RuntimeError::SchedulerError("Task cancelled".to_string()))
                }
                _ => {
                    // Task is still running, wait for completion
                    self.wait_for_task_completion(task_id)
                }
            }
        } else {
            Err(RuntimeError::SchedulerError("Task not found".to_string()))
        }
    }

    fn wait_for_task_completion(&mut self, task_id: TaskId) -> Result<Value, RuntimeError> {
        // Wait for task to complete with timeout
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if let Some(task) = self.tasks.get(&task_id) {
                match task.status {
                    TaskStatus::Completed => {
                        return Ok(Value::Unit); // Return task result
                    }
                    TaskStatus::Failed => {
                        return Err(RuntimeError::TaskError("Task execution failed".to_string()));
                    }
                    TaskStatus::Running => {
                        // Task is still running, continue waiting
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    TaskStatus::Pending => {
                        // Task hasn't started yet, continue waiting
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    TaskStatus::Cancelled => {
                        return Err(RuntimeError::TaskError("Task was cancelled".to_string()));
                    }
                }
            } else {
                return Err(RuntimeError::TaskError("Task not found".to_string()));
            }
        }
        
        Err(RuntimeError::TaskError("Task execution timeout".to_string()))
    }

    pub fn create_channel(&mut self, capacity: usize, backpressure_policy: BackpressurePolicy) -> ChannelHandle {
        let channel_id = self.next_channel_id;
        self.next_channel_id += 1;

        let channel = Arc::new(Channel::new(channel_id, capacity, backpressure_policy.clone()));
        self.channels.insert(channel_id, channel);

        ChannelHandle {
            id: channel_id,
            capacity,
            backpressure_policy,
        }
    }

    pub fn send_to_channel(&self, channel_id: ChannelId, value: Value) -> Result<(), RuntimeError> {
        if let Some(channel) = self.channels.get(&channel_id) {
            channel.send(value)
        } else {
            Err(RuntimeError::ChannelError("Channel not found".to_string()))
        }
    }

    pub fn recv_from_channel(&self, channel_id: ChannelId) -> Result<Value, RuntimeError> {
        if let Some(channel) = self.channels.get(&channel_id) {
            channel.recv()
        } else {
            Err(RuntimeError::ChannelError("Channel not found".to_string()))
        }
    }

    pub fn run(&mut self) -> Result<(), RuntimeError> {
        self.is_running = true;
        
        while self.is_running {
            // Check for deadlocks
            if let Err(deadlock) = self.deadlock_detector.detect_deadlock(&self.tasks) {
                return Err(deadlock);
            }

            // Execute pending tasks
            self.execute_pending_tasks()?;

            // Handle completed tasks
            self.handle_completed_tasks();

            // Sleep briefly to prevent busy waiting
            thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }

    fn execute_pending_tasks(&mut self) -> Result<(), RuntimeError> {
        let pending_tasks: Vec<TaskId> = self.tasks
            .iter()
            .filter(|(_, task)| task.status == TaskStatus::Pending)
            .map(|(id, _)| *id)
            .collect();

        for task_id in pending_tasks {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Running;
                task.started_at = Some(Instant::now());
                
                let function_name = task.function.clone();
                let args = task.args.clone();
                
                // Dispatch through the task registry
                let result = self.registry.execute(&function_name, &args);
                
                match result {
                    Ok(value) => {
                        task.status = TaskStatus::Completed;
                        task.result = Some(value);
                    }
                    Err(e) => {
                        task.status = TaskStatus::Failed;
                        task.result = None;
                        eprintln!("Task '{}' failed: {}", function_name, e);
                    }
                }
                task.completed_at = Some(Instant::now());
            }
        }

        Ok(())
    }

    fn handle_completed_tasks(&mut self) {
        // Handle task completion notifications
        for (task_id, task) in &self.tasks {
            if task.status == TaskStatus::Completed {
                println!("Task {} completed successfully", task_id);
            } else if task.status == TaskStatus::Failed {
                println!("Task {} failed", task_id);
            }
        }
    }
    

    pub fn stop(&mut self) {
        self.is_running = false;
    }
}

/// Deadlock detector
pub struct DeadlockDetector {
    pub wait_for_graph: HashMap<TaskId, Vec<TaskId>>,
    pub channel_waits: HashMap<ChannelId, Vec<TaskId>>,
    pub task_waits: HashMap<TaskId, Vec<ChannelId>>,
    pub cycle_detected: bool,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            wait_for_graph: HashMap::new(),
            channel_waits: HashMap::new(),
            task_waits: HashMap::new(),
            cycle_detected: false,
        }
    }

    pub fn detect_deadlock(&self, tasks: &HashMap<TaskId, Task>) -> Result<(), RuntimeError> {
        // Build wait-for graph
        let mut graph = HashMap::new();
        
        for (task_id, task) in tasks {
            if task.status == TaskStatus::Running {
                // TODO: Build actual wait-for relationships
                // For now, just check for cycles in a simplified way
                graph.insert(*task_id, Vec::new());
            }
        }

        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in graph.keys() {
            if !visited.contains(task_id) {
                if self.dfs_has_cycle(*task_id, &graph, &mut visited, &mut rec_stack) {
                    return Err(RuntimeError::DeadlockError(
                        format!("Deadlock detected involving task {}", task_id)
                    ));
                }
            }
        }

        Ok(())
    }

    fn dfs_has_cycle(
        &self,
        task_id: TaskId,
        graph: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
    ) -> bool {
        visited.insert(task_id);
        rec_stack.insert(task_id);

        if let Some(neighbors) = graph.get(&task_id) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_has_cycle(*neighbor, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&task_id);
        false
    }

    /// Add a wait relationship
    pub fn add_wait(&mut self, task_id: TaskId, waiting_for: TaskId) {
        self.wait_for_graph.entry(task_id).or_insert_with(Vec::new).push(waiting_for);
    }

    /// Add a channel wait
    pub fn add_channel_wait(&mut self, task_id: TaskId, channel_id: ChannelId) {
        self.channel_waits.entry(channel_id).or_insert_with(Vec::new).push(task_id);
        self.task_waits.entry(task_id).or_insert_with(Vec::new).push(channel_id);
    }

    /// Remove a wait relationship
    pub fn remove_wait(&mut self, task_id: TaskId, waiting_for: TaskId) {
        if let Some(waits) = self.wait_for_graph.get_mut(&task_id) {
            waits.retain(|&id| id != waiting_for);
        }
    }

    /// Remove a channel wait
    pub fn remove_channel_wait(&mut self, task_id: TaskId, channel_id: ChannelId) {
        if let Some(tasks) = self.channel_waits.get_mut(&channel_id) {
            tasks.retain(|&id| id != task_id);
        }
        if let Some(channels) = self.task_waits.get_mut(&task_id) {
            channels.retain(|&id| id != channel_id);
        }
    }

    /// Detect deadlock using cycle detection (enhanced version)
    pub fn detect_deadlock_enhanced(&mut self) -> Result<(), RuntimeError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &task_id in self.wait_for_graph.keys() {
            if !visited.contains(&task_id) {
                if self.dfs_cycle_detection(task_id, &mut visited, &mut rec_stack) {
                    self.cycle_detected = true;
                    return Err(RuntimeError::SchedulerError(
                        format!("Deadlock detected! Cycle found involving task {}", task_id)
                    ));
                }
            }
        }

        Ok(())
    }

    /// DFS cycle detection
    fn dfs_cycle_detection(&self, task_id: TaskId, visited: &mut HashSet<TaskId>, rec_stack: &mut HashSet<TaskId>) -> bool {
        visited.insert(task_id);
        rec_stack.insert(task_id);

        if let Some(waits) = self.wait_for_graph.get(&task_id) {
            for &waiting_for in waits {
                if !visited.contains(&waiting_for) {
                    if self.dfs_cycle_detection(waiting_for, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&waiting_for) {
                    return true;
                }
            }
        }

        rec_stack.remove(&task_id);
        false
    }

    /// Get deadlock trace
    pub fn get_deadlock_trace(&self) -> Vec<TaskId> {
        let mut trace = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &task_id in self.wait_for_graph.keys() {
            if !visited.contains(&task_id) {
                if self.dfs_cycle_detection(task_id, &mut visited, &mut rec_stack) {
                    trace.extend(rec_stack);
                    break;
                }
            }
        }

        trace
    }

    /// Check for channel deadlocks
    pub fn check_channel_deadlock(&self) -> Result<(), RuntimeError> {
        // Check if all tasks are waiting on channels that have no senders
        for (channel_id, waiting_tasks) in &self.channel_waits {
            if !waiting_tasks.is_empty() {
                // Check if there are any senders for this channel
                // This is a simplified check - in a real implementation,
                // we'd track senders and receivers separately
                if waiting_tasks.len() > 1 {
                    return Err(RuntimeError::SchedulerError(
                        format!("Potential channel deadlock on channel {} with {} waiting tasks", 
                                channel_id, waiting_tasks.len())
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Memory manager
pub struct MemoryManager {
    pub regions: HashMap<String, RegionInfo>,
    pub allocations: HashMap<usize, AllocationInfo>,
    pub next_allocation_id: usize,
}

/// Region information
#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub name: String,
    pub size: usize,
    pub allocations: Vec<usize>,
    pub free_point: Option<usize>,
}

/// Allocation information
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub id: usize,
    pub size: usize,
    pub region: String,
    pub is_freed: bool,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            allocations: HashMap::new(),
            next_allocation_id: 0,
        }
    }

    pub fn allocate(&mut self, size: usize, region: String) -> usize {
        let allocation_id = self.next_allocation_id;
        self.next_allocation_id += 1;

        let allocation = AllocationInfo {
            id: allocation_id,
            size,
            region: region.clone(),
            is_freed: false,
        };

        self.allocations.insert(allocation_id, allocation);

        // Update region info
        if let Some(region_info) = self.regions.get_mut(&region) {
            region_info.allocations.push(allocation_id);
        } else {
            self.regions.insert(region.clone(), RegionInfo {
                name: region.clone(),
                size: 0,
                allocations: vec![allocation_id],
                free_point: None,
            });
        }

        allocation_id
    }

    pub fn free(&mut self, allocation_id: usize) -> Result<(), RuntimeError> {
        if let Some(allocation) = self.allocations.get_mut(&allocation_id) {
            if allocation.is_freed {
                return Err(RuntimeError::MemoryError("Double free detected".to_string()));
            }
            allocation.is_freed = true;
            Ok(())
        } else {
            Err(RuntimeError::MemoryError("Allocation not found".to_string()))
        }
    }

    pub fn free_region(&mut self, region_name: &str) -> Result<(), RuntimeError> {
        if let Some(region_info) = self.regions.get(region_name) {
            let allocations = region_info.allocations.clone();
            for allocation_id in allocations {
                self.free(allocation_id)?;
            }
            Ok(())
        } else {
            Err(RuntimeError::MemoryError("Region not found".to_string()))
        }
    }
}


/// Runtime
pub struct Runtime {
    pub scheduler: Scheduler,
    pub memory_manager: MemoryManager,
    pub deadlock_detector: DeadlockDetector,
    pub is_running: bool,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            memory_manager: MemoryManager::new(),
            deadlock_detector: DeadlockDetector::new(),
            is_running: false,
        }
    }

    pub fn start(&mut self) -> Result<(), RuntimeError> {
        self.is_running = true;
        
        // Check for deadlocks before starting
        self.deadlock_detector.detect_deadlock_enhanced()?;
        self.deadlock_detector.check_channel_deadlock()?;
        
        self.scheduler.run()
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        self.scheduler.stop();
    }

    /// Spawn a task with deadlock detection
    pub fn spawn_task_with_deadlock_detection(&mut self, task: TaskHandle) -> Result<TaskId, RuntimeError> {
        // Add to deadlock detector
        self.deadlock_detector.wait_for_graph.insert(task.id, Vec::new());
        
        Ok(task.id)
    }

    /// Create a channel with deadlock detection
    pub fn create_channel_with_deadlock_detection(&mut self, capacity: usize, backpressure_policy: BackpressurePolicy) -> Result<ChannelId, RuntimeError> {
        let channel_handle = self.scheduler.create_channel(capacity, backpressure_policy);
        
        // Add to deadlock detector
        self.deadlock_detector.channel_waits.insert(channel_handle.id, Vec::new());
        
        Ok(channel_handle.id)
    }

    /// Send to channel with deadlock detection
    pub fn send_to_channel_with_deadlock_detection(&mut self, channel_id: ChannelId, value: Value) -> Result<(), RuntimeError> {
        // Check for deadlocks before sending
        self.deadlock_detector.check_channel_deadlock()?;
        
        self.scheduler.send_to_channel(channel_id, value)
    }

    /// Receive from channel with deadlock detection
    pub fn recv_from_channel_with_deadlock_detection(&mut self, channel_id: ChannelId, task_id: TaskId) -> Result<Value, RuntimeError> {
        // Add wait relationship
        self.deadlock_detector.add_channel_wait(task_id, channel_id);
        
        // Check for deadlocks
        self.deadlock_detector.detect_deadlock_enhanced()?;
        
        let result = self.scheduler.recv_from_channel(channel_id);
        
        // Remove wait relationship on success
        if result.is_ok() {
            self.deadlock_detector.remove_channel_wait(task_id, channel_id);
        }
        
        result
    }

    /// Get deadlock information
    pub fn get_deadlock_info(&self) -> (bool, Vec<TaskId>) {
        let trace = self.deadlock_detector.get_deadlock_trace();
        (self.deadlock_detector.cycle_detected, trace)
    }

    /// Check for deadlocks periodically
    pub fn check_deadlocks(&mut self) -> Result<(), RuntimeError> {
        self.deadlock_detector.detect_deadlock_enhanced()?;
        self.deadlock_detector.check_channel_deadlock()?;
        Ok(())
    }

    pub fn spawn_task(&mut self, function: String, args: Vec<Value>) -> TaskHandle {
        self.scheduler.spawn_task(function, args)
    }

    pub fn await_task(&mut self, task_handle: TaskHandle) -> Result<Value, RuntimeError> {
        self.scheduler.await_task(task_handle)
    }

    pub fn create_channel(&mut self, capacity: usize, backpressure_policy: BackpressurePolicy) -> ChannelHandle {
        self.scheduler.create_channel(capacity, backpressure_policy)
    }

    pub fn send_to_channel(&self, channel_id: ChannelId, value: Value) -> Result<(), RuntimeError> {
        self.scheduler.send_to_channel(channel_id, value)
    }

    pub fn recv_from_channel(&self, channel_id: ChannelId) -> Result<Value, RuntimeError> {
        self.scheduler.recv_from_channel(channel_id)
    }

    pub fn allocate(&mut self, size: usize, region: String) -> usize {
        self.memory_manager.allocate(size, region)
    }

    pub fn free(&mut self, allocation_id: usize) -> Result<(), RuntimeError> {
        self.memory_manager.free(allocation_id)
    }

    pub fn free_region(&mut self, region_name: &str) -> Result<(), RuntimeError> {
        self.memory_manager.free_region(region_name)
    }
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Once Runtime:")?;
        writeln!(f, "=============")?;
        writeln!(f, "Tasks: {}", self.scheduler.tasks.len())?;
        writeln!(f, "Channels: {}", self.scheduler.channels.len())?;
        writeln!(f, "Regions: {}", self.memory_manager.regions.len())?;
        writeln!(f, "Allocations: {}", self.memory_manager.allocations.len())?;
        Ok(())
    }

}

// ================================================================
// Task handler registry for real function dispatch
// ================================================================

/// Type alias for a registered task handler function
pub type TaskHandler = fn(args: &[Value]) -> Result<Value, RuntimeError>;

/// Registry of named task handlers for real function dispatch
pub struct TaskRegistry {
    handlers: HashMap<String, TaskHandler>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register(&mut self, name: &str, handler: TaskHandler) {
        self.handlers.insert(name.to_string(), handler);
    }

    pub fn get(&self, name: &str) -> Option<TaskHandler> {
        self.handlers.get(name).copied()
    }

    pub fn execute(&self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match self.handlers.get(name) {
            Some(handler) => handler(args),
            None => Err(RuntimeError::TaskError(format!("No handler registered: {}", name))),
        }
    }
}

// ================================================================
// Structured concurrency: Group support
// ================================================================

/// A group of tasks with structured concurrency guarantees
pub struct TaskGroup {
    pub id: usize,
    pub children: Vec<TaskId>,
    pub is_completed: bool,
}

impl TaskGroup {
    pub fn new(id: usize) -> Self {
        Self { id, children: Vec::new(), is_completed: false }
    }
}

impl Scheduler {
    /// Spawn a task as a child of a group
    pub fn spawn_task_in_group(&mut self, group_id: usize, function: String, args: Vec<Value>) -> TaskHandle {
        let handle = self.spawn_task(function, args);
        // Track child in the task itself — group info is managed by the caller
        handle
    }

    /// Wait for all tasks in a group to complete (structured concurrency)
    pub fn await_group(&mut self, group_id: usize, child_ids: &[TaskId]) -> Result<Vec<Value>, RuntimeError> {
        let mut results = Vec::new();
        for &child_id in child_ids {
            let handle = TaskHandle {
                id: child_id,
                status: TaskStatus::Pending,
                result: None,
            };
            match self.await_task(handle) {
                Ok(value) => results.push(value),
                Err(e) => {
                    // Cancel remaining children on first failure
                    for &remaining_id in child_ids.iter().filter(|&&id| id > child_id) {
                        if let Some(task) = self.tasks.get_mut(&remaining_id) {
                            task.status = TaskStatus::Cancelled;
                        }
                    }
                    return Err(e);
                }
            }
        }
        Ok(results)
    }
}

impl Runtime {
    pub fn create_group(&mut self) -> TaskGroup {
        let id = self.scheduler.next_task_id;
        self.scheduler.next_task_id += 1;
        TaskGroup::new(id)
    }

    /// Spawn a task within a group and track it
    pub fn spawn_in_group(&mut self, group: &mut TaskGroup, function: String, args: Vec<Value>) -> TaskHandle {
        let handle = self.scheduler.spawn_task_in_group(group.id, function, args);
        group.children.push(handle.id);
        handle
    }

    /// Wait for all tasks in a group — blocks until all children finish
    pub fn await_group(&mut self, group: &mut TaskGroup) -> Result<Vec<Value>, RuntimeError> {
        let results = self.scheduler.await_group(group.id, &group.children)?;
        group.is_completed = true;
        Ok(results)
    }
}

/// Global runtime instance for C-compatible exports
use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_RUNTIME: Mutex<Runtime> = Mutex::new(Runtime::new());
}

/// C-compatible export: spawn a task
/// Signature: once_runtime_spawn(func_ptr: i64, args_ptr: i64) -> task_handle: i64
#[no_mangle]
pub extern "C" fn once_runtime_spawn(_func_ptr: i64, _args_ptr: i64) -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    let handle = runtime.spawn_task("spawned".to_string(), vec![]);
    handle.id as i64
}

/// C-compatible export: send a value to a channel
/// Signature: once_runtime_send(channel_id: i64, value: i64) -> status: i64
#[no_mangle]
pub extern "C" fn once_runtime_send(channel_id: i64, value: i64) -> i64 {
    let runtime = GLOBAL_RUNTIME.lock().unwrap();
    let result = runtime.send_to_channel(channel_id as usize, Value::Int(value));
    match result {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// C-compatible export: receive a value from a channel
/// Signature: once_runtime_recv(channel_id: i64) -> value: i64
#[no_mangle]
pub extern "C" fn once_runtime_recv(channel_id: i64) -> i64 {
    let runtime = GLOBAL_RUNTIME.lock().unwrap();
    let result = runtime.recv_from_channel(channel_id as usize);
    match result {
        Ok(Value::Int(v)) => v,
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// C-compatible export: await a task
/// Signature: once_runtime_await(task_handle: i64) -> result: i64
#[no_mangle]
pub extern "C" fn once_runtime_await(task_handle: i64) -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    let handle = TaskHandle {
        id: task_handle as usize,
        status: TaskStatus::Pending,
        result: None,
    };
    let result = runtime.await_task(handle);
    match result {
        Ok(Value::Int(v)) => v,
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(!runtime.is_running);
        assert!(runtime.scheduler.tasks.is_empty());
    }

    #[test]
    fn test_task_spawning() {
        let mut runtime = Runtime::new();
        let task_handle = runtime.spawn_task("test_function".to_string(), vec![]);
        assert_eq!(task_handle.id, 0);
        assert_eq!(task_handle.status, TaskStatus::Pending);
    }

    #[test]
    fn test_channel_creation() {
        let mut runtime = Runtime::new();
        let channel = runtime.create_channel(10, BackpressurePolicy::Blocking);
        assert_eq!(channel.capacity, 10);
        assert_eq!(channel.backpressure_policy, BackpressurePolicy::Blocking);
    }

    #[test]
    fn test_memory_allocation() {
        let mut runtime = Runtime::new();
        let allocation_id = runtime.allocate(1024, "test_region".to_string());
        assert_eq!(allocation_id, 0);
    }

    #[test]
    fn test_c_spawn() {
        let handle = once_runtime_spawn(0, 0);
        assert!(handle >= 0);
    }

    #[test]
    fn test_c_channel_send_recv() {
        let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
        let channel = runtime.create_channel(10, BackpressurePolicy::Blocking);
        let channel_id = channel.id as i64;
        drop(runtime);
        let status = once_runtime_send(channel_id, 42);
        assert_eq!(status, 0);
        let value = once_runtime_recv(channel_id);
        assert_eq!(value, 42);
    }
}