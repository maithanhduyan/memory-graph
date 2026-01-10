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

## Features
- **9 Memory Tools**:
  - `create_entities` - Create new entities in the knowledge graph
  - `create_relations` - Create relations between entities
  - `add_observations` - Add observations to existing entities
  - `delete_entities` - Delete entities from the graph
  - `delete_observations` - Delete specific observations from entities
  - `delete_relations` - Delete relations from the graph
  - `read_graph` - Read the entire knowledge graph
  - `search_nodes` - Search for nodes by query
  - `open_nodes` - Open specific nodes by name

## Workflow
1. **Understanding Requirements**: Analyze the developer's requests and understand the context of the Memory Graph MCP Server.
2. **Code Suggestions**: Provide code snippets, functions, or modules that align with the project's architecture and coding standards.
3. **Documentation**: Generate or update documentation to reflect new features or changes in the codebase.
4. **Testing**: Suggest test cases or testing strategies to ensure code quality and reliability.
5. **Feedback Loop**: Continuously learn from developer feedback to improve future suggestions and assistance.

### 1. Khi bắt đầu dự án mới:
AI Agent nên:
1. Scan project structure → tạo Module entities
2. Đọc README, docs → tạo Convention, Decision entities
3. Đọc schema files → tạo Schema entities
4. Hỏi user về business rules → tạo BusinessRule entities

### 2. Khi phát triển tính năng mới:
AI Agent nên:
1. Trước khi code: search Memory để lấy context
2. Khi discover something new: add_observations
3. Khi tạo file/module mới: create_entities
4. Khi fix bug: lưu Bug entity với root cause
5. Khi make decision: lưu Decision entity với reasoning

### 3. Khi switch context:
User: "Giờ làm feature X trong module Y"

AI Agent:
1. open_nodes(["Module: Y"]) → dependencies, patterns
2. search_nodes("Y") → related files, schemas
3. Có đủ context để tiếp tục mà không hỏi lại



Flow hoạt động chuẩn của hệ thống quản lý bộ nhớ:
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

**Important Note**: Always prioritize the integrity and efficiency of the knowledge graph when making suggestions or changes.