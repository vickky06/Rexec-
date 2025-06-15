use std::sync::Arc;
use std::hash::Hash;
use std::collections::{HashMap, BinaryHeap};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use std::cmp::Reverse;



#[derive(Debug)]
pub enum SessionError {
    NotFound(String),
    InvalidLanguage(String),
    ExecutionError(String),
    Unauthenticated(String),
}


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub session_id: String,
    pub language: String,
}


#[derive(Clone, Debug)]
pub struct SessionValue {
    pub image: String,
}

#[derive(Clone, Debug)]
pub struct SessionManagementService {
    pub ttl: Duration, // Default TTL of 1 hour
    pub sessions: Arc<Mutex<HashMap<SessionKey, SessionValue>>>,
    pub expirations: Arc<Mutex<BinaryHeap<Reverse<(Instant, String)>>>>, // Min-heap for expiration times
    pub last_cleanup: Arc<Mutex<Instant>>,
}