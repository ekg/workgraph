# Workgraph Command Reference

Complete reference for all `wg` commands. All commands support `--json` for machine-readable output and `--dir <path>` to specify a custom workgraph directory.

## Table of Contents

- [Task Management](#task-management)
- [Query Commands](#query-commands)
- [Analysis Commands](#analysis-commands)
- [Actor and Resource Management](#actor-and-resource-management)
- [Agency Commands](#agency-commands)
- [Agent Commands](#agent-commands)
- [Service Commands](#service-commands)
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
| `--blocked-by <ID>` | Add dependency on another task (repeatable, comma-separated) |
| `--assign <ACTOR>` | Assign to an actor |
| `--hours <N>` | Estimated hours |
| `--cost <N>` | Estimated cost |
| `-t, --tag <TAG>` | Add tag (repeatable) |
| `--skill <SKILL>` | Required skill (repeatable) |
| `--input <PATH>` | Input file/context needed (repeatable) |
| `--deliverable <PATH>` | Expected output (repeatable) |
| `--max-retries <N>` | Maximum retry attempts |
| `--model <MODEL>` | Preferred model for this task (haiku, sonnet, opus) |
| `--verify <CRITERIA>` | Verification criteria — task requires review before done |

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

# Task with model override
wg add "Quick formatting fix" --model haiku

# Task requiring review
wg add "Security audit" --verify "All findings documented with severity ratings"
```

---

### `wg edit`

Modify an existing task's fields without replacing it.

```bash
wg edit <ID> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--title <TEXT>` | Update task title |
| `-d, --description <TEXT>` | Update task description |
| `--add-blocked-by <ID>` | Add a blocked-by dependency (repeatable) |
| `--remove-blocked-by <ID>` | Remove a blocked-by dependency (repeatable) |
| `--add-tag <TAG>` | Add a tag (repeatable) |
| `--remove-tag <TAG>` | Remove a tag (repeatable) |
| `--add-skill <SKILL>` | Add a required skill (repeatable) |
| `--remove-skill <SKILL>` | Remove a required skill (repeatable) |
| `--model <MODEL>` | Update preferred model |

Triggers a `graph_changed` IPC notification to the service daemon, so the coordinator picks up changes immediately.

**Examples:**

```bash
# Change title
wg edit my-task --title "Better title"

# Add a dependency
wg edit my-task --add-blocked-by other-task

# Swap tags
wg edit my-task --remove-tag stale --add-tag urgent

# Change model
wg edit my-task --model opus
```

---

### `wg done`

Mark a task as completed.

```bash
wg done <ID>
```

Sets status to `done`, records `completed_at` timestamp, and unblocks dependent tasks. Fails for verified tasks (use `wg submit` instead).

**Example:**
```bash
wg done design-api
# Automatically unblocks tasks that were waiting on design-api
```

---

### `wg submit`

Submit a verified task for review.

```bash
wg submit <ID> [--actor <ACTOR>]
```

Sets status to `pending-review`. Used for tasks created with `--verify` that require approval before completion.

**Example:**
```bash
wg submit security-audit --actor claude
```

---

### `wg approve`

Approve a pending-review task (marks as done).

```bash
wg approve <ID> [--actor <ACTOR>]
```

**Example:**
```bash
wg approve security-audit --actor erik
```

---

### `wg reject`

Reject a pending-review task (returns to open for rework).

```bash
wg reject <ID> [--reason <TEXT>] [--actor <ACTOR>]
```

**Example:**
```bash
wg reject security-audit --reason "Missing OWASP top 10 coverage" --actor erik
```

---

### `wg fail`

Mark a task as failed (can be retried later).

```bash
wg fail <ID> [--reason <TEXT>]
```

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

Abandoned is a terminal state — the task will not be retried.

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

---

### `wg claim`

Claim a task for work (sets status to in-progress).

```bash
wg claim <ID> [--actor <ACTOR>]
```

Claiming sets `started_at` timestamp and assigns the task. Prevents double-work in multi-agent scenarios.

---

### `wg unclaim`

Release a claimed task back to open status.

```bash
wg unclaim <ID>
```

---

### `wg reclaim`

Reclaim a task from a dead/unresponsive agent.

```bash
wg reclaim <ID> --from <ACTOR> --to <ACTOR>
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
wg log implement-api "Completed endpoint handlers" --actor erik
wg log implement-api --list
```

---

### `wg assign`

Assign an agent identity to a task (or clear the assignment).

```bash
wg assign <TASK> <AGENT-HASH>    # Assign agent to task
wg assign <TASK> --clear         # Remove assignment
```

When the service spawns that task, the agent's role and motivation are injected into the prompt. The agent hash can be a prefix (minimum 4 characters).

**Example:**
```bash
wg assign my-task a3f7c21d
wg assign my-task --clear
```

---

### `wg show`

Display detailed information about a single task.

```bash
wg show <ID>
```

Shows all task fields including description, logs, timestamps, dependencies, model, and agent assignment.

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
| `--status <STATUS>` | Filter by status (open, in-progress, done, failed, abandoned, pending-review) |

---

### `wg ready`

List tasks ready to work on (no incomplete blockers).

```bash
wg ready
```

Shows only open tasks where all dependencies are done and any `not_before` timestamp has passed.

---

### `wg blocked`

Show direct blockers of a task.

```bash
wg blocked <ID>
```

---

### `wg why-blocked`

Show the full transitive chain explaining why a task is blocked.

```bash
wg why-blocked <ID>
```

---

### `wg impact`

Show what tasks depend on a given task (forward analysis).

```bash
wg impact <ID>
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

---

### `wg status`

Quick one-screen status overview of the project.

```bash
wg status
```

---

## Analysis Commands

### `wg bottlenecks`

Find tasks blocking the most downstream work.

```bash
wg bottlenecks
```

---

### `wg critical-path`

Show the longest dependency chain (determines minimum project duration).

```bash
wg critical-path
```

---

### `wg forecast`

Estimate project completion based on velocity and remaining work.

```bash
wg forecast
```

---

### `wg velocity`

Show task completion velocity over time.

```bash
wg velocity [--weeks <N>]
```

---

### `wg aging`

Show task age distribution — how long tasks have been open.

```bash
wg aging
```

---

### `wg structure`

Analyze graph structure — entry points, dead ends, high-impact roots.

```bash
wg structure
```

---

### `wg loops`

Analyze cycles in the graph with classification.

```bash
wg loops
```

---

### `wg workload`

Show actor workload balance and assignment distribution.

```bash
wg workload
```

---

### `wg analyze`

Comprehensive health report combining all analyses.

```bash
wg analyze
```

---

### `wg cost`

Calculate total cost of a task including all dependencies.

```bash
wg cost <ID>
```

---

### `wg plan`

Plan what can be accomplished with given resources.

```bash
wg plan [--budget <N>] [--hours <N>]
```

---

### `wg coordinate`

Show ready tasks for parallel execution dispatch.

```bash
wg coordinate [--max-parallel <N>]
```

---

### `wg dag`

Show ASCII DAG of the dependency graph.

```bash
wg dag [--all] [--status <STATUS>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--all` | Include done tasks |
| `--status <STATUS>` | Filter by status |

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
| `-t, --type <TYPE>` | Actor type: agent or human |
| `--matrix <USER_ID>` | Matrix user ID for human actors (@user:server) |

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
wg resource add <ID> [--name <NAME>] [--type <TYPE>] [--available <N>] [--unit <UNIT>]
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
wg skill list           # list all skills in use
wg skill task <ID>      # show skills for a specific task
wg skill find <SKILL>   # find tasks requiring a specific skill
wg skill install        # install the wg Claude Code skill to ~/.claude/skills/wg/
```

---

### `wg match`

Find actors capable of performing a task.

```bash
wg match <TASK>
```

---

## Agency Commands

The agency system manages composable agent identities (roles + motivations). See [AGENCY.md](AGENCY.md) for the full design.

### `wg agency init`

Seed the agency with starter roles (Programmer, Reviewer, Documenter, Architect) and motivations (Careful, Fast, Thorough, Balanced).

```bash
wg agency init
```

---

### `wg agency stats`

Display aggregated performance statistics across the agency.

```bash
wg agency stats [--min-evals <N>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--min-evals <N>` | Minimum evaluations to consider a pair "explored" (default: 3) |

Shows role leaderboard, motivation leaderboard, synergy matrix, tag breakdown, and under-explored combinations.

---

### `wg role`

Manage roles — the "what" of agent identity.

| Command | Description |
|---------|-------------|
| `wg role add <name> --outcome <text> [--skill <spec>] [-d <text>]` | Create a new role |
| `wg role list` | List all roles |
| `wg role show <id>` | Show details of a role |
| `wg role edit <id>` | Edit a role in `$EDITOR` (re-hashes on save) |
| `wg role rm <id>` | Delete a role |
| `wg role lineage <id>` | Show evolutionary ancestry |

**Skill specifications:**
- `rust` — simple name tag
- `coding:file:///path/to/style.md` — load content from file
- `review:https://example.com/checklist.md` — fetch from URL
- `tone:inline:Write in a clear, technical style` — inline content

---

### `wg motivation`

Manage motivations — the "why" of agent identity. Also aliased as `wg mot`.

| Command | Description |
|---------|-------------|
| `wg motivation add <name> --accept <text> --reject <text> [-d <text>]` | Create a new motivation |
| `wg motivation list` | List all motivations |
| `wg motivation show <id>` | Show details |
| `wg motivation edit <id>` | Edit in `$EDITOR` (re-hashes on save) |
| `wg motivation rm <id>` | Delete a motivation |
| `wg motivation lineage <id>` | Show evolutionary ancestry |

---

### `wg agent create`

Create a new agent (role + motivation pairing).

```bash
wg agent create <NAME> --role <ROLE-ID> --motivation <MOTIVATION-ID>
```

IDs can be prefixes (minimum unique match).

---

### `wg agent list|show|rm|lineage|performance`

| Command | Description |
|---------|-------------|
| `wg agent list` | List all agents |
| `wg agent show <id>` | Show agent details with resolved role/motivation |
| `wg agent rm <id>` | Remove an agent |
| `wg agent lineage <id>` | Show agent + role + motivation ancestry |
| `wg agent performance <id>` | Show evaluation history for an agent |

---

### `wg evaluate`

Trigger evaluation of a completed task.

```bash
wg evaluate <TASK> [--evaluator-model <MODEL>] [--dry-run]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--evaluator-model <MODEL>` | Model for the evaluator (overrides config) |
| `--dry-run` | Show the evaluator prompt without executing |

The task must be done, pending-review, or failed. Spawns an evaluator agent that scores the task across four dimensions:
- **correctness** (40%) — output matches desired outcome
- **completeness** (30%) — all aspects addressed
- **efficiency** (15%) — no unnecessary steps
- **style_adherence** (15%) — project conventions and constraints followed

Scores propagate to the agent, role, and motivation performance records.

---

### `wg evolve`

Trigger an evolution cycle to improve roles and motivations based on performance data.

```bash
wg evolve [--strategy <STRATEGY>] [--budget <N>] [--model <MODEL>] [--dry-run]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--strategy <name>` | Evolution strategy (default: `all`) |
| `--budget <N>` | Maximum number of operations to apply |
| `--model <MODEL>` | LLM model for the evolver agent |
| `--dry-run` | Show the evolver prompt without executing |

**Strategies:**
| Strategy | Description |
|----------|-------------|
| `mutation` | Modify a single existing role to improve weak dimensions |
| `crossover` | Combine traits from two high-performing roles |
| `gap-analysis` | Create entirely new roles/motivations for unmet needs |
| `retirement` | Remove consistently poor-performing entities |
| `motivation-tuning` | Adjust trade-offs on existing motivations |
| `all` | Use all strategies as appropriate (default) |

---

## Agent Commands

### `wg agent run`

Run the autonomous agent loop (wake/check/work/sleep cycle).

```bash
wg agent run --actor <ACTOR> [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--actor <ACTOR>` | Actor ID for this agent (required) |
| `--once` | Run only one iteration then exit |
| `--interval <SECONDS>` | Sleep interval between iterations |
| `--max-tasks <N>` | Stop after completing N tasks |
| `--reset-state` | Reset agent state (discard saved statistics) |

---

### `wg spawn`

Spawn an agent to work on a specific task.

```bash
wg spawn <TASK> --executor <NAME> [--model <MODEL>] [--timeout <DURATION>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--executor <NAME>` | Executor to use: claude, shell, or custom config name (required) |
| `--model <MODEL>` | Model override (haiku, sonnet, opus) |
| `--timeout <DURATION>` | Timeout (e.g., 30m, 1h, 90s) |

Model selection priority: CLI `--model` > task's `.model` > `coordinator.model` > `agent.model`.

---

### `wg next`

Find the best next task for an actor.

```bash
wg next --actor <ACTOR>
```

---

### `wg exec`

Execute a task's shell command (claim + run + done/fail).

```bash
wg exec <TASK> [--actor <ACTOR>] [--dry-run]
wg exec <TASK> --set <CMD>     # set the exec command
wg exec <TASK> --clear         # clear the exec command
```

---

### `wg trajectory`

Show context-efficient task trajectory (optimal claim order).

```bash
wg trajectory <TASK> [--actor <ACTOR>]
```

---

### `wg heartbeat`

Record agent heartbeat or check for stale agents.

```bash
wg heartbeat <ACTOR>                           # record heartbeat
wg heartbeat --check [--threshold <MINUTES>]   # check for stale actors
wg heartbeat --check --agents                  # check for stale agents
```

---

### `wg agents`

List running agents (from the service registry).

```bash
wg agents [--alive] [--dead] [--working] [--idle]
```

---

### `wg kill`

Terminate running agent(s).

```bash
wg kill <AGENT-ID> [--force]   # kill single agent
wg kill --all [--force]         # kill all agents
```

---

### `wg dead-agents`

Detect and clean up dead agents.

```bash
wg dead-agents --check [--threshold <MINUTES>]  # check without modifying
wg dead-agents --cleanup [--threshold <MINUTES>] # mark dead and unclaim tasks
wg dead-agents --remove                          # remove dead agents from registry
wg dead-agents --processes                       # check if agent processes are still running
```

---

## Service Commands

### `wg service start`

Start the agent service daemon.

```bash
wg service start [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--port <PORT>` | Port for HTTP API (optional) |
| `--socket <PATH>` | Unix socket path (default: /tmp/wg-{project}.sock) |
| `--max-agents <N>` | Max parallel agents (overrides config) |
| `--executor <NAME>` | Executor for spawned agents (overrides config) |
| `--interval <SECS>` | Background poll interval in seconds (overrides config) |
| `--model <MODEL>` | Model for spawned agents (overrides config) |

---

### `wg service stop`

Stop the agent service daemon.

```bash
wg service stop [--force] [--kill-agents]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--force` | SIGKILL the daemon immediately |
| `--kill-agents` | Also kill running agents (by default they continue) |

---

### `wg service status`

Show daemon PID, uptime, agent summary, and coordinator state.

```bash
wg service status
```

---

### `wg service reload`

Re-read config.toml without restarting (or apply specific overrides).

```bash
wg service reload [--max-agents <N>] [--executor <NAME>] [--interval <SECS>] [--model <MODEL>]
```

---

### `wg service pause`

Pause the coordinator. Running agents continue, but no new agents are spawned.

```bash
wg service pause
```

---

### `wg service resume`

Resume the coordinator. Triggers an immediate tick.

```bash
wg service resume
```

---

### `wg service tick`

Run a single coordinator tick and exit (debug mode).

```bash
wg service tick [--max-agents <N>] [--executor <NAME>] [--model <MODEL>]
```

---

### `wg service install`

Generate a systemd user service file for the wg service daemon.

```bash
wg service install
```

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

---

### `wg graph`

Output the full graph data.

```bash
wg graph [--archive] [--since <DATE>] [--until <DATE>]
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
| `--format <FMT>` | Output format: dot, mermaid, ascii (default: dot) |
| `-o, --output <FILE>` | Render directly to file (requires graphviz) |

---

### `wg archive`

Archive completed tasks to a separate file.

```bash
wg archive [--dry-run] [--older <DURATION>] [--list]
```

---

### `wg reschedule`

Reschedule a task (set `not_before` timestamp).

```bash
wg reschedule <ID> [--after <HOURS>] [--at <TIMESTAMP>]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--after <HOURS>` | Hours from now until task is ready |
| `--at <TIMESTAMP>` | Specific ISO 8601 timestamp |

---

### `wg artifact`

Manage task artifacts (produced outputs).

```bash
wg artifact <TASK> [<PATH>] [--remove]
```

Without a path, lists artifacts. With a path, adds it (or removes with `--remove`).

---

### `wg config`

View or modify project configuration.

```bash
wg config [OPTIONS]
```

With no options (or `--show`), displays current configuration.

**Options:**
| Option | Description |
|--------|-------------|
| `--show` | Display current configuration |
| `--init` | Create default config file |
| `--executor <NAME>` | Set agent executor (claude, opencode, codex, shell) |
| `--model <MODEL>` | Set agent model |
| `--set-interval <SECS>` | Set agent sleep interval |
| `--max-agents <N>` | Set coordinator max agents |
| `--coordinator-interval <SECS>` | Set coordinator tick interval |
| `--poll-interval <SECS>` | Set service daemon background poll interval |
| `--coordinator-executor <NAME>` | Set coordinator executor |
| `--auto-evaluate <BOOL>` | Enable/disable automatic evaluation |
| `--auto-assign <BOOL>` | Enable/disable automatic identity assignment |
| `--assigner-model <MODEL>` | Set model for assigner agents |
| `--evaluator-model <MODEL>` | Set model for evaluator agents |
| `--evolver-model <MODEL>` | Set model for evolver agents |
| `--assigner-agent <HASH>` | Set assigner agent (content-hash) |
| `--evaluator-agent <HASH>` | Set evaluator agent (content-hash) |
| `--evolver-agent <HASH>` | Set evolver agent (content-hash) |
| `--retention-heuristics <TEXT>` | Set retention heuristics (prose policy for evolver) |

**Examples:**

```bash
# View config
wg config

# Set executor and model
wg config --executor claude --model opus

# Enable the full agency automation loop
wg config --auto-evaluate true --auto-assign true

# Set per-role model overrides
wg config --assigner-model haiku --evaluator-model opus --evolver-model opus
```

---

### `wg quickstart`

Print a concise cheat sheet for agent onboarding.

```bash
wg quickstart
```

---

### `wg tui`

Launch the interactive terminal dashboard.

```bash
wg tui [--refresh-rate <MS>]
```

Default refresh rate: 2000ms.

---

## Global Options

All commands support these options:

| Option | Description |
|--------|-------------|
| `--dir <PATH>` | Workgraph directory (default: .workgraph) |
| `--json` | Output as JSON |
| `-h, --help` | Show help (use `--help-all` for full command list) |
| `--help-all` | Show all commands in help output |
| `-a, --alphabetical` | Sort help output alphabetically |
| `-V, --version` | Show version |
