# GIT BRANCHING STRATEGY GUIDE
## Git Branching hiệu quả

- **master**: production

- **develop**: tích hợp các tính năng

- **feature/**: phát triển tính năng

- **release/**: chuẩn bị phát hành

- **hotfix/**: sửa lỗi khẩn cấp

Ví dụ mẫu:

```mermaid
gitGraph
    commit id: "Initial Project"

    branch develop
    checkout develop
    commit id: "Setup MCP Server"
    commit id: "Add Embedding Manager"

    branch feature/vietnamese-text
    checkout feature/vietnamese-text
    commit id: "Vietnamese Chunker"
    commit id: "Text Processing"

    checkout develop
    merge feature/vietnamese-text
    commit id: "Merge Vietnamese Support"

    branch feature/batch-processing
    checkout feature/batch-processing
    commit id: "Batch Processor"
    commit id: "Memory Optimization"
    commit id: "Performance Metrics"

    checkout develop
    merge feature/batch-processing
    commit id: "Merge Batch Processing"

    branch master
    checkout master
    commit id: "Production v1.0"

    checkout develop
    commit id: "Error Handling"
    commit id: "Input Validation"

    branch hotfix/fix-psutil-import
    checkout hotfix/fix-psutil-import
    commit id: "Fix psutil import error"
    commit id: "Resolve conflicts"
    commit id: "Update .gitignore"

    checkout master
    merge hotfix/fix-psutil-import
    commit id: "Hotfix Applied"

    checkout develop
    merge master
    commit id: "Sync with Master"

    branch feature/security-enhancements
    checkout feature/security-enhancements
    commit id: "Security Validation"
    commit id: "Audit Logging"

    checkout develop
    branch release/v2.0
    checkout release/v2.0
    commit id: "Prepare Release"
    commit id: "Documentation"
    commit id: "Final Testing"

    checkout master
    merge release/v2.0
    commit id: "Production v2.0"

    checkout develop
    merge release/v2.0
    commit id: "Release Merged Back"

```