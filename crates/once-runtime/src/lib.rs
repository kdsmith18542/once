pub mod channel;
pub mod deadlock;
pub mod effect;
pub mod group;
pub mod memory;
pub mod scheduler;
pub mod task;
pub mod value;
pub mod worker;

use crate::channel::Channel;
use crate::effect::EffectRegistry;
use crate::group::TaskGroup;
use crate::scheduler::Scheduler;
use crate::value::current_task_id;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

pub use value::{
    clear_current_task, set_current_task, BackpressurePolicy, ChannelHandle, ChannelId,
    RuntimeError, TaskHandle, TaskId, TaskStatus, Value,
};
pub use task::Task;
pub use scheduler::WorkerStats;
pub use deadlock::DeadlockDetector;
pub use memory::MemoryManager;

pub struct Runtime {
    pub scheduler: Scheduler,
    pub memory_manager: MemoryManager,
    pub is_running: bool,
    pub groups: HashMap<usize, Arc<Mutex<TaskGroup>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            memory_manager: MemoryManager::new(),
            is_running: false,
            groups: HashMap::new(),
        }
    }

    pub fn start(&self) -> Result<(), RuntimeError> {
        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner.is_running = true;
        }

        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner.deadlock_detector.detect_deadlock_enhanced()?;
            inner.deadlock_detector.check_channel_deadlock()?;
        }

        self.scheduler.run()
    }

    pub fn set_deterministic(&self, deterministic: bool) {
        self.scheduler.set_deterministic(deterministic);
    }

    pub fn stop(&self) {
        self.scheduler.inner.lock().unwrap().is_running = false;
    }

    pub fn spawn_task_with_deadlock_detection(
        &mut self,
        task: TaskHandle,
    ) -> Result<TaskId, RuntimeError> {
        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner
                .deadlock_detector
                .wait_for_graph
                .insert(task.id, Vec::new());
        }
        Ok(task.id)
    }

    pub fn create_channel_with_deadlock_detection(
        &mut self,
        capacity: usize,
        backpressure_policy: BackpressurePolicy,
    ) -> Result<ChannelId, RuntimeError> {
        let channel_handle = self.scheduler.create_channel(capacity, backpressure_policy);

        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner
                .deadlock_detector
                .channel_waits
                .insert(channel_handle.id, Vec::new());
        }

        self.scheduler
            .inner
            .lock()
            .unwrap()
            .deadlock_detector
            .check_channel_deadlock()?;

        Ok(channel_handle.id)
    }

    pub fn send_to_channel_with_deadlock_detection(
        &mut self,
        channel_id: ChannelId,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let task_id = current_task_id().unwrap_or(0);
        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner
                .deadlock_detector
                .add_channel_wait(task_id, channel_id);
            inner.deadlock_detector.detect_deadlock_enhanced()?;
        }

        let result = self.scheduler.send_to_channel(channel_id, value);

        {
            let mut inner = self.scheduler.inner.lock().unwrap();
            inner
                .deadlock_detector
                .remove_channel_wait(task_id, channel_id);
        }

        result
    }

    pub fn recv_from_channel_with_deadlock_detection(
        &mut self,
        channel_id: ChannelId,
    ) -> Result<Value, RuntimeError> {
        {
            let inner = self.scheduler.inner.lock().unwrap();
            let trace = inner.deadlock_detector.get_deadlock_trace();
            let (cycle_detected, _trace) = (inner.deadlock_detector.cycle_detected, trace);
            drop(inner);
            let _ = cycle_detected;
        }

        self.scheduler.recv_from_channel(channel_id)
    }

    pub fn get_deadlock_trace(&self) -> (bool, Vec<TaskId>) {
        let inner = self.scheduler.inner.lock().unwrap();
        let trace = inner.deadlock_detector.get_deadlock_trace();
        (inner.deadlock_detector.cycle_detected, trace)
    }

    pub fn check_deadlocks(&self) -> Result<(), RuntimeError> {
        let mut inner = self.scheduler.inner.lock().unwrap();
        inner.deadlock_detector.detect_deadlock_enhanced()?;
        inner.deadlock_detector.check_channel_deadlock()?;
        Ok(())
    }

    pub fn spawn_task(&mut self, function: String, args: Vec<Value>) -> TaskHandle {
        self.scheduler.spawn_task(function, args)
    }

    pub fn await_task(&mut self, task_handle: TaskHandle) -> Result<Value, RuntimeError> {
        self.scheduler.await_task(task_handle)
    }

    pub fn create_channel(
        &mut self,
        capacity: usize,
        backpressure_policy: BackpressurePolicy,
    ) -> ChannelHandle {
        self.scheduler.create_channel(capacity, backpressure_policy)
    }

    pub fn send_to_channel(
        &self,
        channel_id: ChannelId,
        value: Value,
    ) -> Result<(), RuntimeError> {
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

    pub fn create_group(&mut self) -> usize {
        let mut inner = self.scheduler.inner.lock().unwrap();
        let id = inner.next_task_id;
        inner.next_task_id += 1;
        let group = Arc::new(Mutex::new(TaskGroup::new(id)));
        self.scheduler
            .register_group_membership(id, Arc::clone(&group));
        self.groups.insert(id, group);
        id
    }

    pub fn spawn_in_group(
        &mut self,
        group_id: usize,
        function: String,
        args: Vec<Value>,
    ) -> TaskHandle {
        let handle = self.scheduler.spawn_task(function, args);
        if let Some(group) = self.groups.get(&group_id) {
            if let Ok(mut group_guard) = group.lock() {
                group_guard.children.push(handle.id);
            }
            self.scheduler.register_group_child(group_id, handle.id);
        }
        handle
    }

    pub fn spawn_in_group_named(
        &mut self,
        group_id: usize,
        function: String,
        args: Vec<Value>,
    ) -> TaskHandle {
        self.spawn_in_group(group_id, function, args)
    }

    pub fn await_group(&mut self, group_id: usize) -> Result<Vec<Value>, RuntimeError> {
        let group = self
            .groups
            .get(&group_id)
            .cloned()
            .ok_or_else(|| RuntimeError::TaskError("Group not found".to_string()))?;
        let children = {
            let group_guard = group.lock().unwrap();
            group_guard.children.clone()
        };
        let results = self.scheduler.await_group(Arc::clone(&group), &children)?;
        {
            let mut group_guard = group.lock().unwrap();
            group_guard.is_completed = true;
        }
        Ok(results)
    }

    pub fn spawn_actor(&mut self, name: &str, handler: crate::task::TaskHandler) -> TaskHandle {
        self.scheduler.spawn_actor(name, handler)
    }

    pub fn spawn_actor_with_init(
        &mut self,
        name: &str,
        handler: crate::task::TaskHandler,
        init_args: Vec<Value>,
    ) -> TaskHandle {
        self.scheduler.spawn_actor_with_init(name, handler, init_args)
    }

    pub fn execute_with_override(
        &mut self,
        effect: &str,
        handler_name: &str,
        args: &[Value],
        overrides: &EffectRegistry,
    ) -> Result<Value, RuntimeError> {
        if let Some(result) = overrides.try_dispatch(effect, args) {
            return result;
        }
        self.scheduler
            .registry
            .lock()
            .unwrap()
            .execute(handler_name, args)
    }
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.scheduler.inner.lock().unwrap();
        writeln!(f, "Once Runtime:")?;
        writeln!(f, "=============")?;
        writeln!(f, "Tasks: {}", inner.tasks.len())?;
        writeln!(f, "Channels: {}", inner.channels.len())?;
        writeln!(f, "Regions: {}", self.memory_manager.regions.len())?;
        writeln!(f, "Allocations: {}", self.memory_manager.allocations.len())?;
        Ok(())
    }
}

// ================================================================
// Global runtime instance and C-compatible FFI exports
// ================================================================

use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_RUNTIME: Mutex<Runtime> = Mutex::new(Runtime::new());
}



#[no_mangle]
pub extern "C" fn once_runtime_spawn(_func_ptr: i64, _args_ptr: i64) -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    let handle = runtime.spawn_task("spawned".to_string(), vec![]);
    handle.id as i64
}

#[no_mangle]
pub extern "C" fn once_runtime_send(channel_id: i64, value: i64) -> i64 {
    let runtime = GLOBAL_RUNTIME.lock().unwrap();
    let result = runtime.send_to_channel(channel_id as usize, Value::Int(value));
    match result {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

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

#[no_mangle]
pub extern "C" fn once_runtime_create_group() -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    runtime.create_group() as i64
}

#[no_mangle]
pub extern "C" fn once_runtime_spawn_in_group(
    group_id: i64,
    _func_ptr: i64,
    _args_ptr: i64,
) -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    let handle =
        runtime.spawn_in_group(group_id as usize, "spawned".to_string(), vec![]);
    handle.id as i64
}

#[no_mangle]
pub extern "C" fn once_runtime_await_group(group_id: i64) -> i64 {
    let mut runtime = GLOBAL_RUNTIME.lock().unwrap();
    match runtime.await_group(group_id as usize) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn once_runtime_capture_error_context(result_val: i64) -> i64 {
    let current_task = current_task_id().unwrap_or(0);
    let msg = format!("try block in task {} at unknown location", current_task);
    push_error_context(current_task as u64, msg);
    result_val
}

#[no_mangle]
pub extern "C" fn once_runtime_load_length(collection: i64) -> i64 {
    // Load the length field from the collection header.
    // The length is stored at offset 8 in the collection struct (after the vtable pointer).
    if collection == 0 {
        return 0;
    }
    unsafe {
        let len_ptr = (collection as *const i64).add(1);
        *len_ptr
    }
}

thread_local! {
    static ERROR_CONTEXT: std::cell::RefCell<Vec<(u64, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn push_error_context(id: u64, message: String) {
    ERROR_CONTEXT.with(|ctx| ctx.borrow_mut().push((id, message)));
}

pub fn pop_error_context() -> Option<(u64, String)> {
    ERROR_CONTEXT.with(|ctx| ctx.borrow_mut().pop())
}

pub fn current_error_context() -> Vec<(u64, String)> {
    ERROR_CONTEXT.with(|ctx| ctx.borrow().clone())
}

// ================================================================
// FFI Bridge: std::io and std::net ABI exports
// ================================================================

#[no_mangle]
pub extern "C" fn once_io_alloc(size: i64) -> i64 {
    if size <= 0 {
        return 0;
    }
    unsafe {
        let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
        std::alloc::alloc(layout) as i64
    }
}

#[no_mangle]
pub extern "C" fn once_io_free(ptr: i64, size: i64) {
    if ptr == 0 || size <= 0 {
        return;
    }
    unsafe {
        let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
        std::alloc::dealloc(ptr as *mut u8, layout);
    }
}

#[no_mangle]
pub extern "C" fn once_io_print(ptr: i64, len: i64) -> i64 {
    if ptr == 0 || len <= 0 {
        return -1;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        let s = std::str::from_utf8(slice).unwrap_or("<invalid utf8>");
        print!("{}", s);
        0
    }
}

#[no_mangle]
pub extern "C" fn once_io_read_line(buf_ptr: i64, buf_cap: i64) -> i64 {
    if buf_ptr == 0 || buf_cap <= 0 {
        return -1;
    }
    unsafe {
        let buf = std::slice::from_raw_parts_mut(buf_ptr as *mut u8, buf_cap as usize);
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(n) => {
                let bytes = line.as_bytes();
                let copy_len = bytes.len().min(buf.len());
                buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
                n as i64
            }
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_io_file_open(path_ptr: i64, path_len: i64, mode: i64) -> i64 {
    if path_ptr == 0 || path_len <= 0 {
        return -1;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(path_ptr as *const u8, path_len as usize);
        let path = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let file = match mode {
            0 => std::fs::File::open(path),
            1 => std::fs::File::create(path),
            2 => std::fs::OpenOptions::new().append(true).open(path),
            _ => return -1,
        };
        match file {
            Ok(f) => Box::into_raw(Box::new(f)) as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_io_file_read(fd: i64, buf_ptr: i64, buf_len: i64) -> i64 {
    if fd == 0 || buf_ptr == 0 || buf_len <= 0 {
        return -1;
    }
    unsafe {
        let file = &mut *(fd as *mut std::fs::File);
        let buf = std::slice::from_raw_parts_mut(buf_ptr as *mut u8, buf_len as usize);
        match std::io::Read::read(file, buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_io_file_write(fd: i64, buf_ptr: i64, buf_len: i64) -> i64 {
    if fd == 0 || buf_ptr == 0 || buf_len <= 0 {
        return -1;
    }
    unsafe {
        let file = &mut *(fd as *mut std::fs::File);
        let buf = std::slice::from_raw_parts(buf_ptr as *const u8, buf_len as usize);
        match std::io::Write::write(file, buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_io_file_close(fd: i64) -> i64 {
    if fd == 0 {
        return -1;
    }
    unsafe {
        let _file = Box::from_raw(fd as *mut std::fs::File);
        0
    }
}

#[no_mangle]
pub extern "C" fn once_net_connect(host_ptr: i64, host_len: i64, port: i64) -> i64 {
    if host_ptr == 0 || host_len <= 0 || port <= 0 {
        return -1;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(host_ptr as *const u8, host_len as usize);
        let host = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let addr = format!("{}:{}", host, port);
        match std::net::TcpStream::connect(&addr) {
            Ok(stream) => Box::into_raw(Box::new(stream)) as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_net_listen(host_ptr: i64, host_len: i64, port: i64) -> i64 {
    if host_ptr == 0 || host_len <= 0 || port <= 0 {
        return -1;
    }
    unsafe {
        let slice = std::slice::from_raw_parts(host_ptr as *const u8, host_len as usize);
        let host = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let addr = format!("{}:{}", host, port);
        match std::net::TcpListener::bind(&addr) {
            Ok(listener) => Box::into_raw(Box::new(listener)) as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_net_accept(listener_fd: i64) -> i64 {
    if listener_fd == 0 {
        return -1;
    }
    unsafe {
        let listener = &*(listener_fd as *mut std::net::TcpListener);
        match listener.accept() {
            Ok((stream, _addr)) => Box::into_raw(Box::new(stream)) as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_net_read(stream_fd: i64, buf_ptr: i64, buf_len: i64) -> i64 {
    if stream_fd == 0 || buf_ptr == 0 || buf_len <= 0 {
        return -1;
    }
    unsafe {
        let stream = &mut *(stream_fd as *mut std::net::TcpStream);
        let buf = std::slice::from_raw_parts_mut(buf_ptr as *mut u8, buf_len as usize);
        match std::io::Read::read(stream, buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_net_write(stream_fd: i64, buf_ptr: i64, buf_len: i64) -> i64 {
    if stream_fd == 0 || buf_ptr == 0 || buf_len <= 0 {
        return -1;
    }
    unsafe {
        let stream = &mut *(stream_fd as *mut std::net::TcpStream);
        let buf = std::slice::from_raw_parts(buf_ptr as *const u8, buf_len as usize);
        match std::io::Write::write(stream, buf) {
            Ok(n) => n as i64,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn once_net_close(fd: i64) -> i64 {
    if fd == 0 {
        return -1;
    }
    unsafe {
        let _stream = Box::from_raw(fd as *mut std::net::TcpStream);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ok_handler(_args: &[Value]) -> Result<Value, RuntimeError> {
        Ok(Value::Int(1))
    }

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(!runtime.is_running);
        let inner = runtime.scheduler.inner.lock().unwrap();
        assert!(inner.tasks.is_empty());
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

    #[test]
    fn test_group_child_completion_notifies_on_task_finish() {
        let mut runtime = Runtime::new();
        runtime
            .scheduler
            .register_handler("test_ok_handler", test_ok_handler);

        let group_id = runtime.create_group();
        let task_handle = runtime.spawn_in_group(
            group_id,
            "test_ok_handler".to_string(),
            vec![],
        );

        let ran = crate::worker::execute_task(
            task_handle.id,
            false,
            0,
            &runtime.scheduler.inner,
            &runtime.scheduler.registry,
        );
        assert!(ran);

        let group = runtime.groups.get(&group_id).unwrap();
        let (completed_count, is_done) = {
            let group_guard = group.lock().unwrap();
            let count = *group_guard.completed_count.lock().unwrap();
            let done = *group_guard.completion_mutex.lock().unwrap();
            (count, done)
        };
        assert_eq!(completed_count, 1);
        assert!(is_done);
    }
}
