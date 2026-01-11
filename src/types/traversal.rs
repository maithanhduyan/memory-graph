//! Graph traversal types

use serde::{Deserialize, Serialize};

use super::Entity;

/// Path step for traverse query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String,
    #[serde(rename = "targetType")]
    pub target_type: Option<String>,
}

impl PathStep {
    /// Create a new path step
    pub fn new(relation_type: String, direction: String) -> Self {
        Self {
            relation_type,
            direction,
            target_type: None,
        }
    }

    /// Create a new path step with target type filter
    pub fn with_target_type(
        relation_type: String,
        direction: String,
        target_type: String,
    ) -> Self {
        Self {
            relation_type,
            direction,
            target_type: Some(target_type),
        }
    }
}

/// Single path in traversal result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalPath {
    pub nodes: Vec<String>,
    pub relations: Vec<String>,
}

impl TraversalPath {
    /// Create a new traversal path
    pub fn new(nodes: Vec<String>, relations: Vec<String>) -> Self {
        Self { nodes, relations }
    }
}

/// Result of traverse query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    #[serde(rename = "startNode")]
    pub start_node: String,
    pub paths: Vec<TraversalPath>,
    #[serde(rename = "endNodes")]
    pub end_nodes: Vec<Entity>,
}

impl TraversalResult {
    /// Create a new traversal result
    pub fn new(start_node: String, paths: Vec<TraversalPath>, end_nodes: Vec<Entity>) -> Self {
        Self {
            start_node,
            paths,
            end_nodes,
        }
    }
}
