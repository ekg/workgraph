# Workgraph Skill

Use this skill to interact with the workgraph task management system.

## Invocation

- `/wg` - Show ready tasks and current status
- `/wg ready` - List tasks ready to work on
- `/wg add <title>` - Add a new task
- `/wg done <id>` - Mark a task complete
- `/wg plan` - Show project status and suggest next steps

## Instructions

When this skill is invoked, use the `wg` CLI tool located at `./target/debug/workgraph` (or just `workgraph` if installed).

### Default behavior (`/wg` with no args)

1. Run `./target/debug/workgraph ready` to show tasks ready to work on
2. Run `./target/debug/workgraph check` to verify graph health
3. Summarize the current state for the user

### Adding tasks (`/wg add <title>`)

Parse the user's request and run:
```bash
./target/debug/workgraph add "<title>" [options]
```

Available options:
- `--id <id>` - Custom task ID (auto-generated if omitted)
- `--blocked-by <id>` - Task this is blocked by (can repeat)
- `--assign <actor>` - Assign to an actor
- `--hours <n>` - Estimated hours
- `--cost <n>` - Estimated cost
- `-t <tag>` - Add a tag (can repeat)

### Completing tasks (`/wg done <id>`)

```bash
./target/debug/workgraph done <id>
```

After marking done, show the updated ready list.

### Planning (`/wg plan`)

1. Run `./target/debug/workgraph list` to see all tasks
2. Run `./target/debug/workgraph ready` to see what's actionable
3. Run `./target/debug/workgraph check` to verify no issues
4. Analyze the dependency graph and suggest which task(s) to work on next
5. If there are blocked tasks, explain what needs to happen to unblock them

### Working on a task

When starting work on a task from the workgraph:

1. Announce which task you're working on
2. Do the work
3. Run tests to verify
4. Commit the changes
5. Mark the task done: `./target/debug/workgraph done <id>`
6. Show the updated ready list

### JSON output for scripting

Add `--json` flag for machine-readable output:
```bash
./target/debug/workgraph --json ready
./target/debug/workgraph --json list
```

### Graph visualization

Generate DOT format for visualization:
```bash
./target/debug/workgraph graph | dot -Tpng -o graph.png
```

## Data Location

The workgraph data lives in `.workgraph/graph.jsonl` in the project root. This file is:
- Human-readable and editable
- Git-friendly (one JSON object per line)
- The single source of truth for all tasks

## Current Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize workgraph in current directory |
| `add` | Add a new task |
| `done` | Mark task as done |
| `ready` | List tasks with no open blockers |
| `blocked <id>` | Show what's blocking a task |
| `check` | Verify graph integrity |
| `list` | List all tasks |
| `graph` | Output DOT format |
| `cost <id>` | Calculate cost including dependencies |

## Future Commands (planned)

| Command | Description | Status |
|---------|-------------|--------|
| `actor` | Manage actors (humans/agents) | planned |
| `resource` | Manage resources (budgets) | planned |
| `plan` | Feasibility planning | planned |
| `claim/unclaim` | Agent coordination | planned |
| `verify` | Formal verification export | planned |
