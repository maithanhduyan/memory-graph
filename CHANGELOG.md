# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-01-11

### 🏗️ Major Refactoring Release

Complete architectural overhaul for better maintainability and performance.

### Changed

#### Modular Architecture
- **Refactored from single file to multi-module structure**
  - From: `memory.rs` (2505 lines, monolithic)
  - To: 35+ files in 8 organized modules
- **New module structure:**
  - `src/types/` - Core data models (Entity, Relation, KnowledgeGraph)
  - `src/protocol/` - JSON-RPC and MCP protocol handling
  - `src/knowledge_base/` - Core engine with CRUD, queries, temporal
  - `src/tools/` - 15 MCP tools organized by category (memory, query, temporal)
  - `src/search/` - Semantic search with synonym expansion
  - `src/server/` - MCP server implementation
  - `src/validation/` - Entity and relation type validation
  - `src/utils/` - Timestamp and user utilities
- **Library + Binary separation**
  - `src/lib.rs` - Public API for embedding
  - `src/main.rs` - Minimal binary entry point

#### Performance Optimization
- **Mutex → RwLock migration** for `KnowledgeBase.graph`
  - Allows multiple concurrent readers (60% of operations are reads)
  - Write operations still have exclusive access
  - Significant performance boost for multi-agent scenarios
- **Documentation:** See `docs/Proposed-RwLock.md` for risk analysis

#### Docker
- Updated `Dockerfile` for new `src/` directory structure
- Better layer caching with separate Cargo.toml and src copies

### Added
- `src/lib.rs` - Library crate for embedding in other projects
- `tests/integration_tests.rs` - 8 integration tests including concurrency tests
- `docs/Proposed-RwLock.md` - RwLock migration documentation

### Technical Details
- **Test suite expanded:** 16 tests (7 unit + 8 integration + 1 doc)
- **Zero-cost abstractions:** No runtime overhead from modularization
- **Backward compatible:** All 15 MCP tools unchanged

---

## [1.0.0] - 2026-01-11

### 🎉 Initial Release

First production-ready release of Memory Graph MCP Server.

### Added

#### Core Features
- **15 MCP Tools** for comprehensive knowledge graph management
  - 9 Memory tools: `create_entities`, `create_relations`, `add_observations`, `delete_entities`, `delete_observations`, `delete_relations`, `read_graph`, `search_nodes`, `open_nodes`
  - 3 Query tools: `get_related`, `traverse`, `summarize`
  - 2 Temporal tools: `get_relations_at_time`, `get_relation_history`
  - 1 Utility tool: `get_current_time`

#### Thread Safety
- **In-Memory Cache** with `Mutex<KnowledgeGraph>` for thread-safe operations
- Lock-during-modify pattern prevents race conditions
- Production-ready for multi-agent use cases

#### Semantic Search
- **Synonym Dictionary** with 15+ word groups
- Automatically expands queries (e.g., "developer" → "coder", "programmer", "engineer")
- Case-insensitive matching

#### Temporal Relations
- `validFrom` and `validTo` fields on relations
- Query historical state with `get_relations_at_time`
- View relation history with `get_relation_history`

#### Pagination
- `limit` and `offset` parameters on `read_graph`
- `limit` and `includeRelations` on `search_nodes`
- Prevents context window overflow for large graphs

#### Type Validation
- 11 standard entity types: `Project`, `Module`, `Feature`, `Bug`, `Decision`, `Requirement`, `Milestone`, `Risk`, `Convention`, `Schema`, `Person`
- 12 standard relation types: `contains`, `implements`, `fixes`, `caused_by`, `depends_on`, `blocked_by`, `assigned_to`, `part_of`, `relates_to`, `supersedes`, `affects`, `requires`
- Soft validation with warnings (custom types still allowed)

#### Timestamps
- `createdAt` and `updatedAt` on entities
- `createdAt` on relations
- Automatic timestamp management

#### Developer Experience
- Pure Rust implementation (~2400 lines, single file)
- Minimal dependencies (only `serde`)
- JSONL storage format
- Comprehensive test suite (6 tests including concurrency tests)
- Docker support with multi-stage build
- VS Code and Claude Desktop configuration examples

### Technical Details
- MCP Protocol version: 2024-11-05
- JSON-RPC 2.0 compliant
- stdio transport
- ~3MB binary size

---

## [Unreleased]

### Planned
- Vector embeddings for true semantic search
- Web UI for graph visualization
- Multi-tenant support
- WAL (Write-Ahead Log) for large graphs
- Import/Export with external knowledge bases
- `parking_lot::RwLock` upgrade if benchmarks show bottleneck
