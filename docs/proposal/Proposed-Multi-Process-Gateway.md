# Proposed: Multi-Process Gateway Architecture

> **Status:** 📋 Proposed
> **Date:** 2026-01-14
> **Author:** AI Agent (Mai Thành Duy An)
> **Priority:** 🟠 High
> **Complexity:** High
> **Estimated Effort:** 4-6 weeks
> **Reviewed by:** Pending

---

## 📋 Executive Summary

Kiến trúc Multi-Process Gateway cho phép Memory Graph scale horizontally bằng cách chia knowledge graph thành nhiều shards, mỗi shard chạy trong process riêng với file storage độc lập. Gateway layer đóng vai trò router, aggregator và load balancer, cho phép hệ thống xử lý >500K entities với throughput cao và failure isolation.

**Business Value:**
- Scale từ team nhỏ (<20 người) lên enterprise (>100 người)
- Parallel writes không block nhau giữa các domains
- Fault tolerance - 1 shard crash không ảnh hưởng toàn bộ hệ thống

---

## 🎯 Goals & Non-Goals

### Goals
- [x] Design multi-process architecture với Gateway + Shards
- [ ] Support 500K-1M+ entities với response time <100ms
- [ ] Domain isolation (Sprint shard independent từ Risk shard)
- [ ] Hot reload shards không cần restart Gateway
- [ ] Cross-shard query aggregation (transparent to clients)
- [ ] Backward compatible với single-file mode

### Non-Goals
- ❌ Distributed consensus (Raft/Paxos) - quá phức tạp cho phase này
- ❌ Auto-sharding based on load - manual config trước
- ❌ Cross-datacenter replication
- ❌ Multi-tenant isolation (different orgs)

---

## 📊 Current State vs Proposed State

| Aspect | Current | Proposed |
|--------|---------|----------|
| **Process** | Single process | Gateway + N Shard processes |
| **File** | Single `memory.jsonl` | Multiple domain files |
| **Lock** | Global RwLock | Per-shard RwLock |
| **Scale** | ~50K entities | 500K-1M+ entities |
| **Failure** | All-or-nothing | Isolated per shard |
| **Write Throughput** | ~1K ops/sec | ~5K ops/sec (parallel) |
| **Cross-domain Query** | O(1) lookup | O(shards) aggregation |

---

## 🏗️ Architecture / Design

### System Diagram

```
                                    ┌─────────────────────────────────────┐
                                    │           MCP Clients               │
                                    │  (Cursor, Claude, VS Code, etc.)    │
                                    └─────────────────┬───────────────────┘
                                                      │
                                                      ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              GATEWAY PROCESS (Port 3030)                             │
├─────────────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│  │   MCP/SSE    │  │   REST API   │  │  WebSocket   │  │     Health Monitor       │ │
│  │   Handler    │  │   Handler    │  │   Handler    │  │  (Shard health checks)   │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────────┬─────────────┘ │
│         │                 │                 │                       │               │
│         └─────────────────┴────────┬────────┴───────────────────────┘               │
│                                    ▼                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────────────┐│
│  │                           REQUEST ROUTER                                         ││
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────────┐  ││
│  │  │  Entity Type    │  │  Shard Registry │  │      Query Planner              │  ││
│  │  │  → Shard Map    │  │  (Health, Addr) │  │  (Single vs Multi-shard query)  │  ││
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────────────┘│
│                                    │                                                 │
│  ┌─────────────────────────────────┼─────────────────────────────────────────────┐  │
│  │                         IPC LAYER (Async)                                      │  │
│  │  Protocol: Unix Socket (Linux) / Named Pipe (Windows) / TCP localhost         │  │
│  └─────────────────────────────────┼─────────────────────────────────────────────┘  │
└────────────────────────────────────┼────────────────────────────────────────────────┘
                                     │
         ┌───────────────┬───────────┼───────────┬───────────────┐
         ▼               ▼           ▼           ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   SHARD 1   │  │   SHARD 2   │  │   SHARD 3   │  │   SHARD 4   │  │   SHARD 5   │
│   Sprint    │  │   Time      │  │   Release   │  │   Risk      │  │   Project   │
│   :3031     │  │   :3032     │  │   :3033     │  │   :3034     │  │   :3035     │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ Entity Types│  │ Entity Types│  │ Entity Types│  │ Entity Types│  │ Entity Types│
│ • Sprint    │  │ • Timesheet │  │ • Release   │  │ • Risk      │  │ • Project   │
│ • Epic      │  │ • Weekly    │  │ • Phase     │  │ • Mitigation│  │ • Module    │
│ • Story     │  │   Summary   │  │ • Deadline  │  │ • Issue     │  │ • Feature   │
│ • Task      │  │ • Standup   │  │ • GanttTask │  │ • Meeting   │  │ • Bug       │
│ • Person    │  │ • Standup   │  │ • Critical  │  │ • Decision  │  │ • Milestone │
│             │  │   Update    │  │   Path      │  │ • ActionItem│  │ • Require   │
│             │  │ • Blocker   │  │             │  │             │  │   ment      │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ RwLock      │  │ RwLock      │  │ RwLock      │  │ RwLock      │  │ RwLock      │
│ EventStore  │  │ EventStore  │  │ EventStore  │  │ EventStore  │  │ EventStore  │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ sprint.     │  │ time.       │  │ release.    │  │ risk.       │  │ project.    │
│ jsonl       │  │ jsonl       │  │ jsonl       │  │ jsonl       │  │ jsonl       │
└─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘
```

### Cross-Shard Relation Handling

```
┌─────────────────────────────────────────────────────────────────────┐
│                     RELATION ROUTING                                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Relation: Task: OAuth → assigned_to → Dev A                        │
│            (Sprint Shard)              (Sprint Shard)                │
│            ✅ SAME SHARD - Direct write                              │
│                                                                      │
│  Relation: Task: OAuth → blocks → Release: v2.0                     │
│            (Sprint Shard)           (Release Shard)                  │
│            ⚠️ CROSS-SHARD - Gateway coordinates                      │
│                                                                      │
│  Strategy for Cross-Shard Relations:                                 │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │  Option A: Dual Write (Recommended)                              ││
│  │  - Write relation to BOTH shards                                 ││
│  │  - Each shard has full relation for its entities                 ││
│  │  - Eventual consistency via event sync                           ││
│  │                                                                   ││
│  │  Option B: Gateway Relation Store                                ││
│  │  - Cross-shard relations stored in Gateway                       ││
│  │  - Single source of truth for cross-shard                        ││
│  │  - Gateway becomes stateful (more complex)                       ││
│  │                                                                   ││
│  │  Option C: Reference Shard                                       ││
│  │  - Designate one shard as "owner" of relation                    ││
│  │  - Other shard has pointer/reference only                        ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### Data Models

```rust
// src/gateway/config.rs
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    /// Gateway HTTP port
    pub port: u16,

    /// Operation mode
    pub mode: GatewayMode,

    /// Health check interval (seconds)
    pub health_check_interval: u64,

    /// Request timeout (milliseconds)
    pub request_timeout_ms: u64,

    /// Shards configuration
    pub shards: Vec<ShardConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum GatewayMode {
    /// Single process, single file (current behavior)
    Standalone,
    /// Single process, multiple files by domain
    SoftSharding,
    /// Multi-process with Gateway routing
    Federated,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShardConfig {
    /// Unique shard identifier
    pub name: String,

    /// Shard process port (for IPC)
    pub port: u16,

    /// Path to shard's JSONL file
    pub file: PathBuf,

    /// Entity types handled by this shard
    pub entity_types: Vec<String>,

    /// Relation types owned by this shard (for cross-shard)
    #[serde(default)]
    pub relation_types: Vec<String>,

    /// Optional: separate event store path
    pub event_store_path: Option<PathBuf>,

    /// Shard weight for load balancing (future)
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 { 100 }

// src/gateway/registry.rs
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ShardRegistry {
    /// Map: EntityType -> ShardInfo
    entity_to_shard: HashMap<String, String>,

    /// Map: ShardName -> ShardConnection
    shards: HashMap<String, Arc<ShardConnection>>,

    /// Healthy shards list
    healthy_shards: RwLock<HashSet<String>>,
}

#[derive(Debug)]
pub struct ShardConnection {
    pub name: String,
    pub address: String,  // "localhost:3031"
    pub client: ShardClient,
    pub last_health_check: RwLock<Instant>,
    pub status: RwLock<ShardStatus>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShardStatus {
    Healthy,
    Degraded,  // Slow responses
    Unhealthy, // Not responding
    Starting,  // Just started, warming up
}

// src/gateway/ipc.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<ShardError>,
    pub latency_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardError {
    pub code: i32,
    pub message: String,
}
```

### Query Planner Logic

```rust
// src/gateway/planner.rs

#[derive(Debug)]
pub enum QueryPlan {
    /// Query targets single shard
    SingleShard {
        shard: String,
        request: ShardRequest,
    },

    /// Query requires multiple shards, aggregate results
    MultiShard {
        shards: Vec<String>,
        requests: Vec<ShardRequest>,
        aggregation: AggregationType,
    },

    /// Query requires all shards (e.g., search_nodes without type filter)
    Broadcast {
        requests: Vec<ShardRequest>,
        aggregation: AggregationType,
    },
}

#[derive(Debug)]
pub enum AggregationType {
    /// Merge entity lists
    MergeEntities,
    /// Merge and dedupe relations
    MergeRelations,
    /// Combine traversal paths
    MergeTraversals,
    /// Sum statistics
    SumStats,
    /// Take first successful result
    FirstSuccess,
}

impl QueryPlanner {
    pub fn plan(&self, tool: &str, params: &Value) -> QueryPlan {
        match tool {
            // Single-shard operations (if entity type known)
            "create_entities" => {
                let entity_type = extract_entity_type(params);
                if let Some(shard) = self.registry.get_shard_for_type(&entity_type) {
                    QueryPlan::SingleShard { shard, request: build_request(tool, params) }
                } else {
                    // Unknown type, use default shard
                    QueryPlan::SingleShard {
                        shard: self.default_shard.clone(),
                        request: build_request(tool, params)
                    }
                }
            }

            // Always broadcast (search across all)
            "search_nodes" => {
                QueryPlan::Broadcast {
                    requests: self.build_broadcast_requests(tool, params),
                    aggregation: AggregationType::MergeEntities,
                }
            }

            // Single shard if entity exists
            "open_nodes" | "get_related" => {
                let entity_name = extract_entity_name(params);
                if let Some(shard) = self.find_entity_shard(&entity_name) {
                    QueryPlan::SingleShard { shard, request: build_request(tool, params) }
                } else {
                    // Entity location unknown, broadcast
                    QueryPlan::Broadcast {
                        requests: self.build_broadcast_requests(tool, params),
                        aggregation: AggregationType::FirstSuccess,
                    }
                }
            }

            // Multi-hop traversal may cross shards
            "traverse" => {
                QueryPlan::MultiShard {
                    shards: self.estimate_traversal_shards(params),
                    requests: self.build_traversal_requests(params),
                    aggregation: AggregationType::MergeTraversals,
                }
            }

            // Cross-shard relation
            "create_relations" => {
                let (from_shard, to_shard) = self.get_relation_shards(params);
                if from_shard == to_shard {
                    QueryPlan::SingleShard { shard: from_shard, request: build_request(tool, params) }
                } else {
                    // Dual write to both shards
                    QueryPlan::MultiShard {
                        shards: vec![from_shard, to_shard],
                        requests: vec![
                            build_request(tool, params),
                            build_request(tool, params),
                        ],
                        aggregation: AggregationType::FirstSuccess,
                    }
                }
            }

            _ => QueryPlan::Broadcast {
                requests: self.build_broadcast_requests(tool, params),
                aggregation: AggregationType::MergeEntities,
            }
        }
    }
}
```

### IPC Protocol

```rust
// src/gateway/ipc/protocol.rs

/// Frame format for IPC messages
///
/// ┌──────────────┬──────────────┬─────────────────────┐
/// │ Length (4B)  │ Type (1B)    │ Payload (N bytes)   │
/// │ Big-endian   │ 0=Req 1=Res  │ MessagePack/JSON    │
/// └──────────────┴──────────────┴─────────────────────┘

#[derive(Debug)]
pub enum IpcTransport {
    /// Unix domain socket (Linux/macOS) - fastest
    UnixSocket(PathBuf),

    /// Named pipe (Windows)
    NamedPipe(String),

    /// TCP localhost (fallback, works everywhere)
    TcpLocal(u16),
}

impl IpcTransport {
    pub fn for_platform(shard_name: &str, port: u16) -> Self {
        #[cfg(unix)]
        {
            IpcTransport::UnixSocket(PathBuf::from(format!("/tmp/memory-graph-{}.sock", shard_name)))
        }

        #[cfg(windows)]
        {
            IpcTransport::NamedPipe(format!(r"\\.\pipe\memory-graph-{}", shard_name))
        }

        // Fallback
        #[cfg(not(any(unix, windows)))]
        {
            IpcTransport::TcpLocal(port)
        }
    }
}

// Connection pool for each shard
pub struct ShardClient {
    transport: IpcTransport,
    pool: Pool<IpcConnection>,
    config: ClientConfig,
}

impl ShardClient {
    pub async fn send(&self, request: ShardRequest) -> Result<ShardResponse> {
        let conn = self.pool.get().await?;

        let start = Instant::now();

        // Serialize request
        let payload = rmp_serde::to_vec(&request)?;

        // Send frame
        conn.write_frame(FrameType::Request, &payload).await?;

        // Read response with timeout
        let response_payload = tokio::time::timeout(
            Duration::from_millis(request.timeout_ms),
            conn.read_frame()
        ).await??;

        let mut response: ShardResponse = rmp_serde::from_slice(&response_payload)?;
        response.latency_ms = start.elapsed().as_millis() as u64;

        Ok(response)
    }
}
```

### Configuration File Format

```toml
# config/gateway.toml

[gateway]
port = 3030
mode = "federated"  # standalone | soft_sharding | federated
health_check_interval = 10  # seconds
request_timeout_ms = 5000

# Default shard for unknown entity types
default_shard = "project"

# Cross-shard relation strategy
cross_shard_strategy = "dual_write"  # dual_write | gateway_store | reference

[logging]
level = "info"
format = "json"

# Shard definitions
[[shards]]
name = "sprint"
port = 3031
file = "data/sprint.jsonl"
entity_types = ["Sprint", "Epic", "Story", "Task", "Person"]
relation_types = ["contains", "assigned_to", "depends_on"]

[[shards]]
name = "time"
port = 3032
file = "data/time.jsonl"
entity_types = ["Timesheet", "WeeklySummary", "Standup", "StandupUpdate", "Blocker"]
relation_types = ["logged_by", "logged_for", "has_blocker"]

[[shards]]
name = "release"
port = 3033
file = "data/release.jsonl"
entity_types = ["Release", "Phase", "Deadline", "GanttTask", "CriticalPath"]
relation_types = ["followed_by", "has_deadline", "includes"]

[[shards]]
name = "risk"
port = 3034
file = "data/risk.jsonl"
entity_types = ["Risk", "Mitigation", "Issue", "Meeting", "Decision", "ActionItem"]
relation_types = ["threatens", "mitigated_by", "produced", "implements"]

[[shards]]
name = "project"
port = 3035
file = "data/project.jsonl"
entity_types = ["Project", "Module", "Feature", "Bug", "Milestone", "Requirement", "Decision", "Convention"]
relation_types = ["implements", "part_of", "affects", "fixes"]
```

---

## 📁 File Structure

```
src/
├── gateway/
│   ├── mod.rs                  ← NEW: Gateway module
│   ├── config.rs               ← NEW: Configuration structs
│   ├── server.rs               ← NEW: Gateway HTTP server
│   ├── router.rs               ← NEW: Request router
│   ├── planner.rs              ← NEW: Query planner
│   ├── registry.rs             ← NEW: Shard registry
│   ├── aggregator.rs           ← NEW: Response aggregator
│   ├── health.rs               ← NEW: Health monitoring
│   └── ipc/
│       ├── mod.rs              ← NEW: IPC module
│       ├── protocol.rs         ← NEW: Frame protocol
│       ├── transport.rs        ← NEW: Transport abstraction
│       ├── client.rs           ← NEW: Shard client
│       └── pool.rs             ← NEW: Connection pool
├── shard/
│   ├── mod.rs                  ← NEW: Shard process module
│   ├── server.rs               ← NEW: Shard IPC server
│   └── handler.rs              ← NEW: Request handler
├── bin/
│   ├── memory-gateway.rs       ← NEW: Gateway binary
│   └── memory-shard.rs         ← NEW: Shard binary
├── main.rs                     ← MODIFY: Add gateway mode
├── lib.rs                      ← MODIFY: Export gateway module
└── config/
    └── gateway.toml            ← NEW: Default config

config/
├── gateway.toml                ← NEW: Gateway configuration
└── examples/
    ├── standalone.toml         ← NEW: Single process example
    ├── soft-sharding.toml      ← NEW: Soft sharding example
    └── federated.toml          ← NEW: Full federation example
```

---

## 🔧 Implementation Plan

### Phase 3.1: Foundation (Week 1)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.1.1 | Define config structs | `src/gateway/config.rs` | 2h | ⬜ |
| 3.1.2 | Create IPC protocol | `src/gateway/ipc/protocol.rs` | 4h | ⬜ |
| 3.1.3 | Implement transport abstraction | `src/gateway/ipc/transport.rs` | 4h | ⬜ |
| 3.1.4 | Add connection pool | `src/gateway/ipc/pool.rs` | 3h | ⬜ |
| 3.1.5 | Create shard client | `src/gateway/ipc/client.rs` | 3h | ⬜ |
| 3.1.6 | Write IPC tests | Tests | 4h | ⬜ |

### Phase 3.2: Shard Process (Week 2)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.2.1 | Create shard binary | `src/bin/memory-shard.rs` | 3h | ⬜ |
| 3.2.2 | Implement shard IPC server | `src/shard/server.rs` | 4h | ⬜ |
| 3.2.3 | Add request handler | `src/shard/handler.rs` | 4h | ⬜ |
| 3.2.4 | Wire up KnowledgeBase | Integration | 4h | ⬜ |
| 3.2.5 | Test single shard | Tests | 3h | ⬜ |
| 3.2.6 | Add graceful shutdown | Shutdown handling | 2h | ⬜ |

### Phase 3.3: Gateway Core (Week 3)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.3.1 | Create shard registry | `src/gateway/registry.rs` | 4h | ⬜ |
| 3.3.2 | Implement query planner | `src/gateway/planner.rs` | 6h | ⬜ |
| 3.3.3 | Add request router | `src/gateway/router.rs` | 4h | ⬜ |
| 3.3.4 | Create response aggregator | `src/gateway/aggregator.rs` | 4h | ⬜ |
| 3.3.5 | Gateway HTTP server | `src/gateway/server.rs` | 4h | ⬜ |
| 3.3.6 | Integration tests | Tests | 4h | ⬜ |

### Phase 3.4: Health & Resilience (Week 4)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.4.1 | Health check system | `src/gateway/health.rs` | 4h | ⬜ |
| 3.4.2 | Shard failure handling | Failover logic | 4h | ⬜ |
| 3.4.3 | Request retry logic | Retry with backoff | 3h | ⬜ |
| 3.4.4 | Circuit breaker | Circuit breaker pattern | 4h | ⬜ |
| 3.4.5 | Metrics collection | Prometheus metrics | 3h | ⬜ |
| 3.4.6 | Stress tests | Load testing | 4h | ⬜ |

### Phase 3.5: Cross-Shard Operations (Week 5)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.5.1 | Cross-shard relation write | Dual write impl | 6h | ⬜ |
| 3.5.2 | Cross-shard traversal | Multi-hop traversal | 6h | ⬜ |
| 3.5.3 | Broadcast search | Parallel search | 4h | ⬜ |
| 3.5.4 | Result deduplication | Dedupe logic | 3h | ⬜ |
| 3.5.5 | Integration tests | E2E tests | 4h | ⬜ |

### Phase 3.6: CLI & Deployment (Week 6)

| # | Task | Output | Est. Time | Status |
|---|------|--------|-----------|--------|
| 3.6.1 | Gateway CLI | `--config`, `--mode` flags | 3h | ⬜ |
| 3.6.2 | Shard CLI | `--shard-name`, `--port` | 2h | ⬜ |
| 3.6.3 | Process manager script | PowerShell/Bash | 4h | ⬜ |
| 3.6.4 | Docker Compose setup | `docker-compose.yml` | 4h | ⬜ |
| 3.6.5 | Documentation | README, Architecture doc | 4h | ⬜ |
| 3.6.6 | Performance benchmarks | Benchmark suite | 4h | ⬜ |

---

## ⚠️ Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Cross-shard query latency | High | Medium | Parallel execution, caching |
| Shard crash during write | High | Low | Event sourcing replay, WAL |
| Network partition (local) | Medium | Low | Health checks, failover |
| Data inconsistency (dual write) | High | Medium | Eventual consistency, conflict resolution |
| Complexity overhead | Medium | High | Fallback to standalone mode |
| Memory overhead (N processes) | Medium | Medium | Shared memory for read-only data |

---

## 📊 Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|--------------------|
| P99 latency (single shard) | <50ms | Prometheus histogram |
| P99 latency (cross-shard) | <150ms | Prometheus histogram |
| Throughput (writes) | >5K ops/sec | Load test |
| Throughput (reads) | >20K ops/sec | Load test |
| Shard startup time | <2s | Timing logs |
| Gateway startup time | <1s | Timing logs |
| Memory per shard (100K entities) | <500MB | Process monitoring |
| Test coverage | >80% | Code coverage |

---

## 🔄 Alternatives Considered

### Option A: Actor Model (Actix)
- **Pros:** Proven pattern, good Rust support
- **Cons:** Single process limit, complex lifecycle
- **Why rejected:** Still single process, doesn't scale CPU

### Option B: SQLite per Shard
- **Pros:** Better query capability, ACID
- **Cons:** Adds dependency, migration complexity
- **Why rejected:** Phase 4 consideration, not Phase 3

### Option C: Embedded Consensus (Raft)
- **Pros:** Strong consistency, auto-failover
- **Cons:** Massive complexity, overkill for local deploy
- **Why rejected:** Enterprise-only need, Phase 5+

---

## 🔗 Dependencies

- **Depends on:**
  - ✅ Event Sourcing Architecture (completed)
  - ✅ RwLock Migration (completed)
  - ✅ WebSocket Real-time (completed)

- **Blocks:**
  - Auto-Sharding based on Load
  - Multi-Datacenter Replication

- **Related:**
  - [Proposed-Team-Collaboration.md](./Proposed-Team-Collaboration.md)
  - [Proposed-Event-Sourcing.md](./Proposed-Event-Sourcing.md)

---

## ❓ Open Questions

1. **Cross-shard transaction**: Nếu dual-write fail 1 shard, rollback hay accept inconsistency?
2. **Entity migration**: Khi cần move entity sang shard khác, quy trình như thế nào?
3. **Shard rebalancing**: Auto-split shard khi quá lớn?
4. **Backup strategy**: Backup từng shard hay coordinated backup?
5. **Query cache**: Gateway-level cache hay per-shard cache?

---

## 📚 References

- [Vitess - MySQL Sharding](https://vitess.io/docs/concepts/shard/)
- [CockroachDB Architecture](https://www.cockroachlabs.com/docs/stable/architecture/overview.html)
- [Tokio IPC Example](https://github.com/tokio-rs/tokio/tree/master/examples)
- [Named Pipes in Rust](https://docs.rs/named-pipe/latest/named_pipe/)
- Related docs: [IDEA.md](../IDEA.md)

---

## 📝 Decision Log

| Date | Decision | Rationale | Decided by |
|------|----------|-----------|------------|
| 2026-01-14 | Use IPC over HTTP for shard communication | Lower latency, less overhead | AI Agent |
| 2026-01-14 | Dual-write for cross-shard relations | Simplest approach, acceptable for local | AI Agent |
| 2026-01-14 | Config file over environment variables | Complex config, easier to manage | AI Agent |

---

## ✅ Approval Checklist

- [ ] Technical review completed
- [ ] Security review (if applicable)
- [ ] Performance impact assessed
- [ ] Documentation updated
- [ ] Tests planned
- [ ] Rollback plan defined (fallback to standalone mode)

---

*Last updated: 2026-01-14*
