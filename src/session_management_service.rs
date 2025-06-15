use crate::config_service::GLOBAL_CONFIG;
use crate::proto::executor::ExecuteRequest;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use tokio::sync::Mutex;
use tonic::Request;
use std::sync::Arc;
use std::time::{Duration, Instant};


use crate::models::session_management_models::{SessionError, SessionKey, SessionValue, SessionManagementService};

pub const SESSION_ID: &str = "session_id";
pub const ANONYMOUS: &str = "anonymous";

use once_cell::sync::OnceCell;

static SINGLETON_SESSION_MANAGEMENT_SERVICE: OnceCell<SessionManagementService> = OnceCell::new();


impl SessionError {
    pub fn message(&self) -> String {
        match self {
            SessionError::NotFound(id) => format!("Session with ID '{}' not found.", id),
            SessionError::InvalidLanguage(lang) => {
                format!("Invalid language specified: '{}'.", lang)
            }
            SessionError::ExecutionError(msg) => format!("Execution error: {}", msg),
            SessionError::Unauthenticated(msg) => format!("Unauthenticated: {}", msg),
        }
    }
}


impl SessionKey {
    pub fn new(session_id: String, language: String) -> Self {
        SessionKey {
            session_id,
            language,
        }
    }
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.session_id, self.language)
    }

    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            Some(SessionKey {
                session_id: parts[0].to_string(),
                language: parts[1].to_string(),
            })
        } else {
            None
        }
    }
}



impl SessionValue {
    pub fn new(image: String) -> Self {
        SessionValue { image }
    }
}

#[async_trait::async_trait]
pub trait SessionManagement {
    async fn need_cleanup(&self) -> bool;
    async fn cleanup_expired_sessions(&self);
    async fn add_session(
        &self,
        session_id: String,
        language: String,
        container_image: String,
    ) -> Result<(), SessionError>;

    async fn delete_session(&self, session_key: &str) -> Result<(), SessionError>;

    async fn get_session_image(
        &self,
        session_id: &str,
        language: &str,
    ) -> Result<String, SessionError>;

    fn get_session_id(&self, request: &Request<ExecuteRequest>) -> Result<String, SessionError>;
}



impl SessionManagementService {
    pub fn new() -> Self {
        println!("Initializing SessionManagementService");
        SessionManagementService {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            expirations: Arc::new(Mutex::new(BinaryHeap::new())),
            ttl: Duration::from_secs(3600),
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn set_last_cleanup(&self, time: Instant) {
        let mut last_cleanup = self.last_cleanup.lock().await;
        *last_cleanup = time;
    }
    pub async fn get_last_cleanup(&self) -> Instant {
        let last_cleanup = self.last_cleanup.lock().await;
        *last_cleanup
    }
}

impl Default for SessionManagementService {
    //first call
    fn default() -> Self {
        if SINGLETON_SESSION_MANAGEMENT_SERVICE.get().is_some() {
            println!("Returning existing SessionManagementService instance");
            return SINGLETON_SESSION_MANAGEMENT_SERVICE.get().unwrap().clone();
        }
        println!("Creating default SessionManagementService");
        SINGLETON_SESSION_MANAGEMENT_SERVICE
            .set(SessionManagementService::new())
            .ok();
        return SINGLETON_SESSION_MANAGEMENT_SERVICE.get().unwrap().clone();
    }
}

#[async_trait::async_trait]
impl SessionManagement for SessionManagementService {
    async fn need_cleanup(&self) -> bool {
        println!("Checking if cleanup is needed...");

        // Lock `self.expirations` once and store the length
        let expirations_len = {
            let expirations = self.expirations.lock().await;
            expirations.len()
        };

        // Access `GLOBAL_CONFIG` outside of the lock
        let max_sessions = GLOBAL_CONFIG.get().unwrap().session_configs.max_sessions;
        let session_timeout = GLOBAL_CONFIG.get().unwrap().session_configs.session_timeout;

        println!(
            "Current session count: {}, Max sessions: {}, Session timeout: {} seconds",
            expirations_len, max_sessions, session_timeout
        );

        // Perform the cleanup check
        max_sessions <= expirations_len || session_timeout <= self.ttl.as_secs()
    }

    async fn add_session(
        &self,
        session_id: String,
        language: String,
        container_image: String,
    ) -> Result<(), SessionError> {
        let expiration_time = Instant::now() + self.ttl;

        let mut sessions = self.sessions.lock().await;
        let key = SessionKey::new(session_id.clone(), language.clone());

        if sessions.contains_key(&key) {
            return Err(SessionError::ExecutionError(format!(
                "Session already exists for ID '{}' and language '{}'",
                session_id, language
            )));
        }
        let key_clone = key.to_string();
        sessions.insert(key, SessionValue::new(container_image));
        {
            let mut expirations = self.expirations.lock().await;
            expirations.push(Reverse((expiration_time, key_clone)));
        } // Spawn cleanup in a background task if needed, making it non-blocking
        if self.need_cleanup().await {
            println!("Cleanup needed, spawning background cleanup task...");
        }
        Ok(())
    }

    async fn delete_session(&self, session_key: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.lock().await;
        let key = SessionKey::from_string(&session_key).ok_or_else(|| {
            SessionError::InvalidLanguage("Invalid session key format".to_string())
        })?;
        if !sessions.contains_key(&key) {
            return Err(SessionError::NotFound(session_key.to_string()));
        }

        if sessions.remove(&key).is_none() {
            return Err(SessionError::NotFound(key.to_string()));
        }

        let svc = self.clone();
        tokio::spawn(async move {
            svc.set_last_cleanup(Instant::now()).await;
        });

        Ok(())
    }

    async fn cleanup_expired_sessions(&self) {
        let now = Instant::now();

        loop {
            // Lock `self.expirations` and check the top of the heap
            let session_to_remove = {
                let expirations = self.expirations.lock().await;
                if let Some(Reverse((expiration_time, session_key))) = expirations.peek() {
                    if *expiration_time <= now {
                        // Clone the session_key to avoid borrowing issues
                        Some(session_key.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // If no session needs to be removed, break the loop
            if let Some(session_key) = session_to_remove {
                // Remove the session outside the lock
                if let Err(e) = self.delete_session(&session_key).await {
                    println!(
                        "Error removing expired session {}: {}",
                        session_key,
                        e.message()
                    );
                } else {
                    println!("Removed expired session: {}", session_key);
                }
            } else {
                break;
            }
        }
    }

    async fn get_session_image(
        &self,
        session_id: &str,
        language: &str,
    ) -> Result<String, SessionError> {
        let sessions = self.sessions.lock().await;
        let key = SessionKey::new(session_id.to_string(), language.to_string());

        match sessions.get(&key) {
            Some(val) => Ok(val.image.clone()),
            None => Err(SessionError::NotFound(session_id.to_string())),
        }
    }

    fn get_session_id(&self, request: &Request<ExecuteRequest>) -> Result<String, SessionError> {
        let session_id = request
            .metadata()
            .get(SESSION_ID)
            .and_then(|v: &tonic::metadata::MetadataValue<tonic::metadata::Ascii>| v.to_str().ok())
            .unwrap_or(ANONYMOUS)
            .to_string();

        if session_id == ANONYMOUS {
            return Err(SessionError::Unauthenticated(
                "Session ID is required for execution.".to_string(),
            ));
        }
        Ok(session_id)
    }
}
