# Workgraph Development Guidelines

This project uses **workgraph** to coordinate work - including work done by AI agents.

## The Workgraph System

Workgraph is a task coordination graph stored in `.workgraph/graph.jsonl`. It tracks:
- **Tasks**: Work items with dependencies, estimates, and status
- **Actors**: Humans and agents who do work
- **Resources**: Budgets, compute, and other constraints

## Agent Protocol

When working on this codebase, follow this protocol:

### 1. Check for Ready Work

Before starting any significant work, check what's ready:

```bash
./target/debug/workgraph ready
```

If there are ready tasks, work on those first unless the user has given explicit instructions.

### 2. Claim Before Working (when implemented)

*Note: claim/unclaim is not yet implemented. For now, announce what you're working on.*

```bash
./target/debug/workgraph claim <task-id>  # Future
```

### 3. Discover Work → Add Tasks

**Critical:** When you discover work that needs doing during development, ADD IT TO WORKGRAPH.

Examples of discovered work:
- "This function needs tests" → `wg add "Add tests for X" --tag tests`
- "This could use better error handling" → `wg add "Improve error handling in Y"`
- "We should document this" → `wg add "Document Z" --tag docs`
- "This blocks something else" → `wg add "Fix A" --blocked-by B`

```bash
./target/debug/workgraph add "Description of work" [--blocked-by X] [--hours N] [--cost N] [-t tag]
```

**Do NOT:**
- Keep work in your head
- Mention "we should do X later" without adding a task
- Start side work without tracking it

### 4. Complete Work → Mark Done

After finishing a task:

```bash
./target/debug/workgraph done <task-id>
```

Then check what's newly unblocked:

```bash
./target/debug/workgraph ready
```

### 5. Verify Graph Health

Periodically check for issues:

```bash
./target/debug/workgraph check
```

## Parallel Agent Pattern

This project supports parallel agent execution using workgraph as the coordination layer:

```
┌─────────────────────────────────────────────────┐
│                  Coordinator                     │
│  1. wg ready --json                             │
│  2. Dispatch agents to ready tasks              │
│  3. Wait for completion                         │
│  4. Repeat until no tasks remain                │
└─────────────────────────────────────────────────┘
        │              │              │
        ▼              ▼              ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ Agent 1 │   │ Agent 2 │   │ Agent 3 │
   │ task-a  │   │ task-b  │   │ task-c  │
   └─────────┘   └─────────┘   └─────────┘
        │              │              │
        ▼              ▼              ▼
   wg done task-a  wg done task-b  wg done task-c
```

### For Background Agents

If you are a background agent spawned to work on a task:

1. You will be told which task to work on
2. Do the work with TDD (tests first)
3. Commit your changes
4. Mark the task done: `./target/debug/workgraph done <id>`
5. Check if you discovered any new work and add it
6. Report completion

## Commands Reference

| Command | Description |
|---------|-------------|
| `wg init` | Initialize workgraph |
| `wg add "<title>"` | Add a task |
| `wg done <id>` | Mark task complete |
| `wg ready` | List ready tasks |
| `wg list` | List all tasks |
| `wg blocked <id>` | Show blockers |
| `wg check` | Verify graph health |
| `wg cost <id>` | Total cost with deps |
| `wg graph` | DOT output |
| `wg --json <cmd>` | JSON output for scripting |

## Project Conventions

- **TDD always**: Write tests first
- **Commit as you go**: Small, atomic commits
- **Task IDs**: Use descriptive kebab-case IDs when provided, or let them auto-generate
- **Dependencies**: Model real dependencies with `--blocked-by`
- **Estimates**: Add `--hours` and `--cost` when known

## Current Development Tasks

Check the current backlog:

```bash
./target/debug/workgraph list
./target/debug/workgraph ready
```

The workgraph itself is tracked in workgraph. Meta!
