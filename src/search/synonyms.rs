//! Synonym dictionary for semantic search

/// Synonym groups - words in same group are considered semantically similar
pub const SYNONYM_GROUPS: &[&[&str]] = &[
    // Developer roles
    &[
        "coder",
        "programmer",
        "developer",
        "engineer",
        "dev",
        "software engineer",
        "software developer",
    ],
    &["frontend", "front-end", "ui developer", "client-side"],
    &["backend", "back-end", "server-side", "api developer"],
    &["fullstack", "full-stack", "full stack"],
    &["devops", "sre", "infrastructure", "platform engineer"],
    // Bug/Issue related
    &[
        "bug", "issue", "defect", "error", "problem", "fault", "glitch",
    ],
    &["fix", "patch", "hotfix", "bugfix", "repair", "resolve"],
    // Feature/Task related
    &["feature", "functionality", "capability", "enhancement"],
    &["task", "ticket", "work item", "story", "user story"],
    &["requirement", "spec", "specification", "req"],
    // Status
    &["done", "completed", "finished", "resolved", "closed"],
    &["pending", "waiting", "blocked", "on hold"],
    &["in progress", "wip", "ongoing", "active", "working"],
    &["todo", "to do", "planned", "backlog"],
    // Priority
    &["critical", "urgent", "p0", "blocker", "showstopper"],
    &["high", "important", "p1"],
    &["medium", "normal", "p2"],
    &["low", "minor", "p3"],
    // Project management
    &["milestone", "release", "version", "sprint"],
    &["deadline", "due date", "target date"],
    &["project", "repo", "repository", "codebase"],
    // Documentation
    &["doc", "docs", "documentation", "readme", "guide"],
    &["api", "interface", "endpoint"],
    // Testing
    &["test", "testing", "qa", "quality assurance"],
    &["unit test", "unittest"],
    &["integration test", "e2e", "end-to-end"],
    // Architecture
    &["module", "component", "service", "package"],
    &["database", "db", "datastore", "storage"],
    &["cache", "caching", "redis", "memcached"],
];

/// Get all synonyms for a query term
pub fn get_synonyms(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut synonyms = vec![query_lower.clone()];

    for group in SYNONYM_GROUPS {
        if group.iter().any(|&word| {
            word == query_lower || query_lower.contains(word) || word.contains(&query_lower)
        }) {
            for &word in *group {
                if !synonyms.contains(&word.to_string()) {
                    synonyms.push(word.to_string());
                }
            }
        }
    }

    synonyms
}

/// Check if text matches any of the search terms (including synonyms)
pub fn matches_with_synonyms(text: &str, search_terms: &[String]) -> bool {
    let text_lower = text.to_lowercase();
    search_terms.iter().any(|term| text_lower.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_synonyms_developer() {
        let synonyms = get_synonyms("developer");
        assert!(synonyms.contains(&"developer".to_string()));
        assert!(synonyms.contains(&"coder".to_string()));
        assert!(synonyms.contains(&"programmer".to_string()));
    }

    #[test]
    fn test_get_synonyms_bug() {
        let synonyms = get_synonyms("bug");
        assert!(synonyms.contains(&"bug".to_string()));
        assert!(synonyms.contains(&"issue".to_string()));
        assert!(synonyms.contains(&"defect".to_string()));
    }

    #[test]
    fn test_matches_with_synonyms() {
        let terms = get_synonyms("developer");
        assert!(matches_with_synonyms("I am a coder", &terms));
        assert!(matches_with_synonyms("Software Engineer position", &terms));
        assert!(!matches_with_synonyms("I am a doctor", &terms));
    }
}
