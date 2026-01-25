# Workgraph Command Reference

Complete reference for all `wg` commands. All commands support `--json` for machine-readable output and `--dir <path>` to specify a custom workgraph directory.

## Table of Contents

- [Task Management](#task-management)
- [Query Commands](#query-commands)
- [Analysis Commands](#analysis-commands)
- [Actor and Resource Management](#actor-and-resource-management)
- [Agent Commands](#agent-commands)
- [Utility Commands](#utility-commands)

---

## Task Management

### `wg add`

Add a new task to the graph.

```bash
wg add <TITLE> [OPTIONS]
```

**Arguments:**
- `TITLE` - Task title (required)

**Options:**
| Option | Description |
|--------|-------------|
| `--id <ID>` | Custom task ID (auto-generated from title if not provided) |
| `-d, --description <TEXT>` | Detailed description, acceptance criteria |
| `--blocked-by <ID>` | Add dependency on another task (repeatable) |
| `--assign <ACTOR>` | Assign to an actor |
| `--hours <N>` | Estimated hours |
| `--cost <N>` | Estimated cost |
| `-t, --tag <TAG>` | Add tag (repeatable) |
| `--skill <SKILL>` | Required skill (repeatable) |
| `--input <PATH>` | Input file/context needed (repeatable) |
| `--deliverable <PATH>` | Expected output (repeatable) |
| `--max-retries <N>` | Maximum retry attempts |

**Examples:**

```bash
# Simple task
wg add "Fix login bug"

# Task with dependencies and metadata
wg add "Implement user auth" \
  --id user-auth \
  --blocked-by design-api \
  --hours 8 \
  --skill rust \
  --skill security \
  --deliverable src/auth.rs

# Task with custom ID
wg add "Design database schema" --id db-schema
```

---

### `wg done`

Mark a task as completed.

```bash
wg done <ID>
```

Sets status to `done`, records `completed_at` timestamp, and unblocks dependent tasks.

**Example:**
```bash
wg done design-api
# Automatically unblocks tasks that were waiting on design-api
```

---

### `wg fail`

Mark a task as failed (can be retried later).

```bash
wg fail <ID> [--reason <TEXT>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--reason <TEXT>` | Explanation of why the task failed |

**Example:**
```bash
wg fail deploy-prod --reason "AWS credentials expired"
```

---

### `wg abandon`

Mark a task as abandoned (will not be completed).

```bash
wg abandon <ID> [--reason <TEXT>]
```

Abandoned is a terminal state - the task will not be retried.

**Example:**
```bash
wg abandon legacy-migration --reason "Feature deprecated"
```

---

### `wg retry`

Reset a failed task back to open status for another attempt.

```bash
wg retry <ID>
```

Increments the retry counter and sets status back to `open`.

**Example:**
```bash
wg retry deploy-prod
```

---

### `wg claim`

Claim a task for work (sets status to in-progress).

```bash
wg claim <ID> [--actor <ACTOR>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--actor <ACTOR>` | Actor claiming the task |

Claiming sets `started_at` timestamp and assigns the task. Prevents double-work in multi-agent scenarios.

**Example:**
```bash
wg claim implement-api --actor erik
```

---

### `wg unclaim`

Release a claimed task back to open status.

```bash
wg unclaim <ID>
```

Useful when interrupted or handing off work.

**Example:**
```bash
wg unclaim implement-api
```

---

### `wg log`

Add progress notes to a task or view existing logs.

```bash
# Add a log entry
wg log <ID> <MESSAGE> [--actor <ACTOR>]

# View log entries
wg log <ID> --list
```

**Examples:**
```bash
# Log progress
wg log implement-api "Completed endpoint handlers" --actor erik

# View all log entries
wg log implement-api --list
```

---

### `wg show`

Display detailed information about a single task.

```bash
wg show <ID>
```

Shows all task fields including description, logs, timestamps, and dependencies.

**Example output:**
```
Task: implement-api
Title: Implement API endpoints
Status: in-progress
Assigned: erik

Description:
  Create REST API endpoints for user management

Estimate: 8h

Blocked by:
  design-api (done)

Blocks:
  write-tests
  deploy-staging

Skills: rust, api-design
Inputs: docs/api-spec.md
Deliverables: src/api/

Created: 2026-01-15T10:00:00Z
Started: 2026-01-16T09:00:00Z
```

---

## Query Commands

### `wg list`

List all tasks in the graph.

```bash
wg list [--status <STATUS>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--status <STATUS>` | Filter by status (open, in-progress, done, failed, abandoned) |

**Examples:**
```bash
# All tasks
wg list

# Only open tasks
wg list --status open

# JSON output
wg list --json
```

---

### `wg ready`

List tasks ready to work on (no incomplete blockers).

```bash
wg ready
```

Shows only open tasks where all dependencies are done and any `not_before` timestamp has passed.

**Example output:**
```
Ready tasks (3):
  implement-api - Implement API endpoints (8h)
  write-docs - Write documentation (4h)
  setup-ci - Configure CI pipeline (2h)
```

---

### `wg blocked`

Show direct blockers of a task.

```bash
wg blocked <ID>
```

Lists only immediate (not transitive) blockers that are incomplete.

---

### `wg why-blocked`

Show the full transitive chain explaining why a task is blocked.

```bash
wg why-blocked <ID>
```

**Example output:**
```
deploy-prod is blocked by:
  └─ run-tests (open)
      └─ implement-api (in-progress)
          └─ design-api (done) ✓
```

---

### `wg impact`

Show what tasks depend on a given task (forward analysis).

```bash
wg impact <ID>
```

**Example output:**
```
impact of design-api:
  Direct dependents (2):
    implement-api
    write-docs

  Transitive dependents (5):
    run-tests
    deploy-staging
    deploy-prod
    ...
```

---

### `wg context`

Show available context for a task from its completed dependencies.

```bash
wg context <ID> [--dependents]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--dependents` | Also show tasks that will consume this task's outputs |

Shows artifacts and deliverables from completed blockers that can inform the task.

---

## Analysis Commands

### `wg bottlenecks`

Find tasks blocking the most downstream work.

```bash
wg bottlenecks
```

Ranks tasks by impact - completing high-impact bottlenecks unblocks the most work.

**Example output:**
```
Bottlenecks (tasks blocking most work):
  1. design-api - blocks 12 tasks (42h estimated)
  2. setup-infra - blocks 8 tasks (24h estimated)
  3. define-schema - blocks 5 tasks (16h estimated)
```

---

### `wg critical-path`

Show the longest dependency chain (determines minimum project duration).

```bash
wg critical-path
```

**Example output:**
```
Critical path (5 tasks, 28h estimated):
  design-api (4h)
  └─ implement-api (8h)
      └─ run-tests (4h)
          └─ deploy-staging (4h)
              └─ deploy-prod (8h)
```

---

### `wg forecast`

Estimate project completion based on velocity and remaining work.

```bash
wg forecast
```

Uses historical completion rate to project when work will finish.

**Example output:**
```
Project Forecast
================
Completed: 45 tasks
Remaining: 23 tasks (68h estimated)

Velocity (last 4 weeks):
  Week -3: 8 tasks/week
  Week -2: 12 tasks/week
  Week -1: 10 tasks/week
  Current: 6 tasks/week (partial)

Average velocity: 10.5 tasks/week
Estimated completion: ~2.2 weeks (Feb 8, 2026)
```

---

### `wg velocity`

Show task completion velocity over time.

```bash
wg velocity [--weeks <N>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--weeks <N>` | Number of weeks to show (default: 4) |

---

### `wg aging`

Show task age distribution - how long tasks have been open.

```bash
wg aging
```

Identifies stale tasks that may need attention.

**Example output:**
```
Task Age Distribution
=====================
< 1 day:   5 tasks
1-7 days:  12 tasks
1-4 weeks: 8 tasks
> 1 month: 3 tasks (review recommended)

Oldest tasks:
  legacy-cleanup (45 days)
  docs-update (32 days)
```

---

### `wg structure`

Analyze graph structure - entry points, dead ends, high-impact roots.

```bash
wg structure
```

**Example output:**
```
Graph Structure
===============
Entry points (no blockers): 5 tasks
Dead ends (nothing depends on them): 8 tasks
High-impact roots: design-api, setup-infra

Orphan references: none
```

---

### `wg loops`

Analyze cycles in the graph with classification.

```bash
wg loops
```

Identifies intentional cycles (iterative work) vs problematic cycles.

---

### `wg workload`

Show actor workload balance and assignment distribution.

```bash
wg workload
```

**Example output:**
```
Actor Workload
==============
erik:      4 tasks (24h)  ████████
alice:     2 tasks (8h)   ███
agent-1:   6 tasks (12h)  ██████
unassigned: 15 tasks
```

---

### `wg analyze`

Comprehensive health report combining all analyses.

```bash
wg analyze
```

Runs bottlenecks, structure, aging, velocity, and other analyses together.

---

### `wg cost`

Calculate total cost of a task including all dependencies.

```bash
wg cost <ID>
```

Sums estimated costs transitively through the dependency graph.

---

### `wg plan`

Plan what can be accomplished with given resources.

```bash
wg plan [--budget <N>] [--hours <N>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--budget <N>` | Available budget in dollars |
| `--hours <N>` | Available work hours |

**Example:**
```bash
wg plan --hours 40
```

---

### `wg coordinate`

Show ready tasks for parallel execution dispatch.

```bash
wg coordinate [--max-parallel <N>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--max-parallel <N>` | Maximum parallel tasks to show |

Useful for dispatching multiple agents to independent work.

---

## Actor and Resource Management

### `wg actor add`

Register a new actor.

```bash
wg actor add <ID> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--name <NAME>` | Display name |
| `--role <ROLE>` | Role (engineer, pm, agent, etc.) |
| `--rate <RATE>` | Hourly rate |
| `--capacity <HOURS>` | Available work hours |
| `-c, --capability <SKILL>` | Capability/skill (repeatable) |
| `--context-limit <TOKENS>` | Max context size for AI agents |
| `--trust-level <LEVEL>` | verified, provisional, or unknown |

**Examples:**
```bash
# Human actor
wg actor add erik --name "Erik" --role engineer -c rust -c design

# AI agent
wg actor add claude-1 \
  --role agent \
  --trust-level provisional \
  --context-limit 200000 \
  -c coding \
  -c documentation
```

---

### `wg actor list`

List all registered actors.

```bash
wg actor list
```

---

### `wg resource add`

Add a new resource.

```bash
wg resource add <ID> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--name <NAME>` | Display name |
| `--type <TYPE>` | Resource type (money, compute, time) |
| `--available <N>` | Available amount |
| `--unit <UNIT>` | Unit (usd, hours, gpu-hours) |

**Example:**
```bash
wg resource add budget --type money --available 10000 --unit usd
wg resource add compute --type compute --available 100 --unit gpu-hours
```

---

### `wg resource list`

List all resources.

```bash
wg resource list
```

---

### `wg resources`

Show resource utilization (committed vs available).

```bash
wg resources
```

---

### `wg skills`

List and find skills across tasks.

```bash
wg skills [--task <ID>] [--find <SKILL>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--task <ID>` | Show skills for a specific task |
| `--find <SKILL>` | Find tasks requiring a specific skill |

**Examples:**
```bash
# List all skills in use
wg skills

# Find tasks needing rust skills
wg skills --find rust
```

---

### `wg match`

Find actors capable of performing a task.

```bash
wg match <TASK>
```

Matches task skill requirements against actor capabilities.

---

## Agent Commands

### `wg agent`

Run autonomous agent loop (wake/check/work/sleep cycle).

```bash
wg agent --actor <ACTOR> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--actor <ACTOR>` | Actor ID for this agent (required) |
| `--once` | Run only one iteration then exit |
| `--interval <SECONDS>` | Sleep interval between iterations |
| `--max-tasks <N>` | Stop after completing N tasks |

See [Agent Guide](./AGENT-GUIDE.md) for detailed usage.

---

### `wg next`

Find the best next task for an actor.

```bash
wg next --actor <ACTOR>
```

Considers skills, trust level, and task availability to recommend work.

---

### `wg exec`

Execute a task's shell command (claim + run + done/fail).

```bash
wg exec <TASK> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--actor <ACTOR>` | Actor performing execution |
| `--dry-run` | Show command without running |
| `--set <CMD>` | Set the exec command for a task |
| `--clear` | Clear the exec command |

**Examples:**
```bash
# Set a command for a task
wg exec run-tests --set "cargo test"

# Execute the task
wg exec run-tests --actor ci-bot

# Preview without running
wg exec run-tests --dry-run
```

---

### `wg trajectory`

Show context-efficient task trajectory (optimal claim order).

```bash
wg trajectory <TASK> [--actor <ACTOR>]
```

Computes task ordering that minimizes context switching for AI agents.

---

### `wg heartbeat`

Record agent heartbeat or check for stale agents.

```bash
# Record heartbeat
wg heartbeat <ACTOR>

# Check for stale agents
wg heartbeat --check [--threshold <MINUTES>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--check` | Check for stale actors |
| `--threshold <N>` | Minutes without heartbeat before stale (default: 5) |

---

## Utility Commands

### `wg init`

Initialize a new workgraph in the current directory.

```bash
wg init
```

Creates `.workgraph/` directory with `graph.jsonl`.

---

### `wg check`

Check the graph for issues (cycles, orphan references).

```bash
wg check
```

Validates graph integrity and reports problems.

---

### `wg graph`

Output the full graph data.

```bash
wg graph
```

---

### `wg viz`

Visualize the graph with filtering options.

```bash
wg viz [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--all` | Include done tasks |
| `--status <STATUS>` | Filter by status |
| `--critical-path` | Highlight critical path in red |
| `--format <FMT>` | Output format: dot, mermaid (default: dot) |
| `-o, --output <FILE>` | Render directly to file (requires graphviz) |

**Examples:**
```bash
# DOT format to stdout
wg viz

# Mermaid format
wg viz --format mermaid

# Render PNG
wg viz -o graph.png

# With critical path highlighted
wg viz --critical-path -o critical.png
```

---

### `wg archive`

Archive completed tasks to a separate file.

```bash
wg archive [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be archived |
| `--older <DURATION>` | Only archive tasks older than (e.g., 7d, 30d) |
| `--list` | List archived tasks |

**Examples:**
```bash
# Archive all done tasks
wg archive

# Archive tasks completed more than 30 days ago
wg archive --older 30d

# Preview
wg archive --dry-run
```

---

### `wg reschedule`

Reschedule a task (set `not_before` timestamp).

```bash
wg reschedule <ID> <TIMESTAMP>
```

Task will not appear in `wg ready` until the timestamp passes.

**Example:**
```bash
wg reschedule deploy-prod "2026-02-01T09:00:00Z"
```

---

### `wg artifact`

Manage task artifacts (produced outputs).

```bash
# Add artifact
wg artifact <TASK> <PATH>

# List artifacts
wg artifact <TASK>

# Remove artifact
wg artifact <TASK> <PATH> --remove
```

---

### `wg config`

View or modify project configuration.

```bash
wg config [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--show` | Display current configuration |
| `--init` | Create default config file |
| `--executor <NAME>` | Set executor (claude, opencode, codex) |
| `--model <MODEL>` | Set model |
| `--set-interval <SECS>` | Set agent sleep interval |

**Examples:**
```bash
# View config
wg config --show

# Set executor
wg config --executor claude --model opus-4-5
```

---

## Global Options

All commands support these options:

| Option | Description |
|--------|-------------|
| `--dir <PATH>` | Workgraph directory (default: .workgraph) |
| `--json` | Output as JSON |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

**Example:**
```bash
# Use alternate directory
wg --dir /path/to/project/.workgraph list

# JSON output for scripting
wg list --json | jq '.[] | select(.status == "open")'
```
