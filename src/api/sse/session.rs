//! Session management for SSE connections

use std::collections::HashMap;
use tokio::sync::RwLock;

use super::ClientSession;

/// Session manager for tracking connected clients
pub struct SessionManager {
    sessions: RwLock<HashMap<String, ClientSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a new session ID
    pub fn generate_session_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("sess_{:x}", timestamp)
    }

    /// Create a new session
    pub async fn create_session(&self, user: String, api_key: Option<String>) -> ClientSession {
        let session_id = Self::generate_session_id();
        let session = ClientSession {
            session_id: session_id.clone(),
            user,
            api_key,
            connected_at: chrono::Utc::now().timestamp(),
        };

        self.sessions.write().await.insert(session_id, session.clone());
        session
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<ClientSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get active session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Validate API key and return user info
    /// For now, we use a simple format: "user:secret" or just accept any non-empty key
    pub fn validate_api_key(api_key: &str) -> Option<String> {
        if api_key.is_empty() {
            return None;
        }

        // Simple format: "username:secret" or just "username"
        // In production, you'd validate against a database
        if let Some(colon_pos) = api_key.find(':') {
            let username = &api_key[..colon_pos];
            if !username.is_empty() {
                return Some(username.to_string());
            }
        }

        // Accept any non-empty key as anonymous user
        Some(format!("api-user-{}", &api_key[..8.min(api_key.len())]))
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key_with_username() {
        let user = SessionManager::validate_api_key("alice:secret123");
        assert_eq!(user, Some("alice".to_string()));
    }

    #[test]
    fn test_validate_api_key_anonymous() {
        let user = SessionManager::validate_api_key("some-random-key");
        assert_eq!(user, Some("api-user-some-ran".to_string()));
    }

    #[test]
    fn test_validate_api_key_empty() {
        let user = SessionManager::validate_api_key("");
        assert_eq!(user, None);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new();

        // Create session
        let session = manager.create_session("alice".to_string(), Some("key123".to_string())).await;
        assert!(session.session_id.starts_with("sess_"));
        assert_eq!(session.user, "alice");

        // Get session
        let retrieved = manager.get_session(&session.session_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user, "alice");

        // Count
        assert_eq!(manager.session_count().await, 1);

        // Remove
        manager.remove_session(&session.session_id).await;
        assert_eq!(manager.session_count().await, 0);
    }
}
