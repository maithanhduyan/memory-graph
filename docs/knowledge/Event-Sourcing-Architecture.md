# Event Sourcing Architecture

> **Type:** Knowledge Document
> **Created:** 2026-01-11
> **Tags:** #architecture #event-sourcing #design-pattern

---

## 📖 Định nghĩa

**Event Sourcing** là pattern lưu trữ dữ liệu dưới dạng **chuỗi sự kiện bất biến** (immutable events) thay vì **trạng thái hiện tại** (current state).

```
Traditional: State = Current Value
Event Sourcing: State = f(Event₁, Event₂, ..., Eventₙ)
```

---

## 🎯 Vấn đề Giải quyết

### Trong Kiến trúc Phần mềm

| Vấn đề | State-based (Truyền thống) | Event Sourcing |
|--------|---------------------------|----------------|
| **"Tại sao dữ liệu thay đổi?"** | ❌ Không biết, chỉ có state cuối | ✅ Có event log đầy đủ |
| **Debug production bug** | ❌ Không reproduce được | ✅ Replay events → reproduce |
| **Audit/Compliance** | ❌ Phải code thêm audit table | ✅ Built-in, mọi thay đổi đều là event |
| **Temporal queries** | ❌ "User ở đâu tuần trước?" - Không biết | ✅ Replay đến timestamp đó |
| **Undo/Redo** | ❌ Phải implement riêng | ✅ Replay events trừ event cuối |
| **Schema migration** | ❌ Phức tạp, mất data | ✅ Replay với schema mới |
| **Multi-system sync** | ❌ Two-phase commit, distributed locks | ✅ Publish events, eventual consistency |

### Trong Thế giới Thực (Analogies)

| Domain | "State-based" | "Event Sourcing" |
|--------|---------------|------------------|
| **Ngân hàng** | Số dư: 1,000,000đ | Lịch sử giao dịch: Nạp 500k, Rút 200k... |
| **Kế toán** | Bảng cân đối cuối kỳ | Sổ cái (Ledger) - mọi bút toán |
| **Y tế** | "Bệnh nhân khỏe" | Hồ sơ bệnh án - mọi lần khám |
| **Pháp lý** | "Tài sản thuộc về A" | Chuỗi chứng từ: Mua, bán, thừa kế... |
| **Git** | Working directory | Commit history |
| **Blockchain** | Account balance | Transaction log |

---

## 🏗️ Core Concepts

### 1. Event (Sự kiện)

```rust
pub struct Event {
    pub event_id: u64,           // Unique, auto-increment
    pub event_type: String,      // "entity_created", "observation_added"
    pub timestamp: i64,          // Unix timestamp
    pub user: String,            // Who triggered
    pub data: serde_json::Value, // Event payload
}
```

**Đặc tính của Event:**
- **Immutable**: Không thể sửa, không thể xóa
- **Ordered**: Có thứ tự xác định (event_id, timestamp)
- **Past tense**: Đã xảy ra ("OrderPlaced", không phải "PlaceOrder")

### 2. Event Store (Kho sự kiện)

```
┌─────────────────────────────────────────┐
│              EVENT STORE                │
├─────────────────────────────────────────┤
│ [1] 2026-01-01 entity_created "Bug:X"   │
│ [2] 2026-01-01 observation_added "..."  │
│ [3] 2026-01-02 relation_created ...     │
│ [4] 2026-01-03 entity_deleted "Bug:X"   │
│ ...                                     │
│ [N] 2026-01-11 entity_created "..."     │
└─────────────────────────────────────────┘
         ↓ Replay
┌─────────────────────────────────────────┐
│          CURRENT STATE (Graph)          │
│  Entities: [...], Relations: [...]      │
└─────────────────────────────────────────┘
```

### 3. Projection (Chiếu)

**Projection** = Materialized view từ events cho use case cụ thể.

```rust
// Same events, different projections
fn project_to_graph(events: &[Event]) -> KnowledgeGraph { ... }
fn project_to_timeline(events: &[Event]) -> Vec<TimelineEntry> { ... }
fn project_to_user_activity(events: &[Event]) -> HashMap<User, Vec<Action>> { ... }
```

### 4. Snapshot (Ảnh chụp)

**Snapshot** = Cache của state tại một thời điểm, để tránh replay từ đầu.

```
Startup:
1. Load snapshot (state tại event #1000)
2. Replay events #1001 → #1050
3. Ready! (chỉ replay 50 events thay vì 1050)
```

**⚠️ Important:** Snapshot là **cache**, KHÔNG phải **source of truth**.

---

## 📊 So sánh với CRUD

```
┌─────────────────────────────────────────────────────────────┐
│                    CRUD (State-based)                       │
├─────────────────────────────────────────────────────────────┤
│  CREATE user (id=1, name="Alice", city="NYC")               │
│  UPDATE user SET city="LA" WHERE id=1                       │
│  DELETE user WHERE id=1                                     │
│                                                             │
│  Result: User không tồn tại. Lịch sử? Không biết.           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Event Sourcing                           │
├─────────────────────────────────────────────────────────────┤
│  [1] UserRegistered {id=1, name="Alice", city="NYC"}        │
│  [2] UserMoved {id=1, from="NYC", to="LA"}                  │
│  [3] UserDeactivated {id=1, reason="GDPR request"}          │
│                                                             │
│  Result: Biết toàn bộ lifecycle của user.                   │
└─────────────────────────────────────────────────────────────┘
```

---

## ⚖️ Trade-offs

| ✅ Pros | ❌ Cons |
|---------|---------|
| Complete audit trail | Storage tăng (events accumulate) |
| Debug & reproduce issues | Complexity cao hơn |
| Temporal queries native | Query performance (phải replay/snapshot) |
| Schema evolution flexible | Learning curve |
| Natural event-driven architecture | CQRS thường đi kèm |
| Built-in undo/redo | Event versioning phức tạp |

---

## 🔗 Related Patterns

### CQRS (Command Query Responsibility Segregation)

Thường đi kèm Event Sourcing:
- **Command** → Sinh ra Events → Ghi vào Event Store
- **Query** → Đọc từ Projections (optimized views)

```
┌─────────┐     ┌─────────────┐     ┌─────────────┐
│ Command │────►│ Event Store │────►│ Projections │
└─────────┘     └─────────────┘     └──────┬──────┘
                                           │
┌─────────┐                                │
│  Query  │◄───────────────────────────────┘
└─────────┘
```

### Event-Driven Architecture

Events có thể được publish cho external systems:
```
Event Store → Message Broker → Consumers (Analytics, Search, Notifications)
```

---

## 🧠 Áp dụng cho AI Memory Graph

| Vấn đề AI Agent gặp | Event Sourcing giải quyết |
|---------------------|---------------------------|
| **"AI quên context"** | Replay events → Nhớ toàn bộ |
| **"AI hallucinate"** | Events = Ground truth, không suy đoán |
| **"Debug AI behavior"** | Xem event log: AI đã quyết định gì |
| **"Rollback AI mistake"** | Bỏ qua event cuối khi rebuild |
| **"Team collaboration"** | Ai tạo event nào, khi nào, tại sao |
| **"Time travel"** | "Dự án ở trạng thái gì tuần trước?" |

**IDEA.md Alignment:**
> *"JSONL = ký ức bất biến. Không update, không delete."*

Đây chính là Event Sourcing pattern.

---

## 📚 References

- [Event Sourcing - Martin Fowler](https://martinfowler.com/eaaDev/EventSourcing.html)
- [CQRS Journey - Microsoft](https://docs.microsoft.com/en-us/previous-versions/msp-n-p/jj554200(v=pandp.10))
- [Event Store Database](https://www.eventstore.com/)
- [Kafka as Event Store](https://www.confluent.io/blog/okay-store-data-apache-kafka/)

---

## 🎓 Key Takeaway

> **"Don't just store WHERE you are. Store HOW you got there."**

```
State = Destination
Events = Journey

Knowing the journey lets you:
- Understand why you're here
- Retrace your steps
- Take a different path (replay with different rules)
```

---

*See also: [Proposed-Event-Sourcing.md](../proposal/Proposed-Event-Sourcing.md)*
