//! Memory Graph MCP Server - Single File Implementation
//! A knowledge graph server implementing the Model Context Protocol (MCP)
//! using pure Rust with minimal dependencies.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Types
// ============================================================================

pub type McpResult<T> = Result<T, Box<dyn std::error::Error>>;

// ============================================================================
// Standard Entity & Relation Types (Soft Validation)
// ============================================================================

/// Standard entity types for software project management
const STANDARD_ENTITY_TYPES: &[&str] = &[
    "Project", "Module", "Feature", "Bug", "Decision",
    "Requirement", "Milestone", "Risk", "Convention", "Schema", "Person",
];

/// Standard relation types for software project management
const STANDARD_RELATION_TYPES: &[&str] = &[
    "contains", "implements", "fixes", "caused_by", "depends_on",
    "blocked_by", "assigned_to", "part_of", "relates_to", "supersedes",
    "affects", "requires",
];

/// Check if entity type is standard, return warning if not
fn validate_entity_type(entity_type: &str) -> Option<String> {
    if STANDARD_ENTITY_TYPES.iter().any(|&t| t.eq_ignore_ascii_case(entity_type)) {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard entityType '{}'. Recommended: {:?}",
            entity_type, STANDARD_ENTITY_TYPES
        ))
    }
}

/// Check if relation type is standard, return warning if not
fn validate_relation_type(relation_type: &str) -> Option<String> {
    if STANDARD_RELATION_TYPES.iter().any(|&t| t.eq_ignore_ascii_case(relation_type)) {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard relationType '{}'. Recommended: {:?}",
            relation_type, STANDARD_RELATION_TYPES
        ))
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Get current user from git config or OS environment
fn get_current_user() -> String {
    // 1. Try Git Config (preferred for project context)
    if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // 2. Try OS Environment Variable
    env::var("USER") // Linux/Mac
        .or_else(|_| env::var("USERNAME")) // Windows
        .unwrap_or_else(|_| "anonymous".to_string())
}

/// Default user for serde deserialization
fn default_user() -> String {
    "system".to_string()
}

/// Check if string is empty or "system" (for skip_serializing_if)
fn is_default_user(val: &str) -> bool {
    val.is_empty() || val == "system"
}

// ============================================================================
// Synonym Dictionary for Semantic Search
// ============================================================================

/// Synonym groups - words in same group are considered semantically similar
const SYNONYM_GROUPS: &[&[&str]] = &[
    // Developer roles
    &["coder", "programmer", "developer", "engineer", "dev", "software engineer", "software developer"],
    &["frontend", "front-end", "ui developer", "client-side"],
    &["backend", "back-end", "server-side", "api developer"],
    &["fullstack", "full-stack", "full stack"],
    &["devops", "sre", "infrastructure", "platform engineer"],

    // Bug/Issue related
    &["bug", "issue", "defect", "error", "problem", "fault", "glitch"],
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
fn get_synonyms(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut synonyms = vec![query_lower.clone()];

    for group in SYNONYM_GROUPS {
        if group.iter().any(|&word| word == query_lower || query_lower.contains(word) || word.contains(&query_lower)) {
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
fn matches_with_synonyms(text: &str, search_terms: &[String]) -> bool {
    let text_lower = text.to_lowercase();
    search_terms.iter().any(|term| text_lower.contains(term))
}

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(rename = "createdBy", default = "default_user", skip_serializing_if = "is_default_user")]
    pub created_by: String,
    #[serde(rename = "updatedBy", default = "default_user", skip_serializing_if = "is_default_user")]
    pub updated_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "is_zero")]
    pub updated_at: u64,
}

fn is_zero(val: &u64) -> bool {
    *val == 0
}

/// Relation between entities with temporal validity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
    #[serde(rename = "createdBy", default = "default_user", skip_serializing_if = "is_default_user")]
    pub created_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "validFrom", default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<u64>,
    #[serde(rename = "validTo", default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<u64>,
}

/// Knowledge graph containing entities and relations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

/// Observation to add to an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

/// Observation deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationDeletion {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub observations: Vec<String>,
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

/// Path step for traverse query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String,
    #[serde(rename = "targetType")]
    pub target_type: Option<String>,
}

/// Single path in traversal result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalPath {
    pub nodes: Vec<String>,
    pub relations: Vec<String>,
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

/// Brief entity info for summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBrief {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub brief: String,
}

// ============================================================================
// JSON-RPC Types
// ============================================================================

#[derive(Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Value,
}

#[derive(Serialize, Debug)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: ErrorObject,
}

#[derive(Serialize, Debug)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ============================================================================
// MCP Types
// ============================================================================

#[derive(Serialize, Debug)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// ============================================================================
// Tool Trait
// ============================================================================

pub trait Tool: Send + Sync {
    fn definition(&self) -> McpTool;
    fn execute(&self, params: Value) -> McpResult<Value>;
}

// ============================================================================
// Knowledge Base
// ============================================================================

/// Knowledge base with in-memory cache for thread-safe operations
pub struct KnowledgeBase {
    memory_file_path: String,
    graph: Mutex<KnowledgeGraph>,
    current_user: String,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let default_memory_path = current_dir.join("memory.jsonl");

        let memory_file_path = match env::var("MEMORY_FILE_PATH") {
            Ok(path) => {
                if Path::new(&path).is_absolute() {
                    path
                } else {
                    current_dir.join(path).to_string_lossy().to_string()
                }
            }
            Err(_) => default_memory_path.to_string_lossy().to_string(),
        };

        // Detect current user once at startup
        let current_user = get_current_user();

        // Load graph from file at startup (or create empty if not exists)
        let graph = Self::load_graph_from_file(&memory_file_path).unwrap_or_default();

        Self {
            memory_file_path,
            graph: Mutex::new(graph),
            current_user,
        }
    }

    /// Load graph from file (static helper for initialization)
    fn load_graph_from_file(file_path: &str) -> McpResult<KnowledgeGraph> {
        if !Path::new(file_path).exists() {
            return Ok(KnowledgeGraph::default());
        }

        let content = fs::read_to_string(file_path)?;
        let mut graph = KnowledgeGraph::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entity) = serde_json::from_str::<Entity>(line) {
                if !entity.name.is_empty() && !entity.entity_type.is_empty() {
                    graph.entities.push(entity);
                    continue;
                }
            }

            if let Ok(relation) = serde_json::from_str::<Relation>(line) {
                if !relation.from.is_empty() && !relation.to.is_empty() {
                    graph.relations.push(relation);
                }
            }
        }

        Ok(graph)
    }

    /// Get a clone of the current graph (thread-safe read)
    fn load_graph(&self) -> McpResult<KnowledgeGraph> {
        Ok(self.graph.lock().unwrap().clone())
    }

    /// Persist graph to file (internal helper, expects caller to hold lock)
    fn persist_to_file(&self, graph: &KnowledgeGraph) -> McpResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.memory_file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut content = String::new();

        for entity in &graph.entities {
            content.push_str(&serde_json::to_string(entity)?);
            content.push('\n');
        }

        for relation in &graph.relations {
            content.push_str(&serde_json::to_string(relation)?);
            content.push('\n');
        }

        fs::write(&self.memory_file_path, content)?;
        Ok(())
    }

    /// Create new entities (thread-safe: holds lock during entire operation)
    pub fn create_entities(&self, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
        let mut graph = self.graph.lock().unwrap();
        let existing_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
        let now = current_timestamp();

        let mut created = Vec::new();
        for mut entity in entities {
            if !existing_names.contains(&entity.name) {
                // Auto-fill user info if not provided
                if entity.created_by.is_empty() || entity.created_by == "system" {
                    entity.created_by = self.current_user.clone();
                }
                if entity.updated_by.is_empty() || entity.updated_by == "system" {
                    entity.updated_by = self.current_user.clone();
                }
                entity.created_at = now;
                entity.updated_at = now;
                created.push(entity.clone());
                graph.entities.push(entity);
            }
        }

        self.persist_to_file(&graph)?;
        Ok(created)
    }

    /// Create new relations (thread-safe: holds lock during entire operation)
    pub fn create_relations(&self, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
        let mut graph = self.graph.lock().unwrap();
        let entity_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
        let now = current_timestamp();

        let existing_relations: HashSet<String> = graph.relations
            .iter()
            .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
            .collect();

        let mut created = Vec::new();
        for mut relation in relations {
            if entity_names.contains(&relation.from) && entity_names.contains(&relation.to) {
                let key = format!("{}|{}|{}", relation.from, relation.to, relation.relation_type);
                if !existing_relations.contains(&key) {
                    // Auto-fill user info if not provided
                    if relation.created_by.is_empty() || relation.created_by == "system" {
                        relation.created_by = self.current_user.clone();
                    }
                    relation.created_at = now;
                    created.push(relation.clone());
                    graph.relations.push(relation);
                }
            }
        }

        self.persist_to_file(&graph)?;
        Ok(created)
    }

    /// Add observations to entities (thread-safe: holds lock during entire operation)
    pub fn add_observations(&self, observations: Vec<Observation>) -> McpResult<Vec<Observation>> {
        let mut graph = self.graph.lock().unwrap();
        let mut added = Vec::new();
        let now = current_timestamp();

        for obs in observations {
            if let Some(entity) = graph.entities.iter_mut().find(|e| e.name == obs.entity_name) {
                let existing: HashSet<String> = entity.observations.iter().cloned().collect();
                let mut new_contents = Vec::new();

                for content in &obs.contents {
                    if !existing.contains(content) {
                        entity.observations.push(content.clone());
                        new_contents.push(content.clone());
                    }
                }

                if !new_contents.is_empty() {
                    entity.updated_at = now;
                    entity.updated_by = self.current_user.clone();
                    added.push(Observation {
                        entity_name: obs.entity_name.clone(),
                        contents: new_contents,
                    });
                }
            }
        }

        self.persist_to_file(&graph)?;
        Ok(added)
    }

    /// Delete entities (thread-safe: holds lock during entire operation)
    pub fn delete_entities(&self, entity_names: Vec<String>) -> McpResult<()> {
        let mut graph = self.graph.lock().unwrap();
        let names_to_delete: HashSet<String> = entity_names.into_iter().collect();

        graph.entities.retain(|e| !names_to_delete.contains(&e.name));
        graph.relations.retain(|r| {
            !names_to_delete.contains(&r.from) && !names_to_delete.contains(&r.to)
        });

        self.persist_to_file(&graph)?;
        Ok(())
    }

    /// Delete observations from entities (thread-safe: holds lock during entire operation)
    pub fn delete_observations(&self, deletions: Vec<ObservationDeletion>) -> McpResult<()> {
        let mut graph = self.graph.lock().unwrap();

        for deletion in deletions {
            if let Some(entity) = graph.entities.iter_mut().find(|e| e.name == deletion.entity_name) {
                let to_remove: HashSet<String> = deletion.observations.into_iter().collect();
                entity.observations.retain(|o| !to_remove.contains(o));
            }
        }

        self.persist_to_file(&graph)?;
        Ok(())
    }

    /// Delete relations (thread-safe: holds lock during entire operation)
    pub fn delete_relations(&self, relations: Vec<Relation>) -> McpResult<()> {
        let mut graph = self.graph.lock().unwrap();

        let to_delete: HashSet<String> = relations
            .iter()
            .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
            .collect();

        graph.relations.retain(|r| {
            let key = format!("{}|{}|{}", r.from, r.to, r.relation_type);
            !to_delete.contains(&key)
        });

        self.persist_to_file(&graph)?;
        Ok(())
    }

    /// Read graph with optional pagination
    pub fn read_graph(&self, limit: Option<usize>, offset: Option<usize>) -> McpResult<KnowledgeGraph> {
        let graph = self.load_graph()?;

        let offset = offset.unwrap_or(0);

        let entities: Vec<Entity> = if let Some(lim) = limit {
            graph.entities.into_iter().skip(offset).take(lim).collect()
        } else {
            graph.entities.into_iter().skip(offset).collect()
        };

        let entity_names: HashSet<String> = entities.iter().map(|e| e.name.clone()).collect();

        let relations: Vec<Relation> = graph.relations
            .into_iter()
            .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
            .collect();

        Ok(KnowledgeGraph { entities, relations })
    }

    /// Search nodes by query with synonym expansion, optional limit and relation inclusion
    pub fn search_nodes(&self, query: &str, limit: Option<usize>, include_relations: bool) -> McpResult<KnowledgeGraph> {
        let graph = self.load_graph()?;

        // Expand query with synonyms for semantic matching
        let search_terms = get_synonyms(query);

        let mut matching_entities: Vec<Entity> = graph.entities
            .into_iter()
            .filter(|e| {
                matches_with_synonyms(&e.name, &search_terms) ||
                matches_with_synonyms(&e.entity_type, &search_terms) ||
                e.observations.iter().any(|o| matches_with_synonyms(o, &search_terms))
            })
            .collect();

        // Apply limit if specified
        if let Some(lim) = limit {
            matching_entities.truncate(lim);
        }

        let matching_relations = if include_relations {
            let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();
            graph.relations
                .into_iter()
                .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
                .collect()
        } else {
            Vec::new()
        };

        Ok(KnowledgeGraph {
            entities: matching_entities,
            relations: matching_relations,
        })
    }

    /// Open specific nodes by names
    pub fn open_nodes(&self, names: Vec<String>) -> McpResult<KnowledgeGraph> {
        let graph = self.load_graph()?;
        let name_set: HashSet<String> = names.into_iter().collect();

        let matching_entities: Vec<Entity> = graph.entities
            .into_iter()
            .filter(|e| name_set.contains(&e.name))
            .collect();

        let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();

        let matching_relations: Vec<Relation> = graph.relations
            .into_iter()
            .filter(|r| entity_names.contains(&r.from) && entity_names.contains(&r.to))
            .collect();

        Ok(KnowledgeGraph {
            entities: matching_entities,
            relations: matching_relations,
        })
    }

    /// Get related entities
    pub fn get_related(
        &self,
        entity_name: &str,
        relation_type: Option<&str>,
        direction: &str,
    ) -> McpResult<RelatedEntities> {
        let graph = self.load_graph()?;
        let mut related = Vec::new();

        for relation in &graph.relations {
            let matches = match direction {
                "outgoing" => relation.from == entity_name,
                "incoming" => relation.to == entity_name,
                "both" => relation.from == entity_name || relation.to == entity_name,
                _ => false,
            };

            if !matches {
                continue;
            }

            if let Some(rt) = relation_type {
                if relation.relation_type != rt {
                    continue;
                }
            }

            let target_name = if relation.from == entity_name {
                &relation.to
            } else {
                &relation.from
            };

            if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name) {
                related.push(RelatedEntity {
                    relation_type: relation.relation_type.clone(),
                    direction: if relation.from == entity_name {
                        "outgoing".to_string()
                    } else {
                        "incoming".to_string()
                    },
                    entity: entity.clone(),
                });
            }
        }

        Ok(RelatedEntities {
            entity: entity_name.to_string(),
            relations: related,
        })
    }

    /// Traverse graph following path pattern
    pub fn traverse(
        &self,
        start: &str,
        path: Vec<PathStep>,
        max_results: usize,
    ) -> McpResult<TraversalResult> {
        let graph = self.load_graph()?;

        // Track paths: (current_node, path_so_far, relations_so_far)
        let mut current_paths: Vec<(String, Vec<String>, Vec<String>)> =
            vec![(start.to_string(), vec![start.to_string()], vec![])];

        for step in &path {
            let mut next_paths = Vec::new();

            for (node, nodes_path, rels_path) in &current_paths {
                // Find related entities for this step
                for relation in &graph.relations {
                    let (matches, target_name) = match step.direction.as_str() {
                        "out" => {
                            if relation.from == *node && relation.relation_type == step.relation_type
                            {
                                (true, &relation.to)
                            } else {
                                (false, &relation.to)
                            }
                        }
                        "in" => {
                            if relation.to == *node && relation.relation_type == step.relation_type {
                                (true, &relation.from)
                            } else {
                                (false, &relation.from)
                            }
                        }
                        _ => (false, &relation.to),
                    };

                    if !matches {
                        continue;
                    }

                    // Check target type if specified
                    if let Some(ref target_type) = step.target_type {
                        if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name)
                        {
                            if &entity.entity_type != target_type {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    let mut new_nodes = nodes_path.clone();
                    new_nodes.push(target_name.clone());
                    let mut new_rels = rels_path.clone();
                    new_rels.push(step.relation_type.clone());

                    next_paths.push((target_name.clone(), new_nodes, new_rels));
                }
            }

            if next_paths.len() > max_results {
                next_paths.truncate(max_results);
            }

            current_paths = next_paths;
        }

        // Build result
        let mut paths = Vec::new();
        let mut end_node_names: HashSet<String> = HashSet::new();

        for (end_node, nodes, rels) in current_paths {
            end_node_names.insert(end_node);
            paths.push(TraversalPath {
                nodes,
                relations: rels,
            });
        }

        let end_nodes: Vec<Entity> = graph
            .entities
            .iter()
            .filter(|e| end_node_names.contains(&e.name))
            .cloned()
            .collect();

        Ok(TraversalResult {
            start_node: start.to_string(),
            paths,
            end_nodes,
        })
    }

    /// Summarize entities
    pub fn summarize(
        &self,
        entity_names: Option<Vec<String>>,
        entity_type: Option<String>,
        format: &str,
    ) -> McpResult<Summary> {
        let graph = self.load_graph()?;

        let entities: Vec<&Entity> = graph
            .entities
            .iter()
            .filter(|e| {
                if let Some(ref names) = entity_names {
                    names.contains(&e.name)
                } else if let Some(ref et) = entity_type {
                    &e.entity_type == et
                } else {
                    true
                }
            })
            .collect();

        match format {
            "brief" => self.format_brief(&entities),
            "detailed" => self.format_detailed(&entities),
            "stats" => self.format_stats(&entities),
            _ => self.format_brief(&entities),
        }
    }

    fn format_brief(&self, entities: &[&Entity]) -> McpResult<Summary> {
        let briefs: Vec<EntityBrief> = entities
            .iter()
            .map(|e| {
                let brief = e
                    .observations
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .chars()
                    .take(100)
                    .collect::<String>();
                EntityBrief {
                    name: e.name.clone(),
                    entity_type: e.entity_type.clone(),
                    brief,
                }
            })
            .collect();

        Ok(Summary {
            total_entities: entities.len(),
            entities: Some(briefs),
            ..Default::default()
        })
    }

    fn format_detailed(&self, entities: &[&Entity]) -> McpResult<Summary> {
        let briefs: Vec<EntityBrief> = entities
            .iter()
            .map(|e| {
                let brief = e.observations.join("; ");
                EntityBrief {
                    name: e.name.clone(),
                    entity_type: e.entity_type.clone(),
                    brief,
                }
            })
            .collect();

        Ok(Summary {
            total_entities: entities.len(),
            entities: Some(briefs),
            ..Default::default()
        })
    }

    fn format_stats(&self, entities: &[&Entity]) -> McpResult<Summary> {
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_priority: HashMap<String, usize> = HashMap::new();

        for entity in entities {
            *by_type.entry(entity.entity_type.clone()).or_insert(0) += 1;

            for obs in &entity.observations {
                if obs.starts_with("Status:") {
                    let status = obs.trim_start_matches("Status:").trim().to_string();
                    *by_status.entry(status).or_insert(0) += 1;
                }
                if obs.starts_with("Priority:") {
                    let priority = obs.trim_start_matches("Priority:").trim().to_string();
                    *by_priority.entry(priority).or_insert(0) += 1;
                }
            }
        }

        Ok(Summary {
            total_entities: entities.len(),
            entities: None,
            by_status: if by_status.is_empty() {
                None
            } else {
                Some(by_status)
            },
            by_type: Some(by_type),
            by_priority: if by_priority.is_empty() {
                None
            } else {
                Some(by_priority)
            },
        })
    }

    /// Get relations valid at a specific point in time
    pub fn get_relations_at_time(&self, timestamp: Option<u64>, entity_name: Option<&str>) -> McpResult<Vec<Relation>> {
        let graph = self.load_graph()?;
        let check_time = timestamp.unwrap_or_else(current_timestamp);

        let relations: Vec<Relation> = graph.relations
            .into_iter()
            .filter(|r| {
                // Filter by entity if specified
                if let Some(name) = entity_name {
                    if r.from != name && r.to != name {
                        return false;
                    }
                }

                // Check temporal validity
                let valid_from_ok = match r.valid_from {
                    Some(vf) => check_time >= vf,
                    None => true, // No start time means always valid from past
                };

                let valid_to_ok = match r.valid_to {
                    Some(vt) => check_time <= vt,
                    None => true, // No end time means still valid
                };

                valid_from_ok && valid_to_ok
            })
            .collect();

        Ok(relations)
    }

    /// Get historical relations (including expired ones)
    pub fn get_relation_history(&self, entity_name: &str) -> McpResult<Vec<Relation>> {
        let graph = self.load_graph()?;

        let relations: Vec<Relation> = graph.relations
            .into_iter()
            .filter(|r| r.from == entity_name || r.to == entity_name)
            .collect();

        Ok(relations)
    }
}

// ============================================================================
// Memory Tools Implementation
// ============================================================================

pub struct CreateEntitiesTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl CreateEntitiesTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_entities".to_string(),
            description: "Create multiple new entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entities": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "The name of the entity" },
                                "entityType": { "type": "string", "description": "The type of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Initial observations about the entity"
                                },
                                "createdBy": { "type": "string", "description": "Who created this entity (auto-filled from git/env if not provided)" },
                                "updatedBy": { "type": "string", "description": "Who last updated this entity (auto-filled from git/env if not provided)" }
                            },
                            "required": ["name", "entityType"]
                        }
                    }
                },
                "required": ["entities"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entities: Vec<Entity> = serde_json::from_value(
            params.get("entities").cloned().unwrap_or(json!([]))
        )?;

        // Collect warnings for non-standard types
        let warnings: Vec<String> = entities.iter()
            .filter_map(|e| validate_entity_type(&e.entity_type))
            .collect();

        let created = self.kb.create_entities(entities)?;

        let response = if warnings.is_empty() {
            serde_json::to_string_pretty(&created)?
        } else {
            format!("{}\n\n{}", serde_json::to_string_pretty(&created)?, warnings.join("\n"))
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": response
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct CreateRelationsTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl CreateRelationsTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_relations".to_string(),
            description: "Create multiple new relations between entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" },
                                "createdBy": { "type": "string", "description": "Who created this relation (auto-filled from git/env if not provided)" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let relations: Vec<Relation> = serde_json::from_value(
            params.get("relations").cloned().unwrap_or(json!([]))
        )?;

        // Collect warnings for non-standard relation types
        let warnings: Vec<String> = relations.iter()
            .filter_map(|r| validate_relation_type(&r.relation_type))
            .collect();

        let created = self.kb.create_relations(relations)?;

        let response = if warnings.is_empty() {
            serde_json::to_string_pretty(&created)?
        } else {
            format!("{}\n\n{}", serde_json::to_string_pretty(&created)?, warnings.join("\n"))
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": response
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct AddObservationsTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl AddObservationsTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for AddObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "add_observations".to_string(),
            description: "Add new observations to existing entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "observations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "contents": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observation contents to add"
                                }
                            },
                            "required": ["entityName", "contents"]
                        }
                    }
                },
                "required": ["observations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let observations: Vec<Observation> = serde_json::from_value(
            params.get("observations").cloned().unwrap_or(json!([]))
        )?;
        let added = self.kb.add_observations(observations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&added)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct DeleteEntitiesTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl DeleteEntitiesTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_entities".to_string(),
            description: "Delete multiple entities from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "An array of entity names to delete"
                    }
                },
                "required": ["entityNames"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Vec<String> = serde_json::from_value(
            params.get("entityNames").cloned().unwrap_or(json!([]))
        )?;
        self.kb.delete_entities(entity_names)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Entities deleted successfully"
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct DeleteObservationsTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl DeleteObservationsTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_observations".to_string(),
            description: "Delete specific observations from entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deletions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observations to delete"
                                }
                            },
                            "required": ["entityName", "observations"]
                        }
                    }
                },
                "required": ["deletions"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let deletions: Vec<ObservationDeletion> = serde_json::from_value(
            params.get("deletions").cloned().unwrap_or(json!([]))
        )?;
        self.kb.delete_observations(deletions)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Observations deleted successfully"
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct DeleteRelationsTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl DeleteRelationsTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_relations".to_string(),
            description: "Delete multiple relations from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let relations: Vec<Relation> = serde_json::from_value(
            params.get("relations").cloned().unwrap_or(json!([]))
        )?;
        self.kb.delete_relations(relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Relations deleted successfully"
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct ReadGraphTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl ReadGraphTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for ReadGraphTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "read_graph".to_string(),
            description: "Read the knowledge graph with optional pagination. Use limit/offset to avoid context overflow with large graphs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return. Recommended: 50-100 for large graphs"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of entities to skip (for pagination)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
        let offset = params.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
        let graph = self.kb.read_graph(limit, offset)?;

        let total_msg = if limit.is_some() || offset.is_some() {
            format!(" (showing {} entities)", graph.entities.len())
        } else {
            String::new()
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("{}{}", serde_json::to_string_pretty(&graph)?, total_msg)
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct SearchNodesTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl SearchNodesTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SearchNodesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "search_nodes".to_string(),
            description: "Search for nodes in the knowledge graph. Returns matching entities with optional relations.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to match against entity names, types, and observations"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return (default: no limit)"
                    },
                    "includeRelations": {
                        "type": "boolean",
                        "description": "Whether to include relations connected to matching entities (default: true)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
        let include_relations = params.get("includeRelations")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let graph = self.kb.search_nodes(query, limit, include_relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------

pub struct OpenNodesTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl OpenNodesTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for OpenNodesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "open_nodes".to_string(),
            description: "Open specific nodes in the knowledge graph by their names".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "names": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "An array of entity names to retrieve"
                    }
                },
                "required": ["names"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let names: Vec<String> = serde_json::from_value(
            params.get("names").cloned().unwrap_or(json!([]))
        )?;
        let graph = self.kb.open_nodes(names)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------
// Get Related Tool
// ----------------------------------------------------------------------------

pub struct GetRelatedTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl GetRelatedTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelatedTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_related".to_string(),
            description: "Get entities related to a specific entity".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "Name of the entity to find relations for"
                    },
                    "relationType": {
                        "type": "string",
                        "description": "Filter by relation type (optional)"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["outgoing", "incoming", "both"],
                        "default": "both",
                        "description": "Direction of relations"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params
            .get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("Missing entityName")?;
        let relation_type = params.get("relationType").and_then(|v| v.as_str());
        let direction = params
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        let related = self.kb.get_related(entity_name, relation_type, direction)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&related)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------
// Traverse Tool
// ----------------------------------------------------------------------------

pub struct TraverseTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl TraverseTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for TraverseTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "traverse".to_string(),
            description: "Traverse the graph following a path pattern for multi-hop queries"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "startNode": {
                        "type": "string",
                        "description": "Starting entity name"
                    },
                    "path": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "relationType": {
                                    "type": "string",
                                    "description": "Type of relation to follow"
                                },
                                "direction": {
                                    "type": "string",
                                    "enum": ["out", "in"],
                                    "description": "Direction: out (outgoing) or in (incoming)"
                                },
                                "targetType": {
                                    "type": "string",
                                    "description": "Filter by target entity type (optional)"
                                }
                            },
                            "required": ["relationType", "direction"]
                        },
                        "description": "Path pattern to follow"
                    },
                    "maxResults": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum number of results"
                    }
                },
                "required": ["startNode", "path"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let start_node = params
            .get("startNode")
            .and_then(|v| v.as_str())
            .ok_or("Missing startNode")?;

        let path: Vec<PathStep> = serde_json::from_value(
            params.get("path").cloned().unwrap_or(json!([]))
        )?;

        let max_results = params
            .get("maxResults")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let result = self.kb.traverse(start_node, path, max_results)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------
// Summarize Tool
// ----------------------------------------------------------------------------

pub struct SummarizeTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl SummarizeTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SummarizeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "summarize".to_string(),
            description: "Get a condensed summary of entities".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Specific entities to summarize (optional)"
                    },
                    "entityType": {
                        "type": "string",
                        "description": "Summarize all entities of this type (optional)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["brief", "detailed", "stats"],
                        "default": "brief",
                        "description": "Output format: brief (first observation), detailed (all observations), stats (statistics)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Option<Vec<String>> = params
            .get("entityNames")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let entity_type = params
            .get("entityType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("brief");

        let summary = self.kb.summarize(entity_names, entity_type, format)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&summary)?
            }]
        }))
    }
}

// ----------------------------------------------------------------------------
// Time Tool
// ----------------------------------------------------------------------------

fn get_current_time() -> Value {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap();
    let timestamp = duration.as_secs();
    let millis = duration.as_millis() as u64;

    // Calculate datetime components
    let secs = timestamp as i64;

    // Days since epoch
    let days = secs / 86400;
    let remaining = secs % 86400;

    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate year, month, day
    let (year, month, day) = days_to_ymd(days);

    // Format ISO 8601
    let iso8601 = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    );

    // Format readable
    let weekday = get_weekday(days);
    let month_name = get_month_name(month);
    let readable = format!(
        "{}, {} {} {} {:02}:{:02}:{:02} UTC",
        weekday, day, month_name, year, hours, minutes, seconds
    );

    json!({
        "timestamp": timestamp,
        "timestamp_ms": millis,
        "iso8601": iso8601,
        "readable": readable,
        "components": {
            "year": year,
            "month": month,
            "day": day,
            "hour": hours,
            "minute": minutes,
            "second": seconds,
            "weekday": weekday
        }
    })
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm to convert days since epoch to year/month/day
    let remaining_days = days + 719468; // Days from year 0 to 1970

    let era = remaining_days / 146097;
    let doe = remaining_days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };

    (year, month, day)
}

fn get_weekday(days: i64) -> &'static str {
    match (days + 4) % 7 {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

fn get_month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

// ----------------------------------------------------------------------------
// Temporal Query Tools
// ----------------------------------------------------------------------------

pub struct GetRelationsAtTimeTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl GetRelationsAtTimeTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationsAtTimeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relations_at_time".to_string(),
            description: "Get relations that are valid at a specific point in time. Useful for querying historical state of the knowledge graph.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timestamp": {
                        "type": "integer",
                        "description": "Unix timestamp to query. If not provided, uses current time."
                    },
                    "entityName": {
                        "type": "string",
                        "description": "Optional: filter relations involving this entity"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let timestamp = params.get("timestamp").and_then(|v| v.as_u64());
        let entity_name = params.get("entityName").and_then(|v| v.as_str());

        let relations = self.kb.get_relations_at_time(timestamp, entity_name)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "queryTime": timestamp.unwrap_or_else(current_timestamp),
                    "relations": relations
                }))?
            }]
        }))
    }
}

pub struct GetRelationHistoryTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl GetRelationHistoryTool {
    pub fn new(kb: std::sync::Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationHistoryTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relation_history".to_string(),
            description: "Get all relations (current and historical) for an entity. Shows temporal validity (validFrom/validTo) for each relation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "The name of the entity to get relation history for"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params.get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("entityName is required")?;

        let relations = self.kb.get_relation_history(entity_name)?;
        let current_time = current_timestamp();

        // Mark each relation as current or historical
        let annotated: Vec<Value> = relations.iter().map(|r| {
            let is_current = match (r.valid_from, r.valid_to) {
                (Some(vf), Some(vt)) => current_time >= vf && current_time <= vt,
                (Some(vf), None) => current_time >= vf,
                (None, Some(vt)) => current_time <= vt,
                (None, None) => true,
            };

            json!({
                "from": r.from,
                "to": r.to,
                "relationType": r.relation_type,
                "validFrom": r.valid_from,
                "validTo": r.valid_to,
                "isCurrent": is_current
            })
        }).collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "entity": entity_name,
                    "currentTime": current_time,
                    "relations": annotated
                }))?
            }]
        }))
    }
}

pub struct GetCurrentTimeTool;

impl GetCurrentTimeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetCurrentTimeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_current_time".to_string(),
            description: "Get the current datetime and timestamp".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    fn execute(&self, _params: Value) -> McpResult<Value> {
        let time_info = get_current_time();
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&time_info)?
            }]
        }))
    }
}

// ============================================================================
// MCP Server
// ============================================================================

pub struct McpServer {
    server_info: ServerInfo,
    tools: HashMap<String, Box<dyn Tool>>,
    reader: BufReader<io::Stdin>,
    writer: BufWriter<io::Stdout>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            server_info: ServerInfo {
                name: "memory".to_string(),
                version: "1.0.0".to_string(),
            },
            tools: HashMap::new(),
            reader: BufReader::new(io::stdin()),
            writer: BufWriter::new(io::stdout()),
        }
    }

    pub fn with_info(info: ServerInfo) -> Self {
        Self {
            server_info: info,
            tools: HashMap::new(),
            reader: BufReader::new(io::stdin()),
            writer: BufWriter::new(io::stdout()),
        }
    }

    pub fn register_tool(&mut self, tool: Box<dyn Tool>) -> &mut Self {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
        self
    }

    pub fn run(&mut self) -> McpResult<()> {
        let mut line = String::new();
        while self.reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.handle_request(trimmed)?;
            }
            line.clear();
        }
        Ok(())
    }

    fn handle_request(&mut self, request_str: &str) -> McpResult<()> {
        let request: JsonRpcRequest = match serde_json::from_str(request_str) {
            Ok(req) => req,
            Err(e) => {
                self.send_error_response(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!({"details": e.to_string()})),
                )?;
                return Ok(());
            }
        };

        if request.jsonrpc != "2.0" {
            self.send_error_response(
                request.id.unwrap_or(Value::Null),
                -32600,
                "Invalid Request",
                Some(json!({"details": "jsonrpc must be '2.0'"})),
            )?;
            return Ok(());
        }

        let id = request.id.clone().unwrap_or(Value::Null);

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params),
            "notifications/initialized" => Ok(()), // Notification, no response
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tool_call(id, request.params),
            "ping" => self.send_success_response(id, json!({})),
            _ => self.send_error_response(
                id,
                -32601,
                "Method not found",
                Some(json!({"method": request.method})),
            ),
        }
    }

    fn handle_initialize(&mut self, id: Value, _params: Option<Value>) -> McpResult<()> {
        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.server_info.name,
                "version": self.server_info.version
            }
        });
        self.send_success_response(id, result)
    }

    fn handle_tools_list(&mut self, id: Value) -> McpResult<()> {
        let tools: Vec<McpTool> = self.tools.values().map(|t| t.definition()).collect();
        let result = json!({ "tools": tools });
        self.send_success_response(id, result)
    }

    fn handle_tool_call(&mut self, id: Value, params: Option<Value>) -> McpResult<()> {
        let params = params.ok_or("Missing parameters")?;
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing tool name")?;

        let tool = match self.tools.get(tool_name) {
            Some(tool) => tool,
            None => {
                self.send_error_response(
                    id,
                    -32602,
                    "Unknown tool",
                    Some(json!({"tool": tool_name})),
                )?;
                return Ok(());
            }
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        match tool.execute(arguments) {
            Ok(result) => self.send_success_response(id, result),
            Err(e) => self.send_error_response(
                id,
                -32603,
                "Tool execution error",
                Some(json!({"details": e.to_string()})),
            ),
        }
    }

    fn send_success_response(&mut self, id: Value, result: Value) -> McpResult<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        };
        let json = serde_json::to_string(&response)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    fn send_error_response(
        &mut self,
        id: Value,
        code: i32,
        message: &str,
        data: Option<Value>,
    ) -> McpResult<()> {
        let response = JsonRpcError {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject {
                code,
                message: message.to_string(),
                data,
            },
        };
        let json = serde_json::to_string(&response)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() -> McpResult<()> {
    let kb = std::sync::Arc::new(KnowledgeBase::new());

    let server_info = ServerInfo {
        name: "memory".to_string(),
        version: "1.0.0".to_string(),
    };

    let mut server = McpServer::with_info(server_info);

    // Register all 9 memory tools + 3 query tools + 2 temporal tools + 1 time tool
    server.register_tool(Box::new(CreateEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(CreateRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(AddObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(ReadGraphTool::new(kb.clone())));
    server.register_tool(Box::new(SearchNodesTool::new(kb.clone())));
    server.register_tool(Box::new(OpenNodesTool::new(kb.clone())));
    // Query tools
    server.register_tool(Box::new(GetRelatedTool::new(kb.clone())));
    server.register_tool(Box::new(TraverseTool::new(kb.clone())));
    server.register_tool(Box::new(SummarizeTool::new(kb.clone())));
    // Temporal tools
    server.register_tool(Box::new(GetRelationsAtTimeTool::new(kb.clone())));
    server.register_tool(Box::new(GetRelationHistoryTool::new(kb.clone())));
    // Time tool
    server.register_tool(Box::new(GetCurrentTimeTool::new()));

    server.run()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn setup_test_kb() -> (KnowledgeBase, String) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_file = format!("test_memory_{}_{}.jsonl", std::process::id(), id);

        // Create a KnowledgeBase with explicit file path and empty graph
        let kb = KnowledgeBase {
            memory_file_path: temp_file.clone(),
            graph: Mutex::new(KnowledgeGraph::default()),
            current_user: "test_user".to_string(),
        };
        (kb, temp_file)
    }

    fn cleanup(file_path: &str) {
        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn test_create_entities() {
        let (kb, temp_file) = setup_test_kb();

        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                entity_type: "Person".to_string(),
                observations: vec!["Lives in NYC".to_string()],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];

        let created = kb.create_entities(entities).unwrap();
        assert_eq!(created.len(), 2);
        // Verify user was auto-filled
        assert_eq!(created[0].created_by, "test_user");
        assert_eq!(created[0].updated_by, "test_user");

        let graph = kb.read_graph(None, None).unwrap();
        assert_eq!(graph.entities.len(), 2);

        cleanup(&temp_file);
    }

    #[test]
    fn test_create_relations() {
        let (kb, temp_file) = setup_test_kb();

        // First create entities
        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        kb.create_entities(entities).unwrap();

        // Then create relations
        let relations = vec![
            Relation {
                from: "Alice".to_string(),
                to: "Bob".to_string(),
                relation_type: "knows".to_string(),
                created_by: String::new(),
                created_at: 0,
                valid_from: None,
                valid_to: None,
            },
        ];

        let created = kb.create_relations(relations).unwrap();
        assert_eq!(created.len(), 1);
        // Verify user was auto-filled
        assert_eq!(created[0].created_by, "test_user");

        let graph = kb.read_graph(None, None).unwrap();
        assert_eq!(graph.relations.len(), 1);

        cleanup(&temp_file);
    }

    #[test]
    fn test_search_nodes() {
        let (kb, temp_file) = setup_test_kb();

        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                entity_type: "Person".to_string(),
                observations: vec!["Software Engineer".to_string()],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec!["Doctor".to_string()],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        kb.create_entities(entities).unwrap();

        let result = kb.search_nodes("Alice", None, true).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Alice");

        let result = kb.search_nodes("Engineer", None, true).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Alice");

        cleanup(&temp_file);
    }

    #[test]
    fn test_delete_entities() {
        let (kb, temp_file) = setup_test_kb();

        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        kb.create_entities(entities).unwrap();

        kb.delete_entities(vec!["Alice".to_string()]).unwrap();

        let graph = kb.read_graph(None, None).unwrap();
        assert_eq!(graph.entities.len(), 1);
        assert_eq!(graph.entities[0].name, "Bob");

        cleanup(&temp_file);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let (kb, temp_file) = setup_test_kb();
        let kb = Arc::new(kb);

        // Spawn multiple threads simulating concurrent agents
        let mut handles = vec![];

        for i in 0..10 {
            let kb_clone = Arc::clone(&kb);
            let handle = thread::spawn(move || {
                // Each "agent" creates an entity
                let entity = Entity {
                    name: format!("Agent{}", i),
                    entity_type: "Person".to_string(),
                    observations: vec![format!("Created by thread {}", i)],
                    created_by: String::new(),
                    updated_by: String::new(),
                    created_at: 0,
                    updated_at: 0,
                };
                kb_clone.create_entities(vec![entity]).unwrap();

                // Each agent also reads the graph
                let graph = kb_clone.read_graph(None, None).unwrap();
                assert!(graph.entities.len() >= 1);

                // Each agent adds an observation
                let obs = Observation {
                    entity_name: format!("Agent{}", i),
                    contents: vec![format!("Observation from thread {}", i)],
                };
                let _ = kb_clone.add_observations(vec![obs]);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Verify final state
        let graph = kb.read_graph(None, None).unwrap();
        assert_eq!(graph.entities.len(), 10, "All 10 entities should exist");

        // Verify all entities have observations
        for entity in &graph.entities {
            assert!(entity.observations.len() >= 1, "Entity should have observations");
        }

        cleanup(&temp_file);
    }

    #[test]
    fn test_concurrent_read_write() {
        use std::sync::Arc;
        use std::thread;

        let (kb, temp_file) = setup_test_kb();

        // Pre-populate with some entities
        for i in 0..5 {
            let entity = Entity {
                name: format!("Entity{}", i),
                entity_type: "Module".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            };
            kb.create_entities(vec![entity]).unwrap();
        }

        let kb = Arc::new(kb);
        let mut handles = vec![];

        // 5 reader threads
        for _ in 0..5 {
            let kb_clone = Arc::clone(&kb);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let graph = kb_clone.read_graph(None, None).unwrap();
                    assert!(graph.entities.len() >= 5);
                    let _ = kb_clone.search_nodes("Entity", None, true);
                }
            });
            handles.push(handle);
        }

        // 3 writer threads
        for i in 0..3 {
            let kb_clone = Arc::clone(&kb);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let obs = Observation {
                        entity_name: format!("Entity{}", i),
                        contents: vec![format!("Update {} from writer {}", j, i)],
                    };
                    let _ = kb_clone.add_observations(vec![obs]);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Verify no data corruption
        let graph = kb.read_graph(None, None).unwrap();
        assert_eq!(graph.entities.len(), 5, "Original entities should still exist");

        cleanup(&temp_file);
    }
}
