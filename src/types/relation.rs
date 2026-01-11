//! Relation types for the knowledge graph

use serde::{Deserialize, Serialize};

use super::{default_user, is_default_user, is_zero, Entity};

/// Relation between entities with temporal validity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
    #[serde(
        rename = "createdBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub created_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "validFrom", default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<u64>,
    #[serde(rename = "validTo", default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<u64>,
}

impl Relation {
    /// Create a new relation
    pub fn new(from: String, to: String, relation_type: String) -> Self {
        Self {
            from,
            to,
            relation_type,
            created_by: String::new(),
            created_at: 0,
            valid_from: None,
            valid_to: None,
        }
    }

    /// Create a new relation with temporal validity
    pub fn with_validity(
        from: String,
        to: String,
        relation_type: String,
        valid_from: Option<u64>,
        valid_to: Option<u64>,
    ) -> Self {
        Self {
            from,
            to,
            relation_type,
            created_by: String::new(),
            created_at: 0,
            valid_from,
            valid_to,
        }
    }
}

/// Related entity with relation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntity {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String,
    pub entity: Entity,
}

/// Result of get_related query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntities {
    pub entity: String,
    pub relations: Vec<RelatedEntity>,
}
