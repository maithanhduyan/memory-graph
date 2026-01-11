# Proposed: Mutex → RwLock Migration

> **Status**: ✅ Approved
> **Date**: 2026-01-11
> **Risk Level**: 🟢 LOW

---

## 📋 Executive Summary

Đề xuất chuyển đổi `std::sync::Mutex` sang `std::sync::RwLock` trong `KnowledgeBase` để tối ưu hiệu năng cho hệ thống **read-heavy**.

---

## 🎯 Motivation

### Phân tích Access Pattern

| Operation Type | Count | Tools |
|----------------|-------|-------|
| **Read** | 9 | `search_nodes`, `read_graph`, `open_nodes`, `get_related`, `traverse`, `summarize`, `get_relations_at_time`, `get_relation_history` |
| **Write** | 6 | `create_entities`, `create_relations`, `add_observations`, `delete_entities`, `delete_observations`, `delete_relations` |

**Kết luận**: Hệ thống là **Read-Heavy** (60% read vs 40% write).

### Vấn đề với Mutex

```
Thread A: search_nodes() → lock() → READ → unlock()
Thread B: search_nodes() → BLOCKED (chờ Thread A)
Thread C: search_nodes() → BLOCKED (chờ Thread A, B)
```

Với Mutex, tất cả operations (kể cả read-only) phải chờ tuần tự.

### Giải pháp với RwLock

```
Thread A: search_nodes() → read() → READ
Thread B: search_nodes() → read() → READ (PARALLEL!)
Thread C: search_nodes() → read() → READ (PARALLEL!)
```

RwLock cho phép **multiple concurrent readers**.

---

## 📊 Risk Analysis

### Memory Safety

| Risk | Level | Mitigation |
|------|-------|------------|
| Race condition (memory) | 🟢 LOW | RwLock guarantees: exclusive write OR multiple reads |
| Race condition (file I/O) | 🟢 LOW | `persist_to_file()` called inside write lock scope |
| Stale read | 🟢 LOW | Readers clone data → consistent snapshot |
| Deadlock | 🟢 LOW | No nested locks in codebase |
| Data loss | 🟢 LOW | `fs::write()` is atomic (write-replace pattern) |

### Code Pattern Verification

**Current pattern (SAFE):**
```rust
pub fn create_entities(kb: &KnowledgeBase, ...) {
    let mut graph = kb.graph.lock().unwrap();  // Lock acquired
    // ... modify graph ...
    kb.persist_to_file(&graph)?;               // Persist INSIDE lock
    Ok(created)                                // Lock released on return
}
```

**After RwLock (STILL SAFE):**
```rust
pub fn create_entities(kb: &KnowledgeBase, ...) {
    let mut graph = kb.graph.write().unwrap(); // Write lock acquired
    // ... modify graph ...
    kb.persist_to_file(&graph)?;               // Persist INSIDE lock
    Ok(created)                                // Lock released on return
}
```

---

## 🔧 Implementation Plan

### Files to Modify

| File | Changes |
|------|---------|
| `src/knowledge_base/mod.rs` | `Mutex<T>` → `RwLock<T>`, `.lock()` → `.read()` |
| `src/knowledge_base/crud.rs` | `.lock()` → `.write()` (6 places) |

### Code Changes

#### 1. mod.rs - Struct Definition

```diff
- use std::sync::Mutex;
+ use std::sync::RwLock;

pub struct KnowledgeBase {
    pub(crate) memory_file_path: String,
-   pub(crate) graph: Mutex<KnowledgeGraph>,
+   pub(crate) graph: RwLock<KnowledgeGraph>,
    pub(crate) current_user: String,
}
```

#### 2. mod.rs - Initialization

```diff
Self {
    memory_file_path,
-   graph: Mutex::new(graph),
+   graph: RwLock::new(graph),
    current_user,
}
```

#### 3. mod.rs - load_graph()

```diff
pub(crate) fn load_graph(&self) -> McpResult<KnowledgeGraph> {
-   Ok(self.graph.lock().unwrap().clone())
+   Ok(self.graph.read().unwrap().clone())
}
```

#### 4. crud.rs - All Write Operations

```diff
pub fn create_entities(kb: &KnowledgeBase, entities: Vec<Entity>) {
-   let mut graph = kb.graph.lock().unwrap();
+   let mut graph = kb.graph.write().unwrap();
    // ... rest unchanged
}
```

---

## 📈 Expected Performance Impact

| Scenario | Mutex | RwLock | Improvement |
|----------|-------|--------|-------------|
| 10 concurrent reads | Sequential | Parallel | ~10x faster |
| 5 reads + 1 write | All blocked | Reads wait for write only | ~5x faster |
| 10 concurrent writes | Sequential | Sequential | Same |

---

## ✅ Testing Plan

1. **Unit tests**: Run existing test suite
2. **Concurrent test**: `test_concurrent_access` in integration tests
3. **Stress test**: Manual testing with multiple MCP clients

---

## 🚀 Rollback Plan

If issues arise, revert changes:
```diff
- use std::sync::RwLock;
+ use std::sync::Mutex;
```

Single commit, easy to revert.

---

## 📝 Decision

**APPROVED** - Proceed with implementation.

- Risk is low
- Performance benefit is significant for read-heavy workloads
- Code changes are minimal and well-understood
- Existing test suite provides safety net
