# workgraph

Structured task coordination for complex work. Use this when you need to break down, track, and execute multi-step projects.

## When to use workgraph

- Projects with multiple dependent tasks
- Work that spans multiple sessions
- Coordinating between humans and AI agents
- Anything where you need to track "what's done, what's next, what's blocked"

## Quick check

```bash
wg ready    # what can I work on?
wg list     # all tasks
wg analyze  # project health
```

## The protocol

### Starting work

1. Check what's available:
   ```bash
   wg ready
   ```

2. Claim a task:
   ```bash
   wg claim <task-id> --actor claude
   ```

3. Understand the task:
   ```bash
   wg show <task-id>      # full details
   wg context <task-id>   # inputs from dependencies
   ```

### While working

Log progress (helps with context recovery if interrupted):
```bash
wg log <task-id> "Completed X, now working on Y"
```

If you produce output files:
```bash
wg artifact <task-id> path/to/output
```

### Finishing

Success:
```bash
wg done <task-id>
```

Failed (can retry later):
```bash
wg fail <task-id> --reason "why it failed"
```

Need to stop mid-task:
```bash
wg unclaim <task-id>
```

### Discovering new work

Add tasks as you discover them:
```bash
wg add "New task title" --blocked-by current-task
```

Check impact:
```bash
wg impact <task-id>  # what depends on this?
```

## Planning work

Break down a goal:
```bash
wg add "Goal: Ship the feature"
wg add "Design the API"
wg add "Implement backend" --blocked-by design-the-api
wg add "Write tests" --blocked-by implement-backend
wg add "Update docs" --blocked-by implement-backend
```

Add metadata:
```bash
wg add "Complex task" \
  --hours 4 \
  --skill rust \
  --deliverable src/feature.rs \
  --blocked-by prerequisite-task
```

Check the plan:
```bash
wg critical-path  # longest chain
wg bottlenecks    # what to prioritize
wg forecast       # when will it be done?
```

## Analysis commands

| Command | What it tells you |
|---------|-------------------|
| `wg ready` | Tasks you can work on now |
| `wg list` | All tasks with status |
| `wg show <id>` | Full task details |
| `wg why-blocked <id>` | Why can't this start? |
| `wg impact <id>` | What depends on this? |
| `wg bottlenecks` | Highest-impact tasks |
| `wg critical-path` | Longest dependency chain |
| `wg forecast` | Completion estimate |
| `wg analyze` | Full health report |
| `wg context <id>` | Available inputs |
| `wg trajectory <id>` | Optimal claim order |

## Key behaviors

1. **Always claim before working** - prevents conflicts with other agents
2. **Log as you go** - helps recovery if interrupted
3. **Mark done immediately** - unblocks dependent tasks
4. **Add tasks as you discover them** - keep the graph current
5. **Check `wg ready` after completing** - see what's unblocked

## Multi-agent coordination

If multiple agents are working:
- Claims are atomic (no double-work)
- Use `wg coordinate` to see parallel opportunities
- Each agent should have a unique actor ID

## All commands

```
wg init              # start a workgraph
wg add <title>       # create task
wg done <id>         # complete task
wg fail <id>         # mark failed
wg abandon <id>      # give up on task
wg retry <id>        # retry failed task
wg claim <id>        # take a task
wg unclaim <id>      # release a task
wg reclaim <id>      # take from dead agent
wg log <id> <msg>    # add progress note
wg show <id>         # task details
wg list              # all tasks
wg ready             # available tasks
wg blocked <id>      # direct blockers
wg why-blocked <id>  # full blocker chain
wg impact <id>       # dependents
wg context <id>      # available inputs
wg trajectory <id>   # optimal claim order
wg bottlenecks       # high-impact tasks
wg critical-path     # longest chain
wg forecast          # completion estimate
wg velocity          # completion rate
wg aging             # task age distribution
wg workload          # actor assignments
wg analyze           # health report
wg actor add <id>    # register actor
wg actor list        # list actors
wg artifact <id> <p> # record output
wg exec <id>         # run task command
wg agent --actor <x> # autonomous loop
wg config            # view/set config
```

All commands support `--json` for structured output.
