use crate::task::Task;
use crate::value::{ChannelId, RuntimeError, TaskId, TaskStatus};
use std::collections::{HashMap, HashSet};

pub struct DeadlockDetector {
    pub wait_for_graph: HashMap<TaskId, Vec<TaskId>>,
    pub channel_waits: HashMap<ChannelId, Vec<TaskId>>,
    pub task_waits: HashMap<TaskId, Vec<ChannelId>>,
    pub channel_senders: HashMap<ChannelId, Vec<TaskId>>,
    pub cycle_detected: bool,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            wait_for_graph: HashMap::new(),
            channel_waits: HashMap::new(),
            task_waits: HashMap::new(),
            channel_senders: HashMap::new(),
            cycle_detected: false,
        }
    }

    pub fn register_sender(&mut self, task_id: TaskId, channel_id: ChannelId) {
        self.channel_senders
            .entry(channel_id)
            .or_insert_with(Vec::new)
            .push(task_id);
    }

    pub fn deregister_sender(&mut self, task_id: TaskId, channel_id: ChannelId) {
        if let Some(senders) = self.channel_senders.get_mut(&channel_id) {
            senders.retain(|&id| id != task_id);
        }
    }

    pub fn detect_deadlock(&self, tasks: &HashMap<TaskId, Task>) -> Result<(), RuntimeError> {
        let graph = &self.wait_for_graph;

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in graph.keys() {
            if !visited.contains(task_id) {
                if self.dfs_has_cycle(*task_id, graph, &mut visited, &mut rec_stack) {
                    return Err(RuntimeError::DeadlockError(format!(
                        "Deadlock detected involving task {}",
                        task_id
                    )));
                }
            }
        }

        for (channel_id, waiting_tasks) in &self.channel_waits {
            if !waiting_tasks.is_empty() {
                let has_active_sender = self
                    .channel_senders
                    .get(channel_id)
                    .map(|senders| {
                        senders.iter().any(|sid| {
                            tasks
                                .get(sid)
                                .map(|t| t.status == TaskStatus::Running)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);

                if !has_active_sender {
                    return Err(RuntimeError::DeadlockError(format!(
                        "Channel deadlock: {} task(s) waiting on channel {} with no active senders",
                        waiting_tasks.len(),
                        channel_id
                    )));
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

    pub fn add_wait(&mut self, task_id: TaskId, waiting_for: TaskId) {
        self.wait_for_graph
            .entry(task_id)
            .or_insert_with(Vec::new)
            .push(waiting_for);
    }

    pub fn add_channel_wait(&mut self, task_id: TaskId, channel_id: ChannelId) {
        self.channel_waits
            .entry(channel_id)
            .or_insert_with(Vec::new)
            .push(task_id);
        self.task_waits
            .entry(task_id)
            .or_insert_with(Vec::new)
            .push(channel_id);
    }

    pub fn remove_wait(&mut self, task_id: TaskId, waiting_for: TaskId) {
        if let Some(waits) = self.wait_for_graph.get_mut(&task_id) {
            waits.retain(|&id| id != waiting_for);
        }
    }

    pub fn remove_channel_wait(&mut self, task_id: TaskId, channel_id: ChannelId) {
        if let Some(tasks) = self.channel_waits.get_mut(&channel_id) {
            tasks.retain(|&id| id != task_id);
        }
        if let Some(channels) = self.task_waits.get_mut(&task_id) {
            channels.retain(|&id| id != channel_id);
        }
    }

    pub fn detect_deadlock_enhanced(&mut self) -> Result<(), RuntimeError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &task_id in self.wait_for_graph.keys() {
            if !visited.contains(&task_id) {
                if self.dfs_cycle_detection(task_id, &mut visited, &mut rec_stack) {
                    self.cycle_detected = true;
                    return Err(RuntimeError::SchedulerError(format!(
                        "Deadlock detected! Cycle found involving task {}",
                        task_id
                    )));
                }
            }
        }

        Ok(())
    }

    fn dfs_cycle_detection(
        &self,
        task_id: TaskId,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
    ) -> bool {
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

    pub fn check_channel_deadlock(&self) -> Result<(), RuntimeError> {
        for (channel_id, waiting_tasks) in &self.channel_waits {
            if !waiting_tasks.is_empty() {
                let active_sender_count = self
                    .channel_senders
                    .get(channel_id)
                    .map(|s| s.len())
                    .unwrap_or(0);

                if active_sender_count == 0 {
                    return Err(RuntimeError::SchedulerError(format!(
                        "Channel deadlock on channel {} with {} waiting tasks and no senders",
                        channel_id,
                        waiting_tasks.len()
                    )));
                }
            }
        }

        Ok(())
    }
}
