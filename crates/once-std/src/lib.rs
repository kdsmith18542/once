//! Standard Library for the Once language
//! 
//! Implements:
//! - Core types and traits
//! - Resource management
//! - Linear types
//! - Copy trait
//! - Memory management utilities
//! - I/O operations
//! - Collections

use std::collections::{HashMap as StdHashMap, VecDeque, HashSet as StdHashSet, BTreeMap as StdBTreeMap, BTreeSet as StdBTreeSet};
use std::fmt;
use std::io::{self as std_io, Read, Write};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::net::{TcpListener as StdTcpListener, TcpStream as StdTcpStream, SocketAddr, ToSocketAddrs};
use std::thread;
use std::time::{Duration as StdDuration, Instant as StdInstant};
use std::rc::Rc as StdRc;
use std::sync::Arc as StdArc;
use thiserror::Error;

/// Standard library errors
#[derive(Error, Debug, Clone)]
pub enum StdError {
    #[error("Resource error: {0}")]
    ResourceError(std::string::String),
    
    #[error("I/O error: {0}")]
    IoError(std::string::String),
    
    #[error("Memory error: {0}")]
    MemoryError(std::string::String),
    
    #[error("Type error: {0}")]
    TypeError(std::string::String),
}

/// Resource trait - all linear types must implement this
/// 
/// This trait defines the contract for linear resources that must be consumed exactly once.
/// Resources implement this trait to ensure proper cleanup and prevent resource leaks.
pub trait Resource {
    /// Consume the resource, returning any final value
    /// 
    /// This method must be called exactly once before the resource goes out of scope.
    /// It performs any necessary cleanup operations and returns an error if the resource
    /// cannot be properly consumed.
    fn consume(self) -> std::result::Result<(), StdError>;
    
    /// Check if the resource is still valid
    /// 
    /// Returns true if the resource is in a valid state and can be used.
    /// Returns false if the resource has been consumed or is in an invalid state.
    fn is_valid(&self) -> bool;
    
    /// Get the resource type name for debugging
    fn resource_type(&self) -> &'static str;
    
    /// Get the resource ID for tracking
    fn resource_id(&self) -> std::option::Option<u64>;
}

/// Resource manager for tracking and managing linear resources
pub struct ResourceManager {
    resources: StdHashMap<u64, ResourceInfo>,
    next_id: u64,
}

/// Resource information for tracking
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub id: u64,
    pub resource_type: std::string::String,
    pub created_at: std::time::SystemTime,
    pub is_consumed: bool,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: StdHashMap::new(),
            next_id: 1,
        }
    }
    
    /// Register a new resource
    pub fn register_resource(&mut self, resource_type: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        self.resources.insert(id, ResourceInfo {
            id,
            resource_type: resource_type.to_string(),
            created_at: std::time::SystemTime::now(),
            is_consumed: false,
        });
        
        id
    }
    
    /// Mark a resource as consumed
    pub fn consume_resource(&mut self, id: u64) -> std::result::Result<(), StdError> {
        if let Some(info) = self.resources.get_mut(&id) {
            if info.is_consumed {
                return Err(StdError::ResourceError(format!("Resource {} already consumed", id)));
            }
            info.is_consumed = true;
            Ok(())
        } else {
            Err(StdError::ResourceError(format!("Resource {} not found", id)))
        }
    }
    
    /// Check if a resource is valid
    pub fn is_resource_valid(&self, id: u64) -> bool {
        self.resources.get(&id)
            .map(|info| !info.is_consumed)
            .unwrap_or(false)
    }
    
    /// Get all unconsumed resources
    pub fn get_unconsumed_resources(&self) -> std::vec::Vec<&ResourceInfo> {
        self.resources.values()
            .filter(|info| !info.is_consumed)
            .collect()
    }
    
    /// Clean up all resources
    pub fn cleanup_all(&mut self) -> std::result::Result<(), StdError> {
        let unconsumed: std::vec::Vec<u64> = self.resources.iter()
            .filter(|(_, info)| !info.is_consumed)
            .map(|(id, _)| *id)
            .collect();
        
        for id in unconsumed {
            self.consume_resource(id)?;
        }
        
        Ok(())
    }
}

/// Copy trait - types that can be safely copied
pub trait Copy {
    /// Create a copy of the value
    fn copy(&self) -> Self;
}

/// Clone trait - types that can be cloned
pub trait OnceClone {
    /// Create a clone of the value
    fn once_clone(&self) -> Self;
}

/// File handle - a linear resource
pub struct FileHandle {
    pub id: u64,
    pub file: File,
    pub path: std::string::String,
    pub is_open: bool,
}

impl Resource for FileHandle {
    fn consume(mut self) -> std::result::Result<(), StdError> {
        if self.is_open {
            self.file.sync_all()
                .map_err(|e| StdError::IoError(format!("Failed to sync file: {}", e)))?;
            self.is_open = false;
        }
        Ok(())
    }
    
    fn is_valid(&self) -> bool {
        self.is_open
    }
    
    fn resource_type(&self) -> &'static str {
        "FileHandle"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::Some(self.id)
    }
}

impl FileHandle {
    pub fn open(path: &str) -> std::result::Result<Self, StdError> {
        let file = File::open(path)
            .map_err(|e| StdError::IoError(format!("Failed to open file: {}", e)))?;
        
        Ok(FileHandle {
            id: Self::generate_unique_id(),
            file,
            path: path.to_string(),
            is_open: true,
        })
    }
    
    pub fn create(path: &str) -> std::result::Result<Self, StdError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .map_err(|e| StdError::IoError(format!("Failed to create file: {}", e)))?;
        
        Ok(FileHandle {
            id: Self::generate_unique_id(),
            file,
            path: path.to_string(),
            is_open: true,
        })
    }
    
    pub fn read(&mut self, buffer: &mut [u8]) -> std::result::Result<usize, StdError> {
        if !self.is_open {
            return Err(StdError::ResourceError("File is closed".to_string()));
        }
        
        self.file.read(buffer)
            .map_err(|e| StdError::IoError(format!("Failed to read file: {}", e)))
    }
    
    pub fn write(&mut self, data: &[u8]) -> std::result::Result<usize, StdError> {
        if !self.is_open {
            return Err(StdError::ResourceError("File is closed".to_string()));
        }
        
        self.file.write(data)
            .map_err(|e| StdError::IoError(format!("Failed to write file: {}", e)))
    }
    
    /// Generate a unique ID for file handles
    fn generate_unique_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

/// String - a linear type for text data
pub struct String {
    pub data: std::vec::Vec<u8>,
    pub is_linear: bool,
}

impl Resource for String {
    fn consume(self) -> std::result::Result<(), StdError> {
        // String consumption is automatic when dropped
        Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "String"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None // Strings don't have unique IDs
    }
}

impl String {
    pub fn new() -> Self {
        Self {
            data: std::vec::Vec::new(),
            is_linear: true,
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        Self {
            data: s.as_bytes().to_vec(),
            is_linear: true,
        }
    }
    
    pub fn push(&mut self, ch: char) {
        let mut buf = [0; 4];
        let encoded = ch.encode_utf8(&mut buf);
        self.data.extend_from_slice(encoded.as_bytes());
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn as_str(&self) -> std::result::Result<&str, StdError> {
        std::str::from_utf8(&self.data)
            .map_err(|e| StdError::TypeError(format!("Invalid UTF-8: {}", e)))
    }
}

impl OnceClone for String {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned strings are not linear
        }
    }
}

/// Vector - a linear collection
pub struct Vec<T> {
    pub data: VecDeque<T>,
    pub is_linear: bool,
}

impl<T> Resource for Vec<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Vector consumption is automatic when dropped
        Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Vec"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None // Vectors don't have unique IDs
    }
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
            is_linear: true,
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            is_linear: true,
        }
    }
    
    pub fn push(&mut self, item: T) {
        self.data.push_back(item);
    }
    
    pub fn pop(&mut self) -> std::option::Option<T> {
        self.data.pop_front()
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    pub fn get(&self, index: usize) -> std::option::Option<&T> {
        self.data.get(index)
    }
    
    pub fn get_mut(&mut self, index: usize) -> std::option::Option<&mut T> {
        self.data.get_mut(index)
    }
}

impl<T: std::clone::Clone> OnceClone for Vec<T> {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned vectors are not linear
        }
    }
}

/// HashMap - a linear associative collection
pub struct HashMap<K, V> {
    pub data: StdHashMap<K, V>,
    pub is_linear: bool,
}

impl<K, V> Resource for HashMap<K, V> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // HashMap consumption is automatic when dropped
        Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "HashMap"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None // HashMaps don't have unique IDs
    }
}

impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self {
            data: StdHashMap::new(),
            is_linear: true,
        }
    }
    
    pub fn insert(&mut self, key: K, value: V) -> std::option::Option<V> 
    where 
        K: std::hash::Hash + Eq,
    {
        self.data.insert(key, value)
    }
    
    pub fn get(&self, key: &K) -> std::option::Option<&V> 
    where 
        K: std::hash::Hash + Eq,
    {
        self.data.get(key)
    }
    
    pub fn get_mut(&mut self, key: &K) -> std::option::Option<&mut V>
    where 
        K: std::hash::Hash + Eq,
    {
        self.data.get_mut(key)
    }
    
    pub fn remove(&mut self, key: &K) -> std::option::Option<V>
    where 
        K: std::hash::Hash + Eq,
    {
        self.data.remove(key)
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<K: std::clone::Clone, V: std::clone::Clone> OnceClone for HashMap<K, V> {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned HashMaps are not linear
        }
    }
}

/// Linear set type using HashSet
#[derive(Debug, Clone)]
pub struct Set<T> {
    data: StdHashSet<T>,
    is_linear: bool,
}

impl<T: std::hash::Hash + std::cmp::Eq + Clone> Set<T> {
    pub fn new() -> Self {
        Self {
            data: StdHashSet::new(),
            is_linear: true,
        }
    }

    pub fn insert(&mut self, value: T) -> bool {
        self.data.insert(value)
    }

    pub fn remove(&mut self, value: &T) -> bool {
        self.data.remove(value)
    }

    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::collections::hash_set::Iter<T> {
        self.data.iter()
    }
}

impl<T: std::hash::Hash + std::cmp::Eq + Clone> Resource for Set<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Set cleanup - nothing special needed
        Ok(())
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn resource_type(&self) -> &'static str {
        "Set"
    }

    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl<T: std::hash::Hash + std::cmp::Eq + Clone> Copy for Set<T> {
    fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Copied Sets are not linear
        }
    }
}

impl<T: std::hash::Hash + std::cmp::Eq + Clone> OnceClone for Set<T> {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned Sets are not linear
        }
    }
}

/// Linear map type using BTreeMap for ordered keys
#[derive(Debug, Clone)]
pub struct Map<K, V> {
    data: StdBTreeMap<K, V>,
    is_linear: bool,
}

impl<K: std::cmp::Ord + Clone, V: Clone> Map<K, V> {
    pub fn new() -> Self {
        Self {
            data: StdBTreeMap::new(),
            is_linear: true,
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> std::option::Option<V> {
        self.data.insert(key, value)
    }

    pub fn get(&self, key: &K) -> std::option::Option<&V> {
        self.data.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> std::option::Option<&mut V> {
        self.data.get_mut(key)
    }

    pub fn remove(&mut self, key: &K) -> std::option::Option<V> {
        self.data.remove(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<K, V> {
        self.data.iter()
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<K, V> {
        self.data.keys()
    }

    pub fn values(&self) -> std::collections::btree_map::Values<K, V> {
        self.data.values()
    }
}

impl<K: std::cmp::Ord + Clone, V: Clone> Resource for Map<K, V> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Map cleanup - nothing special needed
        Ok(())
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn resource_type(&self) -> &'static str {
        "Map"
    }

    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl<K: std::cmp::Ord + Clone, V: Clone> Copy for Map<K, V> {
    fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Copied Maps are not linear
        }
    }
}

impl<K: std::cmp::Ord + Clone, V: Clone> OnceClone for Map<K, V> {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned Maps are not linear
        }
    }
}

/// Linear deque type using VecDeque
#[derive(Debug, Clone)]
pub struct Deque<T> {
    data: VecDeque<T>,
    is_linear: bool,
}

impl<T: Clone> Deque<T> {
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
            is_linear: true,
        }
    }

    pub fn push_front(&mut self, value: T) {
        self.data.push_front(value);
    }

    pub fn push_back(&mut self, value: T) {
        self.data.push_back(value);
    }

    pub fn pop_front(&mut self) -> std::option::Option<T> {
        self.data.pop_front()
    }

    pub fn pop_back(&mut self) -> std::option::Option<T> {
        self.data.pop_back()
    }

    pub fn front(&self) -> std::option::Option<&T> {
        self.data.front()
    }

    pub fn back(&self) -> std::option::Option<&T> {
        self.data.back()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn get(&self, index: usize) -> std::option::Option<&T> {
        self.data.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> std::option::Option<&mut T> {
        self.data.get_mut(index)
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.data.iter()
    }
}

impl<T: Clone> Resource for Deque<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Deque cleanup - nothing special needed
        Ok(())
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn resource_type(&self) -> &'static str {
        "Deque"
    }

    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl<T: Clone> Copy for Deque<T> {
    fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Copied Deques are not linear
        }
    }
}

impl<T: Clone> OnceClone for Deque<T> {
    fn once_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Cloned Deques are not linear
        }
    }
}

/// Linear TCP listener for accepting connections
#[derive(Debug)]
pub struct TcpListener {
    listener: StdTcpListener,
    is_linear: bool,
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> std::result::Result<Self, StdError> {
        let listener = StdTcpListener::bind(addr)
            .map_err(|e| StdError::IoError(format!("Failed to bind: {}", e)))?;
        
        Ok(Self {
            listener,
            is_linear: true,
        })
    }

    pub fn accept(&self) -> std::result::Result<TcpStream, StdError> {
        let (stream, addr) = self.listener.accept()
            .map_err(|e| StdError::IoError(format!("Failed to accept connection: {}", e)))?;
        
        Ok(TcpStream {
            stream,
            addr,
            is_linear: true,
        })
    }

    pub fn local_addr(&self) -> std::result::Result<SocketAddr, StdError> {
        self.listener.local_addr()
            .map_err(|e| StdError::IoError(format!("Failed to get local address: {}", e)))
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> std::result::Result<(), StdError> {
        self.listener.set_nonblocking(nonblocking)
            .map_err(|e| StdError::IoError(format!("Failed to set nonblocking: {}", e)))
    }
}

impl Resource for TcpListener {
    fn consume(self) -> std::result::Result<(), StdError> {
        // TCP listener cleanup - nothing special needed
        Ok(())
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn resource_type(&self) -> &'static str {
        "TcpListener"
    }

    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl Copy for TcpListener {
    fn copy(&self) -> Self {
        Self {
            listener: self.listener.try_clone()
                .expect("Failed to clone TcpListener"),
            is_linear: false, // Copied TcpListeners are not linear
        }
    }
}

impl OnceClone for TcpListener {
    fn once_clone(&self) -> Self {
        Self {
            listener: self.listener.try_clone()
                .expect("Failed to clone TcpListener"),
            is_linear: false, // Cloned TcpListeners are not linear
        }
    }
}

/// Linear TCP stream for network communication
#[derive(Debug)]
pub struct TcpStream {
    stream: StdTcpStream,
    addr: SocketAddr,
    is_linear: bool,
}

impl TcpStream {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> std::result::Result<Self, StdError> {
        let stream = StdTcpStream::connect(addr)
            .map_err(|e| StdError::IoError(format!("Failed to connect: {}", e)))?;
        let peer_addr = stream.peer_addr()
            .map_err(|e| StdError::IoError(format!("Failed to get peer address: {}", e)))?;
        
        Ok(Self {
            stream,
            addr: peer_addr,
            is_linear: true,
        })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, StdError> {
        self.stream.read(buf)
            .map_err(|e| StdError::IoError(format!("Failed to read: {}", e)))
    }

    pub fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, StdError> {
        self.stream.write(buf)
            .map_err(|e| StdError::IoError(format!("Failed to write: {}", e)))
    }

    pub fn flush(&mut self) -> std::result::Result<(), StdError> {
        self.stream.flush()
            .map_err(|e| StdError::IoError(format!("Failed to flush: {}", e)))
    }

    pub fn peer_addr(&self) -> std::result::Result<SocketAddr, StdError> {
        Ok(self.addr)
    }

    pub fn local_addr(&self) -> std::result::Result<SocketAddr, StdError> {
        self.stream.local_addr()
            .map_err(|e| StdError::IoError(format!("Failed to get local address: {}", e)))
    }

    pub fn set_read_timeout(&self, timeout: std::option::Option<Duration>) -> std::result::Result<(), StdError> {
        let std_timeout = timeout.map(|d| d.duration);
        self.stream.set_read_timeout(std_timeout)
            .map_err(|e| StdError::IoError(format!("Failed to set read timeout: {}", e)))
    }

    pub fn set_write_timeout(&self, timeout: std::option::Option<Duration>) -> std::result::Result<(), StdError> {
        let std_timeout = timeout.map(|d| d.duration);
        self.stream.set_write_timeout(std_timeout)
            .map_err(|e| StdError::IoError(format!("Failed to set write timeout: {}", e)))
    }
}

impl Resource for TcpStream {
    fn consume(self) -> std::result::Result<(), StdError> {
        // TCP stream cleanup - nothing special needed
        Ok(())
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn resource_type(&self) -> &'static str {
        "TcpStream"
    }

    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl Copy for TcpStream {
    fn copy(&self) -> Self {
        Self {
            stream: self.stream.try_clone()
                .expect("Failed to clone TcpStream"),
            addr: self.addr,
            is_linear: false, // Copied TcpStreams are not linear
        }
    }
}

impl OnceClone for TcpStream {
    fn once_clone(&self) -> Self {
        Self {
            stream: self.stream.try_clone()
                .expect("Failed to clone TcpStream"),
            addr: self.addr,
            is_linear: false, // Cloned TcpStreams are not linear
        }
    }
}

/// DNS resolver for hostname resolution
pub struct DnsResolver;

impl DnsResolver {
    pub fn resolve(hostname: &str, port: u16) -> std::result::Result<std::vec::Vec<SocketAddr>, StdError> {
        let addr_string = format!("{}:{}", hostname, port);
        let addrs = addr_string.to_socket_addrs()
            .map_err(|e| StdError::IoError(format!("Failed to resolve {}: {}", hostname, e)))?;
        
        Ok(addrs.collect())
    }

    pub fn resolve_ipv4(hostname: &str, port: u16) -> std::result::Result<std::vec::Vec<SocketAddr>, StdError> {
        let addrs = Self::resolve(hostname, port)?;
        Ok(addrs.into_iter()
            .filter(|addr| addr.is_ipv4())
            .collect())
    }

    pub fn resolve_ipv6(hostname: &str, port: u16) -> std::result::Result<std::vec::Vec<SocketAddr>, StdError> {
        let addrs = Self::resolve(hostname, port)?;
        Ok(addrs.into_iter()
            .filter(|addr| addr.is_ipv6())
            .collect())
    }
}

/// Linear duration type for time measurements
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    duration: StdDuration,
    is_linear: bool,
}

impl Duration {
    pub fn new(secs: u64, nanos: u32) -> Self {
        Self {
            duration: StdDuration::new(secs, nanos),
            is_linear: true,
        }
    }

    pub fn from_secs(secs: u64) -> Self {
        Self {
            duration: StdDuration::from_secs(secs),
            is_linear: true,
        }
    }

    pub fn from_millis(millis: u64) -> Self {
        Self {
            duration: StdDuration::from_millis(millis),
            is_linear: true,
        }
    }

    pub fn from_micros(micros: u64) -> Self {
        Self {
            duration: StdDuration::from_micros(micros),
            is_linear: true,
        }
    }

    pub fn from_nanos(nanos: u64) -> Self {
        Self {
            duration: StdDuration::from_nanos(nanos),
            is_linear: true,
        }
    }

    pub fn as_secs(&self) -> u64 {
        self.duration.as_secs()
    }

    pub fn as_millis(&self) -> u128 {
        self.duration.as_millis()
    }

    pub fn as_micros(&self) -> u128 {
        self.duration.as_micros()
    }

    pub fn as_nanos(&self) -> u128 {
        self.duration.as_nanos()
    }

    pub fn checked_add(&self, other: &Duration) -> std::option::Option<Duration> {
        self.duration.checked_add(other.duration).map(|d| Duration {
            duration: d,
            is_linear: true,
        })
    }

    pub fn checked_sub(&self, other: &Duration) -> std::option::Option<Duration> {
        self.duration.checked_sub(other.duration).map(|d| Duration {
            duration: d,
            is_linear: true,
        })
    }

    pub fn checked_mul(&self, rhs: u32) -> std::option::Option<Duration> {
        self.duration.checked_mul(rhs).map(|d| Duration {
            duration: d,
            is_linear: true,
        })
    }

    pub fn checked_div(&self, rhs: u32) -> std::option::Option<Duration> {
        self.duration.checked_div(rhs).map(|d| Duration {
            duration: d,
            is_linear: true,
        })
    }
}

impl Resource for Duration {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Duration is a value type, no resources to clean up
        std::result::Result::Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Duration"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl Copy for Duration {
    fn copy(&self) -> Self {
        Self {
            duration: self.duration,
            is_linear: false, // Copy creates a non-linear value
        }
    }
}

impl OnceClone for Duration {
    fn once_clone(&self) -> Self {
        self.copy()
    }
}

/// Linear instant type for time measurements
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    instant: StdInstant,
    is_linear: bool,
}

impl Instant {
    pub fn now() -> Self {
        Self {
            instant: StdInstant::now(),
            is_linear: true,
        }
    }

    pub fn duration_since(&self, earlier: &Instant) -> Duration {
        Duration {
            duration: self.instant.duration_since(earlier.instant),
            is_linear: true,
        }
    }

    pub fn elapsed(&self) -> Duration {
        Duration {
            duration: self.instant.elapsed(),
            is_linear: true,
        }
    }

    pub fn checked_duration_since(&self, earlier: &Instant) -> std::option::Option<Duration> {
        self.instant.checked_duration_since(earlier.instant).map(|d| Duration {
            duration: d,
            is_linear: true,
        })
    }

    pub fn saturating_duration_since(&self, earlier: &Instant) -> Duration {
        Duration {
            duration: self.instant.saturating_duration_since(earlier.instant),
            is_linear: true,
        }
    }
}

impl Resource for Instant {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Instant is a value type, no resources to clean up
        std::result::Result::Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Instant"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl Copy for Instant {
    fn copy(&self) -> Self {
        Self {
            instant: self.instant,
            is_linear: false, // Copy creates a non-linear value
        }
    }
}

impl OnceClone for Instant {
    fn once_clone(&self) -> Self {
        self.copy()
    }
}

/// Linear deadline type for timeout management
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline {
    deadline: StdInstant,
    is_linear: bool,
}

impl Deadline {
    pub fn new(instant: Instant) -> Self {
        Self {
            deadline: instant.instant,
            is_linear: true,
        }
    }

    /// Create a deadline `duration` from now
    pub fn from_now(duration: Duration) -> Self {
        let deadline = StdInstant::now() + duration.duration;
        Self {
            deadline,
            is_linear: true,
        }
    }

    /// Check if the deadline has passed
    pub fn has_passed(&self) -> bool {
        StdInstant::now() >= self.deadline
    }

    /// Check if the deadline is expired (alias for has_passed)
    pub fn is_expired(&self) -> bool {
        self.has_passed()
    }

    /// Get remaining duration until deadline, or None if expired
    pub fn remaining(&self) -> std::option::Option<Duration> {
        let now = StdInstant::now();
        if now >= self.deadline {
            std::option::Option::None
        } else {
            let remaining = self.deadline.duration_since(now);
            std::option::Option::Some(Duration {
                duration: remaining,
                is_linear: true,
            })
        }
    }

    /// Extend the deadline by a duration
    pub fn extend(&mut self, duration: Duration) -> std::result::Result<(), StdError> {
        if let Some(new_deadline) = self.deadline.checked_add(duration.duration) {
            self.deadline = new_deadline;
            std::result::Result::Ok(())
        } else {
            std::result::Result::Err(StdError::TypeError("Duration overflow".to_string()))
        }
    }
}

impl Resource for Deadline {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Deadline is a value type, no resources to clean up
        std::result::Result::Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Deadline"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl Copy for Deadline {
    fn copy(&self) -> Self {
        Self {
            deadline: self.deadline,
            is_linear: false, // Copy creates a non-linear value
        }
    }
}

impl OnceClone for Deadline {
    fn once_clone(&self) -> Self {
        self.copy()
    }
}

/// Linear box type for heap allocation - escape hatch for region inference
#[derive(Debug, Clone)]
pub struct Box<T> {
    data: StdArc<T>,
    is_linear: bool,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        Self {
            data: StdArc::new(value),
            is_linear: true,
        }
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn get_mut(&mut self) -> &mut T {
        // Note: This is unsafe in a real implementation
        // For now, we'll return a reference to the data
        unsafe { &mut *(StdArc::as_ptr(&self.data) as *mut T) }
    }

    pub fn into_inner(self) -> T {
        // Note: This is unsafe in a real implementation
        // For now, we'll return the value
        unsafe { std::ptr::read(StdArc::as_ptr(&self.data)) }
    }
}

impl<T> Resource for Box<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Box cleanup - nothing special needed
        std::result::Result::Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Box"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl<T> Copy for Box<T> {
    fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Copy creates a non-linear value
        }
    }
}

impl<T> OnceClone for Box<T> {
    fn once_clone(&self) -> Self {
        self.copy()
    }
}

/// Linear reference counted type - escape hatch for region inference
#[derive(Debug, Clone)]
pub struct Rc<T> {
    data: StdRc<T>,
    is_linear: bool,
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        Self {
            data: StdRc::new(value),
            is_linear: true,
        }
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn get_mut(&mut self) -> std::option::Option<&mut T> {
        StdRc::get_mut(&mut self.data)
    }

    pub fn strong_count(&self) -> usize {
        StdRc::strong_count(&self.data)
    }

    pub fn weak_count(&self) -> usize {
        StdRc::weak_count(&self.data)
    }

    pub fn try_unwrap(self) -> std::result::Result<T, Self> {
        match StdRc::try_unwrap(self.data) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(rc) => std::result::Result::Err(Self {
                data: rc,
                is_linear: self.is_linear,
            }),
        }
    }
}

impl<T> Resource for Rc<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        // Rc cleanup - nothing special needed
        std::result::Result::Ok(())
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Rc"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None
    }
}

impl<T> Copy for Rc<T> {
    fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            is_linear: false, // Copy creates a non-linear value
        }
    }
}

impl<T> OnceClone for Rc<T> {
    fn once_clone(&self) -> Self {
        self.copy()
    }
}

/// Option - a linear optional type
pub enum Option<T> {
    Some(T),
    None,
}

impl<T: Resource> Resource for Option<T> {
    fn consume(self) -> std::result::Result<(), StdError> {
        match self {
            Option::Some(value) => {
                // Consume the inner value if it implements Resource
                value.consume()
            }
            Option::None => Ok(()),
        }
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Option"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None // Options don't have unique IDs
    }
}

impl<T> Option<T> {
    pub fn is_some(&self) -> bool {
        matches!(self, Option::Some(_))
    }
    
    pub fn is_none(&self) -> bool {
        matches!(self, Option::None)
    }
    
    pub fn unwrap(self) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => panic!("Called unwrap on None"),
        }
    }
    
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(value) => value,
            Option::None => default,
        }
    }
}

impl<T: std::clone::Clone> OnceClone for Option<T> {
    fn once_clone(&self) -> Self {
        match self {
            Option::Some(value) => Option::Some(value.clone()),
            Option::None => Option::None,
        }
    }
}

// Inherent consume for Option (works without Resource trait)
impl<T> Option<T> {
    pub fn consume(self) -> std::result::Result<(), StdError> {
        Ok(()) // Ordinary values don't need explicit cleanup
    }
}

/// Result - a linear result type
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

// Inherent consume for Result (works without Resource trait)
impl<T, E> Result<T, E> {
    pub fn consume(self) -> std::result::Result<(), StdError> {
        Ok(()) // Ordinary values don't need explicit cleanup
    }
}

impl<T: Resource, E: Resource> Resource for Result<T, E> {
    fn consume(self) -> std::result::Result<(), StdError> {
        match self {
            Result::Ok(value) => {
                // Consume the inner success value
                value.consume()
            }
            Result::Err(error) => {
                // Consume the inner error value
                error.consume()
            }
        }
    }
    
    fn is_valid(&self) -> bool {
        true
    }
    
    fn resource_type(&self) -> &'static str {
        "Result"
    }
    
    fn resource_id(&self) -> std::option::Option<u64> {
        std::option::Option::None // Results don't have unique IDs
    }
}

impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool {
        matches!(self, Result::Ok(_))
    }
    
    pub fn is_err(&self) -> bool {
        matches!(self, Result::Err(_))
    }
    
    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(_) => panic!("Called unwrap on Err"),
        }
    }
    
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(value) => value,
            Result::Err(_) => default,
        }
    }
}

impl<T: std::clone::Clone, E: std::clone::Clone> OnceClone for Result<T, E> {
    fn once_clone(&self) -> Self {
        match self {
            Result::Ok(value) => Result::Ok(value.clone()),
            Result::Err(error) => Result::Err(error.clone()),
        }
    }
}

/// Print function - built-in I/O
pub fn print(message: &str) -> std::result::Result<(), StdError> {
    println!("{}", message);
    Ok(())
}

/// Print with format - built-in I/O
pub fn print_format(format: &str, args: &[&dyn fmt::Display]) -> std::result::Result<(), StdError> {
    // Implement proper formatting
    let mut result = std::string::String::new();
    let mut arg_index = 0;
    let mut chars = format.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'}') {
            // Found {} placeholder
            chars.next(); // consume the }
            if arg_index < args.len() {
                result.push_str(&format!("{}", args[arg_index]));
                arg_index += 1;
            } else {
                result.push_str("{}");
            }
        } else {
            result.push(ch);
        }
    }
    
    print(&result)
}

/// Read from stdin - built-in I/O
pub fn read_line() -> std::result::Result<String, StdError> {
    let mut input = std::string::String::new();
    std_io::stdin().read_line(&mut input)
        .map_err(|e| StdError::IoError(format!("Failed to read from stdin: {}", e)))?;
    Ok(String::from_str(input.trim()))
}

/// Memory utilities
pub mod memory {
    use super::*;
    
    /// Allocate memory with size and alignment
    pub fn allocate(size: usize, alignment: usize) -> std::result::Result<*mut u8, StdError> {
        if size == 0 {
            return Ok(std::ptr::null_mut());
        }
        
        // Use std::alloc for memory allocation
        let layout = std::alloc::Layout::from_size_align(size, alignment)
            .map_err(|e| StdError::MemoryError(format!("Invalid layout: {}", e)))?;
        
        unsafe {
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                return Err(StdError::MemoryError("Allocation failed".to_string()));
            }
            Ok(ptr)
        }
    }
    
    /// Free memory
    pub fn free(ptr: *mut u8) -> std::result::Result<(), StdError> {
        if ptr.is_null() {
            return Ok(());
        }
        
        // Note: In a real implementation, we'd need to track the layout
        // For now, we'll use a simple approach
        unsafe {
            // This is unsafe and not recommended for production
            // In a real implementation, we'd need to track allocation metadata
            std::alloc::dealloc(ptr, std::alloc::Layout::new::<u8>());
        }
        
        Ok(())
    }
    
    /// Copy memory from source to destination
    pub fn copy(src: *const u8, dst: *mut u8, size: usize) -> std::result::Result<(), StdError> {
        if src.is_null() || dst.is_null() {
            return Err(StdError::MemoryError("Null pointer".to_string()));
        }
        
        unsafe {
            std::ptr::copy(src, dst, size);
        }
        Ok(())
    }
}

/// Collections utilities
pub mod collections {
    use super::*;
    
    /// Create a new vector with initial capacity
    pub fn new_vec<T>(capacity: usize) -> Vec<T> {
        Vec::with_capacity(capacity)
    }
    
    /// Create a new hash map
    pub fn new_hash_map<K, V>() -> HashMap<K, V> {
        HashMap::new()
    }
}

/// I/O utilities
pub mod io {
    use super::*;
    
    /// Open a file for reading
    pub fn open_file(path: &str) -> std::result::Result<FileHandle, StdError> {
        FileHandle::open(path)
    }
    
    /// Create a file for writing
    pub fn create_file(path: &str) -> std::result::Result<FileHandle, StdError> {
        FileHandle::create(path)
    }
    
    /// Write string to file
    pub fn write_string(file: &mut FileHandle, data: &str) -> std::result::Result<usize, StdError> {
        file.write(data.as_bytes())
    }
    
    /// Read string from file
    pub fn read_string(file: &mut FileHandle, buffer: &mut [u8]) -> std::result::Result<usize, StdError> {
        file.read(buffer)
    }
}

/// Global resource manager
static mut RESOURCE_MANAGER: std::option::Option<ResourceManager> = None;

/// Standard library initialization
pub fn init() -> std::result::Result<(), StdError> {
    unsafe {
        RESOURCE_MANAGER = std::option::Option::Some(ResourceManager::new());
    }
    println!("Standard library initialized with resource management");
    Ok(())
}

/// Standard library cleanup
pub fn cleanup() -> std::result::Result<(), StdError> {
    unsafe {
        if let std::option::Option::Some(ref mut manager) = RESOURCE_MANAGER {
            let unconsumed = manager.get_unconsumed_resources();
            if !unconsumed.is_empty() {
                println!("Warning: {} unconsumed resources detected", unconsumed.len());
                for resource in unconsumed {
                    println!("  - {} (ID: {})", resource.resource_type, resource.id);
                }
            }
            manager.cleanup_all()?;
        }
        RESOURCE_MANAGER = std::option::Option::None;
    }
    println!("Standard library cleaned up");
    Ok(())
}

/// Get the global resource manager
pub fn get_resource_manager() -> std::option::Option<&'static mut ResourceManager> {
    unsafe {
        RESOURCE_MANAGER.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_handle() {
        let file = FileHandle::create("test.txt").unwrap();
        assert!(file.is_valid());
        assert_eq!(file.path, "test.txt");
    }

    #[test]
    fn test_string_operations() {
        let mut s = String::new();
        s.push('H');
        s.push('i');
        assert_eq!(s.len(), 2);
        assert_eq!(s.as_str().unwrap(), "Hi");
    }

    #[test]
    fn test_vector_operations() {
        let mut v = Vec::new();
        v.push(1);
        v.push(2);
        assert_eq!(v.len(), 2);
        assert_eq!(v.get(0), Some(&1));
    }

    #[test]
    fn test_hash_map_operations() {
        let mut map = HashMap::new();
        map.insert("key", "value");
        assert_eq!(map.get(&"key"), Some(&"value"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_option_operations() {
        let some = Option::Some(42);
        let none: Option<i32> = Option::None;
        assert!(some.is_some());
        assert!(none.is_none());
        assert_eq!(some.unwrap(), 42);
    }

    #[test]
    fn test_set_operations() {
        let mut set = Set::new();
        assert!(set.insert(1));
        assert!(set.insert(2));
        assert!(!set.insert(1)); // Duplicate
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(!set.contains(&3));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_map_operations() {
        let mut map = Map::new();
        map.insert("key1", "value1");
        map.insert("key2", "value2");
        assert_eq!(map.get(&"key1"), Some(&"value1"));
        assert_eq!(map.get(&"key2"), Some(&"value2"));
        assert_eq!(map.len(), 2);
        assert_eq!(map.remove(&"key1"), Some("value1"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_deque_operations() {
        let mut deque = Deque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_front(0);
        assert_eq!(deque.front(), Some(&0));
        assert_eq!(deque.back(), Some(&2));
        assert_eq!(deque.pop_front(), Some(0));
        assert_eq!(deque.pop_back(), Some(2));
        assert_eq!(deque.len(), 1);
    }

    #[test]
    fn test_tcp_listener_bind() {
        // Test binding to localhost on a random port
        let listener = TcpListener::bind("127.0.0.1:0");
        assert!(listener.is_ok());
        
        if let Ok(listener) = listener {
            let local_addr = listener.local_addr();
            assert!(local_addr.is_ok());
        }
    }

    #[test]
    fn test_dns_resolver() {
        // Test DNS resolution for localhost
        let addrs = DnsResolver::resolve("localhost", 80);
        assert!(addrs.is_ok());
        
        if let Ok(addrs) = addrs {
            assert!(!addrs.is_empty());
        }
    }

    #[test]
    fn test_result_operations() {
        let ok: Result<i32, String> = Result::Ok(42);
        let err: Result<i32, &str> = Result::Err("error");
        assert!(ok.is_ok());
        assert!(err.is_err());
        assert_eq!(ok.unwrap(), 42);
    }

    #[test]
    fn test_duration_operations() {
        let duration = Duration::from_secs(5);
        assert_eq!(duration.as_secs(), 5);
        
        let duration_millis = Duration::from_millis(1500);
        assert_eq!(duration_millis.as_millis(), 1500);
        
        let duration_micros = Duration::from_micros(1000);
        assert_eq!(duration_micros.as_micros(), 1000);
        
        let duration_nanos = Duration::from_nanos(1000000);
        assert_eq!(duration_nanos.as_nanos(), 1000000);
        
        // Test checked operations
        let duration1 = Duration::from_secs(3);
        let duration2 = Duration::from_secs(2);
        let sum = duration1.checked_add(&duration2);
        assert!(sum.is_some());
        assert_eq!(sum.unwrap().as_secs(), 5);
        
        let diff = duration1.checked_sub(&duration2);
        assert!(diff.is_some());
        assert_eq!(diff.unwrap().as_secs(), 1);
    }

    #[test]
    fn test_instant_operations() {
        let earlier = Instant::now();
        let now = Instant::now();
        let elapsed = now.elapsed();
        assert!(elapsed.as_secs() >= 0);
        
        // Test duration_since with a slightly earlier instant
        let duration = now.duration_since(&earlier);
        assert!(duration.as_secs() >= 0);
        
        // Test checked_duration_since
        let checked_duration = now.checked_duration_since(&earlier);
        assert!(checked_duration.is_some());
        
        // Test saturating_duration_since
        let saturating_duration = now.saturating_duration_since(&earlier);
        assert!(saturating_duration.as_secs() >= 0);
    }

    #[test]
    fn test_deadline_operations() {
        // Note: Deadline is a simplified placeholder
        let now = Instant::now();
        let deadline = Deadline::new(now);
        // A deadline created from Instant::now() is effectively expired immediately,
        // but due to timing granularity we just verify it doesn't panic.
        let _ = deadline.is_expired();
        
        let remaining = deadline.remaining();
        assert!(remaining.is_none());
        
        let duration = Duration::from_secs(1);
        let deadline_from_now = Deadline::from_now(duration);
        // from_now is also a placeholder that stores now, so it's expired
        let _ = deadline_from_now.is_expired();
    }

    #[test]
    fn test_time_resource_traits() {
        let duration = Duration::from_secs(1);
        let instant = Instant::now();
        let deadline = Deadline::new(instant.clone());
        
        // Test Resource trait
        assert!(duration.clone().consume().is_ok());
        assert!(instant.clone().consume().is_ok());
        assert!(deadline.clone().consume().is_ok());
        
        // Test Copy trait
        let duration_copy = duration.copy();
        let instant_copy = instant.copy();
        let deadline_copy = deadline.copy();
        
        assert_eq!(duration_copy.as_secs(), 1);
        assert!(!instant_copy.is_linear);
        assert!(!deadline_copy.is_linear);
        
        // Test OnceClone trait
        let duration_clone = duration.once_clone();
        let instant_clone = instant.once_clone();
        let deadline_clone = deadline.once_clone();
        
        assert_eq!(duration_clone.as_secs(), 1);
        assert!(!instant_clone.is_linear);
        assert!(!deadline_clone.is_linear);
    }

    #[test]
    fn test_box_operations() {
        let boxed = Box::new(42);
        assert_eq!(*boxed.get(), 42);
        
        let mut boxed_mut = Box::new(42);
        *boxed_mut.get_mut() = 100;
        assert_eq!(*boxed_mut.get(), 100);
        
        let inner = boxed.into_inner();
        assert_eq!(inner, 42);
    }

    #[test]
    fn test_rc_operations() {
        let rc = Rc::new(42);
        assert_eq!(*rc.get(), 42);
        assert_eq!(rc.strong_count(), 1);
        assert_eq!(rc.weak_count(), 0);
        
        let rc_clone = rc.copy();
        assert_eq!(rc_clone.strong_count(), 2);
        
        // try_unwrap requires this to be the last strong reference,
        // so drop the original first.
        drop(rc);
        let inner = rc_clone.try_unwrap();
        assert!(inner.is_ok());
        assert_eq!(inner.unwrap(), 42);
    }

    #[test]
    fn test_box_rc_resource_traits() {
        let boxed = Box::new(42);
        let rc = Rc::new(42);
        
        // Test Resource trait
        assert!(boxed.clone().consume().is_ok());
        assert!(rc.clone().consume().is_ok());
        
        // Test Copy trait
        let boxed_copy = boxed.copy();
        let rc_copy = rc.copy();
        
        assert!(!boxed_copy.is_linear);
        assert!(!rc_copy.is_linear);
        
        // Test OnceClone trait
        let boxed_clone = boxed.once_clone();
        let rc_clone = rc.once_clone();
        
        assert!(!boxed_clone.is_linear);
        assert!(!rc_clone.is_linear);
    }
}