# Sample Data for Memory Graph

Thư mục này chứa các file JSONL mẫu cho Memory Graph MCP Server.

## 📁 Files

### Project Management

| File | Mô tả | Entity Types |
|------|-------|--------------|
| `sprint-planning.jsonl` | Sprint/Agile workflow | Sprint, Epic, Story, Task, Person |
| `timesheet.jsonl` | Time tracking | Timesheet, WeeklySummary |
| `daily-standup.jsonl` | Daily standup notes | Standup, StandupUpdate, Blocker |
| `release-planning.jsonl` | Release & Gantt chart | Release, Phase, Deadline, GanttTask, CriticalPath |
| `risk-register.jsonl` | Risk management | Risk, Mitigation, Issue |
| `meeting-notes.jsonl` | Meetings & Decisions | Meeting, Decision, ActionItem |

### Industry Use Cases

| File | Mô tả | Entity Types |
|------|-------|--------------|
| `aml-sample-data.jsonl` | Anti-Money Laundering demo | Customer, Account, Transaction, Fault, Alert |
| `fta-nuclear-loca.jsonl` | Fault Tree Analysis - Nuclear LOCA | TopEvent, Gate, BasicEvent, IntermediateEvent, Mitigation, MinimalCutSet |

## 🚀 Usage

### Import vào Memory Graph

```bash
# Copy sample data vào memory.jsonl
cat data/sample/sprint-planning.jsonl >> memory.jsonl

# Hoặc import tất cả
cat data/sample/*.jsonl >> memory.jsonl
```

### Query Examples

```json
// Tìm tất cả tasks trong sprint hiện tại
{"tool": "search_nodes", "params": {"query": "Sprint-3"}}

// Xem dependencies của task
{"tool": "get_related", "params": {"entityName": "Task: Implement OAuth callback handler"}}

// Tìm blockers đang active
{"tool": "search_nodes", "params": {"query": "Blocker Status: Active"}}

// Traverse critical path
{"tool": "traverse", "params": {
  "startNode": "GanttTask: Auth Module Development",
  "path": [{"relationType": "followed_by", "direction": "out"}]
}}
```

## 📊 Entity Types Reference

### Sprint Management
- **Sprint**: Chu kỳ phát triển (2-4 tuần)
- **Epic**: Nhóm features lớn
- **Story**: User story với story points
- **Task**: Công việc cụ thể (estimate/actual hours)

### Time Tracking
- **Timesheet**: Ghi nhận thời gian làm việc theo ngày
- **WeeklySummary**: Tổng hợp tuần

### Daily Operations
- **Standup**: Daily standup meeting
- **StandupUpdate**: Cập nhật của từng người
- **Blocker**: Vấn đề chặn tiến độ

### Release Planning
- **Release**: Phiên bản release
- **Phase**: Giai đoạn dự án (Dev, QA, UAT, Prod)
- **Deadline**: Hard/soft deadline
- **GanttTask**: Task với timeline cho Gantt chart
- **CriticalPath**: Đường găng dự án

### Risk Management
- **Risk**: Rủi ro tiềm ẩn
- **Mitigation**: Biện pháp giảm thiểu
- **Issue**: Vấn đề đã xảy ra

### Communication
- **Meeting**: Cuộc họp
- **Decision**: Quyết định từ meeting
- **ActionItem**: Việc cần làm

## 🔗 Relation Types

| Type | Mô tả | Example |
|------|-------|---------|
| `contains` | A chứa B | Epic → Story → Task |
| `assigned_to` | Giao việc cho người | Task → Person |
| `depends_on` | A phụ thuộc B | Task → Task |
| `blocks` | A chặn B | Blocker → Task |
| `threatens` | Rủi ro đe dọa | Risk → Milestone |
| `mitigated_by` | Rủi ro được giảm bởi | Risk → Mitigation |
| `produced` | Meeting sinh ra | Meeting → Decision |
| `followed_by` | Thứ tự phase | Phase → Phase |
| `logged_for` | Timesheet cho task | Timesheet → Task |
| `includes` | Sprint bao gồm | Sprint → Epic |

## 💡 Best Practices

1. **Naming Convention**:
   - Prefix với type: `Sprint: Q1-2026-Sprint-3`, `Task: Setup OAuth`
   - Dates trong format ISO: `2026-01-14`

2. **Observations**:
   - Mỗi observation là 1 fact độc lập
   - Dùng key-value format: `Status: In Progress`

3. **Relations**:
   - Luôn tạo relation 2 chiều khi cần thiết
   - Dùng temporal relations cho data thay đổi theo thời gian

4. **Updates**:
   - Dùng `add_observations` thay vì delete + recreate
   - Timestamp tự động track bởi Memory Graph

## 📈 Workflow Diagrams

### Sprint Flow
```
Sprint
├── Epic
│   ├── Story (points)
│   │   ├── Task (hours) → assigned_to → Person
│   │   └── Task (hours) → depends_on → Task
│   └── Story
└── Capacity → Person[]
```

### Release Flow
```
Release
├── Phase: Development → followed_by →
├── Phase: QA Testing → followed_by →
├── Phase: UAT → followed_by →
└── Phase: Production
    └── has_deadline → Deadline
```

### Risk Flow
```
Risk
├── threatens → Milestone/Feature
├── mitigated_by → Mitigation
│   └── implements → Meeting (knowledge sharing)
└── owned_by → Person
```
