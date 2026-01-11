//! Standard entity and relation types with validation

/// Standard entity types for software project management
pub const STANDARD_ENTITY_TYPES: &[&str] = &[
    "Project",
    "Module",
    "Feature",
    "Bug",
    "Decision",
    "Requirement",
    "Milestone",
    "Risk",
    "Convention",
    "Schema",
    "Person",
];

/// Standard relation types for software project management
pub const STANDARD_RELATION_TYPES: &[&str] = &[
    "contains",
    "implements",
    "fixes",
    "caused_by",
    "depends_on",
    "blocked_by",
    "assigned_to",
    "part_of",
    "relates_to",
    "supersedes",
    "affects",
    "requires",
];

/// Check if entity type is standard, return warning if not
pub fn validate_entity_type(entity_type: &str) -> Option<String> {
    if STANDARD_ENTITY_TYPES
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(entity_type))
    {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard entityType '{}'. Recommended: {:?}",
            entity_type, STANDARD_ENTITY_TYPES
        ))
    }
}

/// Check if relation type is standard, return warning if not
pub fn validate_relation_type(relation_type: &str) -> Option<String> {
    if STANDARD_RELATION_TYPES
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(relation_type))
    {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard relationType '{}'. Recommended: {:?}",
            relation_type, STANDARD_RELATION_TYPES
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_standard_entity_type() {
        assert!(validate_entity_type("Project").is_none());
        assert!(validate_entity_type("module").is_none()); // case insensitive
        assert!(validate_entity_type("Person").is_none());
    }

    #[test]
    fn test_validate_non_standard_entity_type() {
        let warning = validate_entity_type("CustomType");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Non-standard entityType"));
    }

    #[test]
    fn test_validate_standard_relation_type() {
        assert!(validate_relation_type("contains").is_none());
        assert!(validate_relation_type("DEPENDS_ON").is_none()); // case insensitive
        assert!(validate_relation_type("implements").is_none());
    }

    #[test]
    fn test_validate_non_standard_relation_type() {
        let warning = validate_relation_type("custom_relation");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Non-standard relationType"));
    }
}
