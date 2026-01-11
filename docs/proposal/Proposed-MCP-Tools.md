# Proposed MCP Tools

> **Status**: Proposed
> **Created**: 2026-01-11
> **Author**: tiach

## 📋 Overview

Đề xuất 3 MCP tools mới để enhance khả năng query của Memory Graph:

| Tool | Priority | Mục đích |
|------|----------|----------|
| `get_related` | High | Lấy entities có relation với entity cho trước |
| `traverse` | Medium | Multi-hop graph traversal |
| `summarize` | Low | Tóm tắt subset của graph |

---

## 1. `get_related` Tool

### Purpose

Tìm tất cả entities có relationship với một entity cụ thể.

### Current Problem

```
Hiện tại để tìm "tất cả files trong Module: Payment":
1. read_graph() → load toàn bộ graph
2. Filter manually trong response
3. Tốn tokens, chậm, phức tạp
```

### Proposed Solution

```json
{
  "name": "get_related",
  "description": "Get entities related to a specific entity",
  "inputSchema": {
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
      },
      "maxDepth": {
        "type": "integer",
        "default": 1,
        "description": "Depth of traversal (1 = direct relations only)"
      }
    },
    "required": ["entityName"]
  }
}
```

### Examples

**Example 1: Find all files in a module**
```json
{
  "name": "get_related",
  "arguments": {
    "entityName": "Module: Payment",
    "relationType": "contains",
    "direction": "outgoing"
  }
}
```

Response:
```json
{
  "entity": "Module: Payment",
  "relations": [
    {
      "relationType": "contains",
      "direction": "outgoing",
      "target": {
        "name": "File: processor.rs",
        "entityType": "File",
        "observations": ["..."]
      }
    },
    {
      "relationType": "contains",
      "direction": "outgoing",
      "target": {
        "name": "File: webhook.rs",
        "entityType": "File",
        "observations": ["..."]
      }
    }
  ]
}
```

**Example 2: Find what depends on a module**
```json
{
  "name": "get_related",
  "arguments": {
    "entityName": "Module: Auth",
    "relationType": "depends_on",
    "direction": "incoming"
  }
}
```

### Implementation (Rust)

```rust
pub struct GetRelatedTool {
    kb: Arc<KnowledgeBase>,
}

impl Tool for GetRelatedTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_related".to_string(),
            description: "Get entities related to a specific entity".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": { "type": "string" },
                    "relationType": { "type": "string" },
                    "direction": {
                        "type": "string",
                        "enum": ["outgoing", "incoming", "both"],
                        "default": "both"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params["entityName"].as_str().unwrap();
        let relation_type = params.get("relationType").and_then(|v| v.as_str());
        let direction = params.get("direction")
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

impl KnowledgeBase {
    pub fn get_related(
        &self,
        entity_name: &str,
        relation_type: Option<&str>,
        direction: &str
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
                    direction: if relation.from == entity_name { "outgoing" } else { "incoming" }.to_string(),
                    entity: entity.clone(),
                });
            }
        }

        Ok(RelatedEntities {
            entity: entity_name.to_string(),
            relations: related,
        })
    }
}
```

---

## 2. `traverse` Tool

### Purpose

Thực hiện multi-hop traversal trên graph để tìm transitive relationships.

### Current Problem

```
Để tìm "tất cả schemas mà Module: Payment phụ thuộc vào":
1. get Module: Payment
2. Tìm files trong module
3. Tìm functions trong files
4. Tìm schemas mà functions sử dụng
→ Phải gọi nhiều tools, logic phức tạp
```

### Proposed Solution

```json
{
  "name": "traverse",
  "description": "Traverse the graph following a path pattern",
  "inputSchema": {
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
            "relationType": { "type": "string" },
            "direction": { "type": "string", "enum": ["out", "in"] },
            "targetType": { "type": "string" }
          }
        },
        "description": "Path pattern to follow"
      },
      "maxResults": {
        "type": "integer",
        "default": 50
      }
    },
    "required": ["startNode", "path"]
  }
}
```

### Examples

**Example 1: Find all schemas used by a module**
```json
{
  "name": "traverse",
  "arguments": {
    "startNode": "Module: Payment",
    "path": [
      { "relationType": "contains", "direction": "out", "targetType": "File" },
      { "relationType": "contains", "direction": "out", "targetType": "Function" },
      { "relationType": "uses", "direction": "out", "targetType": "Schema" }
    ]
  }
}
```

Response:
```json
{
  "startNode": "Module: Payment",
  "paths": [
    {
      "nodes": [
        "Module: Payment",
        "File: processor.rs",
        "Function: process_payment",
        "Schema: orders"
      ],
      "relations": ["contains", "contains", "uses"]
    },
    {
      "nodes": [
        "Module: Payment",
        "File: processor.rs",
        "Function: process_payment",
        "Schema: transactions"
      ],
      "relations": ["contains", "contains", "uses"]
    }
  ],
  "endNodes": [
    { "name": "Schema: orders", "entityType": "Schema", "observations": [...] },
    { "name": "Schema: transactions", "entityType": "Schema", "observations": [...] }
  ]
}
```

**Example 2: Impact analysis - what features are affected by a schema change**
```json
{
  "name": "traverse",
  "arguments": {
    "startNode": "Schema: orders",
    "path": [
      { "relationType": "uses", "direction": "in", "targetType": "Function" },
      { "relationType": "implements", "direction": "out", "targetType": "Feature" }
    ]
  }
}
```

### Implementation Notes

```rust
pub fn traverse(
    &self,
    start: &str,
    path: Vec<PathStep>,
    max_results: usize
) -> McpResult<TraversalResult> {
    let graph = self.load_graph()?;

    // BFS/DFS traversal following path pattern
    let mut current_nodes = vec![start.to_string()];
    let mut all_paths = Vec::new();

    for step in &path {
        let mut next_nodes = Vec::new();

        for node in &current_nodes {
            let related = self.get_related_internal(
                &graph,
                node,
                Some(&step.relation_type),
                &step.direction
            );

            for rel in related {
                if let Some(ref target_type) = step.target_type {
                    if rel.entity.entity_type != *target_type {
                        continue;
                    }
                }
                next_nodes.push(rel.entity.name.clone());
            }
        }

        current_nodes = next_nodes;

        if current_nodes.len() > max_results {
            current_nodes.truncate(max_results);
        }
    }

    // Build result with paths and end nodes
    Ok(TraversalResult {
        start_node: start.to_string(),
        paths: all_paths,
        end_nodes: self.get_entities_by_names(&current_nodes)?,
    })
}
```

---

## 3. `summarize` Tool

### Purpose

Tạo summary ngắn gọn của một subset entities.

### Use Cases

- Quick project overview
- Module summary trước khi làm việc
- Status report của features/milestones

### Proposed Solution

```json
{
  "name": "summarize",
  "description": "Get a condensed summary of entities",
  "inputSchema": {
    "type": "object",
    "properties": {
      "entityNames": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Specific entities to summarize"
      },
      "entityType": {
        "type": "string",
        "description": "Summarize all entities of this type"
      },
      "format": {
        "type": "string",
        "enum": ["brief", "detailed", "stats"],
        "default": "brief"
      }
    }
  }
}
```

### Examples

**Example 1: Project overview**
```json
{
  "name": "summarize",
  "arguments": {
    "entityType": "Module",
    "format": "brief"
  }
}
```

Response:
```json
{
  "summary": {
    "totalEntities": 5,
    "entities": [
      { "name": "Module: API", "brief": "REST endpoints, axum framework" },
      { "name": "Module: Auth", "brief": "JWT authentication, user management" },
      { "name": "Module: Payment", "brief": "Stripe, PayPal integration" },
      { "name": "Module: Order", "brief": "Order processing, inventory" },
      { "name": "Module: Notification", "brief": "Email, SMS, push notifications" }
    ]
  }
}
```

**Example 2: Feature status**
```json
{
  "name": "summarize",
  "arguments": {
    "entityType": "Feature",
    "format": "stats"
  }
}
```

Response:
```json
{
  "summary": {
    "totalEntities": 12,
    "byStatus": {
      "Completed": 5,
      "In Progress": 3,
      "Planned": 4
    },
    "byPriority": {
      "Critical": 2,
      "High": 5,
      "Medium": 3,
      "Low": 2
    }
  }
}
```

### Implementation Notes

```rust
pub fn summarize(
    &self,
    entity_names: Option<Vec<String>>,
    entity_type: Option<String>,
    format: &str
) -> McpResult<Summary> {
    let graph = self.load_graph()?;

    let entities: Vec<&Entity> = graph.entities.iter()
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

fn format_stats(&self, entities: &[&Entity]) -> McpResult<Summary> {
    let mut by_status: HashMap<String, usize> = HashMap::new();

    for entity in entities {
        for obs in &entity.observations {
            if obs.starts_with("Status:") {
                let status = obs.trim_start_matches("Status:").trim();
                *by_status.entry(status.to_string()).or_insert(0) += 1;
            }
        }
    }

    Ok(Summary {
        total_entities: entities.len(),
        by_status,
        ..Default::default()
    })
}
```

---

## 📊 Implementation Priority

| Tool | Priority | Complexity | Value |
|------|----------|------------|-------|
| `get_related` | P0 | Low | High |
| `traverse` | P1 | Medium | High |
| `summarize` | P2 | Low | Medium |

### Recommended Order

1. **Week 1**: Implement `get_related`
   - Core functionality for graph navigation
   - Foundation for `traverse`

2. **Week 2**: Implement `traverse`
   - Builds on `get_related`
   - Enables complex queries

3. **Week 3**: Implement `summarize`
   - Nice-to-have for UX
   - Can be deferred

---

## 🔗 Related Entities

- **Part of**: Memory Graph MCP Server
- **Depends on**: Current KnowledgeBase implementation
- **Enhances**: AI Agent Workflow

---

*Last updated: 2026-01-11*
