# Proposed: Event Sourcing Architecture

> **Status:** 📋 Proposed
> **Date:** 2026-01-11
> **Priority:** High (Foundation for Team Collaboration)
> **Complexity:** Medium
> **Reviewed by:** B3K Analysis

---

## 🎯 Mục tiêu

Chuyển đổi Memory Graph từ **"Overwrite State"** sang **"Event Sourcing"** để:
- Tuân thủ nguyên lý IDEA.md: "Event-first, Immutable memory"
- Hỗ trợ Audit trail và Time travel
- Chuẩn bị nền tảng cho Team Collaboration

---

## 📊 Hiện trạng vs Tương lai

| Khía cạnh | Hiện tại | Event Sourcing |
|-----------|----------|----------------|
| **Storage** | State-based (overwrite) | Event log + Snapshots |
| **Update** | Sửa trực tiếp entity | Append event mới |
| **Delete** | Xóa khỏi file | Append "deleted" event |
| **History** | Mất | Giữ toàn bộ |
| **Conflict** | Last-write-wins | Append-only (no conflict) |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   EVENT SOURCING v1.0                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Write Path:                                               │
│   ┌─────────┐    ┌─────────────┐    ┌──────────────────┐   │
│   │ MCP/UI  │───►│ append to   │───►│ maybe_snapshot() │   │
│   │ Request │    │ events.jsonl│    │ every 1000 events│   │
│   └─────────┘    └─────────────┘    └────────┬─────────┘   │
│                                              │              │
│                                              ▼              │
│                                   ┌──────────────────────┐ │
│                                   │ ATOMIC WRITE:        │ │
│                                   │ 1. Write .tmp        │ │
│                                   │ 2. sync_all()        │ │
│                                   │ 3. rename → latest   │ │
│                                   │ 4. rotate old log    │ │
│                                   └──────────────────────┘ │
│                                                             │
│   Read Path (Startup):                                      │
│   ┌───────────────┐    ┌─────────────────┐                 │
│   │ Load snapshot │───►│ Replay events   │───► Ready!      │
│   │ (latest.jsonl)│    │ after snapshot  │                 │
│   └───────────────┘    └─────────────────┘                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 📁 Storage Layout

```
memory/
├── events.jsonl           ← Active event log (append-only)
├── snapshots/
│   ├── latest.jsonl       ← Current materialized state
│   └── previous.jsonl     ← Backup (1 version back)
└── archive/               ← Optional, for audit trail
    ├── events_001_10000.jsonl.gz
    └── events_10001_20000.jsonl.gz
```

---

## 📝 Event Schema

### Event Types

```jsonl
{"eventType":"entity_created","eventId":1,"ts":1704067200,"user":"Duyan","data":{"name":"Bug:X","entityType":"Bug"}}
{"eventType":"observation_added","eventId":2,"ts":1704067201,"user":"Duyan","data":{"entity":"Bug:X","observation":"Login fails"}}
{"eventType":"relation_created","eventId":3,"ts":1704067202,"user":"Huy","data":{"from":"Bug:X","to":"Module:Auth","relationType":"affects"}}
{"eventType":"entity_updated","eventId":4,"ts":1704067300,"user":"Duyan","data":{"name":"Bug:X","changes":{"entityType":"Feature"}}}
{"eventType":"observation_removed","eventId":5,"ts":1704067301,"user":"Duyan","data":{"entity":"Bug:X","observation":"Login fails"}}
{"eventType":"relation_deleted","eventId":6,"ts":1704067400,"user":"Duyan","data":{"from":"Bug:X","to":"Module:Auth","relationType":"affects"}}
{"eventType":"entity_deleted","eventId":7,"ts":1704067500,"user":"Duyan","data":{"name":"Bug:X","reason":"Duplicate"}}
```

### Event Base Structure

```rust
pub struct Event {
    pub event_type: EventType,
    pub event_id: u64,           // Auto-increment
    pub timestamp: i64,          // Unix timestamp
    pub user: String,            // From git config or API key
    pub data: serde_json::Value, // Event-specific payload
}

pub enum EventType {
    EntityCreated,
    EntityUpdated,
    EntityDeleted,
    ObservationAdded,
    ObservationRemoved,
    RelationCreated,
    RelationDeleted,
}
```

---

## 🛡️ Safety Mechanisms

### 1. Atomic Write (Chống ghi dở)

```rust
fn create_snapshot(&mut self) -> Result<()> {
    let temp_path = "snapshots/latest.tmp";
    let final_path = "snapshots/latest.jsonl";
    let backup_path = "snapshots/previous.jsonl";

    // 1. Write to temp file
    let mut file = File::create(temp_path)?;

    // Write metadata
    let meta = json!({
        "type": "snapshot_meta",
        "last_event_id": self.last_event_id,
        "created_at": current_timestamp(),
        "entity_count": self.entities.len(),
        "relation_count": self.relations.len()
    });
    writeln!(file, "{}", meta)?;

    // Write entities and relations
    for entity in &self.entities {
        writeln!(file, "{}", serde_json::to_string(entity)?)?;
    }
    for relation in &self.relations {
        writeln!(file, "{}", serde_json::to_string(relation)?)?;
    }

    // 2. Flush to disk
    file.sync_all()?;

    // 3. Backup current snapshot
    if Path::new(final_path).exists() {
        fs::rename(final_path, backup_path)?;
    }

    // 4. Atomic swap
    fs::rename(temp_path, final_path)?;

    self.events_since_snapshot = 0;
    Ok(())
}
```

### 2. Log Rotation (Quản lý disk)

```rust
fn rotate_event_log(&self) -> Result<()> {
    let archive_path = format!(
        "archive/events_{}_{}.jsonl",
        self.last_snapshot_event_id,
        self.last_event_id
    );

    // Move current log to archive
    fs::rename("events.jsonl", &archive_path)?;

    // Optional: Compress
    if self.config.compress_archive {
        compress_file(&archive_path)?;
        fs::remove_file(&archive_path)?;
    }

    // Create fresh event log
    File::create("events.jsonl")?;

    Ok(())
}
```

### 3. Startup Recovery

```rust
fn load_from_storage(&mut self) -> Result<()> {
    // 1. Load latest snapshot
    let snapshot = self.load_snapshot("snapshots/latest.jsonl")?;
    self.entities = snapshot.entities;
    self.relations = snapshot.relations;
    let last_event_id = snapshot.meta.last_event_id;

    // 2. Replay events after snapshot
    let events = self.load_events_after(last_event_id)?;
    for event in events {
        self.apply_event(&event)?;
    }

    println!(
        "Loaded {} entities, {} relations. Replayed {} events.",
        self.entities.len(),
        self.relations.len(),
        events.len()
    );

    Ok(())
}
```

---

## ⚙️ Snapshot Policy

### Triggers

| Trigger | Value | Rationale |
|---------|-------|-----------|
| **Event count** | 1,000 events | Primary - bounds startup time |
| **Graceful shutdown** | Always | Minimize replay on restart |
| **Time elapsed** | 24 hours | Backup - for low-activity periods |
| **File size** | 50 MB | Backup - prevents runaway growth |

### Configuration

```rust
pub struct SnapshotConfig {
    pub event_count_threshold: usize,  // Default: 1000
    pub time_threshold_hours: u64,     // Default: 24
    pub file_size_threshold_mb: u64,   // Default: 50
    pub archive_old_events: bool,      // Default: true
    pub compress_archive: bool,        // Default: true
    pub keep_snapshot_count: usize,    // Default: 2 (latest + previous)
}
```

---

## 📊 Performance Estimates

| Entities | Snapshot Size | Events to Replay | Startup Time |
|----------|--------------|------------------|--------------|
| 1K | ~200 KB | ≤1000 | ~10ms |
| 10K | ~2 MB | ≤1000 | ~50ms |
| 100K | ~20 MB | ≤1000 | ~200ms |

### Sync vs Async Decision

```
if entities < 10_000:
    Sync Snapshot (Simple, <50ms pause)
else:
    Async Snapshot (Background thread, no pause)
```

---

## 🔄 Migration Plan

### Phase 1: Event Schema (No breaking change)

1. Add `eventType` field to existing JSONL entries
2. Keep current format working during transition

**Before:**
```jsonl
{"type":"entity","name":"Bug:X","entityType":"Bug","observations":[]}
```

**After:**
```jsonl
{"eventType":"entity_created","eventId":1,"ts":1704067200,"user":"Duyan","data":{"name":"Bug:X","entityType":"Bug","observations":[]}}
```

### Phase 2: Append-Only Mode

1. Modify `create_entities` → append `entity_created` event
2. Modify `add_observations` → append `observation_added` event
3. Modify `delete_entities` → append `entity_deleted` event
4. Rebuild state from events on startup

### Phase 3: Snapshots

1. Implement snapshot creation
2. Implement atomic write
3. Implement startup recovery (snapshot + replay)

### Phase 4: Log Rotation

1. Archive old event logs
2. Optional compression

---

## ⚠️ Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Corrupted snapshot | Cannot start | Keep `previous.jsonl` backup |
| Corrupted event log | Partial data loss | Atomic append, fsync |
| Disk full | Cannot write | Monitor disk, alert at 80% |
| Slow startup (many events) | Bad UX | Snapshot every 1000 events |

---

## 📋 Implementation Checklist

| Task | Priority | Complexity | Status |
|------|----------|------------|--------|
| Event schema definition | P0 | Low | ⬜ |
| Append-only event writing | P0 | Low | ⬜ |
| Event replay on startup | P0 | Medium | ⬜ |
| Atomic snapshot write | P0 | Low | ⬜ |
| Snapshot metadata (last_event_id) | P0 | Low | ⬜ |
| Event count trigger | P0 | Low | ⬜ |
| Graceful shutdown snapshot | P1 | Low | ⬜ |
| Snapshot backup (previous.jsonl) | P1 | Low | ⬜ |
| Log rotation | P2 | Medium | ⬜ |
| Archive compression | P3 | Low | ⬜ |
| Async snapshot (future) | P3 | Medium | ⬜ |

---

## 📚 References

- [Event Sourcing Pattern - Martin Fowler](https://martinfowler.com/eaaDev/EventSourcing.html)
- [IDEA.md - AI Memory & Reasoning Engine](./IDEA.md)
- [Proposed-Team-Collaboration.md](./Proposed-Team-Collaboration.md)

---

*Last updated: 2026-01-11*
