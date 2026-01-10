# Memory Graph MCP Server - AI Agent Instructions

## Project Manager Instructions
You are an AI agent integrated with the Memory Graph <memory>. Your role is to assist developers by providing code suggestions, documentation, and answering questions related to the project.

**Usage Guidelines**:
- Sử dụng đồ thị tri thức để tổ chức và lưu trữ thông tin quan trọng, giúp chống lại việc quên lãng thông tin.
- Hỗ trợ các công cụ quản lý bộ nhớ như tạo thực thể, tạo quan hệ, thêm quan sát, xóa thực thể, xóa quan sát, xóa quan hệ, đọc đồ thị, tìm kiếm nút và mở nút.
- Đảm bảo rằng tất cả các đề xuất tuân thủ giao thức MCP và sử dụng định dạng JSON-RPC 2.
- Giúp duy trì và cải thiện hiệu suất của hệ thống lưu trữ dữ liệu JSONL.
- Cung cấp hướng dẫn chi tiết về cách xây dựng, chạy và sử dụng máy chủ MCP trong môi trường phát triển như VSCode.
- Hỗ trợ phát triển các tính năng mới và sửa lỗi trong mã nguồn Rust của dự án.

## Features - 15 Tools

### Memory Tools (9)
| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `create_entities` | Tạo entities mới | `entities[]` với `name`, `entityType`, `observations[]`, `createdBy?`, `updatedBy?` |
| `create_relations` | Tạo relations giữa entities | `relations[]` với `from`, `to`, `relationType`, `createdBy?`, `validFrom?`, `validTo?` |
| `add_observations` | Thêm observations vào entity | `observations[]` với `entityName`, `contents[]` |
| `delete_entities` | Xóa entities | `entityNames[]` |
| `delete_observations` | Xóa observations cụ thể | `deletions[]` với `entityName`, `observations[]` |
| `delete_relations` | Xóa relations | `relations[]` |
| `read_graph` | Đọc graph với pagination | `limit?`, `offset?` |
| `search_nodes` | Tìm kiếm với synonym matching | `query`, `limit?`, `includeRelations?` |
| `open_nodes` | Mở nodes theo tên | `names[]` |

### Query Tools (3)
| Tool | Description | Use Case |
|------|-------------|----------|
| `get_related` | Lấy entities liên quan | Tìm dependencies, xem connections |
| `traverse` | Duyệt graph theo path pattern | Multi-hop queries, tìm indirect relations |
| `summarize` | Tóm tắt entities | Overview nhanh, statistics |

### Temporal Tools (2)
| Tool | Description | Use Case |
|------|-------------|----------|
| `get_relations_at_time` | Query relations hợp lệ tại timestamp | "Alice ở đâu năm 2024?" |
| `get_relation_history` | Xem toàn bộ lịch sử relations | Track changes over time |

### Utility Tools (1)
| Tool | Description |
|------|-------------|
| `get_current_time` | Lấy timestamp hiện tại |

## Data Model

### Entity Structure
```json
{
  "name": "Feature: Auth",
  "entityType": "Feature",
  "observations": ["Implements JWT", "Uses bcrypt"],
  "createdBy": "Duyan",
  "updatedBy": "Duyan",
  "createdAt": 1704067200,
  "updatedAt": 1704153600
}
```

### Relation Structure (with Temporal Support)
```json
{
  "from": "Alice",
  "to": "NYC",
  "relationType": "lives_in",
  "createdBy": "Duyan",
  "createdAt": 1704067200,
  "validFrom": 1704067200,
  "validTo": 1735689599
}
```

## User Attribution

Server tự động tracking ai tạo/cập nhật data:

| Field | Auto-filled From | Override |
|-------|------------------|----------|
| `createdBy` | Git config `user.name` → OS `USER`/`USERNAME` → "anonymous" | Truyền trong params |
| `updatedBy` | Same as above | Auto-update khi `add_observations` |

**Kịch bản sử dụng:**
```json
// AI tự trích xuất từ git blame
{"entities": [{"name": "Bug: Auth", "createdBy": "Huy", ...}]}

// Hoặc để server auto-fill từ môi trường
{"entities": [{"name": "Feature: X", ...}]}  // createdBy = current user
```

## Semantic Search (Synonym Matching)

Search tự động expand với synonyms:

| Search Query | Also Matches |
|--------------|--------------|
| `coder` | programmer, developer, engineer, dev |
| `bug` | issue, defect, error, problem |
| `done` | completed, finished, resolved |
| `critical` | urgent, p0, blocker |

**Tip**: Không cần lo về từ vựng chính xác, search sẽ tìm semantic equivalents.

## Best Practices

### 1. Pagination cho Large Graphs
```json
// Đừng: read toàn bộ graph
{"tool": "read_graph", "params": {}}

// Nên: dùng pagination
{"tool": "read_graph", "params": {"limit": 50, "offset": 0}}
```

### 2. Temporal Relations cho Data Changes
```json
// Khi data thay đổi, ĐỪNG xóa relation cũ
// Thay vào đó, set validTo và tạo relation mới

// Alice chuyển từ NYC sang Tokyo
// Step 1: Update relation cũ với validTo
// Step 2: Tạo relation mới với validFrom
{
  "from": "Alice", "to": "Tokyo",
  "relationType": "lives_in",
  "validFrom": 1735689600
}
```

### 3. Search với includeRelations
```json
// Khi chỉ cần entities (faster):
{"tool": "search_nodes", "params": {"query": "Bug", "includeRelations": false}}

// Khi cần context đầy đủ:
{"tool": "search_nodes", "params": {"query": "Bug", "includeRelations": true}}
```

## Workflow

### 1. Khi bắt đầu dự án mới:
AI Agent nên:
1. `read_graph(limit: 100)` → scan existing context
2. Scan project structure → `create_entities` cho Modules
3. Đọc README, docs → `create_entities` cho Conventions, Decisions
4. Đọc schema files → `create_entities` cho Schemas
5. Hỏi user về business rules → `create_entities` cho BusinessRules

### 2. Khi phát triển tính năng mới:
1. **Trước khi code**: `search_nodes` để lấy context
2. **Khi discover something new**: `add_observations`
3. **Khi tạo file/module mới**: `create_entities`
4. **Khi fix bug**: tạo Bug entity với root cause
5. **Khi make decision**: tạo Decision entity với reasoning

### 3. Khi switch context:
```
User: "Giờ làm feature X trong module Y"

AI Agent:
1. open_nodes(["Module: Y"]) → dependencies, patterns
2. search_nodes("Y") → related files, schemas
3. get_related("Module: Y") → xem connections
4. Có đủ context để tiếp tục mà không hỏi lại
```

### 4. Khi query historical data:
```
User: "Alice làm việc ở đâu năm 2024?"

AI Agent:
1. get_relations_at_time(timestamp: 1704067200, entityName: "Alice")
2. Trả về relations hợp lệ tại thời điểm đó
```

## Flow hoạt động chuẩn

```
Goal
↓
search_nodes() → get context
↓
Decision → create_entities(Decision)
↓
Action → add_observations()
↓
Error? → create_entities(Bug) → Fix → add_observations(Lesson)
↓
Success → create_relations() với validFrom
↓
Memory updated ✓
```

## Entity Types Reference

| Type | Purpose | Example |
|------|---------|---------|
| `Project` | Dự án chính | "Memory Graph MCP" |
| `Module` | Thành phần code | "Auth Module" |
| `Feature` | Tính năng | "Feature: Pagination" |
| `Bug` | Lỗi đã fix | "Bug: Context Overflow" |
| `Decision` | Quyết định thiết kế | "Decision: Use JSONL" |
| `Requirement` | Yêu cầu | "Req: Multi-tenant" |
| `Milestone` | Mốc dự án | "v1.0 Release" |
| `Risk` | Rủi ro | "Risk: Scale Limit" |
| `Convention` | Coding standards | "Naming Convention" |
| `Schema` | Data structures | "User Schema" || `Person` | Người dùng/Team | "John Doe", "Backend Team" |

## Relation Types Reference

| Type | Description | Example |
|------|-------------|--------|
| `contains` | A chứa B | Project contains Module |
| `implements` | A triển khai B | Module implements Feature |
| `fixes` | A sửa B | Commit fixes Bug |
| `caused_by` | A gây ra bởi B | Bug caused_by Decision |
| `depends_on` | A phụ thuộc B | Feature depends_on Feature |
| `blocked_by` | A bị chặn bởi B | Task blocked_by Bug |
| `assigned_to` | A được giao cho B | Bug assigned_to Person |
| `part_of` | A là phần của B | Module part_of Project |
| `relates_to` | A liên quan B | Generic relation |
| `supersedes` | A thay thế B | Decision supersedes Decision |
| `affects` | A ảnh hưởng B | Bug affects Module |
| `requires` | A yêu cầu B | Feature requires Requirement |

> **Note**: Server sẽ trả về warning nếu dùng type không chuẩn. Vẫn cho phép custom types nhưng khuyến khích dùng standard types.
## Improvement Suggestions

### Đã implement ✅
- [x] Pagination (`limit`/`offset`) cho large graphs
- [x] Temporal relations (`validFrom`/`validTo`)
- [x] Synonym matching cho semantic search
- [x] Timestamps (`createdAt`/`updatedAt`)

### Future Enhancements 🚀
1. **Vector Embeddings**: Upgrade từ synonym matching sang true semantic search với embeddings
2. **Graph Visualization**: Web UI để visualize knowledge graph
3. **Auto-summarization**: Tự động tóm tắt entities khi graph quá lớn
4. **Conflict Detection**: Phát hiện observations mâu thuẫn
5. **Import/Export**: Sync với external knowledge bases
6. **Multi-tenant**: Support nhiều projects trong 1 server

**Important Note**: Always prioritize the integrity and efficiency of the knowledge graph when making suggestions or changes.