//! Semantic search with synonym matching and inverted indexing
//!
//! This module provides semantic search capabilities through:
//! - Synonym expansion for semantic matching
//! - Inverted index for O(1) search operations
//! - Pre-computed synonym HashMap for fast lookups

mod index;
mod synonyms;

pub use index::{IndexStats, SearchIndex};
pub use synonyms::{get_synonyms, get_synonyms_exact, has_synonyms, matches_with_synonyms, SYNONYM_GROUPS};
