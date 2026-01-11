# Proposed: WebSocket Real-time UI Updates

> **Status:** 📋 Proposed
> **Date:** 2026-01-11
> **Priority:** P1 - After Inference Engine
> **Complexity:** Medium
> **Estimated Effort:** ~8 hours

---

## 🎯 Mục tiêu

Cho phép UI tự động cập nhật khi có thay đổi entities/relations từ bất kỳ client nào (AI Agent, API, UI khác) thông qua WebSocket connection.

---

## 📊 So sánh Approaches

| Approach | Pros | Cons | Use Case |
|----------|------|------|----------|
| **Polling** | Simple, HTTP only | Latency cao, waste bandwidth | OK cho 1 user |
| **SSE** | Simple, one-way | Chỉ server→client | OK cho read-heavy |
| **WebSocket** | Bi-directional, real-time | Phức tạp hơn, cần connection management | ✅ **Best for UI** |

**Quyết định:** WebSocket là lựa chọn tối ưu cho real-time UI updates.

---

## 🏗️ Kiến trúc

```
┌─────────────────────────────────────────────────────────────────────┐
│                         MEMORY SERVER                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌─────────────────┐  │
│  │  stdio   │   │   REST   │   │    WS    │   │  SSE (future)   │  │
│  │  (MCP)   │   │  (CRUD)  │   │  (/ws)   │   │  (AI Agents)    │  │
│  └────┬─────┘   └────┬─────┘   └────┬─────┘   └────────┬────────┘  │
│       │              │              │                   │           │
│       └──────────────┼──────────────┼───────────────────┘           │
│                      │              │                               │
│               ┌──────▼──────┐ ┌─────▼──────┐                       │
│               │ Mutation    │ │ Connection │                       │
│               │ Handlers    │ │ Manager    │                       │
│               └──────┬──────┘ └─────┬──────┘                       │
│                      │              │                               │
│               ┌──────▼──────────────▼──────┐                       │
│               │      Event Batcher         │                       │
│               │  (debounce 50ms, max 100)  │                       │
│               └──────────────┬─────────────┘                       │
│                              │                                      │
│               ┌──────────────▼─────────────┐                       │
│               │   broadcast::Sender<T>     │                       │
│               │   (tokio mpsc fanout)      │                       │
│               └──────────────┬─────────────┘                       │
│                              │                                      │
│               ┌──────────────▼─────────────┐                       │
│               │      KnowledgeBase         │                       │
│               │  ┌─────────────────────┐   │                       │
│               │  │ Event Store (append)│   │                       │
│               │  │ Snapshots           │   │                       │
│               │  │ In-memory Graph     │   │                       │
│               │  └─────────────────────┘   │                       │
│               └────────────────────────────┘                       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 📦 Message Protocol

### Client → Server

```typescript
interface ClientMessage {
  type: "subscribe" | "unsubscribe" | "ping";
  channel?: "entities" | "relations" | "all";
  filter?: {
    entityTypes?: string[];      // ["Feature", "Bug"]
    entityNames?: string[];      // ["Memory Graph MCP Server"]
  };
}
```

### Server → Client

```typescript
interface ServerMessage {
  type: "entity_created" | "entity_updated" | "entity_deleted"
      | "relation_created" | "relation_deleted"
      | "batch_update"  // For batched events
      | "pong" | "error";
  sequence_id: number;   // Monotonically increasing
  timestamp: number;
  payload: Entity | Relation | BatchPayload | null;
  user?: string;         // Ai đã thực hiện thay đổi
}

interface BatchPayload {
  entities_created?: Entity[];
  entities_updated?: EntityUpdate[];
  entities_deleted?: string[];
  relations_created?: Relation[];
  relations_deleted?: RelationKey[];
}
```

---

## 🔧 Rust Implementation

### Dependencies

```toml
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
tower-http = { version = "0.5", features = ["cors"] }
```

### Event Types

```rust
// src/api/websocket/events.rs
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphEvent {
    EntityCreated { payload: Entity, user: Option<String> },
    EntityUpdated { name: String, new_observations: Vec<String>, user: Option<String> },
    EntityDeleted { name: String, user: Option<String> },
    RelationCreated { payload: Relation, user: Option<String> },
    RelationDeleted { from: String, to: String, relation_type: String, user: Option<String> },
}

#[derive(Clone, Debug, Serialize)]
pub struct WsMessage {
    #[serde(flatten)]
    pub event: GraphEvent,
    pub sequence_id: u64,
    pub timestamp: i64,
}
```

### App State

```rust
// src/api/websocket/state.rs
use tokio::sync::broadcast;

pub struct AppState {
    pub kb: Arc<RwLock<KnowledgeBase>>,
    pub event_tx: broadcast::Sender<WsMessage>,
    pub sequence_counter: Arc<AtomicU64>,
}

impl AppState {
    pub fn new(kb: Arc<RwLock<KnowledgeBase>>) -> Self {
        let (event_tx, _) = broadcast::channel(1024); // Buffer 1024 events
        Self {
            kb,
            event_tx,
            sequence_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn broadcast(&self, event: GraphEvent) {
        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let msg = WsMessage {
            event,
            sequence_id: seq,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = self.event_tx.send(msg);
    }
}
```

### WebSocket Handler

```rust
// src/api/websocket/handler.rs
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State, Query},
    response::Response,
};

#[derive(Deserialize)]
pub struct WsParams {
    token: Option<String>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // Optional: Validate token
    // if !validate_token(&params.token) { return unauthorized(); }

    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.event_tx.subscribe();

    // Send current sequence_id on connect
    let current_seq = state.sequence_counter.load(Ordering::SeqCst);
    let welcome = json!({
        "type": "connected",
        "current_sequence_id": current_seq
    });
    let _ = socket.send(Message::Text(welcome.to_string())).await;

    loop {
        tokio::select! {
            // Broadcast events to client
            Ok(msg) = rx.recv() => {
                let json = serde_json::to_string(&msg).unwrap();
                if socket.send(Message::Text(json)).await.is_err() {
                    break; // Client disconnected
                }
            }

            // Handle client messages
            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Text(text) => handle_client_message(&text, &mut socket).await,
                    Message::Ping(data) => { let _ = socket.send(Message::Pong(data)).await; }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            // Client disconnected
            else => break,
        }
    }
}

async fn handle_client_message(text: &str, socket: &mut WebSocket) {
    if let Ok(msg) = serde_json::from_str::<ClientMessage>(text) {
        match msg.msg_type.as_str() {
            "ping" => {
                let _ = socket.send(Message::Text(r#"{"type":"pong"}"#.to_string())).await;
            }
            "subscribe" => {
                // TODO: Implement channel filtering
            }
            _ => {}
        }
    }
}
```

### Event Batcher

```rust
// src/api/websocket/batcher.rs
use std::time::Duration;
use tokio::time::interval;

pub struct EventBatcher {
    buffer: Vec<GraphEvent>,
    flush_interval: Duration,
    max_batch_size: usize,
    tx: broadcast::Sender<WsMessage>,
    sequence_counter: Arc<AtomicU64>,
}

impl EventBatcher {
    pub fn new(
        tx: broadcast::Sender<WsMessage>,
        sequence_counter: Arc<AtomicU64>,
    ) -> Self {
        Self {
            buffer: Vec::new(),
            flush_interval: Duration::from_millis(50),
            max_batch_size: 100,
            tx,
            sequence_counter,
        }
    }

    pub fn push(&mut self, event: GraphEvent) {
        self.buffer.push(event);
        if self.buffer.len() >= self.max_batch_size {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let batch = WsMessage {
            event: GraphEvent::BatchUpdate {
                events: std::mem::take(&mut self.buffer),
            },
            sequence_id: seq,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = self.tx.send(batch);
    }

    pub async fn run(mut self, mut rx: mpsc::Receiver<GraphEvent>) {
        let mut timer = interval(self.flush_interval);

        loop {
            tokio::select! {
                _ = timer.tick() => {
                    self.flush();
                }
                Some(event) = rx.recv() => {
                    self.push(event);
                }
            }
        }
    }
}
```

---

## 🌐 JavaScript Client

```javascript
// ui/js/websocket.js
class MemoryGraphWS {
    constructor(url = 'ws://localhost:3030/ws') {
        this.url = url;
        this.ws = null;
        this.reconnectInterval = 5000;
        this.maxReconnectInterval = 30000;
        this.handlers = new Map();
        this.lastSeenSequenceId = -1;
    }

    connect() {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            console.log('🔌 Connected to Memory Graph');
            this.reconnectInterval = 5000; // Reset on successful connect
        };

        this.ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            this.handleMessage(data);
        };

        this.ws.onclose = () => {
            console.log('❌ Disconnected, reconnecting in', this.reconnectInterval, 'ms');
            setTimeout(() => this.connect(), this.reconnectInterval);
            // Exponential backoff
            this.reconnectInterval = Math.min(
                this.reconnectInterval * 1.5,
                this.maxReconnectInterval
            );
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
    }

    handleMessage(data) {
        // Track sequence for gap detection
        if (data.sequence_id !== undefined) {
            if (this.lastSeenSequenceId >= 0 &&
                data.sequence_id > this.lastSeenSequenceId + 1) {
                console.warn('Missed events! Gap:',
                    this.lastSeenSequenceId, '->', data.sequence_id);
                this.requestFullRefresh();
            }
            this.lastSeenSequenceId = data.sequence_id;
        }

        switch (data.type) {
            case 'connected':
                this.handleConnect(data);
                break;

            case 'entity_created':
                this.emit('entityCreated', data.payload);
                this.showNotification(`✨ New: ${data.payload.name}`, 'success');
                break;

            case 'entity_updated':
                this.emit('entityUpdated', data);
                break;

            case 'entity_deleted':
                this.emit('entityDeleted', data.name);
                this.showNotification(`🗑️ Deleted: ${data.name}`, 'warning');
                break;

            case 'relation_created':
                this.emit('relationCreated', data.payload);
                break;

            case 'relation_deleted':
                this.emit('relationDeleted', data);
                break;

            case 'batch_update':
                this.handleBatchUpdate(data.payload);
                break;

            case 'pong':
                // Heartbeat response
                break;
        }
    }

    handleConnect(data) {
        console.log('Server sequence:', data.current_sequence_id);
        if (this.lastSeenSequenceId >= 0 &&
            data.current_sequence_id > this.lastSeenSequenceId + 100) {
            // Gap too large, full refresh needed
            this.requestFullRefresh();
        }
        this.lastSeenSequenceId = data.current_sequence_id;
    }

    handleBatchUpdate(payload) {
        if (payload.entities_created) {
            payload.entities_created.forEach(e => this.emit('entityCreated', e));
        }
        if (payload.entities_updated) {
            payload.entities_updated.forEach(e => this.emit('entityUpdated', e));
        }
        if (payload.entities_deleted) {
            payload.entities_deleted.forEach(name => this.emit('entityDeleted', name));
        }
        if (payload.relations_created) {
            payload.relations_created.forEach(r => this.emit('relationCreated', r));
        }
        if (payload.relations_deleted) {
            payload.relations_deleted.forEach(r => this.emit('relationDeleted', r));
        }

        this.showNotification(`📦 Batch update: ${this.countBatchItems(payload)} items`);
    }

    countBatchItems(payload) {
        return (payload.entities_created?.length || 0) +
               (payload.entities_updated?.length || 0) +
               (payload.entities_deleted?.length || 0) +
               (payload.relations_created?.length || 0) +
               (payload.relations_deleted?.length || 0);
    }

    requestFullRefresh() {
        console.log('🔄 Requesting full refresh...');
        this.emit('fullRefreshNeeded');
    }

    // Event emitter pattern
    on(eventType, handler) {
        if (!this.handlers.has(eventType)) {
            this.handlers.set(eventType, []);
        }
        this.handlers.get(eventType).push(handler);
        return this; // Chainable
    }

    emit(eventType, data) {
        const handlers = this.handlers.get(eventType) || [];
        handlers.forEach(h => h(data));
    }

    send(data) {
        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        }
    }

    ping() {
        this.send({ type: 'ping' });
    }

    subscribe(channel, filter = {}) {
        this.send({ type: 'subscribe', channel, filter });
    }

    showNotification(message, type = 'info') {
        // Override this in your app
        console.log(`[${type}] ${message}`);
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}

// Usage example
const memoryWS = new MemoryGraphWS('ws://localhost:3030/ws');

memoryWS
    .on('entityCreated', (entity) => {
        console.log('New entity:', entity.name);
        addEntityToList(entity);
        refreshGraph();
    })
    .on('entityDeleted', (name) => {
        console.log('Deleted:', name);
        removeEntityFromList(name);
        refreshGraph();
    })
    .on('relationCreated', (relation) => {
        console.log('New relation:', relation.from, '->', relation.to);
        refreshGraph();
    })
    .on('fullRefreshNeeded', () => {
        // Fetch fresh data via REST API
        fetchGraphData().then(data => {
            renderEntities(data.entities);
            renderRelations(data.relations);
        });
    });

memoryWS.connect();

// Heartbeat every 30 seconds
setInterval(() => memoryWS.ping(), 30000);
```

---

## 🔐 Authentication

### Strategy: Query Parameter Token

```javascript
// Client
const token = localStorage.getItem('api_token');
const ws = new WebSocket(`ws://localhost:3030/ws?token=${token}`);
```

```rust
// Server
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match validate_token(&params.token) {
        Ok(user) => ws.on_upgrade(move |socket| handle_socket(socket, state, user)),
        Err(_) => (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    }
}
```

---

## 🔄 Reconnection Strategy: "Snapshot then Subscribe"

Khi client reconnect:

1. **Connect** → Nhận `current_sequence_id` từ server
2. **Compare** → So sánh với `last_seen_sequence_id` của client
3. **Decide:**
   - Gap ≤ 100: Server có thể replay từ buffer (optional)
   - Gap > 100: **Full Refresh** via REST API
4. **Subscribe** → Bắt đầu nhận events mới

```javascript
memoryWS.on('connected', async (data) => {
    const gap = data.current_sequence_id - memoryWS.lastSeenSequenceId;

    if (gap > 100) {
        // Full refresh
        const response = await fetch('/api/graph');
        const graph = await response.json();
        renderGraph(graph);
    }

    memoryWS.lastSeenSequenceId = data.current_sequence_id;
});
```

---

## 📊 Channel Hierarchy (Future)

```
all
├── entities
│   ├── entities:Feature
│   ├── entities:Bug
│   └── entities:Decision
└── relations
    ├── relations:depends_on
    └── relations:implements
```

Client subscribe theo nhu cầu:
- Graph visualization → `all`
- Bug tracker panel → `entities:Bug`
- Dependency view → `relations:depends_on`

---

## 🚀 Implementation Phases

| Phase | Task | Effort | Priority |
|-------|------|--------|----------|
| **1** | Basic WS endpoint + broadcast channel | 2h | P0 |
| **2** | Integrate with mutation tools (emit events) | 2h | P0 |
| **3** | Event Batcher (debounce 50ms) | 1h | P1 |
| **4** | Sequence ID tracking | 1h | P1 |
| **5** | UI JavaScript client | 1.5h | P0 |
| **6** | Reconnection + error handling | 0.5h | P1 |
| **Total** | | **~8h** | |

---

## ⚠️ Considerations

### 1. Connection Management
- Heartbeat ping/pong mỗi 30s để detect dead connections
- Reconnection với exponential backoff (5s → 10s → 20s → max 30s)
- Max connections limit (100 concurrent)

### 2. Flow Control
- Event batcher: debounce 50ms, max 100 events per batch
- Tránh flood UI khi batch import

### 3. Hybrid Mode
- `stdio` vẫn hoạt động cho AI Agents (MCP)
- `HTTP/WS` song song cho UI
- CLI flag: `--mode stdio|http|both`

---

## 📁 Proposed File Structure

```
src/
├── api/                       # NEW
│   ├── mod.rs
│   ├── websocket/
│   │   ├── mod.rs
│   │   ├── handler.rs         # WS upgrade + message loop
│   │   ├── events.rs          # GraphEvent enum
│   │   ├── batcher.rs         # Event batching
│   │   └── state.rs           # AppState with broadcast channel
│   └── http.rs                # Axum router setup
└── ...

ui/
├── js/
│   ├── app.js
│   ├── websocket.js           # NEW - MemoryGraphWS class
│   └── ...
└── ...
```

---

## 🔗 Integration với Event Sourcing

```
Event Sourcing (Source of Truth)
         │
         ▼
   ┌─────────────┐
   │ EventStore  │───── Persist to disk
   │   .append() │
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐
   │  broadcast  │───── Real-time to WebSocket clients
   │   .send()   │
   └─────────────┘
```

Mỗi mutation:
1. Write event to EventStore (durable)
2. Broadcast to WebSocket (real-time)

---

## 📚 References

- [Axum WebSocket](https://docs.rs/axum/latest/axum/extract/ws/index.html)
- [Tokio Broadcast Channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [WebSocket API (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)

---

*Last updated: 2026-01-11*
