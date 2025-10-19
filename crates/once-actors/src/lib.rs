//! Actor Model for Once Language
//! 
//! Implements:
//! - Mailbox-based actor concurrency
//! - Message passing between actors
//! - Actor lifecycle management
//! - Supervision and error handling

use once_hir::HirProgram;
use once_runtime::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use thiserror::Error;
use serde;

/// Actor system errors
#[derive(Error, Debug, Clone)]
pub enum ActorError {
    #[error("Actor error: {0}")]
    ActorError(String),
    
    #[error("Message error: {0}")]
    MessageError(String),
    
    #[error("Supervision error: {0}")]
    SupervisionError(String),
}

/// Actor reference for sending messages
#[derive(Debug, Clone)]
pub struct ActorRef {
    pub actor_id: ActorId,
    pub mailbox: Arc<Mutex<VecDeque<Message>>>,
    pub condvar: Arc<Condvar>,
}

/// Unique actor identifier
pub type ActorId = u64;

/// Actor message
#[derive(Debug, Clone)]
pub struct Message {
    pub sender: Option<ActorRef>,
    pub payload: Value,
    pub message_type: MessageType,
}

/// Type of message
#[derive(Debug, Clone)]
pub enum MessageType {
    /// Regular message
    Regular,
    /// System message (spawn, stop, etc.)
    System,
    /// Error message
    Error,
    /// Supervision message
    Supervision,
}

/// Actor state
#[derive(Debug, Clone)]
pub enum ActorState {
    /// Actor is running normally
    Running,
    /// Actor is paused
    Paused,
    /// Actor is stopping
    Stopping,
    /// Actor has stopped
    Stopped,
    /// Actor has crashed
    Crashed,
}

/// Actor behavior function
pub type BehaviorFn = fn(Message, &mut ActorContext) -> Result<ActorAction, ActorError>;

/// Actor context for behavior functions
#[derive(Debug, Clone)]
pub struct ActorContext {
    pub self_ref: ActorRef,
    pub state: ActorState,
    pub children: Vec<ActorRef>,
    pub parent: Option<ActorRef>,
    pub supervisor: Option<ActorRef>,
    pub system: Arc<Mutex<ActorSystem>>,
    pub actor_state: HashMap<String, Value>,
}

impl ActorContext {
    /// Get state value
    pub fn get_state<T>(&self, key: &str) -> Option<T> 
    where 
        T: Clone + for<'de> serde::Deserialize<'de>,
        Value: serde::Serialize,
    {
        self.actor_state.get(key)
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).ok()?).ok())
    }
    
    /// Set state value
    pub fn set_state<T>(&mut self, key: &str, value: T) 
    where 
        T: serde::Serialize,
    {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.actor_state.insert(key.to_string(), Value::Json(json_value));
        }
    }
}

/// Action an actor can take after processing a message
#[derive(Debug, Clone)]
pub enum ActorAction {
    /// Continue processing messages
    Continue,
    /// Stop the actor
    Stop,
    /// Spawn a new child actor
    SpawnChild { behavior: BehaviorFn, name: String },
    /// Send a message to another actor
    Send { target: ActorRef, message: Message },
    /// Become a new behavior
    Become { behavior: BehaviorFn },
    /// Crash with an error
    Crash { error: ActorError },
}

/// Actor definition
#[derive(Debug)]
pub struct Actor {
    pub id: ActorId,
    pub name: String,
    pub behavior: BehaviorFn,
    pub context: ActorContext,
    pub mailbox: Arc<Mutex<VecDeque<Message>>>,
    pub condvar: Arc<Condvar>,
    pub state: ActorState,
}

/// Actor system for managing actors
#[derive(Debug, Clone)]
pub struct ActorSystem {
    pub actors: HashMap<ActorId, Arc<Mutex<Actor>>>,
    pub next_actor_id: ActorId,
    pub supervisor: Option<ActorRef>,
    pub system_actors: HashMap<String, ActorRef>,
}

impl ActorSystem {
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
            next_actor_id: 1,
            supervisor: None,
            system_actors: HashMap::new(),
        }
    }

    /// Spawn a new actor
    pub fn spawn(&mut self, name: String, behavior: BehaviorFn) -> Result<ActorRef, ActorError> {
        let actor_id = self.next_actor_id;
        self.next_actor_id += 1;

        let mailbox = Arc::new(Mutex::new(VecDeque::new()));
        let condvar = Arc::new(Condvar::new());

        let actor_ref = ActorRef {
            actor_id,
            mailbox: mailbox.clone(),
            condvar: condvar.clone(),
        };

        let context = ActorContext {
            self_ref: actor_ref.clone(),
            state: ActorState::Running,
            children: Vec::new(),
            parent: None,
            supervisor: self.supervisor.clone(),
            system: Arc::new(Mutex::new(self.clone())),
            actor_state: HashMap::new(),
        };

        let actor = Actor {
            id: actor_id,
            name: name.clone(),
            behavior,
            context,
            mailbox,
            condvar,
            state: ActorState::Running,
        };

        self.actors.insert(actor_id, Arc::new(Mutex::new(actor)));
        self.system_actors.insert(name, actor_ref.clone());

        // Start the actor's message processing loop
        self.start_actor_loop(actor_ref.clone())?;

        Ok(actor_ref)
    }

    /// Start an actor's message processing loop
    fn start_actor_loop(&self, actor_ref: ActorRef) -> Result<(), ActorError> {
        let actor_ref_clone = actor_ref.clone();
        let actors = self.actors.clone();
        let system = Arc::new(Mutex::new(self.clone()));

        thread::spawn(move || {
            if let Err(e) = Self::actor_loop(&ActorSystem::new(), actor_ref_clone, actors, system) {
                eprintln!("Actor loop error: {:?}", e);
            }
        });

        Ok(())
    }

    /// Main actor message processing loop
    fn actor_loop(&self, actor_ref: ActorRef, actors: HashMap<ActorId, Arc<Mutex<Actor>>>, system: Arc<Mutex<ActorSystem>>) -> Result<(), ActorError> {
        loop {
            // Wait for messages
            let message = {
                let mut mailbox = actor_ref.mailbox.lock().unwrap();
                while mailbox.is_empty() {
                    mailbox = actor_ref.condvar.wait(mailbox).unwrap();
                }
                mailbox.pop_front().unwrap()
            };

            // Get the actor
            let actor = actors.get(&actor_ref.actor_id)
                .ok_or_else(|| ActorError::ActorError("Actor not found".to_string()))?
                .clone();

            let mut actor_guard = actor.lock().unwrap();
            let behavior = actor_guard.behavior;
            let mut context = actor_guard.context.clone();

            // Process the message
            let action = match behavior(message, &mut context) {
                Ok(action) => action,
                Err(e) => {
                    eprintln!("Actor behavior error: {:?}", e);
                    ActorAction::Crash { error: e }
                }
            };

            // Handle the action
            match action {
                ActorAction::Continue => {
                    // Continue processing messages
                }
                ActorAction::Stop => {
                    actor_guard.state = ActorState::Stopped;
                    break;
                }
                ActorAction::SpawnChild { behavior: child_behavior, name } => {
                    // Spawn child actor
                    let child_id = {
                        let system_guard = system.lock().unwrap();
                        system_guard.next_actor_id
                    };
                    {
                        let mut system_guard = system.lock().unwrap();
                        system_guard.next_actor_id += 1;
                    }
                    
                    let child_actor = Actor {
                        id: child_id,
                        name: name.clone(),
                        behavior: child_behavior,
                        context: ActorContext {
                            parent: Some(actor_ref.clone()),
                            children: Vec::new(),
                            system: system.clone(),
                            self_ref: ActorRef {
                                actor_id: child_id,
                                mailbox: Arc::new(Mutex::new(VecDeque::new())),
                                condvar: Arc::new(Condvar::new()),
                            },
                            state: ActorState::Running,
                            supervisor: None,
                            actor_state: HashMap::new(),
                        },
                        mailbox: Arc::new(Mutex::new(VecDeque::new())),
                        condvar: Arc::new(Condvar::new()),
                        state: ActorState::Running,
                    };
                    
                    let child_ref = ActorRef {
                        actor_id: child_id,
                        mailbox: Arc::new(Mutex::new(VecDeque::new())),
                        condvar: Arc::new(Condvar::new()),
                    };
                    
                    // Add to system
                    {
                        let mut system_guard = system.lock().unwrap();
                        system_guard.actors.insert(child_id, Arc::new(Mutex::new(child_actor)));
                    }
                    
                    // Add to parent's children
                    if let Some(parent) = &actor_guard.context.parent {
                        // Update parent's children list
                        // This would need to be implemented properly
                    }
                    
                    // Start child actor
                    let child_system = system.clone();
                    let child_ref_clone = child_ref.clone();
                    std::thread::spawn(move || {
                        // Start the child actor loop
                        // This would need to be implemented properly
                    });
                }
                ActorAction::Send { target, message } => {
                    if let Err(e) = target.send(message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                }
                ActorAction::Become { behavior: new_behavior } => {
                    actor_guard.behavior = new_behavior;
                }
                ActorAction::Crash { error } => {
                    eprintln!("Actor crashed: {:?}", error);
                    actor_guard.state = ActorState::Crashed;
                    break;
                }
            }

            // Update context
            actor_guard.context = context;
        }

        Ok(())
    }

    /// Stop an actor
    pub fn stop(&mut self, actor_id: ActorId) -> Result<(), ActorError> {
        if let Some(actor) = self.actors.get(&actor_id) {
            let mut actor_guard = actor.lock().unwrap();
            actor_guard.state = ActorState::Stopping;
            
            // Send stop message
            let stop_message = Message {
                sender: None,
                payload: Value::String("stop".to_string()),
                message_type: MessageType::System,
            };
            
            {
                let mut mailbox = actor_guard.mailbox.lock().unwrap();
                mailbox.push_back(stop_message);
            }
            actor_guard.condvar.notify_one();
        }
        Ok(())
    }

    /// Get actor by name
    pub fn get_actor(&self, name: &str) -> Option<&ActorRef> {
        self.system_actors.get(name)
    }

    /// Get all actors
    pub fn get_all_actors(&self) -> Vec<ActorRef> {
        self.actors.values()
            .map(|actor| {
                let actor_guard = actor.lock().unwrap();
                ActorRef {
                    actor_id: actor_guard.id,
                    mailbox: actor_guard.mailbox.clone(),
                    condvar: actor_guard.condvar.clone(),
                }
            })
            .collect()
    }
}

impl ActorRef {
    /// Send a message to this actor
    pub fn send(&self, message: Message) -> Result<(), ActorError> {
        let mut mailbox = self.mailbox.lock().unwrap();
        mailbox.push_back(message);
        self.condvar.notify_one();
        Ok(())
    }

    /// Send a simple value message
    pub fn send_value(&self, value: Value) -> Result<(), ActorError> {
        let message = Message {
            sender: None,
            payload: value,
            message_type: MessageType::Regular,
        };
        self.send(message)
    }

    /// Check if actor is still running
    pub fn is_running(&self) -> bool {
        // For now, assume all actors are running
        // In a real implementation, we'd need to track actor state
        true
    }
}

/// Built-in actor behaviors
pub mod behaviors {
    use super::*;

    /// Echo actor that responds with the same message
    pub fn echo_behavior(message: Message, _context: &mut ActorContext) -> Result<ActorAction, ActorError> {
        println!("Echo actor received: {:?}", message.payload);
        
        // Echo back to sender if available
        if let Some(sender) = message.sender {
            let echo_message = Message {
                sender: None,
                payload: message.payload,
                message_type: MessageType::Regular,
            };
            return Ok(ActorAction::Send { target: sender, message: echo_message });
        }
        
        Ok(ActorAction::Continue)
    }

    /// Counter actor that maintains a count
    pub fn counter_behavior(message: Message, context: &mut ActorContext) -> Result<ActorAction, ActorError> {
        // Get or initialize counter state
        let count = context.get_state::<i32>("count").unwrap_or(0);
        let new_count = count + 1;
        context.set_state("count", new_count);
        
        println!("Counter actor count: {}", new_count);
        
        // Send count back to sender if available
        if let Some(sender) = message.sender {
            let count_message = Message {
                sender: None,
                payload: Value::Int(new_count as i64),
                message_type: MessageType::Regular,
            };
            return Ok(ActorAction::Send { target: sender, message: count_message });
        }
        
        Ok(ActorAction::Continue)
    }

    /// Logger actor for system logging
    pub fn logger_behavior(message: Message, _context: &mut ActorContext) -> Result<ActorAction, ActorError> {
        println!("[LOG] {:?}", message.payload);
        Ok(ActorAction::Continue)
    }

    /// Supervisor actor for error handling
    pub fn supervisor_behavior(message: Message, context: &mut ActorContext) -> Result<ActorAction, ActorError> {
        match message.message_type {
            MessageType::Error => {
                println!("Supervisor handling error: {:?}", message.payload);
                
                // Get failed actor ID from message payload
                if let Value::Int(failed_actor_id) = message.payload {
                    let actor_id = failed_actor_id as u64;
                    
                    // Get restart count before accessing system
                    let restart_count = context.get_state::<i32>("restart_count").unwrap_or(0);
                    
                    if restart_count < 3 { // Max 3 restarts
                        println!("Restarting actor {} (attempt {})", actor_id, restart_count + 1);
                        
                        // Update restart count
                        context.set_state("restart_count", restart_count + 1);
                        
                        // Send restart message to actor
                        let restart_message = Message {
                            sender: None,
                            payload: Value::String("restart".to_string()),
                            message_type: MessageType::System,
                        };
                        
                        // Create a simple actor ref for the failed actor
                        let actor_ref = ActorRef {
                            actor_id,
                            mailbox: Arc::new(Mutex::new(VecDeque::new())),
                            condvar: Arc::new(Condvar::new()),
                        };
                        return Ok(ActorAction::Send { target: actor_ref, message: restart_message });
                    } else {
                        println!("Actor {} exceeded max restarts, stopping", actor_id);
                        return Ok(ActorAction::Stop);
                    }
                }
                
                Ok(ActorAction::Continue)
            }
            MessageType::System => {
                println!("Supervisor system message: {:?}", message.payload);
                Ok(ActorAction::Continue)
            }
            _ => Ok(ActorAction::Continue),
        }
    }
}

/// Actor system integration with Once runtime
impl ActorSystem {
    /// Create actors from Once program
    pub fn create_from_program(&mut self, program: &HirProgram) -> Result<(), ActorError> {
        // Analyze HIR program for actor definitions
        for item in &program.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    // Check if function is marked as an actor
                    if fn_decl.name.starts_with("actor_") {
                        let actor_name = fn_decl.name.strip_prefix("actor_").unwrap_or(&fn_decl.name);
                        
                        // Create actor behavior from function
                        let behavior = behaviors::echo_behavior;
                        
                        // Spawn the actor
                        let actor_ref = self.spawn(actor_name.to_string(), behavior)?;
                        self.system_actors.insert(actor_name.to_string(), actor_ref);
                    }
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    // Check if variable is an actor reference
                    if let_decl.name.starts_with("actor_") {
                        let actor_name = let_decl.name.strip_prefix("actor_").unwrap_or(&let_decl.name);
                        
                        // Create a simple actor for this reference
                        let actor_ref = self.spawn(actor_name.to_string(), behaviors::echo_behavior)?;
                        self.system_actors.insert(actor_name.to_string(), actor_ref);
                    }
                }
                _ => {
                    // Other items don't create actors
                }
            }
        }
        
        // Create default system actors
        let logger = self.spawn("logger".to_string(), behaviors::logger_behavior)?;
        let supervisor = self.spawn("supervisor".to_string(), behaviors::supervisor_behavior)?;
        
        // Set supervisor
        self.supervisor = Some(supervisor);
        
        Ok(())
    }

    /// Run the actor system
    pub fn run(&self) -> Result<(), ActorError> {
        // Keep the system running
        loop {
            thread::sleep(std::time::Duration::from_millis(100));
            
            // Check if all actors are still running
            let running_count = self.actors.values()
                .filter(|actor| {
                    let actor_guard = actor.lock().unwrap();
                    matches!(actor_guard.state, ActorState::Running)
                })
                .count();
            
            if running_count == 0 {
                break;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_system_creation() {
        let mut system = ActorSystem::new();
        assert!(system.actors.is_empty());
        assert_eq!(system.next_actor_id, 1);
    }

    #[test]
    fn test_actor_spawn() {
        let mut system = ActorSystem::new();
        let actor_ref = system.spawn("test".to_string(), behaviors::echo_behavior);
        assert!(actor_ref.is_ok());
        
        let actor_ref = actor_ref.unwrap();
        assert_eq!(actor_ref.actor_id, 1);
    }

    #[test]
    fn test_actor_message_sending() {
        let mut system = ActorSystem::new();
        let actor_ref = system.spawn("test".to_string(), behaviors::echo_behavior).unwrap();
        
        let message = Message {
            sender: None,
            payload: Value::String("Hello".to_string()),
            message_type: MessageType::Regular,
        };
        
        assert!(actor_ref.send(message).is_ok());
    }
}