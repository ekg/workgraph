# Workgraph Agent Guide

This guide covers operating AI agents with workgraph, including the autonomous agent runtime, task selection, coordination patterns, and integration with AI coding assistants.

## Table of Contents

- [Overview](#overview)
- [Agent Architecture](#agent-architecture)
- [Setting Up Agents](#setting-up-agents)
- [The Agent Loop](#the-agent-loop)
- [Task Selection](#task-selection)
- [Manual Agent Operation](#manual-agent-operation)
- [Multi-Agent Coordination](#multi-agent-coordination)
- [Context and Trajectories](#context-and-trajectories)
- [Configuration](#configuration)
- [Monitoring Agents](#monitoring-agents)
- [Best Practices](#best-practices)

---

## Overview

Workgraph supports AI agent operation through:

1. **Autonomous mode**: `wg agent` runs a continuous wake/check/work/sleep loop
2. **Manual mode**: Agents use `wg next`, `wg claim`, `wg done` for step-by-step control
3. **Exec mode**: `wg exec` runs shell commands attached to tasks

Agents are registered as actors with capabilities, and workgraph matches them to appropriate tasks.

---

## Agent Architecture

### The Wake/Check/Work/Sleep Cycle

```
     ┌──────────────────────────────────────┐
     │                                      │
     v                                      │
   WAKE                                     │
     │                                      │
     │  Record heartbeat                    │
     │                                      │
     v                                      │
   CHECK                                    │
     │                                      │
     │  Find ready tasks                    │
     │  Match to agent capabilities         │
     │  Select best task                    │
     │                                      │
     ├──── No work? ───────────────────────>│
     │                                      │
     v                                      │
   WORK                                     │
     │                                      │
     │  Claim task                          │
     │  Execute (if exec command set)       │
     │  Mark done or failed                 │
     │                                      │
     v                                      │
   SLEEP ──────────────────────────────────>┘
     │
     │  Wait interval seconds
     │
     └── (repeat until stopped)
```

### Agent Identity

Each agent needs an actor record:

```bash
wg actor add agent-1 \
  --name "Claude Agent 1" \
  --role agent \
  --trust-level provisional \
  -c coding \
  -c documentation \
  -c testing
```

The actor record stores:
- **capabilities**: Skills the agent can apply
- **trust_level**: verified (full trust), provisional (limited), unknown
- **context_limit**: Maximum context tokens (for trajectory planning)
- **last_seen**: Heartbeat timestamp (for detecting dead agents)

---

## Setting Up Agents

### 1. Initialize the Workgraph

```bash
wg init
```

### 2. Register Agent Actors

```bash
# Primary coding agent
wg actor add claude-main \
  --name "Claude Main" \
  --role agent \
  --trust-level provisional \
  -c rust \
  -c python \
  -c documentation

# Secondary agent for testing
wg actor add claude-test \
  --name "Claude Test" \
  --role agent \
  --trust-level provisional \
  -c testing \
  -c review
```

### 3. Configure Agent Settings

```bash
# Set defaults for agent behavior
wg config --executor claude --model opus-4-5 --set-interval 10
```

Or edit `.workgraph/config.toml` directly:

```toml
[agent]
executor = "claude"
model = "opus-4-5"
interval = 10           # seconds between iterations
max_tasks = 50          # stop after N tasks (optional)
heartbeat_timeout = 5   # minutes before agent considered dead

[project]
name = "My Project"
```

### 4. Add Tasks with Skill Requirements

```bash
wg add "Implement user service" \
  --skill rust \
  --skill api-design \
  --deliverable src/services/user.rs

wg add "Write unit tests" \
  --blocked-by implement-user-service \
  --skill testing \
  --deliverable tests/user_test.rs
```

---

## The Agent Loop

### Running the Autonomous Agent

```bash
# Run continuously
wg agent --actor claude-main

# Run single iteration (good for testing)
wg agent --actor claude-main --once

# Custom interval
wg agent --actor claude-main --interval 30

# Stop after completing 10 tasks
wg agent --actor claude-main --max-tasks 10
```

### Agent Output

```
Agent 'claude-main' starting...
   Interval: 10s | Once: false | Max tasks: None

-> Working on: implement-api - Implement API endpoints
  Executing: cargo build
  | Compiling api v0.1.0
  | Finished dev [unoptimized + debuginfo]
Completed: implement-api

-> Working on: run-tests - Run test suite
  Executing: cargo test
  | running 15 tests
  | test result: ok. 15 passed
Completed: run-tests

No work available, sleeping 10s...
```

### Tasks with Exec Commands

For fully automated execution, attach shell commands to tasks:

```bash
# Set exec command
wg exec run-tests --set "cargo test"
wg exec build --set "cargo build --release"
wg exec deploy --set "./scripts/deploy.sh"

# Agent will automatically run these
wg agent --actor ci-agent
```

### Tasks Without Exec Commands

When a task has no exec command, the agent claims it and reports:

```
-> Working on: design-api - Design API schema
  No exec command - task claimed for external execution
  Complete with: wg done design-api
```

The task remains in-progress for manual or AI-assisted completion.

---

## Task Selection

### How Tasks Are Selected

The `wg next` and `wg agent` commands score tasks by:

1. **Skill match**: Tasks with matching requirements score higher
2. **No missing skills**: Tasks where agent has all required skills get a bonus
3. **Exec command**: Tasks with automation commands score higher
4. **Trust level**: Verified agents get a small bonus
5. **General tasks**: Tasks with no skill requirements are available to all

```
Score calculation:
  +10 for each matched skill
  -5 for each missing required skill
  +20 if all required skills matched
  +5 if task has no skill requirements
  +15 if task has exec command
  +5 for verified trust level
```

### Viewing Task Selection

```bash
# See what task would be selected
wg next --actor claude-main

# Output:
# Next task for claude-main:
#   implement-api - Implement API endpoints
#   Skills: rust, api-design (all matched)
#   Inputs: docs/api-spec.md
#   Deliverables: src/api/
```

---

## Manual Agent Operation

For AI assistants (like Claude Code) that work interactively:

### Protocol

1. **Check for work**
   ```bash
   wg ready
   ```

2. **Select and claim a task**
   ```bash
   wg next --actor claude
   wg claim <task-id> --actor claude
   ```

3. **View task details**
   ```bash
   wg show <task-id>
   ```

4. **Do the work** (coding, documentation, etc.)

5. **Log progress**
   ```bash
   wg log <task-id> "Completed implementation" --actor claude
   ```

6. **Mark complete or failed**
   ```bash
   wg done <task-id>
   # or
   wg fail <task-id> --reason "Blocked by missing dependency"
   ```

7. **Check what's unblocked**
   ```bash
   wg ready
   ```

### Example Session

```bash
$ wg ready
Ready tasks (3):
  design-api - Design API schema (4h)
  setup-ci - Configure CI pipeline (2h)
  write-readme - Write README (1h)

$ wg claim design-api --actor claude
Claimed: design-api

$ wg show design-api
Task: design-api
Title: Design API schema
Status: in-progress
Assigned: claude
...

# (do the work)

$ wg log design-api "Defined REST endpoints for users, posts, comments"
Log added

$ wg done design-api
Done: design-api
Newly unblocked:
  implement-api
  write-api-docs
```

---

## Multi-Agent Coordination

### Parallel Execution

Multiple agents can work simultaneously on independent tasks:

```bash
# See what can run in parallel
wg coordinate --max-parallel 4

# Output:
# Parallel execution slots (4 available):
#   1. implement-api (rust, api-design)
#   2. write-docs (documentation)
#   3. setup-ci (devops)
#   4. design-ui (frontend)
```

### Claim Atomicity

Claims are atomic - if two agents try to claim the same task, only one succeeds:

```bash
# Agent 1
$ wg claim implement-api --actor agent-1
Claimed: implement-api

# Agent 2 (simultaneous)
$ wg claim implement-api --actor agent-2
Error: Task 'implement-api' is already claimed by agent-1
```

### Heartbeats and Dead Agent Detection

Agents record heartbeats to indicate they're alive:

```bash
# Record heartbeat (done automatically by wg agent)
wg heartbeat agent-1

# Check for dead agents
wg heartbeat --check --threshold 5

# Output:
# Stale agents (no heartbeat in 5 minutes):
#   agent-2 (last seen: 2026-01-15T10:00:00Z, claimed: implement-api)
```

### Reclaiming Dead Agent Work

If an agent dies with claimed tasks:

```bash
# Unclaim the task
wg unclaim implement-api

# Or reassign
wg claim implement-api --actor agent-3
```

---

## Context and Trajectories

### Context Inheritance

Tasks can specify inputs (what they need) and deliverables (what they produce):

```bash
wg add "Design API" \
  --deliverable docs/api-spec.md

wg add "Implement API" \
  --blocked-by design-api \
  --input docs/api-spec.md \
  --deliverable src/api/
```

View available context:

```bash
wg context implement-api

# Output:
# Context for implement-api:
#   From design-api (done):
#     Artifacts:
#       docs/api-spec.md
#     Deliverables:
#       docs/api-spec.md
```

### Trajectory Planning

For AI agents with limited context windows, trajectories minimize context switching:

```bash
wg trajectory implement-api --actor claude-main

# Output:
# Optimal trajectory from implement-api:
#   1. implement-api (uses: src/, docs/api-spec.md)
#   2. write-api-tests (uses: src/, test/)
#   3. update-api-docs (uses: docs/, src/api/)
#
# Context overlap: 85%
# Estimated tokens: 45000
```

The trajectory groups related tasks that share context (files, directories, concepts).

---

## Configuration

### Config File

`.workgraph/config.toml`:

```toml
[agent]
# AI executor: claude, opencode, codex, shell
executor = "claude"

# Model for AI execution
model = "opus-4-5"

# Seconds between agent loop iterations
interval = 10

# Stop after this many tasks (optional)
max_tasks = 100

# Minutes without heartbeat = dead agent
heartbeat_timeout = 5

# Command template for AI execution
# Placeholders: {model}, {prompt}, {task_id}, {workdir}
command_template = "claude --model {model} --print \"{prompt}\""

[project]
name = "My Project"
description = "Project description"
default_skills = ["coding", "documentation"]
```

### Runtime Configuration

```bash
# View current config
wg config --show

# Set executor
wg config --executor opencode

# Set interval
wg config --set-interval 30

# Initialize default config
wg config --init
```

---

## Monitoring Agents

### Check Agent Status

```bash
# List actors with last_seen
wg actor list --json | jq '.[] | select(.role == "agent")'

# Check for stale agents
wg heartbeat --check

# View workload distribution
wg workload
```

### View Agent Activity

```bash
# Tasks claimed by an agent
wg list --json | jq '.[] | select(.assigned == "claude-main")'

# Recent completions
wg list --status done --json | jq 'sort_by(.completed_at) | reverse | .[0:5]'
```

### Agent Statistics

After `wg agent` completes:

```
Agent statistics:
  Iterations: 15
  Tasks completed: 12
  Tasks failed: 1
  Idle iterations: 2
```

With `--json`:

```json
{
  "iterations": 15,
  "tasks_completed": 12,
  "tasks_failed": 1,
  "idle_iterations": 2
}
```

---

## Best Practices

### Task Design

1. **Use clear skill requirements**: Help task selection match agents to appropriate work
   ```bash
   wg add "Implement auth" --skill rust --skill security
   ```

2. **Specify inputs and deliverables**: Enable context inheritance
   ```bash
   wg add "Write tests" --input src/auth.rs --deliverable tests/auth_test.rs
   ```

3. **Add exec commands for automation**: Enable fully autonomous execution
   ```bash
   wg exec run-tests --set "cargo test"
   ```

4. **Use max_retries for flaky tasks**:
   ```bash
   wg add "Deploy to staging" --max-retries 3
   ```

### Agent Operation

1. **Start with `--once`**: Test agent behavior before continuous operation
   ```bash
   wg agent --actor claude --once
   ```

2. **Use appropriate intervals**: Too short wastes resources, too long delays work
   ```bash
   wg agent --actor claude --interval 30
   ```

3. **Monitor heartbeats**: Detect dead agents quickly
   ```bash
   wg heartbeat --check --threshold 3
   ```

4. **Log progress**: Create audit trail of agent work
   ```bash
   wg log task-id "Implemented feature X" --actor claude
   ```

### Multi-Agent Setups

1. **Specialize agents**: Give different agents different capabilities
   ```bash
   wg actor add agent-code -c rust -c python
   wg actor add agent-docs -c documentation -c review
   wg actor add agent-test -c testing -c qa
   ```

2. **Use coordinate for dispatch**: See what can run in parallel
   ```bash
   wg coordinate --max-parallel 3
   ```

3. **Handle failures gracefully**: Failed tasks can be retried
   ```bash
   wg retry failed-task
   ```

### Integration with AI Assistants

For interactive AI assistants (Claude Code, Cursor, etc.):

1. **Add CLAUDE.md instructions**:
   ```markdown
   # Agent Protocol
   1. Check `wg ready` before starting work
   2. Claim tasks: `wg claim <id> --actor claude`
   3. When done: `wg done <id>`
   4. If you discover new work, add it: `wg add "..." --blocked-by X`
   ```

2. **Use show for context**:
   ```bash
   wg show task-id  # Get full task details
   wg context task-id  # Get inherited context
   ```

3. **Track progress**:
   ```bash
   wg log task-id "Working on X"
   wg log task-id "Completed Y, moving to Z"
   ```

---

## JSON Output Examples

### Ready Tasks

```bash
wg ready --json
```

```json
[
  {
    "id": "implement-api",
    "title": "Implement API endpoints",
    "hours": 8,
    "skills": ["rust", "api-design"],
    "inputs": ["docs/api-spec.md"],
    "deliverables": ["src/api/"]
  }
]
```

### Next Task

```bash
wg next --actor claude --json
```

```json
{
  "task": {
    "id": "implement-api",
    "title": "Implement API endpoints",
    "skills": ["rust", "api-design"]
  },
  "score": 45,
  "matched_skills": ["rust", "api-design"],
  "missing_skills": []
}
```

### Agent Stats

```bash
wg agent --actor claude --once --json
```

```json
{
  "iterations": 1,
  "tasks_completed": 1,
  "tasks_failed": 0,
  "idle_iterations": 0
}
```
