use serde;

thread_local! {
    pub(crate) static CURRENT_TASK_ID: std::cell::Cell<Option<TaskId>> = const { std::cell::Cell::new(None) };
}

pub fn set_current_task(task_id: TaskId) {
    CURRENT_TASK_ID.with(|cell| cell.set(Some(task_id)));
}

pub fn clear_current_task() {
    CURRENT_TASK_ID.with(|cell| cell.set(None));
}

pub fn current_task_id() -> Option<TaskId> {
    CURRENT_TASK_ID.with(|cell| cell.get())
}

#[derive(Debug, Clone, thiserror::Error)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskHandle {
    pub id: TaskId,
    pub status: TaskStatus,
    pub result: Option<Box<Value>>,
}

pub type TaskId = usize;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelHandle {
    pub id: ChannelId,
    pub capacity: usize,
    pub backpressure_policy: BackpressurePolicy,
}

pub type ChannelId = usize;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackpressurePolicy {
    Blocking,
    Dropping,
    Erroring,
}
