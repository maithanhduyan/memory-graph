# Memory Graph MCP Server
[![Rust](https://github.com/maithanhduyan/memory-graph/actions/workflows/rust.yml/badge.svg)](https://github.com/maithanhduyan/memory-graph/actions/workflows/rust.yml)

A Knowledge Graph MCP (Model Context Protocol) server implemented in Rust with minimal dependencies.

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

- **Persistent Storage**: Data is stored in JSONL format
- **Pure Rust**: Minimal dependencies (only `serde` and `serde_json`)
- **MCP Protocol**: Full JSON-RPC 2.0 implementation

## Build

```bash
cargo build --release
```

## Run

```bash
MEMORY_FILE_PATH=./memory.jsonl ./target/release/memory-server
```

On Windows:
```powershell
$env:MEMORY_FILE_PATH="./memory.jsonl"; .\target\release\memory-server.exe
```

## Usage

### In VSCode

Add to your `.vscode/mcp.json`:

```json
{
    "servers": {
        "memory": {
            "type": "stdio",
            "command": "${workspaceFolder}/target/release/memory-server.exe",
            "args": [],
            "env": {
                "MEMORY_FILE_PATH": "${workspaceFolder}/memory.jsonl"
            }
        }
    }
}
```

## Data Format

The knowledge graph is stored in JSONL format. Each line is either an entity or a relation:

**Entity:**
```json
{"name":"Alice","entityType":"Person","observations":["Lives in NYC","Software Engineer"]}
```

**Relation:**
```json
{"from":"Alice","to":"Bob","relationType":"knows"}
```

## Example Usage

### Create Entities
```json
{
    "entities": [
        {
            "name": "Alice",
            "entityType": "Person",
            "observations": ["Software Engineer", "Lives in NYC"]
        },
        {
            "name": "Bob",
            "entityType": "Person",
            "observations": ["Doctor"]
        }
    ]
}
```

### Create Relations
```json
{
    "relations": [
        {
            "from": "Alice",
            "to": "Bob",
            "relationType": "knows"
        }
    ]
}
```

### Search Nodes
```json
{
    "query": "Engineer"
}
```

## Tests

Run the tests with:

```bash
cargo test
```
