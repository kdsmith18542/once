use crate::value::{RuntimeError, TaskId, TaskStatus, Value};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone)]
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

pub type TaskHandler = fn(args: &[Value]) -> Result<Value, RuntimeError>;

pub struct TaskRegistry {
    handlers: HashMap<String, TaskHandler>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
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
            None => Err(RuntimeError::TaskError(format!(
                "No handler registered: {}",
                name
            ))),
        }
    }
}
