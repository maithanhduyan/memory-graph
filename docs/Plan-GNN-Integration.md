# GNN Integration - Future Development Plan

> **Status**: Planned
> **Priority**: Medium
> **Target**: Post-Production
> **Owner**: tiach
> **Created**: 2026-01-11

## 📋 Overview

Tích hợp Graph Neural Network (GNN) vào Memory Graph MCP Server để nâng cao khả năng của AI Agent trong việc:
- Tìm kiếm semantic
- Dự đoán relations
- Suy luận trên graph

## 🎯 Capabilities

### 1. Semantic Search
Tìm nodes "tương tự" về ý nghĩa thay vì chỉ text matching.

```
Current:  search_nodes("storage") → chỉ match text "storage"
With GNN: search_nodes("data persistence") → tìm được "Feature: Persistent Storage"
```

**Approach**:
- Embed entity text: `[{entityType}] {name}: {observations...}`
- Sử dụng cosine similarity để rank results
- Combine với text search (hybrid search)

### 2. Link Prediction
Đề xuất relations mới dựa trên graph structure.

```
Input:  Entity A có relations tương tự Entity B
Output: "Suggest: Entity A --relates_to--> Entity C"
        (vì Entity B đã có relation với Entity C)
```

**Use cases**:
- Tự động phát hiện dependencies giữa features
- Suggest missing relations trong project management
- Detect potential risks từ patterns

### 3. Node Embeddings
Encode context từ neighboring nodes vào vector representation.

```
Traditional: embed("Alice") = vector từ text "Alice, Person, Software Engineer"
With GNN:    embed("Alice") = vector bao gồm cả context từ:
             - Relations: knows Bob, works_at TechCorp
             - Neighbors: Bob's attributes, TechCorp's attributes
```

**Benefits**:
- Richer representations
- Capture graph structure
- Better similarity matching

### 4. Multi-hop Reasoning
Suy luận qua nhiều relations để trả lời complex queries.

```
Query: "What risks affect features owned by tiach?"

Reasoning path:
tiach --owns--> Feature: Create Entities
                Feature: Create Entities --part_of--> Milestone: MVP Release
                                                      Risk: JSONL Scalability --threatens--> Milestone: Production Ready

Answer: Risk: JSONL Scalability (indirect impact)
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           AI Agent                                  │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        MCP Server (Rust)                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ search_nodes │  │semantic_search│ │ link_predict │              │
│  │ (text)       │  │ (embeddings) │  │ (GNN)        │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                            │                │                       │
│                    ┌───────┴────────────────┴───────┐              │
│                    │        GNN Module              │              │
│                    │  - Message Passing             │              │
│                    │  - Node Aggregation            │              │
│                    │  - Link Scoring                │              │
│                    └───────────────────────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                ▼               ▼               ▼
        ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
        │memory.jsonl │ │ memory.vec  │ │ gnn.model   │
        │(graph data) │ │ (vectors)   │ │ (weights)   │
        └─────────────┘ └─────────────┘ └─────────────┘
```

## 🔧 Implementation Options

### Option A: External Python Service

```
┌────────────┐     HTTP/gRPC      ┌────────────┐
│ Rust MCP   │ ◄───────────────► │ Python GNN │
│ Server     │                    │ Service    │
└────────────┘                    └────────────┘
                                       │
                                       ▼
                              PyTorch Geometric / DGL
```

**Pros**: Mature libraries, easy prototyping
**Cons**: Extra service, latency, deployment complexity

### Option B: Rust Native (Candle/Burn)

```rust
use candle_core::{Tensor, Device};
use candle_nn::{Linear, Module};

struct GNNLayer {
    message_fn: Linear,
    update_fn: Linear,
}

impl GNNLayer {
    fn forward(&self, nodes: &Tensor, edges: &[(usize, usize)]) -> Tensor {
        // Message passing
        let messages = self.aggregate_neighbors(nodes, edges);
        // Update
        self.update_fn.forward(&messages)
    }
}
```

**Pros**: Single binary, fast, no external deps
**Cons**: Less mature, more implementation effort

### Option C: ONNX Runtime

```rust
use ort::{Environment, Session, Value};

fn run_gnn_inference(graph_data: GraphInput) -> Vec<f32> {
    let session = Session::new("gnn_model.onnx")?;
    let outputs = session.run(vec![graph_data.to_tensor()])?;
    outputs[0].extract_tensor()
}
```

**Pros**: Train in Python, deploy in Rust
**Cons**: Need to pre-train model, less flexible

## 📊 GNN Architectures to Consider

| Model | Use Case | Complexity |
|-------|----------|------------|
| **GCN** | Node classification | Low |
| **GraphSAGE** | Inductive learning, scalable | Medium |
| **GAT** | Attention-based, interpretable | Medium |
| **R-GCN** | Relational data (multiple edge types) | High |
| **CompGCN** | Knowledge graph completion | High |

**Recommendation**: Start với **GraphSAGE** hoặc **R-GCN** vì:
- GraphSAGE: Sampling-based, scales tốt
- R-GCN: Designed cho multi-relational graphs (phù hợp với Memory Graph)

## 🗓️ Roadmap

### Phase 1: Semantic Search (Prerequisites)
**Timeline**: Week 1-2 of Production Ready milestone

- [ ] Integrate embedding model (Ollama/ONNX)
- [ ] Create entity text formatter
- [ ] Implement vector storage
- [ ] Add `semantic_search` MCP tool

### Phase 2: Basic GNN
**Timeline**: Post-Production, Week 1-2

- [ ] Choose GNN framework
- [ ] Implement node embedding generation
- [ ] Train basic model on Memory Graph data
- [ ] Evaluate embedding quality

### Phase 3: Link Prediction
**Timeline**: Post-Production, Week 3-4

- [ ] Implement link prediction model
- [ ] Add `suggest_relations` MCP tool
- [ ] Evaluate prediction accuracy

### Phase 4: Multi-hop Reasoning
**Timeline**: Post-Production, Week 5-6

- [ ] Implement path-based reasoning
- [ ] Add `reason` MCP tool
- [ ] Create evaluation benchmark

## 📚 References

- [Graph Neural Networks: A Review](https://arxiv.org/abs/1901.00596)
- [PyTorch Geometric Documentation](https://pytorch-geometric.readthedocs.io/)
- [Candle - Minimalist ML framework for Rust](https://github.com/huggingface/candle)
- [Knowledge Graph Embedding](https://arxiv.org/abs/1503.00759)

## 🔗 Related Entities

- **Depends on**: Feature: Semantic Search
- **Part of**: Memory Graph MCP Server
- **Owner**: tiach
- **Planned for**: Milestone: Production Ready (partial), Post-Production (full)

---

*Last updated: 2026-01-11*
