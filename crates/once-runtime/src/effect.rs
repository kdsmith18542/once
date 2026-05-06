use crate::task::TaskHandler;
use crate::value::{RuntimeError, Value};
use std::collections::HashMap;

pub struct EffectRegistry {
    overrides: HashMap<String, TaskHandler>,
}

impl EffectRegistry {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    pub fn register_override(&mut self, effect: &str, handler: TaskHandler) {
        self.overrides.insert(effect.to_string(), handler);
    }

    pub fn get_override(&self, effect: &str) -> Option<TaskHandler> {
        self.overrides.get(effect).copied()
    }

    pub fn try_dispatch(
        &self,
        effect: &str,
        args: &[Value],
    ) -> Option<Result<Value, RuntimeError>> {
        self.overrides.get(effect).map(|handler| handler(args))
    }

    pub fn clear(&mut self) {
        self.overrides.clear();
    }
}
