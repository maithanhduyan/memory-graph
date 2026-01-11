//! Semantic search with synonym matching
//!
//! This module provides semantic search capabilities through synonym expansion.

mod synonyms;

pub use synonyms::{get_synonyms, matches_with_synonyms, SYNONYM_GROUPS};
