//! Type validation for entities and relations
//!
//! This module provides soft validation for standard entity and relation types.

mod types;

pub use types::{
    validate_entity_type, validate_relation_type, STANDARD_ENTITY_TYPES, STANDARD_RELATION_TYPES,
};
