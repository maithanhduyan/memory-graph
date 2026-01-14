# Proposed: GraphQL API for Efficient Data Fetching

> **Status:** 📋 Proposed
> **Date:** 2026-01-14
> **Author:** Mai Thành Duy An
> **Priority:** 🟠 High
> **Complexity:** Medium
> **Estimated Effort:** 2-3 weeks
> **Reviewed by:** Pending

---

## 📋 Executive Summary

Triển khai GraphQL API layer để UI có thể query chính xác dữ liệu cần thiết, giảm over-fetching và under-fetching. Điều này đặc biệt quan trọng khi knowledge graph lớn với hàng nghìn entities và relations, giúp cải thiện performance và trải nghiệm người dùng.

---

## 🎯 Goals & Non-Goals

### Goals
- [x] UI chỉ fetch đúng fields cần thiết (selective field fetching)
- [x] Giảm payload size 50-80% so với REST
- [x] Single endpoint cho tất cả queries
- [x] Real-time subscriptions cho graph updates
- [x] Strongly typed schema với auto-completion trong IDE
- [x] Batch multiple queries trong 1 request

### Non-Goals
- ❌ Thay thế hoàn toàn REST API (sẽ giữ song song)
- ❌ Implement GraphQL mutations cho write operations (phase 2)
- ❌ Federation với external GraphQL services

---

## 📊 Current State vs Proposed State

| Aspect | Current (REST) | Proposed (GraphQL) |
|--------|----------------|-------------------|
| Data fetching | Fixed response structure | Client specifies exact fields |
| Endpoints | Multiple endpoints | Single `/graphql` endpoint |
| Over-fetching | ❌ Always returns all fields | ✅ Only requested fields |
| Under-fetching | ❌ Multiple requests needed | ✅ Nested queries in 1 request |
| Real-time | WebSocket (custom) | GraphQL Subscriptions |
| Type safety | OpenAPI (optional) | Schema-first, strongly typed |
| Payload size | ~5-10 KB per entity | ~0.5-2 KB per entity |

### Example: Current REST vs GraphQL

**Current REST - Get entity with relations:**
```bash
# Request 1: Get entity
GET /api/entities/Customer%3A%20Le%20Van%20C

# Request 2: Get related entities
GET /api/entities/Customer%3A%20Le%20Van%20C/related

# Response: ~8KB, includes all fields
```

**Proposed GraphQL:**
```graphql
query {
  entity(name: "Customer: Le Van C") {
    name
    entityType
    observations
    relations(direction: OUTGOING) {
      to { name entityType }
      relationType
    }
  }
}
# Response: ~1.5KB, only requested fields
```

---

## 🏗️ Architecture / Design

### System Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         CLIENT (UI)                               │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐      │
│  │ Apollo Client  │  │ GraphQL Code   │  │ Subscription   │      │
│  │ (Cache)        │  │ Generator      │  │ Client         │      │
│  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘      │
└──────────┼───────────────────┼───────────────────┼───────────────┘
           │                   │                   │
           ▼                   ▼                   ▼
┌──────────────────────────────────────────────────────────────────┐
│                    GRAPHQL GATEWAY                                │
│                    POST /graphql                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    async-graphql                            │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │  │
│  │  │ Query    │  │ Mutation │  │Subscription│ │ Schema   │   │  │
│  │  │ Resolver │  │ Resolver │  │ Resolver  │ │ Stitching│   │  │
│  │  └────┬─────┘  └────┬─────┘  └─────┬─────┘ └──────────┘   │  │
│  └───────┼─────────────┼──────────────┼──────────────────────┘  │
└──────────┼─────────────┼──────────────┼──────────────────────────┘
           │             │              │
           ▼             ▼              ▼
┌──────────────────────────────────────────────────────────────────┐
│                    MEMORY-GRAPH CORE                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │
│  │ KnowledgeBase│  │ EventStore │  │ Inference   │               │
│  │ (CRUD/Query)│  │ (Persist)  │  │ Engine      │               │
│  └─────────────┘  └─────────────┘  └─────────────┘               │
└──────────────────────────────────────────────────────────────────┘
```

### GraphQL Schema

```graphql
# schema.graphql

scalar DateTime
scalar JSON

# ============================================
# ENUMS
# ============================================

enum RelationDirection {
  INCOMING
  OUTGOING
  BOTH
}

enum SummaryFormat {
  BRIEF
  DETAILED
  STATS
}

enum AlertStatus {
  OPEN
  INVESTIGATING
  ESCALATED
  CLOSED
  FALSE_POSITIVE
}

# ============================================
# TYPES
# ============================================

type Entity {
  name: ID!
  entityType: String!
  observations: [String!]!
  createdBy: String
  updatedBy: String
  createdAt: DateTime
  updatedAt: DateTime

  # Nested queries - resolve on demand
  relations(
    direction: RelationDirection = BOTH
    relationType: String
    limit: Int = 50
  ): [Relation!]!

  relatedEntities(
    direction: RelationDirection = BOTH
    relationType: String
    limit: Int = 50
  ): [Entity!]!

  inferred(
    maxDepth: Int = 3
    minConfidence: Float = 0.5
  ): InferenceResult
}

type Relation {
  from: Entity!
  to: Entity!
  relationType: String!
  createdBy: String
  createdAt: DateTime
  validFrom: DateTime
  validTo: DateTime
}

type InferenceResult {
  inferredRelations: [InferredRelation!]!
  stats: InferStats!
}

type InferredRelation {
  relation: Relation!
  confidence: Float!
  ruleName: String!
  explanation: String!
}

type InferStats {
  nodesVisited: Int!
  pathsFound: Int!
  maxDepthReached: Int!
  executionTimeMs: Int!
}

type TraversalPath {
  nodes: [String!]!
  relations: [String!]!
}

type TraversalResult {
  startNode: String!
  paths: [TraversalPath!]!
  endNodes: [Entity!]!
}

type GraphSummary {
  totalEntities: Int!
  totalRelations: Int!
  entityTypes: [EntityTypeCount!]!
  relationTypes: [RelationTypeCount!]!
}

type EntityTypeCount {
  entityType: String!
  count: Int!
}

type RelationTypeCount {
  relationType: String!
  count: Int!
}

type SearchResult {
  entities: [Entity!]!
  totalCount: Int!
  hasMore: Boolean!
}

# ============================================
# INPUTS
# ============================================

input TraversalStep {
  relationType: String!
  direction: RelationDirection!
  targetType: String
}

input EntityFilter {
  entityType: String
  createdAfter: DateTime
  createdBefore: DateTime
  hasObservation: String
}

input PaginationInput {
  limit: Int = 50
  offset: Int = 0
}

# ============================================
# QUERIES
# ============================================

type Query {
  # Single entity lookup
  entity(name: ID!): Entity

  # Multiple entities by names
  entities(names: [ID!]!): [Entity!]!

  # Search entities
  searchEntities(
    query: String!
    filter: EntityFilter
    pagination: PaginationInput
    includeRelations: Boolean = false
  ): SearchResult!

  # Get all entities with pagination
  allEntities(
    filter: EntityFilter
    pagination: PaginationInput
  ): SearchResult!

  # Graph traversal
  traverse(
    startNode: ID!
    path: [TraversalStep!]!
    maxResults: Int = 50
  ): TraversalResult!

  # Get related entities
  related(
    entityName: ID!
    direction: RelationDirection = BOTH
    relationType: String
  ): [Relation!]!

  # Inference
  infer(
    entityName: ID!
    maxDepth: Int = 3
    minConfidence: Float = 0.5
  ): InferenceResult!

  # Graph summary/stats
  graphSummary: GraphSummary!

  # Temporal queries
  relationsAtTime(
    entityName: ID
    timestamp: DateTime!
  ): [Relation!]!

  relationHistory(entityName: ID!): [Relation!]!
}

# ============================================
# MUTATIONS (Phase 2)
# ============================================

type Mutation {
  # Entity operations
  createEntity(
    name: String!
    entityType: String!
    observations: [String!]
  ): Entity!

  addObservations(
    entityName: ID!
    observations: [String!]!
  ): Entity!

  deleteEntity(name: ID!): Boolean!

  # Relation operations
  createRelation(
    from: ID!
    to: ID!
    relationType: String!
    validFrom: DateTime
    validTo: DateTime
  ): Relation!

  deleteRelation(
    from: ID!
    to: ID!
    relationType: String!
  ): Boolean!
}

# ============================================
# SUBSCRIPTIONS
# ============================================

type Subscription {
  # Real-time entity updates
  entityChanged(entityName: ID): Entity!

  # Real-time relation updates
  relationChanged: Relation!

  # New alerts (AML use case)
  alertCreated: Entity!

  # Graph stats update
  graphStatsUpdated: GraphSummary!
}
```

### Data Loaders (N+1 Prevention)

```rust
use async_graphql::dataloader::{DataLoader, Loader};

pub struct EntityLoader {
    kb: Arc<RwLock<KnowledgeBase>>,
}

#[async_trait]
impl Loader<String> for EntityLoader {
    type Value = Entity;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Entity>, Self::Error> {
        // Batch load entities in single query
        let kb = self.kb.read().await;
        let mut result = HashMap::new();

        for name in keys {
            if let Some(entity) = kb.get_entity(name) {
                result.insert(name.clone(), entity.clone());
            }
        }

        Ok(result)
    }
}

pub struct RelationLoader {
    kb: Arc<RwLock<KnowledgeBase>>,
}

#[async_trait]
impl Loader<String> for RelationLoader {
    type Value = Vec<Relation>;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[String]) -> Result<HashMap<String, Vec<Relation>>, Self::Error> {
        // Batch load relations for multiple entities
        let kb = self.kb.read().await;
        let mut result = HashMap::new();

        for entity_name in keys {
            let relations = kb.get_relations_for(entity_name);
            result.insert(entity_name.clone(), relations);
        }

        Ok(result)
    }
}
```

### Resolver Implementation

```rust
use async_graphql::{Context, Object, Result, ID};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get single entity by name
    async fn entity(&self, ctx: &Context<'_>, name: ID) -> Result<Option<Entity>> {
        let loader = ctx.data::<DataLoader<EntityLoader>>()?;
        Ok(loader.load_one(name.to_string()).await?)
    }

    /// Search entities with filtering
    async fn search_entities(
        &self,
        ctx: &Context<'_>,
        query: String,
        filter: Option<EntityFilter>,
        pagination: Option<PaginationInput>,
        include_relations: Option<bool>,
    ) -> Result<SearchResult> {
        let kb = ctx.data::<Arc<RwLock<KnowledgeBase>>>()?;
        let kb = kb.read().await;

        let limit = pagination.as_ref().map(|p| p.limit).unwrap_or(50);
        let offset = pagination.as_ref().map(|p| p.offset).unwrap_or(0);

        let (entities, relations) = kb.search_nodes(
            &query,
            Some(limit as usize),
            include_relations.unwrap_or(false),
        );

        Ok(SearchResult {
            entities: entities.into_iter().skip(offset as usize).collect(),
            total_count: entities.len() as i32,
            has_more: entities.len() > (offset + limit) as usize,
        })
    }

    /// Graph traversal
    async fn traverse(
        &self,
        ctx: &Context<'_>,
        start_node: ID,
        path: Vec<TraversalStep>,
        max_results: Option<i32>,
    ) -> Result<TraversalResult> {
        let kb = ctx.data::<Arc<RwLock<KnowledgeBase>>>()?;
        let kb = kb.read().await;

        let path_spec: Vec<_> = path.iter().map(|s| {
            (s.relation_type.clone(), s.direction.clone(), s.target_type.clone())
        }).collect();

        let result = kb.traverse(
            &start_node,
            &path_spec,
            max_results.unwrap_or(50) as usize,
        );

        Ok(result.into())
    }

    /// Inference for hidden relations
    async fn infer(
        &self,
        ctx: &Context<'_>,
        entity_name: ID,
        max_depth: Option<i32>,
        min_confidence: Option<f32>,
    ) -> Result<InferenceResult> {
        let kb = ctx.data::<Arc<RwLock<KnowledgeBase>>>()?;
        let kb = kb.read().await;

        let result = kb.infer(
            &entity_name,
            max_depth.unwrap_or(3) as usize,
            min_confidence.unwrap_or(0.5),
        );

        Ok(result.into())
    }
}
```

---

## 📁 File Structure

```
src/
├── api/
│   ├── mod.rs              ← MODIFY (add graphql module)
│   ├── http.rs             ← MODIFY (add /graphql endpoint)
│   └── graphql/
│       ├── mod.rs          ← NEW
│       ├── schema.rs       ← NEW (schema definition)
│       ├── query.rs        ← NEW (query resolvers)
│       ├── mutation.rs     ← NEW (mutation resolvers)
│       ├── subscription.rs ← NEW (subscription resolvers)
│       ├── types.rs        ← NEW (GraphQL types)
│       └── loaders.rs      ← NEW (DataLoaders for N+1)
├── Cargo.toml              ← MODIFY (add async-graphql)
config/
└── graphql.toml            ← NEW (optional config)
ui/
├── js/
│   └── graphql-client.js   ← NEW
└── index.html              ← MODIFY (add Apollo)
```

---

## 🔧 Implementation Plan

### Phase 1: Core GraphQL Setup (Week 1)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 1.1 | Add async-graphql dependency | Cargo.toml | 1h | ⬜ |
| 1.2 | Create GraphQL schema types | types.rs | 4h | ⬜ |
| 1.3 | Implement Query resolvers | query.rs | 8h | ⬜ |
| 1.4 | Add DataLoaders | loaders.rs | 4h | ⬜ |
| 1.5 | Integrate with Axum | http.rs | 2h | ⬜ |
| 1.6 | Add GraphQL Playground | Built-in | 1h | ⬜ |

### Phase 2: Advanced Features (Week 2)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 2.1 | Implement Subscriptions | subscription.rs | 6h | ⬜ |
| 2.2 | Add query complexity limits | Config | 2h | ⬜ |
| 2.3 | Implement Mutations | mutation.rs | 6h | ⬜ |
| 2.4 | Add authentication | JWT integration | 4h | ⬜ |
| 2.5 | Write tests | Unit + Integration | 6h | ⬜ |

### Phase 3: UI Integration (Week 3)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.1 | Add Apollo Client to UI | graphql-client.js | 4h | ⬜ |
| 3.2 | Generate TypeScript types | Codegen | 2h | ⬜ |
| 3.3 | Migrate graph.js to GraphQL | graph.js | 6h | ⬜ |
| 3.4 | Migrate search to GraphQL | app.js | 4h | ⬜ |
| 3.5 | Add real-time subscriptions | websocket.js | 4h | ⬜ |
| 3.6 | Performance testing | Benchmark | 4h | ⬜ |

---

## ⚠️ Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Query complexity attacks | High | Medium | Implement query depth/complexity limits |
| N+1 query problems | Medium | High | Use DataLoader pattern consistently |
| Learning curve for team | Low | Medium | Provide documentation + examples |
| Breaking existing REST clients | High | Low | Keep REST API, deprecate gradually |
| Large query results | Medium | Medium | Enforce pagination, add `@cost` directive |

---

## 📊 Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|--------------------|
| Payload size reduction | ≥50% | Compare REST vs GraphQL response sizes |
| Requests per page load | ≤2 | Browser network tab |
| Time to first meaningful paint | <500ms | Lighthouse |
| Cache hit rate | >80% | Apollo Client metrics |
| Query latency (p95) | <100ms | Server metrics |

---

## 🔄 Alternatives Considered

### Option A: JSON:API
- **Pros:** Standardized, sparse fieldsets support
- **Cons:** Still REST-based, less flexible than GraphQL
- **Why rejected:** Limited nested query capabilities

### Option B: OData
- **Pros:** Powerful query language, Microsoft ecosystem
- **Cons:** Complex, XML-heavy legacy, less modern tooling
- **Why rejected:** Overkill for our use case, poor Rust support

### Option C: gRPC
- **Pros:** Very fast, strongly typed, streaming
- **Cons:** Binary protocol, harder to debug, no browser native support
- **Why rejected:** Requires grpc-web proxy, worse developer experience

### Option D: Custom query language
- **Pros:** Tailored to our needs
- **Cons:** No ecosystem, must maintain parser, learning curve
- **Why rejected:** Reinventing the wheel

---

## 🔗 Dependencies

- **Depends on:**
  - Current REST API infrastructure
  - WebSocket implementation (for subscriptions)
- **Blocks:**
  - Mobile app development (will benefit from GraphQL)
  - Third-party integrations
- **Related:**
  - [Proposed-WebSocket-UI.md](Proposed-WebSocket-UI.md)

---

## ❓ Open Questions

1. Nên support cả REST và GraphQL song song bao lâu trước khi deprecate REST?
2. Có cần implement GraphQL Federation để scale sau này không?
3. Subscription transport: dùng WebSocket hay Server-Sent Events?
4. Có cần rate limiting riêng cho GraphQL không?

---

## 📚 References

- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/en/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [Apollo Client](https://www.apollographql.com/docs/react/)
- [DataLoader Pattern](https://github.com/graphql/dataloader)
- [GraphQL Security](https://cheatsheetseries.owasp.org/cheatsheets/GraphQL_Cheat_Sheet.html)

---

## 📝 Decision Log

| Date | Decision | Rationale | Decided by |
|------|----------|-----------|------------|
| 2026-01-14 | Use async-graphql over juniper | Better async support, more active | Author |
| 2026-01-14 | Keep REST API parallel | Backward compatibility | Author |

---

## ✅ Approval Checklist

- [ ] Technical review completed
- [ ] Security review (if applicable)
- [ ] Performance impact assessed
- [ ] Documentation updated
- [ ] Tests planned
- [ ] Rollback plan defined

---

## 📊 Appendix: Query Examples

### A. UI Dashboard - Fetch Graph Overview

```graphql
query DashboardData {
  graphSummary {
    totalEntities
    totalRelations
    entityTypes { entityType count }
  }

  # Recent alerts (AML)
  searchEntities(query: "Alert:", pagination: { limit: 5 }) {
    entities {
      name
      observations
      createdAt
    }
  }
}
```

### B. Entity Detail View

```graphql
query EntityDetail($name: ID!) {
  entity(name: $name) {
    name
    entityType
    observations
    createdBy
    createdAt

    # Only fetch related if expanded
    relations(direction: BOTH, limit: 20) {
      relationType
      to { name entityType }
      from { name entityType }
    }

    # Inference on demand
    inferred(maxDepth: 2) {
      inferredRelations {
        confidence
        explanation
        relation {
          to { name }
          relationType
        }
      }
    }
  }
}
```

### C. AML Investigation

```graphql
query AMLInvestigation($customer: ID!) {
  # Customer info
  customer: entity(name: $customer) {
    name
    observations
  }

  # Direct relations
  direct: related(entityName: $customer, direction: BOTH) {
    relationType
    to { name entityType observations }
    from { name entityType }
  }

  # Inferred connections
  hidden: infer(entityName: $customer, maxDepth: 4, minConfidence: 0.3) {
    inferredRelations {
      confidence
      explanation
      relation {
        to { name entityType }
        relationType
      }
    }
    stats { nodesVisited pathsFound }
  }

  # Transaction flow
  flow: traverse(
    startNode: $customer
    path: [
      { relationType: "owns", direction: OUTGOING }
      { relationType: "debits", direction: OUTGOING }
      { relationType: "credits", direction: OUTGOING }
    ]
    maxResults: 50
  ) {
    paths { nodes relations }
    endNodes { name entityType observations }
  }
}
```

### D. Real-time Subscription

```graphql
subscription WatchAlerts {
  alertCreated {
    name
    entityType
    observations
    createdAt
  }
}
```

---

*Last updated: 2026-01-14*
