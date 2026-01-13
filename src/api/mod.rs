//! API module for HTTP and WebSocket endpoints
//!
//! This module provides REST API and WebSocket real-time updates for the Memory Graph UI.
//!
//! ## Endpoints
//!
//! ### WebSocket
//! - `GET /ws` - Real-time graph updates
//!
//! ### REST API
//! - `GET /api/graph` - Full graph snapshot (for client recovery)
//! - `GET /api/graph/stats` - Graph statistics
//! - `GET /api/entities` - List entities with pagination
//! - `GET /api/entities/:name` - Get single entity with relations
//! - `GET /api/relations` - List relations with filters
//! - `GET /api/search` - Search nodes

pub mod http;
pub mod rest;
pub mod websocket;
