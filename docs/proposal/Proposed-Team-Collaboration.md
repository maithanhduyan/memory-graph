# Proposed: Team Collaboration Architecture

> **Status:** 📋 Proposed
> **Date:** 2026-01-11
> **Priority:** Future Enhancement
> **Complexity:** High

---

## 🎯 Mục tiêu

Chuyển đổi Memory Graph từ **"Công cụ cá nhân"** sang **"Hệ điều hành Team"** - nơi PM, BA, Dev, Tester cùng làm việc trên một Knowledge Graph duy nhất.

---

## 📊 Hiện trạng vs Tương lai

| Khía cạnh | Hiện tại (v1.x) | Tương lai (v2.x) |
|-----------|-----------------|------------------|
| **Transport** | stdio (local) | HTTP/SSE (network) |
| **Users** | Single user | Multi-user team |
| **Storage** | JSONL file | Database (PostgreSQL/SQLite) |
| **Auth** | Không có | API Key + RBAC |
| **Sync** | Không cần | Real-time events |

---

## 🏗️ Kiến trúc đề xuất: "Memory Hub"

### Hybrid Protocol Strategy

Không chọn 1 protocol, mà dùng đúng protocol cho đúng đối tượng:

```
┌─────────────────────────────────────────────────────────────┐
│                        MEMORY HUB                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  MCP SSE    │  │  GraphQL    │  │  REST API           │ │
│  │  (AI Agents)│  │  (Dashboard)│  │  (CI/CD, Scripts)   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                     │            │
│         └────────────────┼─────────────────────┘            │
│                          │                                  │
│                    ┌─────▼─────┐                           │
│                    │   Axum    │                           │
│                    │  Gateway  │                           │
│                    └─────┬─────┘                           │
│                          │                                  │
│                    ┌─────▼─────┐                           │
│                    │ Knowledge │                           │
│                    │   Base    │                           │
│                    │ (RwLock)  │                           │
│                    └─────┬─────┘                           │
│                          │                                  │
│                    ┌─────▼─────┐                           │
│                    │  Storage  │                           │
│                    │ (DB/File) │                           │
│                    └───────────┘                           │
└─────────────────────────────────────────────────────────────┘
```

### Protocol Comparison

| Protocol | Target Users | Use Case | Priority |
|----------|--------------|----------|----------|
| **MCP SSE** | AI Agents (Cursor, Claude) | Dev hỏi AI về context | **P0 - Bắt buộc** |
| **GraphQL** | PM, BA, Frontend Dashboard | Query linh hoạt, visualize graph | **P1 - Rất nên** |
| **REST** | CI/CD, Scripts, 3rd party | Simple integrations | **P2 - Tùy chọn** |

---

## 🔧 Technical Implementation

### Phase 1: MCP SSE Transport (1-2 weeks)

**Mục tiêu:** Cho phép AI Agents kết nối qua HTTP thay vì stdio.

```rust
// src/server/transport.rs
pub trait Transport: Send + Sync {
    fn read(&self) -> McpResult<String>;
    fn write(&self, msg: &str) -> McpResult<()>;
}

pub struct StdioTransport { ... }  // Hiện tại
pub struct SseTransport { ... }    // Mới - cho remote AI agents
```

**Dependencies:**
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["cors"] }
```

**Endpoints:**
```
GET  /sse              → SSE stream cho AI agent
POST /messages         → Nhận tool calls từ AI
GET  /health           → Health check
```

### Phase 2: GraphQL API (2-3 weeks)

**Mục tiêu:** Dashboard cho PM/BA visualize Knowledge Graph.

```graphql
type Query {
  entities(limit: Int, offset: Int, type: String): [Entity!]!
  entity(name: String!): Entity
  search(query: String!, includeRelations: Boolean): SearchResult!
  traverse(startNode: String!, path: [PathStep!]!): TraversalResult!
  relationsAtTime(timestamp: Int!, entityName: String): [Relation!]!
}

type Mutation {
  createEntities(input: [EntityInput!]!): [Entity!]!
  createRelations(input: [RelationInput!]!): [Relation!]!
  addObservations(input: [ObservationInput!]!): [Entity!]!
  deleteEntities(names: [String!]!): DeleteResult!
}

type Subscription {
  entityCreated: Entity!
  relationCreated: Relation!
  graphUpdated: GraphEvent!
}
```

**Dependencies:**
```toml
[dependencies]
async-graphql = "7"
async-graphql-axum = "7"
```

### Phase 3: Authentication & Authorization (1 week)

**API Key Authentication:**
```rust
// src/api/auth/mod.rs
pub struct ApiKey {
    pub key: String,
    pub user: String,
    pub role: Role,
    pub created_at: i64,
}

pub enum Role {
    Admin,      // Full access
    Developer,  // CRUD all
    Viewer,     // Read only
}
```

**Middleware:**
```rust
async fn auth_middleware(
    headers: HeaderMap,
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    // Validate and inject user into request
    // ...
}
```

### Phase 4: Database Migration (2 weeks)

**Từ JSONL → SQLite/PostgreSQL:**

```sql
-- entities table
CREATE TABLE entities (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    entity_type VARCHAR(100) NOT NULL,
    observations JSONB DEFAULT '[]',
    created_by VARCHAR(100),
    updated_by VARCHAR(100),
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- relations table
CREATE TABLE relations (
    id SERIAL PRIMARY KEY,
    from_entity VARCHAR(255) NOT NULL,
    to_entity VARCHAR(255) NOT NULL,
    relation_type VARCHAR(100) NOT NULL,
    created_by VARCHAR(100),
    created_at BIGINT NOT NULL,
    valid_from BIGINT,
    valid_to BIGINT,
    UNIQUE(from_entity, to_entity, relation_type, valid_from)
);

-- events table (audit log)
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    user_id VARCHAR(100),
    created_at BIGINT NOT NULL
);
```

---

## 📁 Proposed Folder Structure

```
src/
├── main.rs                    # CLI: chọn mode (stdio/http/both)
├── lib.rs
├── knowledge_base/            # (giữ nguyên)
├── protocol/
│   ├── mcp.rs
│   └── mod.rs
├── server/
│   ├── mod.rs
│   ├── stdio.rs               # MCP stdio (hiện tại)
│   ├── http.rs                # HTTP gateway (mới)
│   └── transport.rs           # Transport abstraction (mới)
├── api/                       # NEW
│   ├── mod.rs
│   ├── rest/
│   │   ├── mod.rs
│   │   ├── entities.rs
│   │   ├── relations.rs
│   │   └── health.rs
│   ├── graphql/
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   ├── query.rs
│   │   └── mutation.rs
│   ├── sse/
│   │   ├── mod.rs
│   │   └── mcp_handler.rs
│   └── auth/
│       ├── mod.rs
│       ├── api_key.rs
│       └── rbac.rs
├── storage/                   # NEW
│   ├── mod.rs
│   ├── jsonl.rs               # Hiện tại
│   ├── sqlite.rs              # Mới
│   └── postgres.rs            # Mới
└── ...
```

---

## ⚠️ Risks & Mitigations

### Risk 1: Race Conditions
- **Problem:** Multi-client write cùng lúc
- **Mitigation:**
  - Phase 1: Single instance + RwLock (đủ cho 5-10 users)
  - Phase 2: Database với proper locking

### Risk 2: Context Overflow
- **Problem:** Graph lớn → tràn AI context window
- **Mitigation:**
  - Pagination (đã có)
  - Smart summarization
  - Future: Vector search để chỉ lấy relevant nodes

### Risk 3: Data Conflicts
- **Problem:** PM và Dev update cùng entity
- **Mitigation:**
  - Event sourcing: Lưu events, không overwrite
  - Optimistic locking: Version field
  - Future: CRDT cho offline-first

---

## 🚀 Implementation Roadmap

```
Phase 1 (Month 1-2)          Phase 2 (Month 3-4)          Phase 3 (Month 5+)
┌─────────────────┐          ┌─────────────────┐          ┌─────────────────┐
│ MCP SSE Server  │          │ GraphQL API     │          │ Web Dashboard   │
│ API Key Auth    │    →     │ Event Sourcing  │    →     │ Real-time Sync  │
│ Single Instance │          │ SQLite Storage  │          │ Multi-tenant    │
└─────────────────┘          └─────────────────┘          └─────────────────┘
     MVP for 5 devs              Team of 10-20              Enterprise ready
```

---

## 📝 Open Questions

1. **Conflict Resolution Strategy?**
   - Last-write-wins? (Simple)
   - Manual merge như Git?
   - CRDT? (Complex)

2. **Offline Support?**
   - Local cache per user?
   - Sync when online?

3. **Authorization Granularity?**
   - Entity-level permissions?
   - Type-based access control?

4. **Deployment Model?**
   - Self-hosted only?
   - Cloud offering?

---

## 📚 References

- [MCP SSE Transport Spec](https://modelcontextprotocol.io/docs/concepts/transports#server-sent-events-sse)
- [Axum Framework](https://github.com/tokio-rs/axum)
- [async-graphql](https://github.com/async-graphql/async-graphql)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)

---

*Last updated: 2026-01-11*
