# AML Transaction Monitoring với Memory-Graph

## Tổng quan

Document này mô tả cách sử dụng Memory-Graph MCP Server như một hệ thống **Anti-Money Laundering (AML) Transaction Monitoring** với khả năng phát hiện giao dịch đáng ngờ thông qua phân tích đồ thị tri thức.

### Mục tiêu

- Mô hình hóa mạng lưới khách hàng, tài khoản và giao dịch
- Áp dụng Fault Tree Analysis để phát hiện patterns rửa tiền
- Sử dụng Graph Inference để phát hiện mối liên hệ ẩn
- Tạo alerts tự động dựa trên rules định sẵn

### Tại sao Memory-Graph phù hợp cho AML?

| Tính năng | Ứng dụng AML |
|-----------|--------------|
| **Knowledge Graph** | Mô hình hóa mạng lưới entities phức tạp |
| **Relations** | Theo dõi dòng tiền qua nhiều tầng |
| **Inference Engine** | Phát hiện quan hệ ẩn, beneficial owners |
| **Traverse** | Truy vết nguồn gốc và đích đến của tiền |
| **Temporal Queries** | Phân tích hành vi theo thời gian |
| **Observations** | Ghi nhận chi tiết và red flags |

---

## Kiến trúc hệ thống

```
┌─────────────────────────────────────────────────────────────────┐
│                    AML MONITORING SYSTEM                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   Banking    │    │  Transaction │    │   External   │      │
│  │   Core       │───▶│   Gateway    │◀───│   Sources    │      │
│  └──────────────┘    └──────┬───────┘    └──────────────┘      │
│                             │                                   │
│                             ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                 MEMORY-GRAPH MCP SERVER                   │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │  │
│  │  │  Entities  │  │  Relations │  │ Inference  │          │  │
│  │  │  (KYC)     │  │  (Flows)   │  │  Engine    │          │  │
│  │  └────────────┘  └────────────┘  └────────────┘          │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │  │
│  │  │ Fault Tree │  │  Traverse  │  │  Temporal  │          │  │
│  │  │  Rules     │  │  Analysis  │  │  Queries   │          │  │
│  │  └────────────┘  └────────────┘  └────────────┘          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                             │                                   │
│                             ▼                                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   Alert      │    │  Case       │    │   Reporting  │      │
│  │   Queue      │───▶│  Management │───▶│   (SAR/STR)  │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Model

### 1. Entity Types

#### Customer Entity
```json
{
  "name": "Customer: [Unique ID or Name]",
  "entityType": "Customer",
  "observations": [
    "ID: C001",
    "Risk Level: Low|Medium|High|Critical",
    "Account opened: YYYY-MM-DD",
    "Occupation: [Job Title]",
    "Country: [Country Code]",
    "⚠️ PEP (Politically Exposed Person): Yes/No",
    "⚠️ Sanctions List: Yes/No",
    "KYC Status: Verified|Pending|Failed",
    "Annual Income: [Amount]",
    "Source of Funds: [Description]"
  ]
}
```

#### Account Entity
```json
{
  "name": "Account: [Account Number]",
  "entityType": "Account",
  "observations": [
    "Owner: [Customer Name]",
    "Type: Savings|Checking|Business|Corporate",
    "Balance: [Amount] [Currency]",
    "Status: Active|Frozen|Closed|Under Review",
    "Opened: YYYY-MM-DD",
    "Branch: [Branch Code]",
    "⚠️ Unusual balance for profile: Yes/No"
  ]
}
```

#### Transaction Entity
```json
{
  "name": "TXN-[ID]: [Description]",
  "entityType": "Transaction",
  "observations": [
    "Type: Cash Deposit|Wire Transfer|Internal Transfer|ATM|POS",
    "Amount: [Value] [Currency]",
    "Time: YYYY-MM-DD HH:MM:SS",
    "From: [Source Account/Cash]",
    "To: [Destination Account]",
    "Status: Completed|Processing|Blocked|Under Review",
    "Channel: Branch|Online|Mobile|ATM",
    "Location: [City, Country]",
    "⚠️ [Red Flag Description]"
  ]
}
```

#### Alert Entity
```json
{
  "name": "Alert: AML-[Year]-[Sequence]",
  "entityType": "Alert",
  "observations": [
    "Type: [Alert Type]",
    "Customer: [Customer Name]",
    "Account: [Account Number]",
    "Trigger: [Rule Description]",
    "Total Amount: [Value]",
    "Risk Score: [0-100]/100",
    "Status: Open|Investigating|Escalated|Closed|False Positive",
    "Assigned To: [Team/Person]",
    "SAR Filed: Yes/No",
    "Priority: P1|P2|P3|P4"
  ]
}
```

#### Fault Entity (Detection Rules)
```json
{
  "name": "Fault: [Pattern Name]",
  "entityType": "Fault",
  "observations": [
    "Description: [What this pattern detects]",
    "Pattern: [How to identify]",
    "Threshold: [Specific trigger conditions]",
    "Risk Score: [Base score]/100",
    "Category: Structuring|Layering|Integration|Unusual Activity"
  ]
}
```

### 2. Relation Types

| Relation | From | To | Description |
|----------|------|-----|-------------|
| `owns` | Customer | Account | Khách hàng sở hữu tài khoản |
| `credits` | Transaction | Account | Giao dịch ghi có vào tài khoản |
| `debits` | Account | Transaction | Tài khoản ghi nợ cho giao dịch |
| `triggers` | Transaction | Fault | Giao dịch kích hoạt rule |
| `caused_by` | Alert | Fault | Alert được tạo bởi rule |
| `investigates` | Alert | Customer | Alert điều tra khách hàng |
| `associated_with` | Customer | Customer | Khách hàng liên quan nhau |
| `transacts_with` | Customer | Customer | Có giao dịch với nhau |
| `contains` | FaultTree | Fault | Fault Tree chứa các rules |
| `beneficial_owner` | Person | Customer | Chủ sở hữu hưởng lợi |

---

## Fault Tree Analysis

### Cấu trúc Fault Tree

```
                         Money Laundering Detected
                              (TOP EVENT)
                                   │
                            ┌──────┴──────┐
                            │   OR Gate   │
                            └──────┬──────┘
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
    ┌─────────────┐         ┌─────────────┐         ┌─────────────┐
    │ PLACEMENT   │         │  LAYERING   │         │ INTEGRATION │
    │  Detected   │         │  Detected   │         │  Detected   │
    └──────┬──────┘         └──────┬──────┘         └──────┬──────┘
           │                       │                       │
     ┌─────┴─────┐           ┌─────┴─────┐           ┌─────┴─────┐
     │  OR Gate  │           │  OR Gate  │           │  OR Gate  │
     └─────┬─────┘           └─────┬─────┘           └─────┬─────┘
           │                       │                       │
    ┌──────┼──────┐         ┌──────┼──────┐         ┌──────┼──────┐
    ▼      ▼      ▼         ▼      ▼      ▼         ▼      ▼      ▼
  Struct  Smurfing Cash   Rapid  Complex Shell   Large  Business Real
  -uring          Deposits Xfers  Web     Co.    Invest Mixing   Estate
```

### Detection Rules

#### 1. Structuring (Smurfing)
```
Fault: Structuring
├── Description: Chia nhỏ giao dịch để tránh báo cáo
├── Pattern: Nhiều giao dịch ngay dưới ngưỡng báo cáo
├── Threshold: >3 giao dịch 180-199M VND trong 24h
├── Risk Score: 85/100
└── AND conditions:
    ├── Amount: 90-99% của reporting threshold
    ├── Frequency: >3 transactions
    ├── Timeframe: Within 24 hours
    └── Same beneficial owner
```

#### 2. Layering
```
Fault: Layering
├── Description: Chuyển tiền qua nhiều tầng để che giấu nguồn gốc
├── Pattern: Tiền di chuyển nhanh qua 3+ tài khoản
├── Threshold: >5 transfers trong 48h
├── Risk Score: 90/100
└── AND conditions:
    ├── Accounts: 3+ different accounts
    ├── Transfers: Rapid succession (<15 min apart)
    ├── Amount: Similar amounts (variance <10%)
    └── Final destination: Different from origin country
```

#### 3. Shell Company Transfer
```
Fault: Shell Company Transfer
├── Description: Chuyển tiền đến công ty bình phong
├── Pattern: Wire transfer đến tax haven + shell company
├── Threshold: >500M VND đến high-risk jurisdiction
├── Risk Score: 95/100
└── OR conditions:
    ├── Destination: Known tax haven
    ├── Beneficiary: No physical address
    ├── Company: No real business activity
    └── Ownership: Obscured beneficial ownership
```

#### 4. Unusual Activity
```
Fault: Unusual Activity
├── Description: Hoạt động bất thường so với profile
├── Pattern: Giao dịch vượt 5x mức bình thường
├── Threshold: Deviation >300% từ baseline
├── Risk Score: 70/100
└── OR conditions:
    ├── Volume: 5x normal monthly volume
    ├── Geography: Transactions in new high-risk countries
    ├── Time: Activity at unusual hours
    └── Type: New transaction types for customer
```

---

## Implementation Guide

### Bước 1: Khởi tạo Fault Tree

```json
// Tạo Fault Tree Root
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "AML: Fault Tree Root",
        "entityType": "FaultTree",
        "observations": [
          "Purpose: Detect Money Laundering",
          "Method: Graph-based pattern analysis",
          "Threshold: Transactions > 200M VND trigger review",
          "Logic: OR gate - any child fault triggers alert"
        ]
      }
    ]
  }
}

// Tạo các Fault nodes
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "Fault: Structuring",
        "entityType": "Fault",
        "observations": [
          "Description: Breaking large amounts into smaller deposits",
          "Pattern: Multiple deposits just below reporting threshold",
          "Threshold: >3 transactions of 180-199M VND within 24h",
          "Risk Score: 85/100"
        ]
      },
      // ... other faults
    ]
  }
}

// Liên kết Fault Tree
{
  "tool": "create_relations",
  "params": {
    "relations": [
      {"from": "AML: Fault Tree Root", "to": "Fault: Structuring", "relationType": "contains"},
      {"from": "AML: Fault Tree Root", "to": "Fault: Layering", "relationType": "contains"},
      // ...
    ]
  }
}
```

### Bước 2: Import Customer Data (KYC)

```json
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "Customer: [Name]",
        "entityType": "Customer",
        "observations": [
          "ID: C001",
          "Risk Level: Medium",
          "Account opened: 2023-01-15",
          "Occupation: Business Owner",
          "Country: Vietnam",
          "KYC Status: Verified"
        ]
      }
    ]
  }
}
```

### Bước 3: Real-time Transaction Monitoring

```json
// Mỗi transaction được thêm vào graph
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "TXN-001: Deposit 195M",
        "entityType": "Transaction",
        "observations": [
          "Type: Cash Deposit",
          "Amount: 195,000,000 VND",
          "Time: 2026-01-14 09:15:00",
          "From: Cash",
          "To: ACC-001",
          "⚠️ Just below 200M threshold"
        ]
      }
    ]
  }
}

// Tạo relations cho transaction
{
  "tool": "create_relations",
  "params": {
    "relations": [
      {"from": "TXN-001: Deposit 195M", "to": "Account: ACC-001", "relationType": "credits"}
    ]
  }
}
```

### Bước 4: Rule Matching & Alert Generation

```json
// Khi phát hiện pattern, tạo relation đến Fault
{
  "tool": "create_relations",
  "params": {
    "relations": [
      {"from": "TXN-001: Deposit 195M", "to": "Fault: Structuring", "relationType": "triggers"}
    ]
  }
}

// Nếu đủ điều kiện, tạo Alert
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "Alert: AML-2026-001",
        "entityType": "Alert",
        "observations": [
          "Type: Structuring Alert",
          "Customer: Nguyen Van A",
          "Account: ACC-001",
          "Trigger: 3 deposits just below threshold within 6 hours",
          "Total Amount: 585,000,000 VND",
          "Risk Score: 85/100",
          "Status: Open"
        ]
      }
    ]
  }
}
```

### Bước 5: Investigation với Graph Queries

```json
// Tìm tất cả entities liên quan đến khách hàng đáng ngờ
{
  "tool": "get_related",
  "params": {
    "entityName": "Customer: Le Van C",
    "direction": "both"
  }
}

// Sử dụng inference để phát hiện quan hệ ẩn
{
  "tool": "infer",
  "params": {
    "entityName": "Customer: Le Van C",
    "maxDepth": 5,
    "minConfidence": 0.3
  }
}

// Trace dòng tiền qua Fault Tree
{
  "tool": "traverse",
  "params": {
    "startNode": "AML: Fault Tree Root",
    "path": [
      {"direction": "out", "relationType": "contains"},
      {"direction": "in", "relationType": "triggers"}
    ]
  }
}
```

---

## Queries cho AML Investigation

### 1. Tìm tất cả giao dịch đáng ngờ

```json
{
  "tool": "search_nodes",
  "params": {
    "query": "⚠️",
    "includeRelations": true
  }
}
```

### 2. Trace dòng tiền từ nguồn đến đích

```json
{
  "tool": "traverse",
  "params": {
    "startNode": "Account: ACC-001",
    "path": [
      {"direction": "out", "relationType": "debits"},
      {"direction": "out", "relationType": "credits"}
    ],
    "maxResults": 50
  }
}
```

### 3. Tìm mạng lưới khách hàng liên quan

```json
{
  "tool": "infer",
  "params": {
    "entityName": "Customer: Suspect",
    "maxDepth": 3,
    "minConfidence": 0.5
  }
}
```

### 4. Lấy lịch sử thay đổi quan hệ

```json
{
  "tool": "get_relation_history",
  "params": {
    "entityName": "Customer: Le Van C"
  }
}
```

### 5. Query trạng thái tại thời điểm cụ thể

```json
{
  "tool": "get_relations_at_time",
  "params": {
    "entityName": "Customer: Le Van C",
    "timestamp": 1704067200
  }
}
```

---

## Risk Scoring Model

### Base Risk Scores

| Fault Type | Base Score | Weight |
|------------|------------|--------|
| Shell Company Transfer | 95 | 1.0 |
| Layering | 90 | 0.95 |
| Structuring | 85 | 0.9 |
| Unusual Activity | 70 | 0.8 |

### Risk Amplifiers

| Factor | Multiplier |
|--------|------------|
| PEP (Politically Exposed Person) | 1.5x |
| Sanctions List Match | 2.0x |
| High-Risk Country | 1.3x |
| New Account (<6 months) | 1.2x |
| Cash Transactions | 1.1x |
| Multiple Alerts (>3) | 1.4x |

### Composite Risk Score Calculation

```
Final Score = min(100, Base Score × Σ(Amplifiers))

Example:
- Base: 85 (Structuring)
- PEP: 1.5x
- New Account: 1.2x
- Final: min(100, 85 × 1.5 × 1.2) = 100
```

---

## Alert Workflow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Detection  │────▶│   Triage    │────▶│Investigation│
│  (Auto)     │     │   (L1)      │     │   (L2)      │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                    ┌──────────────────────────┼──────────────────────────┐
                    │                          │                          │
                    ▼                          ▼                          ▼
             ┌─────────────┐           ┌─────────────┐           ┌─────────────┐
             │   False     │           │  Escalate   │           │   File      │
             │  Positive   │           │   (L3)      │           │   SAR       │
             └─────────────┘           └─────────────┘           └─────────────┘
```

### Alert Status Transitions

```
Open → Investigating → [Escalated|False Positive|SAR Filed|Closed]
```

### SLA by Priority

| Priority | Initial Review | Investigation | Resolution |
|----------|---------------|---------------|------------|
| P1 (Critical) | 1 hour | 24 hours | 48 hours |
| P2 (High) | 4 hours | 48 hours | 5 days |
| P3 (Medium) | 24 hours | 5 days | 15 days |
| P4 (Low) | 48 hours | 10 days | 30 days |

---

## Reporting

### Suspicious Activity Report (SAR)

Khi alert được xác nhận, tạo SAR entity:

```json
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "SAR: 2026-001",
        "entityType": "Report",
        "observations": [
          "Type: Suspicious Activity Report",
          "Related Alert: AML-2026-002",
          "Subject: Le Van C",
          "Activity Type: Shell Company Transfer",
          "Amount Involved: 2,000,000,000 VND",
          "Filed Date: 2026-01-14",
          "Filed By: Compliance Officer",
          "Regulatory Body: State Bank of Vietnam",
          "Status: Submitted"
        ]
      }
    ]
  }
}
```

### Dashboard Metrics

Sử dụng `summarize` để lấy statistics:

```json
{
  "tool": "summarize",
  "params": {
    "entityType": "Alert",
    "format": "stats"
  }
}
```

---

## Integration Points

### 1. Core Banking System

```
Banking Core → Memory-Graph
├── Customer onboarding → create_entities (Customer)
├── Account opening → create_entities (Account)
├── Transaction posting → create_entities (Transaction) + create_relations
└── Account updates → add_observations
```

### 2. External Data Sources

```
External Sources → Memory-Graph
├── Sanctions lists → add_observations (Customer)
├── PEP databases → add_observations (Customer)
├── Adverse media → add_observations (Customer)
└── Company registries → create_entities (verify shell companies)
```

### 3. Case Management

```
Memory-Graph → Case Management
├── New alerts → create_entities (Alert)
├── Investigation progress → add_observations (Alert)
├── Case closure → add_observations (Alert) + SAR filing
└── False positive → add_observations (Alert)
```

---

## Best Practices

### 1. Entity Naming Convention

```
Customer: [Full Name or ID]
Account: [Account Number]
TXN-[Sequential]: [Short Description]
Alert: AML-[Year]-[Sequence]
Fault: [Pattern Name]
SAR: [Year]-[Sequence]
```

### 2. Observation Standards

- Luôn prefix với emoji cảnh báo: `⚠️`
- Ghi rõ timestamp cho mọi event
- Include quantitative thresholds
- Reference related entities

### 3. Relation Hygiene

- Sử dụng temporal relations cho thay đổi
- Không xóa relations, đánh dấu `validTo`
- Link transactions đến cả source và destination

### 4. Performance Optimization

- Sử dụng pagination khi query large graphs
- Limit inference depth cho real-time queries
- Archive old transactions định kỳ

---

## Regulatory Compliance

### Vietnam Regulations

- **Circular 09/2023/TT-NHNN**: Reporting threshold 200M VND
- **Law on Anti-Money Laundering 2022**: SAR requirements
- **Decree 116/2013/NĐ-CP**: Customer due diligence

### International Standards

- **FATF Recommendations**: Risk-based approach
- **Basel Committee**: Customer due diligence
- **Wolfsberg Principles**: Correspondent banking

---

## Appendix

### A. Sample Data Generation Script

```python
# Python script để generate sample data
import json
from datetime import datetime, timedelta
import random

def generate_transactions(customer_id, account_id, count=10):
    transactions = []
    base_time = datetime.now()

    for i in range(count):
        amount = random.randint(50_000_000, 250_000_000)
        txn = {
            "name": f"TXN-{i+1:03d}: {'Deposit' if random.random() > 0.5 else 'Transfer'} {amount//1_000_000}M",
            "entityType": "Transaction",
            "observations": [
                f"Type: {'Cash Deposit' if random.random() > 0.5 else 'Wire Transfer'}",
                f"Amount: {amount:,} VND",
                f"Time: {(base_time - timedelta(hours=i)).isoformat()}",
                f"From: {'Cash' if random.random() > 0.5 else 'ACC-XXX'}",
                f"To: {account_id}"
            ]
        }

        if 180_000_000 <= amount <= 199_000_000:
            txn["observations"].append("⚠️ Just below 200M threshold")

        transactions.append(txn)

    return transactions
```

### B. Entity Type Reference

| Type | Purpose | Example |
|------|---------|---------|
| `Customer` | KYC information | Customer: Nguyen Van A |
| `Account` | Bank accounts | Account: ACC-001 |
| `Transaction` | Financial transactions | TXN-001: Deposit 195M |
| `Alert` | AML alerts | Alert: AML-2026-001 |
| `Fault` | Detection rules | Fault: Structuring |
| `FaultTree` | Rule hierarchy | AML: Fault Tree Root |
| `Report` | Regulatory reports | SAR: 2026-001 |

### C. Relation Type Reference

| Type | Description | Example |
|------|-------------|---------|
| `owns` | Ownership | Customer owns Account |
| `credits` | Money in | Transaction credits Account |
| `debits` | Money out | Account debits Transaction |
| `triggers` | Rule match | Transaction triggers Fault |
| `caused_by` | Alert cause | Alert caused_by Fault |
| `investigates` | Investigation | Alert investigates Customer |
| `associated_with` | Connection | Customer associated_with Customer |
| `transacts_with` | Transaction link | Customer transacts_with Customer |
| `beneficial_owner` | True owner | Person beneficial_owner Company |

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-14 | Initial documentation |

## Contributors

- Mai Thành Duy An - Initial implementation
