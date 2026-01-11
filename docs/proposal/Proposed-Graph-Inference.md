# Proposed: Graph Inference Engine (Layer 1)

> **Status**: 📋 Proposed | ✅ Approved | ✔️ Completed
> **Date**: 2026-01-11
> **Risk Level**: 🟢 LOW
> **Approach**: Option A - Lazy Inference (Runtime Only)

---

## 📋 Executive Summary

Thêm **Inference Engine** để Memory Graph có khả năng **suy luận chủ động** - phát hiện quan hệ ẩn dựa trên logic rules, không chỉ trả lời câu hỏi được hỏi.

### Vision

```
LLM (Neural) = Trực giác, sáng tạo, hay hallucinate
Graph (Symbolic) = Lưu trữ, logic, chính xác, cứng nhắc

Inference Engine = Bridge giữa hai thế giới
                 = "Vỏ não trước trán" cho AI Agent
```

---

## 🎯 Goals

1. **Transitive Reasoning**: A depends_on B, B depends_on C → A depends_on C (indirect)
2. **Risk Propagation**: Risk threatens Module, Feature part_of Module → Risk threatens Feature
3. **Impact Analysis**: Thay đổi X ảnh hưởng gì? (multi-hop)
4. **Root Cause Discovery**: Bug Y do đâu gây ra? (reverse traversal)

---

## 🏗️ Architecture

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Persist vs Runtime | **Runtime** | No cache invalidation, always fresh |
| New tool vs Enhance | **New tool `infer`** | SRP - separation of concerns |
| Algorithm | **BFS** | Shortest path first (Occam's Razor) |
| Depth limit | **3 hops** | Đủ sâu, không gây nhiễu |
| Cycle handling | **visited HashSet** | Prevent infinite loops |
| Confidence | **Decay per hop** | 0.95^n for strong relations |

### System Flow

```
┌─────────────────────────────────────────────────────────────┐
│  AI Agent asks: "Feature Login có rủi ro gì không?"         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  MCP Tool: infer(entityName: "Feature: Login")              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Inference Engine                                           │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 1. Load Graph (RwLock read)                         │   │
│  │ 2. BFS from target node                             │   │
│  │ 3. Apply rules (TransitiveDependency, etc.)         │   │
│  │ 4. Filter by min_confidence                         │   │
│  │ 5. Return InferredRelations + Stats                 │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Response:                                                  │
│  "Login thuộc module Auth, Auth bị đe dọa bởi SQL Injection"│
│  "→ Login có rủi ro gián tiếp từ SQL Injection (85%)"       │
└─────────────────────────────────────────────────────────────┘
```

---

## 📦 Data Models

### InferredRelation

```rust
pub struct InferredRelation {
    pub relation: Relation,      // The inferred relation
    pub confidence: f32,         // 0.0 - 1.0
    pub rule_name: String,       // "Transitive Dependency"
    pub explanation: String,     // "Path: A -> B -> C (81%)"
}
```

### InferStats

```rust
pub struct InferStats {
    pub nodes_visited: usize,
    pub paths_found: usize,
    pub max_depth_reached: usize,
    pub execution_time_ms: u64,
}
```

### InferResult

```rust
pub struct InferResult {
    pub target: String,
    pub inferred_relations: Vec<InferredRelation>,
    pub stats: InferStats,
}
```

---

## 🧠 Inference Rules

### Rule 1: Transitive Dependency

**Logic**: IF A `depends_on` B AND B `depends_on` C THEN A `depends_on_indirect` C

**Algorithm**: BFS from start node, follow `depends_on` edges

**Confidence Decay**:
```
depends_on, implements  → 0.95 per hop (strong)
affects, caused_by      → 0.90 per hop (medium)
relates_to              → 0.70 per hop (weak)
default                 → 0.80 per hop
```

**Example**:
```
Feature: Semantic Search
    └── depends_on → Feature: GNN Integration (1.0)
                         └── depends_on → Milestone: Production Ready (0.95)

Inferred: Feature: Semantic Search → depends_on_indirect → Milestone: Production Ready
Confidence: 1.0 × 0.95 = 0.95 (95%)
```

### Rule 2: Risk Propagation (Future)

**Logic**: IF Risk `threatens` Module AND Feature `part_of` Module THEN Risk `threatens_indirect` Feature

### Rule 3: Ownership Chain (Future)

**Logic**: IF Person `owns` Module AND Module `contains` Feature THEN Person `responsible_for` Feature

---

## 🛡️ Safety Protocols

### 1. Target-Centric (No Full Graph Scan)

```rust
// ❌ NEVER do this
fn infer_all(graph: &KnowledgeGraph) -> Vec<InferredRelation>

// ✅ Always scope to target
fn infer(graph: &KnowledgeGraph, target: &str) -> Vec<InferredRelation>
```

### 2. Depth Limit

```rust
const MAX_DEPTH: usize = 3;

if path.len() > MAX_DEPTH + 1 {
    continue; // Stop exploring
}
```

### 3. Cycle Detection

```rust
let mut visited: HashSet<String> = HashSet::new();

if visited.contains(&next_node) {
    continue; // Skip cycles
}
visited.insert(next_node.clone());
```

### 4. Confidence Threshold

```rust
const MIN_CONFIDENCE: f32 = 0.5;

if new_confidence < MIN_CONFIDENCE {
    continue; // Prune low-confidence paths
}
```

---

## 📁 File Structure

```
src/
├── types/
│   ├── mod.rs              ← Add: pub mod inference;
│   └── inference.rs        ← NEW: InferredRelation, InferStats, InferResult
│
├── knowledge_base/
│   ├── mod.rs              ← Add: pub mod inference;
│   └── inference/
│       ├── mod.rs          ← NEW: InferenceEngine, InferenceRule trait
│       └── rules.rs        ← NEW: TransitiveDependencyRule
│
└── tools/
    ├── mod.rs              ← Add: pub mod inference;
    └── inference/
        ├── mod.rs          ← NEW: register tools
        └── infer.rs        ← NEW: InferTool
```

---

## 🔧 Tool API

### `infer`

**Description**: Reasoning engine - discovers hidden relations using logical rules.

**Input**:
```json
{
  "entityName": "Feature: Login",
  "minConfidence": 0.5,
  "maxDepth": 3
}
```

**Output**:
```json
{
  "target": "Feature: Login",
  "inferred_relations": [
    {
      "relation": {
        "from": "Feature: Login",
        "to": "Risk: SQL Injection",
        "relation_type": "threatened_by_indirect"
      },
      "confidence": 0.85,
      "rule_name": "Risk Propagation",
      "explanation": "Feature: Login -> Module: Auth -> Risk: SQL Injection (85%)"
    }
  ],
  "stats": {
    "nodes_visited": 12,
    "paths_found": 3,
    "max_depth_reached": 2,
    "execution_time_ms": 5
  }
}
```

---

## 📊 Performance Analysis

### Time Complexity

| Component | Complexity | Notes |
|-----------|------------|-------|
| BFS traversal | O(V + E) | V = visited nodes, E = edges explored |
| Relation lookup | O(E) | Linear scan (v1.0) |
| Total per query | O(V × E) | Worst case |

### Future Optimization (v2.0)

```rust
// Pre-build adjacency list for O(1) lookup
let adjacency: HashMap<String, Vec<&Relation>> = build_adjacency(&graph);
```

### Memory Usage

- **Visited set**: O(V) where V ≤ graph size
- **Queue**: O(V) maximum
- **Results**: O(paths_found)

---

## 🧪 Test Cases

### 1. Simple Transitive

```
A -> B -> C
Expected: A -> C (indirect, 90%)
```

### 2. Cycle Detection

```
A -> B -> C -> A
Expected: No infinite loop, finite results
```

### 3. Confidence Cutoff

```
A -> B -> C -> D -> E (5 hops)
min_confidence = 0.7
Expected: Stop at depth 3 (0.95³ = 0.86 > 0.7, 0.95⁴ = 0.81 > 0.7, 0.95⁵ = 0.77 > 0.7)
```

### 4. BFS Shortest Path

```
A -> B -> D (2 hops, 90%)
A -> C -> E -> D (3 hops, 86%)
Expected: B path returned first
```

### 5. Diamond Pattern

```
    A
   / \
  B   C
   \ /
    D
Expected: D visited once, both paths valid
```

---

## 🚀 Implementation Phases

### Phase 1: Core Infrastructure
- [ ] Create `src/types/inference.rs`
- [ ] Create `src/knowledge_base/inference/mod.rs`
- [ ] Create `src/knowledge_base/inference/rules.rs`

### Phase 2: Tool Integration
- [ ] Create `src/tools/inference/mod.rs`
- [ ] Create `src/tools/inference/infer.rs`
- [ ] Register in `src/tools/mod.rs`

### Phase 3: Exports & Tests
- [ ] Update `src/types/mod.rs`
- [ ] Update `src/knowledge_base/mod.rs`
- [ ] Update `src/lib.rs`
- [ ] Write unit tests
- [ ] Write integration tests

### Phase 4: Validation
- [ ] Build & test
- [ ] Update CHANGELOG.md
- [ ] Update memory graph with completion

---

## 📈 Success Metrics

| Metric | Target |
|--------|--------|
| Build | ✅ No errors |
| Tests | ✅ All pass |
| Performance | < 100ms for 1000-node graph |
| Memory | < 10MB additional per query |

---

## 🔮 Future Enhancements

1. **More Rules**: Risk Propagation, Ownership Chain, Conflict Detection
2. **Bidirectional**: Infer incoming relations (who depends on me?)
3. **Configurable Decay**: Per-rule decay factors
4. **Caching (v2.0)**: Optional materialized view for hot paths
5. **Adjacency Index**: O(1) neighbor lookup
6. **Rule DSL**: Define rules without code changes

---

## 📝 References

- [Research: Graph Reasoning Capability Analysis](../memory.jsonl) - Entity in knowledge graph
- [Feature: Rule-based Inference Engine](../memory.jsonl) - Feature definition
- [Insight: Graph as Prefrontal Cortex for AI](../memory.jsonl) - Vision statement
