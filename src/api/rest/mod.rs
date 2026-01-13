//! REST API module for HTTP endpoints
//!
//! Provides REST endpoints for client recovery and data access:
//! - `GET /api/graph` - Full graph snapshot
//! - `GET /api/entities` - List entities with pagination
//! - `GET /api/entities/:name` - Get single entity
//! - `GET /api/relations` - List relations
//! - `GET /api/search` - Search nodes

pub mod entities;
pub mod graph;
pub mod relations;
pub mod search;

use serde::{Deserialize, Serialize};

/// Common pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Maximum number of items to return (default: 100, max: 1000)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of items to skip
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

impl PaginationParams {
    /// Normalize limit to max 1000
    pub fn normalized_limit(&self) -> usize {
        self.limit.min(1000)
    }
}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Current sequence ID for cache invalidation
    pub sequence_id: u64,
    /// Total count (for paginated responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T, sequence_id: u64) -> Self {
        Self {
            data,
            sequence_id,
            total: None,
        }
    }

    pub fn with_total(data: T, sequence_id: u64, total: usize) -> Self {
        Self {
            data,
            sequence_id,
            total: Some(total),
        }
    }
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "NOT_FOUND".to_string(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "BAD_REQUEST".to_string(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "INTERNAL_ERROR".to_string(),
        }
    }
}
