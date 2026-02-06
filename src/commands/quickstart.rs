use anyhow::Result;

const QUICKSTART_TEXT: &str = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                         WORKGRAPH AGENT QUICKSTART                           ║
╚══════════════════════════════════════════════════════════════════════════════╝

⚠ COORDINATOR SERVICE REMINDER ⚠
─────────────────────────────────────────
  Check if the coordinator is running:  wg service status

  If it IS running, your job is to DEFINE work, not DISPATCH it.
  Add tasks and dependencies — the coordinator handles the rest.
  Never manually 'wg spawn' or 'wg claim' while the service is running;
  you'll collide with the coordinator and get 'already claimed' errors.

  If it is NOT running, choose a mode below.

SERVICE MODE (recommended for parallel work)
─────────────────────────────────────────
  wg service start --max-agents 5  # Start coordinator with parallelism limit

  The coordinator automatically spawns agents on ready tasks. Just add tasks:

  wg add "Do the thing" --blocked-by prerequisite-task

  Monitor with wg agents and wg list. Do NOT manually wg spawn or wg claim —
  the coordinator handles this.

  wg service status           # Check if running, see last tick
  wg agents                   # Who's working on what
  wg list                     # What's done, what's pending
  wg tui                      # Interactive dashboard

MANUAL MODE (no service running)
─────────────────────────────────────────
  wg ready                    # See tasks available to work on
  wg claim <task-id>          # Claim a task (sets status to in-progress)
  wg log <task-id> "message"  # Log progress as you work
  wg done <task-id>           # Mark task complete (use 'wg submit' if verified)

DISCOVERING & ADDING WORK
─────────────────────────────────────────
  wg list                     # List all tasks
  wg list --status open       # Filter by status (open, in-progress, done, etc.)
  wg show <task-id>           # View task details and context
  wg add "Title" -d "Desc"    # Add new task
  wg add "X" --blocked-by Y   # Add task blocked by another

TASK STATE COMMANDS
─────────────────────────────────────────
  wg done <task-id>           # Complete (non-verified tasks)
  wg submit <task-id>         # Submit for review (verified tasks)
  wg fail <task-id> --reason  # Mark failed (can be retried)
  wg abandon <task-id>        # Give up permanently

CONTEXT & ARTIFACTS
─────────────────────────────────────────
  wg context <task-id>        # See context from dependencies
  wg artifact <task-id> path  # Record output file/artifact
  wg log <task-id> --list     # View task's progress log

TIPS
─────────────────────────────────────────
• If the coordinator is running: add tasks → it dispatches automatically
• If no coordinator: ready → claim → work → done
• Run 'wg log' BEFORE starting work to track progress
• Use 'wg context' to understand what dependencies produced
• If 'wg done' fails, the task may require verification — use 'wg submit'
• Check 'wg blocked <task-id>' if a task isn't appearing in ready list
"#;

pub fn run(json: bool) -> Result<()> {
    if json {
        let output = serde_json::json!({
            "modes": {
                "service": {
                    "description": "Recommended for parallel work. Coordinator dispatches automatically.",
                    "start": "wg service start --max-agents 5",
                    "workflow": "Add tasks with dependencies → coordinator spawns agents on ready tasks",
                    "warning": "Do NOT manually wg spawn or wg claim while the service is running",
                    "monitor": ["wg service status", "wg agents", "wg list", "wg tui"]
                },
                "manual": {
                    "description": "For when no service is running. You claim and work tasks yourself.",
                    "workflow": ["wg ready", "wg claim <task-id>", "wg log <task-id> \"msg\"", "wg done <task-id>"]
                }
            },
            "commands": {
                "discovery": {
                    "list": "List all tasks",
                    "show": "View task details and context",
                    "add": "Add a new task",
                    "ready": "See tasks available to work on (manual mode)"
                },
                "work": {
                    "claim": "Claim a task for work (manual mode only)",
                    "log": "Log progress as you work",
                    "context": "See context from dependencies",
                    "artifact": "Record output file/artifact"
                },
                "completion": {
                    "done": "Mark task complete (non-verified)",
                    "submit": "Submit for review (verified tasks)",
                    "fail": "Mark failed (can be retried)",
                    "abandon": "Give up permanently"
                }
            },
            "tips": [
                "If the coordinator is running: add tasks with dependencies, it dispatches automatically",
                "If no coordinator: ready → claim → work → done",
                "Run 'wg log' BEFORE starting work to track progress",
                "Use 'wg context' to understand what dependencies produced",
                "If 'wg done' fails, the task may require verification - use 'wg submit'",
                "Check 'wg blocked <task-id>' if a task isn't appearing in ready list"
            ]
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", QUICKSTART_TEXT.trim());
    }

    Ok(())
}
