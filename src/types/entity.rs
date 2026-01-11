//! Entity types for the knowledge graph

use serde::{Deserialize, Serialize};

use super::{default_user, is_default_user, is_zero};

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(
        rename = "createdBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub created_by: String,
    #[serde(
        rename = "updatedBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub updated_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "is_zero")]
    pub updated_at: u64,
}

impl Entity {
    /// Create a new entity with default values
    pub fn new(name: String, entity_type: String) -> Self {
        Self {
            name,
            entity_type,
            observations: Vec::new(),
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        }
    }

    /// Create a new entity with observations
    pub fn with_observations(name: String, entity_type: String, observations: Vec<String>) -> Self {
        Self {
            name,
            entity_type,
            observations,
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// Brief entity info for summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBrief {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub brief: String,
}
