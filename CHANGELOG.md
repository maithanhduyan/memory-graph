# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
