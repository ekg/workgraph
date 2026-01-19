# workgraph

Task coordination for humans and AI agents.

## Install

```bash
cargo build --release
```

The binary will be at `./target/release/workgraph` (or use `cargo install --path .`).

## Quick Start

```bash
# Initialize a new workgraph in current directory
wg init

# Add tasks
wg add "Design API"
wg add "Implement backend" --blocked-by design-api
wg add "Write tests" --blocked-by implement-backend

# See what's ready to work on
wg ready

# Claim a task
wg claim design-api

# Mark it done
wg done design-api
```

## Commands

| Command | Description |
|---------|-------------|
| `wg init` | Initialize workgraph in current directory |
| `wg add "task title"` | Add a new task |
| `wg done <id>` | Mark task as complete |
| `wg ready` | List tasks ready to work on |
| `wg claim <id>` | Claim a task for work |
| `wg why-blocked <id>` | Show why a task is blocked |
| `wg impact <id>` | Show what depends on this task |
| `wg bottlenecks` | Find tasks blocking the most work |
| `wg forecast` | Estimate when work will complete |

## Storage

All data lives in `.workgraph/graph.jsonl` — one JSON object per line. Human-readable, version-control friendly, easy to parse.

## License

[MIT](LICENSE)
