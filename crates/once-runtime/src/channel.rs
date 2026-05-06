use crate::value::{BackpressurePolicy, ChannelId, RuntimeError};
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

pub struct Channel<T> {
    pub id: ChannelId,
    pub capacity: usize,
    pub buffer: Mutex<VecDeque<T>>,
    pub senders: usize,
    pub receivers: usize,
    pub backpressure_policy: BackpressurePolicy,
    pub condvar: Condvar,
    pub is_closed: Mutex<bool>,
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
            is_closed: Mutex::new(false),
        }
    }

    pub fn send(&self, value: T) -> Result<(), RuntimeError> {
        if *self.is_closed.lock().unwrap() {
            return Err(RuntimeError::ChannelError("Channel is closed".to_string()));
        }

        match self.backpressure_policy {
            BackpressurePolicy::Blocking => {
                let mut buffer = self.buffer.lock().unwrap();
                while buffer.len() >= self.capacity {
                    drop(buffer);
                    if *self.is_closed.lock().unwrap() {
                        return Err(RuntimeError::ChannelError(
                            "Channel closed while sending".to_string(),
                        ));
                    }
                    buffer = self.buffer.lock().unwrap();
                    if buffer.len() >= self.capacity {
                        buffer = self.condvar.wait(buffer).unwrap();
                    } else {
                        break;
                    }
                }
                buffer.push_back(value);
                self.condvar.notify_all();
                Ok(())
            }
            BackpressurePolicy::Dropping => {
                let mut buffer = self.buffer.lock().unwrap();
                if buffer.len() >= self.capacity {
                    buffer.pop_front();
                }
                buffer.push_back(value);
                self.condvar.notify_all();
                Ok(())
            }
            BackpressurePolicy::Erroring => {
                let mut buffer = self.buffer.lock().unwrap();
                if buffer.len() >= self.capacity {
                    Err(RuntimeError::BackpressureError("Channel full".to_string()))
                } else {
                    buffer.push_back(value);
                    self.condvar.notify_all();
                    Ok(())
                }
            }
        }
    }

    pub fn recv(&self) -> Result<T, RuntimeError> {
        let mut buffer = self.buffer.lock().unwrap();

        while buffer.is_empty() {
            drop(buffer);
            if *self.is_closed.lock().unwrap() {
                return Err(RuntimeError::ChannelError("Channel closed".to_string()));
            }
            buffer = self.buffer.lock().unwrap();
            if buffer.is_empty() {
                buffer = self.condvar.wait(buffer).unwrap();
            } else {
                break;
            }
        }

        match buffer.pop_front() {
            Some(value) => Ok(value),
            None => Err(RuntimeError::ChannelError("Channel closed".to_string())),
        }
    }

    pub fn try_recv(&self) -> Result<Option<T>, RuntimeError> {
        if *self.is_closed.lock().unwrap() {
            return Err(RuntimeError::ChannelError("Channel is closed".to_string()));
        }
        let mut buffer = self.buffer.lock().unwrap();
        Ok(buffer.pop_front())
    }

    pub fn close(&self) {
        *self.is_closed.lock().unwrap() = true;
        self.condvar.notify_all();
    }

    pub fn broadcast(&self) {
        self.condvar.notify_all();
    }

    pub fn is_closed(&self) -> bool {
        *self.is_closed.lock().unwrap()
    }
}
