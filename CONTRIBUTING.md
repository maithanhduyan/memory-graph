# Contributing to Memory Graph

First off, thank you for considering contributing to Memory Graph! 🎉

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code.

## How Can I Contribute?

### 🐛 Reporting Bugs

Before creating bug reports, please check existing issues. When you create a bug report, include as many details as possible:

- **Use a clear and descriptive title**
- **Describe the exact steps to reproduce the problem**
- **Provide specific examples** (JSON payloads, error messages)
- **Describe the behavior you observed and what you expected**
- **Include your environment** (OS, Rust version, MCP client)

### 💡 Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion:

- **Use a clear and descriptive title**
- **Provide a detailed description of the suggested enhancement**
- **Explain why this enhancement would be useful**
- **List any alternatives you've considered**

### 🔧 Pull Requests

1. **Fork the repo** and create your branch from `main`:
   ```bash
   git checkout -b feature/amazing-feature
   ```

2. **Make your changes:**
   - Follow the existing code style
   - Add tests if applicable
   - Update documentation if needed

3. **Run the test suite:**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit your changes:**
   ```bash
   git commit -m "Add amazing feature"
   ```

5. **Push to your fork:**
   ```bash
   git push origin feature/amazing-feature
   ```

6. **Open a Pull Request**

## Development Setup

### Prerequisites

- Rust 1.70+ (`rustup update stable`)
- Git

### Building

```bash
git clone https://github.com/maithanhduyan/memory-graph.git
cd memory-graph
cargo build
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_concurrent_access
```

### Code Style

We use standard Rust formatting:

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy
```

## Project Structure

```
memory-graph/
├── memory.rs          # Main implementation (single file)
├── Cargo.toml         # Dependencies
├── memory.jsonl       # Knowledge graph data (runtime)
├── Dockerfile         # Container build
├── README.md          # Documentation
├── CHANGELOG.md       # Version history
├── CONTRIBUTING.md    # This file
└── .github/
    └── workflows/
        └── rust.yml   # CI/CD
```

## Architecture Overview

The codebase is organized into sections within `memory.rs`:

1. **Types** - Core data structures (`Entity`, `Relation`, `KnowledgeGraph`)
2. **Validation** - Standard types and validation functions
3. **Synonym Dictionary** - Semantic search word groups
4. **KnowledgeBase** - Thread-safe storage with Mutex
5. **Tools** - Individual MCP tool implementations
6. **MCP Server** - JSON-RPC protocol handler
7. **Tests** - Unit and concurrency tests

## Adding a New Tool

1. Create a struct implementing the `Tool` trait:

```rust
pub struct MyNewTool {
    kb: std::sync::Arc<KnowledgeBase>,
}

impl Tool for MyNewTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "my_new_tool".to_string(),
            description: "Description here".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    // Define parameters
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        // Implementation
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Result"
            }]
        }))
    }
}
```

2. Register the tool in `main()`:

```rust
server.register_tool(Box::new(MyNewTool::new(kb.clone())));
```

3. Add tests and update documentation.

## Commit Message Guidelines

- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Move cursor to..." not "Moves cursor to...")
- Keep the first line under 72 characters
- Reference issues when applicable

Examples:
- `Add temporal query support for relations`
- `Fix race condition in concurrent writes`
- `Update README with Docker instructions`

## Questions?

Feel free to open an issue with the `question` label or reach out to the maintainers.

Thank you for contributing! 🙏
