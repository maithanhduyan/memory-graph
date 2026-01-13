# 🧠 Memory Graph MCP Server

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05-blue)](https://modelcontextprotocol.io/)
[![Build](https://github.com/maithanhduyan/memory-graph/actions/workflows/rust.yml/badge.svg)](https://github.com/maithanhduyan/memory-graph/actions/workflows/rust.yml)
[![Release](https://github.com/maithanhduyan/memory-graph/actions/workflows/release.yml/badge.svg)](https://github.com/maithanhduyan/memory-graph/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Stars](https://img.shields.io/github/stars/maithanhduyan/memory-graph?style=social)](https://github.com/maithanhduyan/memory-graph)

**🚀 A blazing-fast Knowledge Graph server for AI Agents**

*Give your AI perfect memory. Forever.*

[Features](#-features) • [Quick Start](#-quick-start) • [API Reference](#-api-reference) • [Architecture](#-architecture)

</div>

---

> ⚠️ **DISCLAIMER: ALL SOURCE CODE IS AI-GENERATED. THE AUTHOR ASSUMES NO LIABILITY FOR PATENT, INTELLECTUAL PROPERTY, OR LEGAL COMPLIANCE ISSUES IN ANY JURISDICTION.**

---

## 🎯 The Problem

AI Agents forget everything between sessions. They hallucinate facts. They lose context.

**Memory Graph fixes this.**

```
Before: "Sorry, I don't have information about your project structure..."
After:  "Based on your Auth Module which implements JWT, I suggest..."
```

---

## ✨ Features

### 🛠️ 15 Powerful Tools

| Category | Tools | Description |
|----------|-------|-------------|
| **Memory** | `create_entities`, `create_relations`, `add_observations`, `delete_entities`, `delete_observations`, `delete_relations`, `read_graph`, `search_nodes`, `open_nodes` | Full CRUD for knowledge graph |
| **Query** | `get_related`, `traverse`, `summarize` | Advanced graph traversal |
| **Temporal** | `get_relations_at_time`, `get_relation_history` | Time-travel queries |
| **Utility** | `get_current_time` | Timestamp helper |

### 🔥 Why Memory Graph?

| Feature | Description |
|---------|-------------|
| **⚡ Blazing Fast** | In-memory cache with file persistence. Nanosecond reads. |
| **🔒 Thread-Safe** | Production-ready with Mutex-based concurrency control |
| **🔍 Semantic Search** | Built-in synonym matching (developer ↔ coder ↔ engineer) |
| **⏰ Time Travel** | Query historical state with `validFrom`/`validTo` |
| **📏 Pagination** | Handle massive graphs with `limit`/`offset` |
| **✅ Type Validation** | Soft warnings for non-standard types |
| **🦀 Pure Rust** | Single binary, ~3MB. Only depends on `serde` |

---

## 🚀 Quick Start

### Option 1: Build from Source

```bash
git clone https://github.com/maithanhduyan/memory-graph.git
cd memory-graph
cargo build --release
```

### Option 2: Docker

```bash
docker run -v $(pwd)/data:/data ghcr.io/maithanhduyan/memory-graph
```

### Configure VS Code

Create `.vscode/mcp.json`:

```json
{
    "servers": {
        "memory": {
            "type": "stdio",
            "command": "${workspaceFolder}/target/release/memory-server.exe",
            "env": {
                "MEMORY_FILE_PATH": "${workspaceFolder}/memory.jsonl"
            }
        }
    }
}
```

### Configure Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "memory": {
      "command": "/path/to/memory-server",
      "env": {
        "MEMORY_FILE_PATH": "/path/to/memory.jsonl"
      }
    }
  }
}
```

---

## 📖 API Reference

### Memory Tools

#### `create_entities`
```json
{
  "entities": [{
    "name": "Auth Module",
    "entityType": "Module",
    "observations": ["Implements JWT", "Uses bcrypt"]
  }]
}
```

#### `create_relations`
```json
{
  "relations": [{
    "from": "Auth Module",
    "to": "User Service",
    "relationType": "depends_on"
  }]
}
```

#### `search_nodes` (with Semantic Search)
```json
{
  "query": "developer",  // Also matches: coder, programmer, engineer
  "limit": 10,
  "includeRelations": true
}
```

#### `read_graph` (with Pagination)
```json
{
  "limit": 50,
  "offset": 0
}
```

### Temporal Queries

#### `get_relations_at_time`
```json
{
  "timestamp": 1704067200,
  "entityName": "Alice"
}
// Returns: Relations valid at that specific point in time
```

#### `get_relation_history`
```json
{
  "entityName": "Alice"
}
// Returns: All relations (current + expired) with isCurrent flag
```

### Graph Traversal

#### `traverse`
```json
{
  "startNode": "Project: MyApp",
  "path": [
    {"relationType": "contains", "direction": "out"},
    {"relationType": "implements", "direction": "out", "targetType": "Feature"}
  ],
  "maxResults": 50
}
```

#### `get_related`
```json
{
  "entityName": "Auth Module",
  "direction": "both",
  "relationType": "depends_on"
}
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    AI Agent (Claude, etc.)                  │
└─────────────────────────────────────────────────────────────┘
                              │ JSON-RPC 2.0 (stdio)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Memory Graph MCP Server                    │
│  ┌───────────────────────────────────────────────────────┐ │
│  │                  MCP Protocol Layer                    │ │
│  │          (initialize, tools/list, tools/call)         │ │
│  └───────────────────────────────────────────────────────┘ │
│                              │                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │                    Tool Registry                       │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐            │ │
│  │  │ Memory   │  │  Query   │  │ Temporal │            │ │
│  │  │ Tools(9) │  │ Tools(3) │  │ Tools(2) │            │ │
│  │  └──────────┘  └──────────┘  └──────────┘            │ │
│  └───────────────────────────────────────────────────────┘ │
│                              │                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │            KnowledgeBase (Thread-Safe)                 │ │
│  │  ┌─────────────────────┐  ┌─────────────────────────┐ │ │
│  │  │  Mutex<Graph>       │  │   Synonym Dictionary    │ │ │
│  │  │  (In-Memory Cache)  │  │   (Semantic Search)     │ │ │
│  │  └─────────────────────┘  └─────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                   ┌────────────────────┐
                   │   memory.jsonl     │
                   │  (JSONL Storage)   │
                   └────────────────────┘
```

### Data Model

**Entity:**
```json
{
  "name": "Auth Module",
  "entityType": "Module",
  "observations": ["Implements JWT", "Uses bcrypt"],
  "createdAt": 1704067200,
  "updatedAt": 1704153600
}
```

**Relation (with Temporal Support):**
```json
{
  "from": "Alice",
  "to": "NYC",
  "relationType": "lives_in",
  "createdAt": 1704067200,
  "validFrom": 1704067200,
  "validTo": 1735689599
}
```

---

## 📊 Standard Types

### Entity Types (11)
`Project` `Module` `Feature` `Bug` `Decision` `Requirement` `Milestone` `Risk` `Convention` `Schema` `Person`

### Relation Types (12)
`contains` `implements` `fixes` `caused_by` `depends_on` `blocked_by` `assigned_to` `part_of` `relates_to` `supersedes` `affects` `requires`

> ⚠️ Custom types are allowed with soft warnings.

---

## 🧪 Testing

```bash
cargo test

# Output:
# test tests::test_create_entities ... ok
# test tests::test_create_relations ... ok
# test tests::test_search_nodes ... ok
# test tests::test_delete_entities ... ok
# test tests::test_concurrent_access ... ok      # 10 threads
# test tests::test_concurrent_read_write ... ok  # 5 readers + 3 writers
#
# test result: ok. 6 passed; 0 failed
```

---

## 🔧 Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `MEMORY_FILE_PATH` | `./memory.jsonl` | Path to knowledge graph storage |

---

## 🗺️ Roadmap

- [x] Core CRUD operations (9 tools)
- [x] Advanced query tools (3 tools)
- [x] Semantic search with synonyms
- [x] Temporal relations (time-travel)
- [x] Pagination support
- [x] Thread-safe in-memory cache
- [x] Type validation with warnings
- [ ] Vector embeddings for true semantic search
- [ ] Web UI for graph visualization
- [ ] Multi-tenant support
- [ ] WAL (Write-Ahead Log) for large graphs

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

```bash
# Fork the repo, then:
git checkout -b feature/amazing-feature
cargo test
git commit -m "Add amazing feature"
git push origin feature/amazing-feature
# Open a Pull Request
```

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with 🦀 Rust and ❤️ for AI Agents**

*If this project helps you, please ⭐ star the repo!*

[Report Bug](https://github.com/maithanhduyan/memory-graph/issues) · [Request Feature](https://github.com/maithanhduyan/memory-graph/issues)

</div>
