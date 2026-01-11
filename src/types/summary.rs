//! Summary types for graph statistics

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::EntityBrief;

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Summary {
    #[serde(rename = "totalEntities")]
    pub total_entities: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<EntityBrief>>,
    #[serde(rename = "byStatus", skip_serializing_if = "Option::is_none")]
    pub by_status: Option<HashMap<String, usize>>,
    #[serde(rename = "byType", skip_serializing_if = "Option::is_none")]
    pub by_type: Option<HashMap<String, usize>>,
    #[serde(rename = "byPriority", skip_serializing_if = "Option::is_none")]
    pub by_priority: Option<HashMap<String, usize>>,
}

impl Summary {
    /// Create an empty summary
    pub fn new(total_entities: usize) -> Self {
        Self {
            total_entities,
            ..Default::default()
        }
    }

    /// Create a summary with entity briefs
    pub fn with_entities(total_entities: usize, entities: Vec<EntityBrief>) -> Self {
        Self {
            total_entities,
            entities: Some(entities),
            ..Default::default()
        }
    }

    /// Create a summary with statistics
    pub fn with_stats(
        total_entities: usize,
        by_type: HashMap<String, usize>,
        by_status: Option<HashMap<String, usize>>,
        by_priority: Option<HashMap<String, usize>>,
    ) -> Self {
        Self {
            total_entities,
            entities: None,
            by_status,
            by_type: Some(by_type),
            by_priority,
        }
    }
}
