use crate::value::TaskId;
use std::sync::{Arc, Condvar, Mutex};

pub struct TaskGroup {
    pub id: usize,
    pub children: Vec<TaskId>,
    pub is_completed: bool,
    pub completion_condvar: Arc<Condvar>,
    pub completion_mutex: Arc<Mutex<bool>>,
    pub completed_count: Arc<Mutex<usize>>,
}

impl TaskGroup {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            children: Vec::new(),
            is_completed: false,
            completion_condvar: Arc::new(Condvar::new()),
            completion_mutex: Arc::new(Mutex::new(false)),
            completed_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn notify_child_complete(&self) {
        let count = {
            let mut cnt = self.completed_count.lock().unwrap();
            *cnt += 1;
            *cnt
        };
        {
            let mut done = self.completion_mutex.lock().unwrap();
            *done = count >= self.children.len();
        }
        self.completion_condvar.notify_all();
    }
}
