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
use std::sync::Mutex;

// ============================================================================
// Types
// ============================================================================

pub type McpResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
}

/// Relation between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
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

pub struct KnowledgeBase {
    memory_file_path: Mutex<String>,
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

        Self {
            memory_file_path: Mutex::new(memory_file_path),
        }
    }

    fn get_memory_file_path(&self) -> String {
        self.memory_file_path.lock().unwrap().clone()
    }

    fn load_graph(&self) -> McpResult<KnowledgeGraph> {
        let file_path = self.get_memory_file_path();

        if !Path::new(&file_path).exists() {
            return Ok(KnowledgeGraph::default());
        }

        let content = fs::read_to_string(&file_path)?;
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

    fn save_graph(&self, graph: &KnowledgeGraph) -> McpResult<()> {
        let file_path = self.get_memory_file_path();

        // Ensure parent directory exists
        if let Some(parent) = Path::new(&file_path).parent() {
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

        fs::write(&file_path, content)?;
        Ok(())
    }

    /// Create new entities
    pub fn create_entities(&self, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
        let mut graph = self.load_graph()?;
        let existing_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();

        let mut created = Vec::new();
        for entity in entities {
            if !existing_names.contains(&entity.name) {
                created.push(entity.clone());
                graph.entities.push(entity);
            }
        }

        self.save_graph(&graph)?;
        Ok(created)
    }

    /// Create new relations
    pub fn create_relations(&self, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
        let mut graph = self.load_graph()?;
        let entity_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();

        let existing_relations: HashSet<String> = graph.relations
            .iter()
            .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
            .collect();

        let mut created = Vec::new();
        for relation in relations {
            // Only create relation if both entities exist and relation doesn't already exist
            if entity_names.contains(&relation.from) && entity_names.contains(&relation.to) {
                let key = format!("{}|{}|{}", relation.from, relation.to, relation.relation_type);
                if !existing_relations.contains(&key) {
                    created.push(relation.clone());
                    graph.relations.push(relation);
                }
            }
        }

        self.save_graph(&graph)?;
        Ok(created)
    }

    /// Add observations to entities
    pub fn add_observations(&self, observations: Vec<Observation>) -> McpResult<Vec<Observation>> {
        let mut graph = self.load_graph()?;
        let mut added = Vec::new();

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
                    added.push(Observation {
                        entity_name: obs.entity_name.clone(),
                        contents: new_contents,
                    });
                }
            }
        }

        self.save_graph(&graph)?;
        Ok(added)
    }

    /// Delete entities
    pub fn delete_entities(&self, entity_names: Vec<String>) -> McpResult<()> {
        let mut graph = self.load_graph()?;
        let names_to_delete: HashSet<String> = entity_names.into_iter().collect();

        graph.entities.retain(|e| !names_to_delete.contains(&e.name));

        // Also remove relations involving deleted entities
        graph.relations.retain(|r| {
            !names_to_delete.contains(&r.from) && !names_to_delete.contains(&r.to)
        });

        self.save_graph(&graph)?;
        Ok(())
    }

    /// Delete observations from entities
    pub fn delete_observations(&self, deletions: Vec<ObservationDeletion>) -> McpResult<()> {
        let mut graph = self.load_graph()?;

        for deletion in deletions {
            if let Some(entity) = graph.entities.iter_mut().find(|e| e.name == deletion.entity_name) {
                let to_remove: HashSet<String> = deletion.observations.into_iter().collect();
                entity.observations.retain(|o| !to_remove.contains(o));
            }
        }

        self.save_graph(&graph)?;
        Ok(())
    }

    /// Delete relations
    pub fn delete_relations(&self, relations: Vec<Relation>) -> McpResult<()> {
        let mut graph = self.load_graph()?;

        let to_delete: HashSet<String> = relations
            .iter()
            .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
            .collect();

        graph.relations.retain(|r| {
            let key = format!("{}|{}|{}", r.from, r.to, r.relation_type);
            !to_delete.contains(&key)
        });

        self.save_graph(&graph)?;
        Ok(())
    }

    /// Read entire graph
    pub fn read_graph(&self) -> McpResult<KnowledgeGraph> {
        self.load_graph()
    }

    /// Search nodes by query
    pub fn search_nodes(&self, query: &str) -> McpResult<KnowledgeGraph> {
        let graph = self.load_graph()?;
        let query_lower = query.to_lowercase();

        let matching_entities: Vec<Entity> = graph.entities
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query_lower) ||
                e.entity_type.to_lowercase().contains(&query_lower) ||
                e.observations.iter().any(|o| o.to_lowercase().contains(&query_lower))
            })
            .collect();

        let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();

        let matching_relations: Vec<Relation> = graph.relations
            .into_iter()
            .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
            .collect();

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
                                }
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
        let created = self.kb.create_entities(entities)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&created)?
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
        let created = self.kb.create_relations(relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&created)?
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
            description: "Read the entire knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    fn execute(&self, _params: Value) -> McpResult<Value> {
        let graph = self.kb.read_graph()?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
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
            description: "Search for nodes in the knowledge graph based on a query".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to match against entity names, types, and observations"
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
        let graph = self.kb.search_nodes(query)?;
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

    // Register all 9 memory tools
    server.register_tool(Box::new(CreateEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(CreateRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(AddObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(ReadGraphTool::new(kb.clone())));
    server.register_tool(Box::new(SearchNodesTool::new(kb.clone())));
    server.register_tool(Box::new(OpenNodesTool::new(kb.clone())));

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

        // Create a KnowledgeBase with explicit file path
        let kb = KnowledgeBase {
            memory_file_path: Mutex::new(temp_file.clone()),
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
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
            },
        ];

        let created = kb.create_entities(entities).unwrap();
        assert_eq!(created.len(), 2);

        let graph = kb.read_graph().unwrap();
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
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
            },
        ];
        kb.create_entities(entities).unwrap();

        // Then create relations
        let relations = vec![
            Relation {
                from: "Alice".to_string(),
                to: "Bob".to_string(),
                relation_type: "knows".to_string(),
            },
        ];

        let created = kb.create_relations(relations).unwrap();
        assert_eq!(created.len(), 1);

        let graph = kb.read_graph().unwrap();
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
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec!["Doctor".to_string()],
            },
        ];
        kb.create_entities(entities).unwrap();

        let result = kb.search_nodes("Alice").unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Alice");

        let result = kb.search_nodes("Engineer").unwrap();
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
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
            },
        ];
        kb.create_entities(entities).unwrap();

        kb.delete_entities(vec!["Alice".to_string()]).unwrap();

        let graph = kb.read_graph().unwrap();
        assert_eq!(graph.entities.len(), 1);
        assert_eq!(graph.entities[0].name, "Bob");

        cleanup(&temp_file);
    }
}
