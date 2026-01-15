# Scaling Memory Graph to Billion Nodes

> **Status:** 📋 Research | **Date:** 2026-01-15 | **Author:** AI Agent

## Executive Summary

Phân tích khả năng mở rộng Memory Graph MCP Server để lưu trữ hàng tỷ node, bao gồm các chiến lược Scale-up, Scale-out, File Sharding, Multi-threading/Clustering, và Network IO/Cryptography.

**Kết luận chính:** Kiến trúc hiện tại phù hợp cho ~50K-500K entities. Để đạt billion-node scale cần:
1. **Storage redesign** - chuyển từ in-memory sang disk-backed
2. **Index structures** - thay `Vec` bằng HashMap/B-tree
3. **Partitioning** - implement domain-based sharding
4. **Lazy loading** - chỉ load partitions cần thiết vào RAM

---

## Table of Contents

1. [Current Architecture Analysis](#1-current-architecture-analysis)
2. [Memory Footprint Estimation](#2-memory-footprint-estimation)
3. [Scale-Up Strategies](#3-scale-up-strategies)
4. [Scale-Out Strategies](#4-scale-out-strategies)
5. [File Sharding Design](#5-file-sharding-design)
6. [Multi-Threading & Clustering](#6-multi-threading--clustering)
7. [Network IO & Cryptography](#7-network-io--cryptography)
8. [Phased Implementation Plan](#8-phased-implementation-plan)
9. [Technology Comparison](#9-technology-comparison)
10. [Recommendations](#10-recommendations)

---

## 1. Current Architecture Analysis

### 1.1 Data Structures

```rust
// src/types/graph.rs
pub struct KnowledgeGraph {
    pub entities: Vec<Entity>,      // ← Linear storage, O(n) lookup
    pub relations: Vec<Relation>,   // ← No indexing
}

// src/knowledge_base/mod.rs
pub struct KnowledgeBase {
    pub(crate) graph: RwLock<KnowledgeGraph>,  // ← ALL DATA IN RAM
    pub(crate) event_store: Option<Mutex<EventStore>>,
    // ...
}
```

### 1.2 Storage Format

| Mode | Files | Description |
|------|-------|-------------|
| **Legacy** | `memory.jsonl` | Single file với entities + relations |
| **Event Sourcing** | `events.jsonl` + `snapshots/` | Append-only log + periodic snapshots |

### 1.3 Concurrency Model

| Resource | Lock Type | Contention |
|----------|-----------|------------|
| `graph` | `RwLock` | Writers block ALL readers |
| `event_store` | `Mutex` | Serialized event appending |

### 1.4 Current Bottlenecks

| Bottleneck | Location | Impact |
|------------|----------|--------|
| **All data in RAM** | `KnowledgeBase::graph` | Memory-bound scaling |
| **Linear search** | `entities.iter().find()` | O(n) entity lookup |
| **Global lock** | `RwLock<KnowledgeGraph>` | Throughput ceiling |
| **Single event file** | `events.jsonl` | I/O bottleneck |
| **Full replay on startup** | `EventStore::replay()` | Slow cold start |

---

## 2. Memory Footprint Estimation

### 2.1 Per-Entity Memory

```rust
pub struct Entity {
    pub name: String,           // 24 bytes + content (~20-100 chars)
    pub entity_type: String,    // 24 bytes + content (~10-30 chars)
    pub observations: Vec<String>, // 24 bytes + N×(24 + content)
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: u64,        // 8 bytes
    pub updated_at: u64,        // 8 bytes
}
// Minimum: ~136 bytes | Realistic: ~300-800 bytes
```

### 2.2 Per-Relation Memory

```rust
pub struct Relation {
    pub from: String,           // 24 bytes + content
    pub to: String,             // 24 bytes + content
    pub relation_type: String,  // 24 bytes + content
    pub created_by: Option<String>,
    pub created_at: u64,
    pub valid_from: Option<u64>,
    pub valid_to: Option<u64>,
}
// Minimum: ~136 bytes | Realistic: ~200-400 bytes
```

### 2.3 Scale Projections

| Scale | Entities | Relations | Est. RAM | Feasibility |
|-------|----------|-----------|----------|-------------|
| **Small** | 10K | 20K | ~10-20 MB | ✅ Trivial |
| **Medium** | 100K | 200K | ~100-200 MB | ✅ Easy |
| **Large** | 1M | 2M | ~1-2 GB | ⚠️ High-RAM server |
| **Huge** | 10M | 20M | ~10-20 GB | ⚠️ Dedicated machine |
| **Massive** | 100M | 200M | ~100-200 GB | ❌ Impractical in-RAM |
| **Billion** | 1B | 2B | ~500 GB - 1 TB | ❌ Impossible in-RAM |

---

## 3. Scale-Up Strategies

### 3.1 Strategy Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      SCALE-UP OPTIONS                        │
├─────────────────┬───────────────┬───────────────────────────┤
│     Approach    │    Effort     │     Capacity Gain         │
├─────────────────┼───────────────┼───────────────────────────┤
│ HashMap Index   │ Low (1 week)  │ 2-5x throughput           │
│ Memory-Mapped   │ Medium (3 wk) │ 10-50x capacity           │
│ Embedded DB     │ Medium (4 wk) │ 100x+ capacity            │
│ Fine-grained    │ Medium (3 wk) │ N/A (throughput only)     │
│ Locks           │               │                           │
└─────────────────┴───────────────┴───────────────────────────┘
```

### 3.2 HashMap Indexing (Quick Win)

**Current:**
```rust
// O(n) linear search
entities.iter().find(|e| e.name == name)
```

**Proposed:**
```rust
pub struct KnowledgeGraph {
    pub entities: HashMap<String, Entity>,  // O(1) lookup by name
    pub relations: Vec<Relation>,
    pub relation_index: HashMap<String, Vec<usize>>,  // Index by from/to
}
```

**Benefits:**
- Entity lookup: O(n) → O(1)
- Relation query by entity: O(n) → O(k) where k = related relations
- Minimal code changes
- No new dependencies

**Implementation Effort:** 1-2 weeks

### 3.3 Memory-Mapped Files (mmap)

**Concept:** Let OS manage which parts of data are in RAM

```rust
use memmap2::MmapMut;

pub struct DiskBackedGraph {
    entity_file: MmapMut,      // OS pages in/out as needed
    relation_file: MmapMut,
    index: HashMap<String, u64>,  // name → file offset
}
```

**Benefits:**
- Capacity limited by disk, not RAM
- OS handles caching intelligently
- Data persists without explicit writes

**Drawbacks:**
- Complex serialization format required
- Platform-specific behavior (Windows vs Linux)
- Variable record sizes complicate offset calculation

**Implementation Effort:** 3-4 weeks

### 3.4 Embedded Database

| Database | Type | Rust Support | Pros | Cons |
|----------|------|--------------|------|------|
| **RocksDB** | Key-Value | `rust-rocksdb` | Battle-tested, Facebook scale | C++ dependency, learning curve |
| **SurrealDB** | Graph-native | Native Rust | Perfect fit, embedded mode | Newer, less proven |
| **SQLite** | Relational | `rusqlite` | Portable, well-known | Not graph-optimized |
| **Sled** | Key-Value | Native Rust | Pure Rust, lock-free | Less mature |
| **ReDB** | Key-Value | Native Rust | Simple, ACID | Limited features |

**Recommended: SurrealDB (embedded mode)**

```rust
use surrealdb::Surreal;
use surrealdb::engine::local::File;

async fn init_db() -> Result<Surreal<File>> {
    let db = Surreal::new::<File>("data/graph.db").await?;
    db.use_ns("memory_graph").use_db("knowledge").await?;
    Ok(db)
}

// Native graph queries
let related: Vec<Entity> = db.query(
    "SELECT * FROM entity WHERE ->relates_to->entity.name = $name"
).bind(("name", "Alice")).await?;
```

**Implementation Effort:** 4-6 weeks

### 3.5 Fine-Grained Locking

**Current:** Single `RwLock` for entire graph

**Proposed:** Sharded locks by partition key

```rust
use dashmap::DashMap;

pub struct ShardedGraph {
    // Each entity type has its own lock
    entities: DashMap<String, DashMap<String, Entity>>,
    // entity_type -> (name -> Entity)
}
```

**Benefits:**
- Parallel writes to different partitions
- No global lock contention
- Compatible with other strategies

**Dependencies:**
```toml
dashmap = "5"
parking_lot = "0.12"  # Faster RwLock implementation
```

---

## 4. Scale-Out Strategies

### 4.1 Strategy Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     SCALE-OUT OPTIONS                        │
├─────────────────┬───────────────┬───────────────────────────┤
│     Approach    │    Effort     │     Capacity Gain         │
├─────────────────┼───────────────┼───────────────────────────┤
│ Multi-Process   │ High (6 wk)   │ 5-10x (per node added)    │
│ Gateway         │               │                           │
├─────────────────┼───────────────┼───────────────────────────┤
│ Distributed DB  │ High (8 wk)   │ Unlimited (with infra)    │
│ Backend         │               │                           │
├─────────────────┼───────────────┼───────────────────────────┤
│ Raft Consensus  │ Very High     │ N/A (HA, not scale)       │
│ Cluster         │ (12+ wk)      │                           │
└─────────────────┴───────────────┴───────────────────────────┘
```

### 4.2 Multi-Process Gateway (Proposed)

> Reference: [Proposed-Multi-Process-Gateway.md](../proposal/Proposed-Multi-Process-Gateway.md)

**Architecture:**
```
                    ┌─────────────────┐
                    │   MCP Clients   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │    Gateway      │
                    │   (Port 3030)   │
                    │  - Routing      │
                    │  - Aggregation  │
                    │  - Health Check │
                    └────────┬────────┘
                             │
        ┌──────────┬─────────┼─────────┬──────────┐
        │          │         │         │          │
   ┌────▼───┐ ┌────▼───┐ ┌───▼────┐ ┌──▼─────┐ ┌──▼─────┐
   │ Shard  │ │ Shard  │ │ Shard  │ │ Shard  │ │ Shard  │
   │ Sprint │ │ Time   │ │Release │ │ Risk   │ │Project │
   │ :3031  │ │ :3032  │ │ :3033  │ │ :3034  │ │ :3035  │
   └────────┘ └────────┘ └────────┘ └────────┘ └────────┘
```

**Sharding Strategy:**
- **Domain-based:** Entity type determines shard
- **Routing table:** Gateway maps entity types to shards
- **Cross-shard relations:** Dual-write to both involved shards

**IPC Options:**
| Transport | Latency | Throughput | Complexity |
|-----------|---------|------------|------------|
| Unix Socket | ~10μs | High | Low |
| Named Pipe (Windows) | ~20μs | Medium | Low |
| TCP localhost | ~50μs | High | Medium |
| gRPC | ~100μs | Very High | High |

### 4.3 Distributed Database Backend

**Option A: PostgreSQL + pg_graphql**
```
┌─────────────────┐     ┌─────────────────┐
│  Memory Graph   │────▶│   PostgreSQL    │
│     Server      │     │  (pg_graphql)   │
└─────────────────┘     └─────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
               ┌────▼────┐         ┌────▼────┐
               │ Replica │         │ Replica │
               └─────────┘         └─────────┘
```

**Option B: SurrealDB Cluster**
```
┌─────────────────┐     ┌─────────────────────────────┐
│  Memory Graph   │────▶│    SurrealDB Cluster        │
│     Server      │     │  ┌───────┐  ┌───────┐       │
└─────────────────┘     │  │Node 1 │──│Node 2 │       │
                        │  └───────┘  └───────┘       │
                        │       │          │          │
                        │  ┌────▼──────────▼────┐     │
                        │  │     TiKV Store     │     │
                        │  └────────────────────┘     │
                        └─────────────────────────────┘
```

### 4.4 Consistent Hashing for Sharding

```rust
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

pub struct ConsistentHash {
    ring: BTreeMap<u64, String>,  // hash -> shard_id
    replicas: usize,              // Virtual nodes per shard
}

impl ConsistentHash {
    pub fn get_shard(&self, key: &str) -> &str {
        let hash = self.hash_key(key);
        self.ring.range(hash..)
            .next()
            .or_else(|| self.ring.iter().next())
            .map(|(_, shard)| shard.as_str())
            .unwrap()
    }

    pub fn add_shard(&mut self, shard_id: &str) {
        for i in 0..self.replicas {
            let key = format!("{}:{}", shard_id, i);
            let hash = self.hash_key(&key);
            self.ring.insert(hash, shard_id.to_string());
        }
    }
}
```

**Benefits:**
- Minimal redistribution when adding/removing shards
- Even distribution with virtual nodes
- Predictable routing

---

## 5. File Sharding Design

### 5.1 Sharding Strategies

| Strategy | Partition Key | Use Case | Pros | Cons |
|----------|---------------|----------|------|------|
| **Domain-based** | `entity_type` | Known entity types | Simple routing | Uneven distribution |
| **Hash-based** | `hash(name)` | Unknown patterns | Even distribution | Cross-shard queries hard |
| **Time-based** | `created_at` | Event logs | Efficient archival | Hot partition for recent |
| **Hybrid** | `type + hash` | Large scale | Balanced | Complex routing |

### 5.2 Domain-Based Sharding

```
data/
├── shards/
│   ├── project/
│   │   ├── events.jsonl
│   │   └── snapshots/
│   ├── feature/
│   │   ├── events.jsonl
│   │   └── snapshots/
│   ├── bug/
│   │   ├── events.jsonl
│   │   └── snapshots/
│   └── person/
│       ├── events.jsonl
│       └── snapshots/
└── global/
    ├── cross_shard_relations.jsonl
    └── routing_table.json
```

**Routing Table:**
```json
{
  "shards": {
    "project": ["Project", "Module", "Repository"],
    "feature": ["Feature", "Task", "Story"],
    "bug": ["Bug", "Issue", "Defect"],
    "person": ["Person", "Team", "Organization"]
  },
  "default": "misc"
}
```

### 5.3 Hash-Based Sharding

```rust
pub fn get_shard(entity_name: &str, num_shards: usize) -> usize {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    entity_name.hash(&mut hasher);
    (hasher.finish() as usize) % num_shards
}

// Entity "Alice" → shard 3
// Entity "Bob" → shard 7
// Entity "Project: X" → shard 1
```

**File Structure:**
```
data/
├── shard_00/
│   ├── events.jsonl
│   └── snapshots/
├── shard_01/
│   ├── events.jsonl
│   └── snapshots/
...
├── shard_15/
│   ├── events.jsonl
│   └── snapshots/
└── metadata/
    └── shard_map.json
```

### 5.4 Time-Based Partitioning (Event Logs)

```
data/
├── events/
│   ├── 2026/
│   │   ├── 01/
│   │   │   ├── 2026-01-01.jsonl
│   │   │   ├── 2026-01-02.jsonl
│   │   │   └── ...
│   │   └── 02/
│   └── archive/
│       └── 2025.tar.gz
└── snapshots/
    └── latest.jsonl
```

**Benefits:**
- Easy archival of old data
- Efficient range queries by time
- Natural log rotation

### 5.5 Cross-Shard Relations

**Challenge:** Relation between entities in different shards

**Solution 1: Dual-Write**
```rust
async fn create_relation(rel: Relation) {
    let from_shard = get_shard(&rel.from);
    let to_shard = get_shard(&rel.to);

    // Write to both shards
    from_shard.write_relation(&rel).await?;
    if from_shard != to_shard {
        to_shard.write_relation(&rel).await?;
    }
}
```

**Solution 2: Global Relation Index**
```rust
pub struct GlobalRelationIndex {
    // (from, to) -> shard containing full relation data
    index: HashMap<(String, String), String>,
}
```

**Solution 3: Relation Router**
```
┌─────────────────────────────────────────────┐
│             Relation Router                  │
│  ┌─────────────────────────────────────┐    │
│  │ from_entity │ to_entity │ shard_id │    │
│  ├─────────────┼───────────┼──────────┤    │
│  │ Alice       │ NYC       │ shard_02 │    │
│  │ Bob         │ Project:X │ shard_05 │    │
│  └─────────────┴───────────┴──────────┘    │
└─────────────────────────────────────────────┘
```

---

## 6. Multi-Threading & Clustering

### 6.1 Current Threading Model

```rust
// Single-threaded MCP handler
async fn handle_request(kb: Arc<KnowledgeBase>, request: Request) {
    let graph = kb.graph.read().unwrap();  // Blocks on global lock
    // ... process
}
```

### 6.2 Improved Threading with DashMap

```rust
use dashmap::DashMap;
use parking_lot::RwLock;

pub struct ConcurrentKnowledgeBase {
    // Sharded by entity type
    entities: DashMap<String, DashMap<String, Entity>>,
    // entity_type -> (name -> Entity)

    // Relations indexed by from entity
    relations_by_from: DashMap<String, Vec<Relation>>,

    // Event store per shard
    event_stores: DashMap<String, RwLock<EventStore>>,
}

impl ConcurrentKnowledgeBase {
    pub fn get_entity(&self, name: &str, entity_type: &str) -> Option<Entity> {
        self.entities
            .get(entity_type)?
            .get(name)
            .map(|e| e.clone())
    }

    pub fn create_entity(&self, entity: Entity) {
        let type_map = self.entities
            .entry(entity.entity_type.clone())
            .or_insert_with(DashMap::new);
        type_map.insert(entity.name.clone(), entity);
    }
}
```

**Benefits:**
- Parallel reads to different entity types
- No global lock contention
- Lock-free reads in many cases

### 6.3 Async I/O Optimization

```rust
use tokio::sync::RwLock as AsyncRwLock;
use tokio::io::{AsyncWriteExt, BufWriter};

pub struct AsyncEventStore {
    file: AsyncRwLock<BufWriter<tokio::fs::File>>,
    buffer_size: usize,
}

impl AsyncEventStore {
    pub async fn append(&self, event: &Event) -> Result<()> {
        let json = serde_json::to_string(event)?;
        let mut file = self.file.write().await;
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;

        // Periodic flush instead of per-write
        if self.should_flush() {
            file.flush().await?;
        }
        Ok(())
    }
}
```

### 6.4 Work-Stealing Thread Pool

```rust
use rayon::prelude::*;

impl KnowledgeBase {
    pub fn search_parallel(&self, query: &str) -> Vec<Entity> {
        let graph = self.graph.read().unwrap();

        graph.entities
            .par_iter()  // Parallel iterator
            .filter(|e| e.matches_query(query))
            .cloned()
            .collect()
    }
}
```

### 6.5 Clustering with Raft Consensus

> **Note:** This is for High Availability, not horizontal scaling

```
┌─────────────────────────────────────────────────────────────┐
│                     Raft Cluster (3 nodes)                   │
│                                                              │
│   ┌─────────┐      ┌─────────┐      ┌─────────┐             │
│   │ Leader  │◄────▶│Follower │◄────▶│Follower │             │
│   │ Node 1  │      │ Node 2  │      │ Node 3  │             │
│   └────┬────┘      └────┬────┘      └────┬────┘             │
│        │                │                │                   │
│   ┌────▼────┐      ┌────▼────┐      ┌────▼────┐             │
│   │  Log    │      │  Log    │      │  Log    │             │
│   │(events) │      │(events) │      │(events) │             │
│   └─────────┘      └─────────┘      └─────────┘             │
│                                                              │
│   All writes go to Leader → replicated to Followers         │
│   Reads can go to any node (with linearizable option)       │
└─────────────────────────────────────────────────────────────┘
```

**Rust Raft Libraries:**
- `raft-rs` (TiKV's implementation)
- `openraft` (Async-first design)

**Use Case:** When you need:
- Automatic failover
- Strong consistency guarantees
- Geographic distribution

**Not for:** Pure horizontal scaling (adds latency, not capacity)

---

## 7. Network IO & Cryptography

### 7.1 Current Network Support

| Protocol | Status | Location |
|----------|--------|----------|
| **stdio** | ✅ Active | `main.rs` |
| **HTTP/REST** | ✅ Active | `api/http.rs` |
| **SSE** | ✅ Active | `api/sse/` |
| **WebSocket** | ✅ Active | `api/websocket/` |

### 7.2 Cluster Communication Protocols

| Protocol | Latency | Throughput | Use Case |
|----------|---------|------------|----------|
| **TCP + MessagePack** | Low | High | Simple IPC |
| **gRPC** | Medium | Very High | Structured APIs |
| **QUIC** | Low | Very High | Modern, multiplexed |
| **Unix Socket** | Very Low | High | Same-machine IPC |

**Recommended: gRPC for cross-node, Unix Socket for same-node**

```toml
# Cargo.toml additions
tonic = "0.10"           # gRPC
prost = "0.12"           # Protocol Buffers
rmp-serde = "1"          # MessagePack
quinn = "0.10"           # QUIC
```

### 7.3 Protocol Buffer Definition

```protobuf
// proto/memory_graph.proto
syntax = "proto3";

package memory_graph;

service MemoryGraphCluster {
    // Cross-shard entity lookup
    rpc GetEntity(EntityRequest) returns (EntityResponse);

    // Cross-shard relation query
    rpc GetRelations(RelationRequest) returns (RelationResponse);

    // Cluster health
    rpc HealthCheck(Empty) returns (HealthResponse);

    // Replication
    rpc ReplicateEvent(Event) returns (Ack);
}

message Entity {
    string name = 1;
    string entity_type = 2;
    repeated string observations = 3;
    uint64 created_at = 4;
    uint64 updated_at = 5;
}
```

### 7.4 TLS/mTLS for Secure Communication

```rust
use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;

async fn create_tls_server() -> Result<TlsAcceptor> {
    let certs = load_certs("certs/server.crt")?;
    let key = load_private_key("certs/server.key")?;

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(client_verifier)  // mTLS
        .with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
```

**Certificate Management:**
```
certs/
├── ca.crt              # Cluster CA
├── server.crt          # Server certificate
├── server.key          # Server private key
├── client.crt          # Client certificate (for mTLS)
└── client.key          # Client private key
```

### 7.5 Encryption at Rest

**Option 1: File-level encryption (simple)**
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

pub fn encrypt_file(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(b"unique nonce"); // Use random in production
    cipher.encrypt(nonce, data)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))
}
```

**Option 2: Transparent encryption (SQLCipher-style)**
```rust
// If using SQLite
use rusqlite::Connection;

let conn = Connection::open("encrypted.db")?;
conn.execute("PRAGMA key = 'your-secret-key'", [])?;
```

### 7.6 JWT Authentication (Already Implemented)

```rust
// Current: src/api/http.rs uses jsonwebtoken
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,        // Subject (user ID)
    exp: usize,         // Expiration
    iat: usize,         // Issued at
    roles: Vec<String>, // User roles
}
```

### 7.7 API Key for Service-to-Service

```rust
pub struct ApiKeyAuth {
    keys: HashMap<String, ServiceConfig>,
}

#[derive(Clone)]
pub struct ServiceConfig {
    pub service_name: String,
    pub allowed_operations: Vec<String>,
    pub rate_limit: u32,
}

impl ApiKeyAuth {
    pub fn verify(&self, api_key: &str) -> Option<&ServiceConfig> {
        self.keys.get(api_key)
    }
}
```

---

## 8. Phased Implementation Plan

### Phase 1: Quick Wins (2-3 weeks)

**Goal:** 5x improvement with minimal changes

| Task | Effort | Impact |
|------|--------|--------|
| Replace `Vec<Entity>` with `HashMap` | 3 days | O(n) → O(1) lookup |
| Add relation indices | 3 days | Faster traversal |
| Increase snapshot threshold | 1 day | Less I/O |
| Add `parking_lot` RwLock | 2 days | Faster locking |

**Expected Capacity:** 100K → 500K entities

### Phase 2: Storage Backend (4-6 weeks)

**Goal:** Disk-backed storage, 100x capacity

| Task | Effort | Impact |
|------|--------|--------|
| Integrate SurrealDB embedded | 2 weeks | Disk-backed storage |
| Implement lazy loading | 1 week | On-demand data loading |
| Add LRU cache layer | 1 week | Hot data in memory |
| Migrate existing data | 1 week | Backward compatibility |

**Expected Capacity:** 500K → 10M+ entities

### Phase 3: Horizontal Scaling (8-12 weeks)

**Goal:** Multi-node deployment

| Task | Effort | Impact |
|------|--------|--------|
| Implement Gateway process | 3 weeks | Request routing |
| Domain-based sharding | 2 weeks | Parallel processing |
| Cross-shard query aggregation | 2 weeks | Unified view |
| gRPC communication | 2 weeks | Inter-node protocol |
| Monitoring & health checks | 1 week | Observability |

**Expected Capacity:** 10M → 1B+ entities (across cluster)

### Phase 4: Enterprise Features (12+ weeks)

**Goal:** Production-ready for enterprise

| Task | Effort | Impact |
|------|--------|--------|
| Raft consensus (HA) | 4 weeks | Fault tolerance |
| mTLS authentication | 2 weeks | Security |
| Encryption at rest | 2 weeks | Compliance |
| Multi-tenant isolation | 3 weeks | SaaS readiness |
| Geographic replication | 4 weeks | Global deployment |

---

## 9. Technology Comparison

### 9.1 Embedded Databases

| Feature | RocksDB | SurrealDB | SQLite | Sled | ReDB |
|---------|---------|-----------|--------|------|------|
| **Language** | C++ | Rust | C | Rust | Rust |
| **Model** | KV | Multi-model | Relational | KV | KV |
| **Graph Support** | ❌ | ✅ Native | ❌ | ❌ | ❌ |
| **Transactions** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Proven at Scale** | ✅ Facebook | ⚠️ Growing | ✅ Everywhere | ⚠️ | ⚠️ |
| **Binary Size** | Large | Medium | Small | Small | Small |
| **Learning Curve** | Medium | Low | Low | Low | Low |

**Recommendation:** SurrealDB for graph-native queries, RocksDB for proven reliability

### 9.2 Distributed Databases

| Feature | CockroachDB | TiDB | YugabyteDB | SurrealDB Cluster |
|---------|-------------|------|------------|-------------------|
| **Protocol** | PostgreSQL | MySQL | PostgreSQL | HTTP/WebSocket |
| **Consistency** | Strong | Strong | Strong | Strong |
| **Graph Queries** | ❌ | ❌ | ❌ | ✅ |
| **Self-hosted** | ✅ | ✅ | ✅ | ✅ |
| **Operational Complexity** | High | High | High | Medium |

### 9.3 Message Queues (for event streaming)

| Feature | Kafka | NATS | Redis Streams | Redpanda |
|---------|-------|------|---------------|----------|
| **Throughput** | Very High | Very High | High | Very High |
| **Latency** | Medium | Low | Low | Low |
| **Persistence** | ✅ | Optional | ✅ | ✅ |
| **Complexity** | High | Low | Low | Medium |

---

## 10. Recommendations

### 10.1 For Personal/Team Use (Current + Phase 1)

**Target:** 50K - 500K entities

```
┌─────────────────────────────────────────┐
│          Single Node Architecture        │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │         Memory Graph              │   │
│  │  ┌────────────────────────────┐  │   │
│  │  │ HashMap<String, Entity>    │  │   │
│  │  │ HashMap<String, Vec<Rel>>  │  │   │
│  │  └────────────────────────────┘  │   │
│  │              │                    │   │
│  │  ┌───────────▼───────────────┐   │   │
│  │  │    Event Sourcing Log     │   │   │
│  │  │    (events.jsonl)         │   │   │
│  │  └───────────────────────────┘   │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Action Items:**
1. ✅ Implement HashMap indexing
2. ✅ Add `parking_lot` for faster locks
3. ✅ Tune snapshot frequency

### 10.2 For Startup/SMB (Phase 2)

**Target:** 500K - 10M entities

```
┌─────────────────────────────────────────┐
│       Disk-Backed Architecture           │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │         Memory Graph              │   │
│  │  ┌────────────────────────────┐  │   │
│  │  │      LRU Cache (Hot)       │  │   │
│  │  └────────────┬───────────────┘  │   │
│  │               │                   │   │
│  │  ┌────────────▼───────────────┐  │   │
│  │  │   SurrealDB (Embedded)     │  │   │
│  │  │   - Graph storage          │  │   │
│  │  │   - Native queries         │  │   │
│  │  └───────────────────────────┘   │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Action Items:**
1. Integrate SurrealDB embedded
2. Implement cache layer
3. Add lazy loading

### 10.3 For Enterprise (Phase 3+)

**Target:** 10M - 1B+ entities

```
┌─────────────────────────────────────────────────────────────┐
│                  Distributed Architecture                    │
│                                                              │
│  ┌─────────────────┐                                        │
│  │    Load         │                                        │
│  │   Balancer      │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│  ┌────────▼────────┐                                        │
│  │    Gateway      │◄──── gRPC ────┐                        │
│  │    Cluster      │               │                        │
│  └────────┬────────┘               │                        │
│           │                        │                        │
│  ┌────────┴────────────────────────┴────────┐               │
│  │                                           │               │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐     │               │
│  │  │ Shard 1 │ │ Shard 2 │ │ Shard N │     │               │
│  │  │ (Raft)  │ │ (Raft)  │ │ (Raft)  │     │               │
│  │  └────┬────┘ └────┬────┘ └────┬────┘     │               │
│  │       │           │           │          │               │
│  │  ┌────▼───────────▼───────────▼────┐     │               │
│  │  │     Distributed Storage          │     │               │
│  │  │     (SurrealDB / TiKV)           │     │               │
│  │  └──────────────────────────────────┘     │               │
│  │                                           │               │
│  └───────────────────────────────────────────┘               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Action Items:**
1. Implement Multi-Process Gateway
2. Set up domain-based sharding
3. Add Raft consensus for HA
4. Deploy monitoring stack

---

## Appendix A: Dependency Additions

```toml
# Cargo.toml additions for scaling

[dependencies]
# Phase 1: Quick Wins
parking_lot = "0.12"       # Faster RwLock

# Phase 2: Storage Backend
surrealdb = { version = "1", features = ["kv-rocksdb"] }
moka = "0.12"              # LRU cache

# Phase 3: Clustering
tonic = "0.10"             # gRPC
prost = "0.12"             # Protocol Buffers
dashmap = "5"              # Concurrent HashMap
rmp-serde = "1"            # MessagePack

# Phase 4: Enterprise
rustls = "0.21"            # TLS
openraft = "0.8"           # Raft consensus
aes-gcm = "0.10"           # Encryption
```

## Appendix B: Benchmark Targets

| Operation | Current | Phase 1 | Phase 2 | Phase 3 |
|-----------|---------|---------|---------|---------|
| Entity lookup | 10ms (1M) | 0.01ms | 0.1ms (disk) | 1ms (network) |
| Create entity | 1ms | 1ms | 5ms | 10ms |
| Traverse 3-hop | 100ms | 10ms | 50ms | 100ms (cross-shard) |
| Full text search | 500ms | 50ms | 100ms | 200ms |
| Startup time | 30s (1M) | 30s | 1s (lazy) | 5s (per shard) |

## Appendix C: Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Data corruption during migration | Medium | High | Backup, staged rollout |
| Performance regression | Medium | Medium | Extensive benchmarking |
| Increased operational complexity | High | Medium | Documentation, automation |
| Cross-shard consistency issues | Medium | High | Careful dual-write logic |
| Cluster partition (split-brain) | Low | Critical | Raft consensus |

---

## References

- [Proposed-Multi-Process-Gateway.md](../proposal/Proposed-Multi-Process-Gateway.md)
- [Proposed-Team-Collaboration.md](../proposal/Proposed-Team-Collaboration.md)
- [Proposed-RwLock.md](../proposal/Proposed-RwLock.md)
- [Event-Sourcing-Architecture.md](../knowledge/Event-Sourcing-Architecture.md)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [RocksDB Wiki](https://github.com/facebook/rocksdb/wiki)
- [Raft Consensus Paper](https://raft.github.io/raft.pdf)
