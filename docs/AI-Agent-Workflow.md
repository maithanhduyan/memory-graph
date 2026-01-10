# AI Agent Workflow with Memory Graph

> **Purpose**: Hướng dẫn AI Agent sử dụng Memory Graph hiệu quả trong software development
> **Created**: 2026-01-11

## 📋 Overview

Memory Graph giúp AI Agent:
- **Nhớ** context qua nhiều conversations
- **Tìm** thông tin liên quan nhanh chóng
- **Học** patterns và conventions của project
- **Tránh** lặp lại sai lầm đã xảy ra

---

## 🏗️ Entity Types cho Software Development

### Core Types

| Type | Mục đích | Khi nào tạo |
|------|----------|-------------|
| **Project** | Tổng quan dự án | Khi bắt đầu làm việc với dự án mới |
| **Module** | Package/module | Khi discover hoặc tạo module mới |
| **File** | File quan trọng | File phức tạp, được reference nhiều |
| **Function** | Function/method | Logic phức tạp, side effects, business-critical |
| **Schema** | Database/API schema | Tables, API endpoints, data structures |

### Knowledge Types

| Type | Mục đích | Khi nào tạo |
|------|----------|-------------|
| **BusinessRule** | Business logic | Khi user giải thích business rules |
| **Convention** | Coding standards | Khi discover patterns trong code |
| **Decision** | Architectural decisions | Khi make hoặc discover decisions |
| **Dependency** | External libs | Dependencies quan trọng/phức tạp |

### Tracking Types

| Type | Mục đích | Khi nào tạo |
|------|----------|-------------|
| **Feature** | Features của project | Planning hoặc implementing features |
| **Bug** | Bugs đã gặp/fix | Sau khi fix bug để tránh lặp lại |
| **Milestone** | Project milestones | Project planning |
| **Risk** | Identified risks | Khi phát hiện potential issues |

---

## 🔄 Workflow Phases

### Phase 1: Project Onboarding

Khi bắt đầu làm việc với dự án mới:

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. SCAN PROJECT STRUCTURE                                       │
│    - Đọc file tree, identify modules                            │
│    - Tạo Project entity với tech stack, architecture            │
│    - Tạo Module entities cho các packages chính                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. READ DOCUMENTATION                                           │
│    - README.md → Project overview, setup                        │
│    - CONTRIBUTING.md → Conventions                              │
│    - Architecture docs → Decisions                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. ANALYZE KEY FILES                                            │
│    - Config files → Dependencies, settings                      │
│    - Schema files → Database structure                          │
│    - Entry points → Main flows                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. ASK USER                                                     │
│    - Business rules không có trong code                         │
│    - Team conventions                                           │
│    - Known issues/gotchas                                       │
└─────────────────────────────────────────────────────────────────┘
```

**Example entities to create:**

```jsonl
{"name":"my-project","entityType":"Project","observations":["Monorepo: backend Rust, frontend React","Database: PostgreSQL","Auth: JWT","Deployment: Docker + K8s"]}
{"name":"Module: API","entityType":"Module","observations":["Path: backend/src/api/","REST endpoints","Uses axum framework","Auth middleware required for all routes except /health"]}
{"name":"Convention: API Response","entityType":"Convention","observations":["All responses wrap in {data, error, meta}","Error format: {code, message, details}","Pagination: cursor-based, not offset"]}
```

### Phase 2: Before Coding

Trước khi implement bất kỳ task nào:

```
┌─────────────────────────────────────────────────────────────────┐
│ User: "Thêm feature export to CSV"                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 1. SEARCH RELATED CONTEXT                                       │
│                                                                 │
│    search_nodes("export")     → existing export features        │
│    search_nodes("CSV")        → any CSV handling code           │
│    search_nodes("download")   → download patterns used          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. GET MODULE CONTEXT                                           │
│                                                                 │
│    open_nodes(["Module: Reports"])  → where to add feature      │
│    → Dependencies, patterns, conventions của module đó          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. CHECK CONVENTIONS                                            │
│                                                                 │
│    search_nodes("Convention")  → coding standards               │
│    → Error handling, naming, patterns to follow                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. CHECK BUSINESS RULES                                         │
│                                                                 │
│    search_nodes("BusinessRule")  → relevant rules               │
│    → Permissions, data access rules, edge cases                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 3: During Coding

Trong quá trình implement:

```
┌─────────────────────────────────────────────────────────────────┐
│ DISCOVER SOMETHING NEW                                          │
│ → add_observations() hoặc create_entities()                     │
│                                                                 │
│ Examples:                                                       │
│ - "À, module này dùng pattern X" → add to Module observations   │
│ - "Function này có side effect Y" → create Function entity      │
│ - "Table này có constraint Z" → add to Schema observations      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ MAKE A DECISION                                                 │
│ → create Decision entity                                        │
│                                                                 │
│ Example:                                                        │
│ {"name":"Decision: Use streaming for large exports",            │
│  "entityType":"Decision",                                       │
│  "observations":["Lý do: Avoid memory issues with 100k+ rows",  │
│                  "Trade-off: More complex code",                │
│                  "Alternative rejected: Load all to memory"]}   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ ENCOUNTER AN ISSUE                                              │
│ → create Bug entity (even if fixed immediately)                 │
│                                                                 │
│ Example:                                                        │
│ {"name":"Bug: CSV encoding UTF-8 BOM",                          │
│  "entityType":"Bug",                                            │
│  "observations":["Issue: Excel không đọc được UTF-8 CSV",       │
│                  "Root cause: Missing BOM header",              │
│                  "Solution: Add 0xEF,0xBB,0xBF at file start",  │
│                  "Prevention: Always add BOM for Excel CSV"]}   │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 4: After Coding

Sau khi hoàn thành task:

```
┌─────────────────────────────────────────────────────────────────┐
│ UPDATE EXISTING ENTITIES                                        │
│                                                                 │
│ - Module có thêm file mới → update Module observations          │
│ - Feature completed → update Feature status                     │
│ - New dependency added → create Dependency entity               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ CREATE NEW ENTITIES                                             │
│                                                                 │
│ - New file with complex logic → File entity                     │
│ - New API endpoint → Schema entity                              │
│ - New business rule implemented → BusinessRule entity           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ DOCUMENT LEARNINGS                                              │
│                                                                 │
│ - Gotchas discovered → add to relevant entities                 │
│ - Patterns established → Convention entity                      │
│ - Technical debt → Risk entity                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔍 Query Patterns

### Pattern 1: Context Loading

Khi cần hiểu một phần của codebase:

```
# Lấy overview của module
open_nodes(["Module: Payment"])

# Tìm tất cả files trong module đó
search_nodes("Payment")

# Lấy conventions áp dụng
search_nodes("Convention")
```

### Pattern 2: Impact Analysis

Khi cần đánh giá impact của thay đổi:

```
# Tìm dependencies
search_nodes("depends_on Payment")

# Tìm functions sử dụng schema
search_nodes("uses orders table")

# Tìm business rules liên quan
search_nodes("BusinessRule order")
```

### Pattern 3: Problem Solving

Khi gặp issue:

```
# Tìm bugs tương tự
search_nodes("Bug encoding")
search_nodes("Bug CSV")

# Tìm decisions liên quan
search_nodes("Decision export")

# Tìm conventions về error handling
search_nodes("Convention error")
```

### Pattern 4: Context Switching

Khi chuyển sang task khác:

```
# Load context của task mới
open_nodes(["Feature: User Management", "Module: Auth"])

# Tìm related entities
search_nodes("User")
search_nodes("authentication")
```

---

## 📝 Observation Guidelines

### Good Observations ✅

```
"Path: backend/src/payment/processor.rs"           # Cụ thể
"Key function: process_payment() - lines 45-120"   # Có reference
"Depends on: Stripe SDK, OrderService"             # Dependencies rõ ràng
"Edge case: Refund > original amount not allowed"  # Actionable info
"Pattern: Repository pattern với trait PaymentRepo"# Technical detail
```

### Bad Observations ❌

```
"This is a payment module"        # Quá chung chung
"Important file"                  # Không có thông tin
"Handles payments"                # Duplicate với name
"Complex logic"                   # Không specific
```

### Observation Categories

Mỗi entity nên có observations theo categories:

| Category | Examples |
|----------|----------|
| **Location** | Path, line numbers, file references |
| **Purpose** | What it does, why it exists |
| **Dependencies** | What it uses, what uses it |
| **Constraints** | Rules, limitations, requirements |
| **Gotchas** | Edge cases, known issues, warnings |
| **Examples** | Usage examples, test cases |

---

## 🔗 Relation Types

### Standard Relations

| Relation | From → To | Meaning |
|----------|-----------|---------|
| `belongs_to` | File → Module | File is part of module |
| `contains` | Module → File | Module contains file |
| `depends_on` | Module → Module | Runtime dependency |
| `imports` | File → File | Import statement |
| `calls` | Function → Function | Function invocation |
| `uses` | Function → Schema | Data access |
| `implements` | Function → BusinessRule | Business logic |
| `affects` | Decision → Feature | Decision impacts feature |
| `threatens` | Risk → Milestone | Risk to delivery |

### When to Create Relations

```
✅ Create relation when:
- Dependency is important to understand
- Relationship affects decision making
- Helps with impact analysis

❌ Don't create relation when:
- Obvious from context (file in same module)
- Too granular (every import)
- Temporary relationship
```

---

## 🎯 Best Practices

### 1. Be Selective

Không lưu mọi thứ - chỉ lưu những gì:
- Khó nhớ hoặc phức tạp
- Được reference nhiều lần
- Chứa business logic quan trọng
- Có gotchas hoặc edge cases

### 2. Keep Updated

- Khi code thay đổi → update observations
- Khi file bị xóa → delete entity
- Khi module refactor → update relations

### 3. Use Consistent Naming

```
Modules:    "Module: {ModuleName}"
Files:      "File: {filename.ext}"
Functions:  "Function: {function_name}"
Schemas:    "Schema: {table_name}"
Rules:      "BusinessRule: {Short Description}"
```

### 4. Link to Code

Luôn include references có thể trace:
```
"File: payment.rs - lines 100-150"
"See: backend/src/payment/README.md"
"Test: tests/payment_test.rs::test_refund"
```

---

## 📊 Memory Graph Health

### Metrics to Monitor

| Metric | Healthy | Warning |
|--------|---------|---------|
| Entities per project | 20-100 | >200 (too granular) |
| Observations per entity | 3-10 | >15 (split entity) |
| Orphan entities | <10% | >30% (add relations) |
| Stale entities | <20% | >50% (need update) |

### Maintenance Tasks

Weekly:
- Review và update stale entities
- Remove obsolete entities
- Add missing relations

Monthly:
- Audit entity types usage
- Consolidate duplicate information
- Update project-level observations

---

*Last updated: 2026-01-11*
