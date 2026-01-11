//! Observation types for entity updates

use serde::{Deserialize, Serialize};

/// Observation to add to an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

impl Observation {
    /// Create a new observation
    pub fn new(entity_name: String, contents: Vec<String>) -> Self {
        Self {
            entity_name,
            contents,
        }
    }
}

/// Observation deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationDeletion {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub observations: Vec<String>,
}

impl ObservationDeletion {
    /// Create a new observation deletion request
    pub fn new(entity_name: String, observations: Vec<String>) -> Self {
        Self {
            entity_name,
            observations,
        }
    }
}
