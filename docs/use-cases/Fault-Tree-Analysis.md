# Fault Tree Analysis (FTA) với Memory-Graph

## Tổng quan

Document này mô tả cách sử dụng Memory-Graph để thực hiện **Fault Tree Analysis (FTA)** - một phương pháp phân tích độ tin cậy được sử dụng rộng rãi trong các ngành công nghiệp quan trọng như hàng không, hạt nhân, ô tô và y tế.

### Fault Tree Analysis là gì?

FTA là phương pháp phân tích **top-down, deductive** để xác định các nguyên nhân gốc rễ có thể dẫn đến sự cố không mong muốn (Top Event). FTA sử dụng:

- **Top Event**: Sự cố không mong muốn cần phân tích
- **Gates**: Logic AND/OR kết hợp các sự kiện
- **Basic Events**: Các sự cố cơ bản (lá của cây)
- **Intermediate Events**: Các sự cố trung gian
- **Minimal Cut Sets**: Tập hợp nhỏ nhất các sự kiện cơ bản gây ra Top Event

### Tại sao Memory-Graph phù hợp cho FTA?

| Tính năng | Ứng dụng FTA |
|-----------|--------------|
| **Knowledge Graph** | Mô hình hóa cấu trúc cây lỗi phức tạp |
| **Relations** | Biểu diễn AND/OR gates và dependencies |
| **Inference Engine** | Tìm failure paths ẩn |
| **Traverse** | Trace từ Top Event đến Basic Events |
| **Temporal Queries** | Phân tích lịch sử sự cố |
| **Observations** | Ghi nhận xác suất, severity, mitigation |

---

## Use Case: Hệ thống làm mát lò phản ứng hạt nhân

### Bối cảnh

Phân tích sự cố **"Loss of Coolant Accident (LOCA)"** - một trong những sự cố nghiêm trọng nhất trong nhà máy điện hạt nhân. Đây là case study kinh điển được dạy trong các khóa học về độ tin cậy hệ thống.

### Cấu trúc Fault Tree

```
                            ┌─────────────────────────┐
                            │     CORE DAMAGE         │
                            │     (TOP EVENT)         │
                            │     P = 1.2 × 10⁻⁵/yr   │
                            └───────────┬─────────────┘
                                        │
                                   ┌────┴────┐
                                   │   AND   │
                                   └────┬────┘
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
                    ▼                   ▼                   ▼
        ┌───────────────────┐ ┌─────────────────┐ ┌─────────────────┐
        │  LOCA INITIATOR   │ │ ECCS FAILURE    │ │ OPERATOR ERROR  │
        │  P = 1 × 10⁻³/yr  │ │ (AND Gate)      │ │ P = 1 × 10⁻²    │
        └─────────┬─────────┘ └────────┬────────┘ └────────┬────────┘
                  │                    │                   │
           ┌──────┴──────┐      ┌──────┴──────┐     ┌──────┴──────┐
           │     OR      │      │     AND     │     │     OR      │
           └──────┬──────┘      └──────┬──────┘     └──────┬──────┘
                  │                    │                   │
    ┌─────────────┼─────────────┐     │      ┌────────────┼────────────┐
    │             │             │     │      │            │            │
    ▼             ▼             ▼     │      ▼            ▼            ▼
┌───────┐   ┌───────┐   ┌───────┐    │  ┌────────┐  ┌────────┐  ┌────────┐
│ Pipe  │   │ Valve │   │ Pump  │    │  │ Stress │  │ Lack   │  │ Wrong  │
│ Break │   │ Fail  │   │ Seal  │    │  │        │  │Training│  │ Action │
│ 10⁻⁴  │   │ 10⁻³  │   │ Leak  │    │  └────────┘  └────────┘  └────────┘
└───────┘   └───────┘   │ 10⁻⁴  │    │
                        └───────┘    │
                              ┌──────┴──────────────────┐
                              │                         │
                              ▼                         ▼
                    ┌─────────────────┐       ┌─────────────────┐
                    │ HPCI FAILURE    │       │ LPCI FAILURE    │
                    │ (OR Gate)       │       │ (OR Gate)       │
                    └────────┬────────┘       └────────┬────────┘
                             │                         │
               ┌─────────────┼─────────────┐          ...
               │             │             │
               ▼             ▼             ▼
          ┌────────┐   ┌────────┐   ┌────────┐
          │ Pump A │   │ Pump B │   │ Power  │
          │ Fail   │   │ Fail   │   │ Loss   │
          │ 10⁻³   │   │ 10⁻³   │   │ 10⁻²   │
          └────────┘   └────────┘   └────────┘
```

**Legend:**
- **LOCA**: Loss of Coolant Accident
- **ECCS**: Emergency Core Cooling System
- **HPCI**: High Pressure Coolant Injection
- **LPCI**: Low Pressure Coolant Injection

---

## Data Model cho FTA

### Entity Types

| Type | Mô tả | Ví dụ |
|------|-------|-------|
| `TopEvent` | Sự cố cấp cao nhất | Core Damage |
| `Gate` | Logic gate (AND/OR) | AND Gate, OR Gate |
| `IntermediateEvent` | Sự cố trung gian | ECCS Failure |
| `BasicEvent` | Sự cố cơ bản | Pump A Failure |
| `Component` | Thiết bị vật lý | Pump A, Valve V1 |
| `Mitigation` | Biện pháp giảm thiểu | Redundant Pump |
| `MinimalCutSet` | Tập cắt tối thiểu | {Pump A, Pump B, Power} |

### Relation Types

| Relation | Mô tả | Ví dụ |
|----------|-------|-------|
| `causes` | A gây ra B | Pipe Break causes LOCA |
| `input_to` | Input của gate | Basic Event input_to Gate |
| `output_from` | Output của gate | Intermediate Event output_from Gate |
| `mitigated_by` | Được giảm thiểu bởi | Event mitigated_by Mitigation |
| `part_of` | Thuộc về hệ thống | Pump A part_of HPCI System |
| `requires` | Yêu cầu để hoạt động | Pump A requires Power |
| `backup_for` | Dự phòng cho | Pump B backup_for Pump A |

### Observation Structure

```json
{
  "name": "BasicEvent: Pump A Failure",
  "entityType": "BasicEvent",
  "observations": [
    "Failure Rate: 1 × 10⁻³ per year",
    "MTBF: 8760 hours",
    "Failure Mode: Mechanical seizure",
    "Detection: Vibration sensor",
    "Severity: 3/5",
    "Last Inspection: 2026-01-01",
    "Maintenance Schedule: Monthly",
    "Redundancy: Pump B available"
  ]
}
```

---

## Implementation với Memory-Graph

### Bước 1: Tạo Fault Tree Structure

```json
// Tạo Top Event
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "TopEvent: Core Damage",
        "entityType": "TopEvent",
        "observations": [
          "Description: Reactor core damage due to insufficient cooling",
          "Probability: 1.2 × 10⁻⁵ per reactor-year",
          "Consequence: Radiation release, plant shutdown",
          "Severity: Catastrophic (5/5)",
          "INES Level: 5-7"
        ]
      }
    ]
  }
}

// Tạo Gates
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "Gate: AND-001",
        "entityType": "Gate",
        "observations": [
          "Type: AND Gate",
          "Description: All inputs must occur for output",
          "Inputs: 3 (LOCA, ECCS Failure, Operator Error)",
          "Output: Core Damage"
        ]
      },
      {
        "name": "Gate: OR-001",
        "entityType": "Gate",
        "observations": [
          "Type: OR Gate",
          "Description: Any input causes output",
          "Inputs: 3 (Pipe Break, Valve Fail, Pump Seal Leak)",
          "Output: LOCA Initiator"
        ]
      }
    ]
  }
}
```

### Bước 2: Tạo Basic Events với Failure Rates

```json
{
  "tool": "create_entities",
  "params": {
    "entities": [
      {
        "name": "BasicEvent: Pipe Break",
        "entityType": "BasicEvent",
        "observations": [
          "Failure Rate: 1 × 10⁻⁴ per year",
          "Failure Mode: Stress corrosion cracking",
          "Detection: Leak detection system",
          "Consequence: Loss of coolant",
          "Severity: 4/5"
        ]
      },
      {
        "name": "BasicEvent: Pump A Failure",
        "entityType": "BasicEvent",
        "observations": [
          "Failure Rate: 1 × 10⁻³ per year",
          "MTBF: 8760 hours",
          "Failure Mode: Mechanical seizure",
          "Detection: Vibration sensor",
          "Consequence: Loss of high pressure injection",
          "Severity: 3/5"
        ]
      },
      {
        "name": "BasicEvent: Human Error",
        "entityType": "BasicEvent",
        "observations": [
          "Probability: 1 × 10⁻² per demand",
          "Error Type: Omission or commission",
          "Contributing Factors: Stress, fatigue, lack of training",
          "Mitigation: Procedures, training, automation"
        ]
      }
    ]
  }
}
```

### Bước 3: Tạo Relations (Fault Tree Connections)

```json
{
  "tool": "create_relations",
  "params": {
    "relations": [
      // Top Event connections
      {"from": "Gate: AND-001", "to": "TopEvent: Core Damage", "relationType": "causes"},

      // AND Gate inputs
      {"from": "IntermediateEvent: LOCA Initiator", "to": "Gate: AND-001", "relationType": "input_to"},
      {"from": "IntermediateEvent: ECCS Failure", "to": "Gate: AND-001", "relationType": "input_to"},
      {"from": "BasicEvent: Human Error", "to": "Gate: AND-001", "relationType": "input_to"},

      // OR Gate for LOCA
      {"from": "BasicEvent: Pipe Break", "to": "Gate: OR-001", "relationType": "input_to"},
      {"from": "BasicEvent: Valve Failure", "to": "Gate: OR-001", "relationType": "input_to"},
      {"from": "BasicEvent: Pump Seal Leak", "to": "Gate: OR-001", "relationType": "input_to"},
      {"from": "Gate: OR-001", "to": "IntermediateEvent: LOCA Initiator", "relationType": "causes"},

      // Redundancy relations
      {"from": "BasicEvent: Pump B Failure", "to": "BasicEvent: Pump A Failure", "relationType": "backup_for"},

      // Common cause failures
      {"from": "BasicEvent: Power Loss", "to": "BasicEvent: Pump A Failure", "relationType": "affects"},
      {"from": "BasicEvent: Power Loss", "to": "BasicEvent: Pump B Failure", "relationType": "affects"}
    ]
  }
}
```

---

## Queries cho FTA

### 1. Trace từ Top Event đến Basic Events

```json
{
  "tool": "traverse",
  "params": {
    "startNode": "TopEvent: Core Damage",
    "path": [
      {"direction": "in", "relationType": "causes"},
      {"direction": "in", "relationType": "input_to"}
    ],
    "maxResults": 100
  }
}
```

### 2. Tìm tất cả Basic Events

```json
{
  "tool": "search_nodes",
  "params": {
    "query": "BasicEvent:",
    "includeRelations": true
  }
}
```

### 3. Tìm Common Cause Failures

```json
{
  "tool": "infer",
  "params": {
    "entityName": "BasicEvent: Power Loss",
    "maxDepth": 3,
    "minConfidence": 0.5
  }
}
```

**Kết quả inference:**
```json
{
  "inferredRelations": [
    {
      "relation": {
        "from": "BasicEvent: Power Loss",
        "to": "IntermediateEvent: ECCS Failure",
        "relationType": "inferred_affects"
      },
      "confidence": 0.72,
      "explanation": "Power Loss -[affects]-> Pump A -[input_to]-> ECCS Failure"
    }
  ]
}
```

### 4. Tìm Minimal Cut Sets

Sử dụng traverse để tìm tất cả paths từ Basic Events đến Top Event:

```json
{
  "tool": "traverse",
  "params": {
    "startNode": "TopEvent: Core Damage",
    "path": [
      {"direction": "in", "relationType": "causes"},
      {"direction": "in", "relationType": "input_to"},
      {"direction": "in", "relationType": "causes"},
      {"direction": "in", "relationType": "input_to"}
    ],
    "maxResults": 50
  }
}
```

---

## Tính toán Xác suất

### Probability Formulas

**OR Gate:**
```
P(output) = 1 - Π(1 - P(input_i))
≈ Σ P(input_i)  (khi P nhỏ)
```

**AND Gate:**
```
P(output) = Π P(input_i)
```

### Ví dụ tính toán với Memory-Graph

```rust
use memory_graph::knowledge_base::inference::{
    get_config, calculate_weighted_score
};

// Lấy observations từ entities để extract probabilities
fn calculate_gate_probability(gate_type: &str, input_probs: &[f64]) -> f64 {
    match gate_type {
        "AND" => input_probs.iter().product(),
        "OR" => 1.0 - input_probs.iter().map(|p| 1.0 - p).product::<f64>(),
        _ => 0.0
    }
}

// Example: OR Gate với 3 inputs
let inputs = [1e-4, 1e-3, 1e-4];  // Pipe, Valve, Pump Seal
let loca_prob = calculate_gate_probability("OR", &inputs);
// ≈ 1.2 × 10⁻³

// Example: AND Gate
let and_inputs = [1.2e-3, 1e-3, 1e-2];  // LOCA, ECCS, Human
let core_damage_prob = calculate_gate_probability("AND", &and_inputs);
// ≈ 1.2 × 10⁻⁸
```

---

## Importance Measures

### Fussell-Vesely Importance

Đo lường contribution của basic event đến system failure:

```
FV(i) = Σ P(MCS containing i) / P(Top Event)
```

### Risk Achievement Worth (RAW)

Đo lường tăng risk nếu component luôn fail:

```
RAW(i) = P(Top | component i always fails) / P(Top)
```

### Risk Reduction Worth (RRW)

Đo lường giảm risk nếu component hoàn hảo:

```
RRW(i) = P(Top) / P(Top | component i never fails)
```

Sử dụng Memory-Graph để tính:

```json
{
  "tool": "add_observations",
  "params": {
    "observations": [
      {
        "entityName": "BasicEvent: Pump A Failure",
        "contents": [
          "FV Importance: 0.45",
          "RAW: 125",
          "RRW: 1.8",
          "⚠️ Critical component - high importance"
        ]
      }
    ]
  }
}
```

---

## Real-World Applications

### 1. Aerospace (NASA, Boeing)

**Use Case**: Space Shuttle Abort Modes Analysis

```
TopEvent: Loss of Vehicle
├── Gate: OR
│   ├── Main Engine Failure (AND: 3 engines)
│   ├── SRB Failure
│   └── Orbiter Structure Failure
```

### 2. Automotive (ISO 26262)

**Use Case**: Autonomous Vehicle Safety

```
TopEvent: Unintended Acceleration
├── Gate: AND
│   ├── Sensor Failure (LiDAR + Radar + Camera)
│   ├── Software Bug
│   └── Brake Override Failure
```

### 3. Medical Devices (FDA)

**Use Case**: Infusion Pump Hazard Analysis

```
TopEvent: Drug Overdose
├── Gate: OR
│   ├── Pump Malfunction
│   ├── Sensor Error
│   ├── Software Error
│   └── User Error
```

### 4. Oil & Gas

**Use Case**: Offshore Platform Fire/Explosion

```
TopEvent: Platform Fire
├── Gate: AND
│   ├── Fuel Leak (OR: pipe, valve, tank)
│   ├── Ignition Source (OR: electrical, static, hot work)
│   └── Safety System Failure
```

---

## Benefits của Memory-Graph cho FTA

| Benefit | Chi tiết |
|---------|----------|
| **Visualization** | Graph UI hiển thị fault tree trực quan |
| **Traceability** | Traverse từ effect đến root cause |
| **Impact Analysis** | Inference tìm affected components |
| **Documentation** | Observations lưu trữ rationale |
| **Collaboration** | Multi-user access, version history |
| **Integration** | REST/GraphQL API cho external tools |
| **Flexibility** | Custom entity/relation types |

---

## Tools Integration

### Export to OpenPSA

```python
# Python script to export FTA to OpenPSA format
import requests

def export_to_openpsa(base_url, top_event):
    # Get fault tree structure
    response = requests.post(f"{base_url}/graphql", json={
        "query": """
            query FaultTree($top: ID!) {
                traverse(startNode: $top, path: [
                    {relationType: "causes", direction: INCOMING},
                    {relationType: "input_to", direction: INCOMING}
                ]) {
                    paths { nodes relations }
                    endNodes { name observations }
                }
            }
        """,
        "variables": {"top": top_event}
    })

    # Convert to OpenPSA XML format
    return convert_to_openpsa_xml(response.json())
```

### Import from Industry Tools

- **CAFTA** (EPRI)
- **SAPHIRE** (NRC)
- **RiskSpectrum** (Lloyd's)
- **OpenFTA** (Open source)

---

## Best Practices

### 1. Naming Convention

```
TopEvent: [System] [Failure Mode]
IntermediateEvent: [Subsystem] [Failure Mode]
BasicEvent: [Component] [Failure Mode]
Gate: [Type]-[Sequential Number]
```

### 2. Observation Standards

```
Failure Rate: [value] per [unit]
MTBF: [hours]
Failure Mode: [description]
Detection: [method]
Severity: [1-5]
Probability: [scientific notation]
```

### 3. Common Cause Analysis

Luôn model Common Cause Failures:
- Power supply failures
- Environmental factors (flood, earthquake)
- Human errors affecting multiple systems
- Software bugs in redundant systems

---

## References

- **NASA Fault Tree Handbook** (NUREG-0492)
- **IEC 61025:2006** - Fault Tree Analysis
- **ISO 26262** - Automotive Functional Safety
- **MIL-STD-1629A** - Failure Mode and Effects Analysis
- **ARP4761** - Aerospace Safety Assessment

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-14 | Initial documentation |

## Contributors

- Mai Thành Duy An - Initial implementation
