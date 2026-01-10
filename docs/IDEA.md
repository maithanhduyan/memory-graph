# Full Architecture: AI Memory & Reasoning Engine

## 1. Nguyên lý thiết kế

* **Event-first**: mọi thứ AI làm đều là sự kiện, không ghi đè
* **Immutable memory**: quá khứ không bị sửa
* **Multi-memory**: timeline (JSONL) + graph + vector
* **LLM không giữ sự thật**: chỉ suy luận trên memory

---

## 2. Sơ đồ tổng thể (logic)

```
┌──────────────┐
│   LLM Core   │
│ (Reasoning)  │
└──────┬───────┘
       │ prompts
       ▼
┌─────────────────────────────┐
│ Memory Access Layer (MAL)    │
│  - permission                │
│  - grounding rules           │
│  - hallucination guard       │
└──────┬───────────┬──────────┘
       │           │
       │           │
       ▼           ▼
┌──────────────┐  ┌──────────────┐
│ Event Store  │  │ Vector Store │
│  (JSONL)     │  │ (Embeddings) │
└──────┬───────┘  └──────┬───────┘
       │                │
       ▼                ▼
┌────────────────────────────────┐
│      Knowledge Graph Engine     │
│  (cause, dependency, history)  │
└────────────────────────────────┘
```

---

## 3. Storage layer (sự thật tuyệt đối)

### 3.1 Event Store – JSONL (Ground Truth)

* Append-only
* Không update, không delete
* 1 dòng = 1 sự kiện nhận thức

**Ví dụ file layout**

```
/memory/
 ├── coding.jsonl
 ├── decisions.jsonl
 ├── errors.jsonl
 └── lessons.jsonl
```

**Vai trò**

* Audit
* Timeline
* Chống ảo giác (fact anchoring)

---

### 3.2 Knowledge Graph (Causal Memory)

**Engine**: Neo4j / Memgraph / custom graph

**Node**

* File
* Function
* Decision
* Bug
* Goal

**Edge**

* CAUSED
* FIXED
* BROKE
* DEPENDS_ON

→ Graph **được build từ JSONL**, không ghi trực tiếp từ LLM

---

### 3.3 Vector Store (Semantic Memory)

**Input**

* decision
* error
* lesson

**Output**

* similarity
* analogy

→ Cho phép AI nhớ "tình huống giống trước đây"

---

## 4. Engine layer (não thật)

### 4.1 Event Ingestor

* Validate schema
* Gắn timestamp, hash
* Reject hallucinated event

---

### 4.2 Graph Builder

* Parse JSONL
* Extract entities
* Update graph edges

---

### 4.3 Memory Query Engine

| Query loại | Engine              |
| ---------- | ------------------- |
| Timeline   | DuckDB / ClickHouse |
| Quan hệ    | Graph DB            |
| Ngữ nghĩa  | Vector DB           |

---

### 4.4 Planner Engine (điểm then chốt)

* Không dùng prompt
* Dựa trên pattern sự kiện

**Ví dụ rule**

```
IF decision exists
AND no action after decision
THEN next_step = action
```

→ AI biết việc tiếp theo **mà không suy đoán**

---

## 5. Memory Access Layer (anti-hallucination)

LLM chỉ được phép:

* đọc event đã tồn tại
* suy luận trên graph
* tạo hypothesis có nhãn rõ

**Cấm**

* invent fact (tạo sự kiện không có thật)
* sửa quá khứ

---

## 6. Flow hoạt động chuẩn

```
Goal
 ↓
Decision
 ↓
Action
 ↓
Observation
 ↓
Error? → Fix → Lesson
 ↓
Graph update
 ↓
Memory embedding
```

---

## 7. Vì sao kiến trúc này vượt trội?

* Không prompt engineering
* Không phụ thuộc LLM cụ thể
* Có thể replay toàn bộ quá khứ
* AI tiến hóa từ kinh nghiệm thật

---

## 8. Mở rộng 10–50 năm

* Multi-agent chung event store
* Distributed JSONL (object storage)
* Self-reflecting graph

---

## 9. Tóm tắt ngắn gọn

* JSONL = ký ức bất biến
* Graph = ý thức nhân quả
* Vector = trực giác
* Planner = ý chí hành động

LLM chỉ là **bộ máy diễn đạt**, không phải trí tuệ.
