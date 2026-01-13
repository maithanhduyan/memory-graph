//! JWT Authentication for MCP SSE Server
//!
//! Provides stateless authentication using JSON Web Tokens (JWT).
//!
//! ## Usage
//! ```bash
//! # Set environment variables
//! MEMORY_JWT_SECRET=your-super-secret-key-at-least-32-chars
//! MEMORY_USERS=alice:password123,bob:secret456,admin:admin-pass
//!
//! # Login to get token
//! curl -X POST http://localhost:3030/auth/token \
//!   -H "Content-Type: application/json" \
//!   -d '{"username":"alice","password":"password123"}'
//!
//! # Use token in requests
//! curl http://localhost:3030/mcp/sse \
//!   -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (username)
    pub sub: String,
    /// User permissions
    pub permissions: Vec<String>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token type: "access" or "refresh"
    pub token_type: String,
}

impl Claims {
    /// Create new access token claims
    pub fn new_access(username: String, permissions: Vec<String>, ttl_seconds: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            permissions,
            iat: now,
            exp: now + ttl_seconds,
            token_type: "access".to_string(),
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh(username: String, ttl_seconds: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            permissions: vec![],
            iat: now,
            exp: now + ttl_seconds,
            token_type: "refresh".to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }

    /// Check if user has permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"*".to_string())
    }
}

/// User information for authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub password_hash: String,
    pub permissions: Vec<String>,
}

/// JWT Authentication manager
pub struct JwtAuth {
    /// Secret key for signing tokens
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// User store (username -> UserInfo)
    users: HashMap<String, UserInfo>,
    /// Access token TTL in seconds (default: 1 hour)
    pub access_token_ttl: i64,
    /// Refresh token TTL in seconds (default: 7 days)
    pub refresh_token_ttl: i64,
}

impl JwtAuth {
    /// Create new JwtAuth with secret key
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            users: HashMap::new(),
            access_token_ttl: 3600,        // 1 hour
            refresh_token_ttl: 604800,     // 7 days
        }
    }

    /// Create from environment variables
    ///
    /// Environment:
    /// - MEMORY_JWT_SECRET: Secret key for signing (required, min 32 chars)
    /// - MEMORY_USERS: Comma-separated user:password pairs (optional)
    /// - MEMORY_ACCESS_TOKEN_TTL: Access token TTL in seconds (optional, default 3600)
    /// - MEMORY_REFRESH_TOKEN_TTL: Refresh token TTL in seconds (optional, default 604800)
    pub fn from_env() -> Result<Self, AuthError> {
        let secret = std::env::var("MEMORY_JWT_SECRET")
            .unwrap_or_else(|_| {
                // Generate random secret if not provided (for development)
                eprintln!("[Auth] WARNING: MEMORY_JWT_SECRET not set, using random secret");
                format!("dev-secret-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
            });

        if secret.len() < 32 {
            return Err(AuthError::InvalidSecret(
                "MEMORY_JWT_SECRET must be at least 32 characters".to_string(),
            ));
        }

        let mut auth = Self::new(&secret);

        // Parse access token TTL
        if let Ok(ttl) = std::env::var("MEMORY_ACCESS_TOKEN_TTL") {
            if let Ok(seconds) = ttl.parse::<i64>() {
                auth.access_token_ttl = seconds;
            }
        }

        // Parse refresh token TTL
        if let Ok(ttl) = std::env::var("MEMORY_REFRESH_TOKEN_TTL") {
            if let Ok(seconds) = ttl.parse::<i64>() {
                auth.refresh_token_ttl = seconds;
            }
        }

        // Parse users from MEMORY_USERS env var
        // Format: "user1:pass1,user2:pass2,admin:adminpass:*"
        // The third part is permissions (optional, default: read,write)
        if let Ok(users_str) = std::env::var("MEMORY_USERS") {
            for user_entry in users_str.split(',') {
                let parts: Vec<&str> = user_entry.trim().split(':').collect();
                if parts.len() >= 2 {
                    let username = parts[0].to_string();
                    let password = parts[1];
                    let permissions = if parts.len() > 2 {
                        parts[2].split('|').map(|s| s.to_string()).collect()
                    } else {
                        vec!["read".to_string(), "write".to_string()]
                    };

                    if let Err(e) = auth.add_user(&username, password, permissions) {
                        eprintln!("[Auth] Failed to add user {}: {}", username, e);
                    }
                }
            }
        }

        // Add default admin user if no users configured (development only)
        if auth.users.is_empty() {
            eprintln!("[Auth] WARNING: No users configured, adding default admin:admin");
            auth.add_user("admin", "admin", vec!["*".to_string()])?;
        }

        eprintln!("[Auth] Loaded {} users", auth.users.len());
        Ok(auth)
    }

    /// Add a user with password and permissions
    pub fn add_user(
        &mut self,
        username: &str,
        password: &str,
        permissions: Vec<String>,
    ) -> Result<(), AuthError> {
        let password_hash = hash(password, DEFAULT_COST)
            .map_err(|e| AuthError::HashError(e.to_string()))?;

        self.users.insert(
            username.to_string(),
            UserInfo {
                username: username.to_string(),
                password_hash,
                permissions,
            },
        );

        Ok(())
    }

    /// Authenticate user with username/password
    pub fn authenticate(&self, username: &str, password: &str) -> Result<&UserInfo, AuthError> {
        let user = self.users.get(username).ok_or(AuthError::InvalidCredentials)?;

        if verify(password, &user.password_hash).unwrap_or(false) {
            Ok(user)
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Generate access and refresh tokens for user
    pub fn generate_tokens(&self, user: &UserInfo) -> Result<TokenPair, AuthError> {
        let access_claims = Claims::new_access(
            user.username.clone(),
            user.permissions.clone(),
            self.access_token_ttl,
        );

        let refresh_claims = Claims::new_refresh(user.username.clone(), self.refresh_token_ttl);

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenError(e.to_string()))?;

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenError(e.to_string()))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl,
        })
    }

    /// Validate a token and return claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data: TokenData<Claims> =
            decode(token, &self.decoding_key, &Validation::default())
                .map_err(|e| AuthError::TokenError(e.to_string()))?;

        if token_data.claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(token_data.claims)
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let claims = self.validate_token(refresh_token)?;

        if claims.token_type != "refresh" {
            return Err(AuthError::InvalidTokenType);
        }

        // Get user to refresh permissions
        let user = self
            .users
            .get(&claims.sub)
            .ok_or(AuthError::UserNotFound)?;

        self.generate_tokens(user)
    }

    /// Validate token from Authorization header
    /// Supports: "Bearer <token>" or just "<token>"
    pub fn validate_authorization(&self, auth_header: &str) -> Result<Claims, AuthError> {
        let token = if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else {
            auth_header
        };

        self.validate_token(token)
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

/// Token pair response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidCredentials,
    InvalidSecret(String),
    TokenError(String),
    TokenExpired,
    InvalidTokenType,
    UserNotFound,
    HashError(String),
    MissingToken,
    InsufficientPermissions,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::InvalidSecret(msg) => write!(f, "Invalid secret: {}", msg),
            AuthError::TokenError(msg) => write!(f, "Token error: {}", msg),
            AuthError::TokenExpired => write!(f, "Token has expired"),
            AuthError::InvalidTokenType => write!(f, "Invalid token type"),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::HashError(msg) => write!(f, "Hash error: {}", msg),
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InsufficientPermissions => write!(f, "Insufficient permissions"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Thread-safe wrapper for JwtAuth
pub type SharedJwtAuth = Arc<JwtAuth>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_auth() -> JwtAuth {
        let mut auth = JwtAuth::new("test-secret-key-that-is-at-least-32-characters-long");
        auth.add_user("alice", "password123", vec!["read".to_string(), "write".to_string()])
            .unwrap();
        auth.add_user("admin", "admin", vec!["*".to_string()])
            .unwrap();
        auth
    }

    #[test]
    fn test_authenticate_valid_user() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123");
        assert!(user.is_ok());
        assert_eq!(user.unwrap().username, "alice");
    }

    #[test]
    fn test_authenticate_invalid_password() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "wrongpassword");
        assert!(matches!(user, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_authenticate_invalid_user() {
        let auth = create_test_auth();
        let user = auth.authenticate("unknown", "password");
        assert!(matches!(user, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_generate_and_validate_tokens() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // Validate access token
        let claims = auth.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.token_type, "access");
        assert!(claims.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn test_refresh_token() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // Refresh using refresh token
        let new_tokens = auth.refresh_access_token(&tokens.refresh_token).unwrap();
        assert!(!new_tokens.access_token.is_empty());
    }

    #[test]
    fn test_claims_permissions() {
        let claims = Claims::new_access(
            "test".to_string(),
            vec!["read".to_string()],
            3600,
        );

        assert!(claims.has_permission("read"));
        assert!(!claims.has_permission("write"));
        assert!(!claims.has_permission("*"));
    }

    #[test]
    fn test_admin_wildcard_permission() {
        let claims = Claims::new_access(
            "admin".to_string(),
            vec!["*".to_string()],
            3600,
        );

        assert!(claims.has_permission("read"));
        assert!(claims.has_permission("write"));
        assert!(claims.has_permission("anything"));
    }

    #[test]
    fn test_validate_authorization_header() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // With "Bearer " prefix
        let claims = auth
            .validate_authorization(&format!("Bearer {}", tokens.access_token))
            .unwrap();
        assert_eq!(claims.sub, "alice");

        // Without prefix
        let claims = auth.validate_authorization(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "alice");
    }
}
