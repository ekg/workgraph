# User Feedback: workgraph agent spawning

## Bug Report: Shell escaping broken in `wg spawn`

### Summary
Agent spawning via `wg spawn` fails due to shell escaping issues when the prompt template contains quotes.

### Reproduction
```bash
wg spawn formalize-multi-pass --executor claude
```

### Observed Behavior
The spawned process shows malformed command with broken quote escaping:

```bash
bash -c echo 'You are working on task: formalize-multi-pass...
  ...tanh'\''(S) → 0...
  ...Mamba2'\''s linear state decays...
' | 'claude' '--print' '--dangerously-skip-permissions'
```

The shell is trying to escape single quotes inside single quotes using `'\''` pattern, but this breaks the pipe to claude. The process hangs in "starting" state indefinitely with empty output logs.

### Expected Behavior
The prompt should be correctly passed to the claude CLI, either via:
- A temp file that gets read as input
- Proper escaping using `$'...'` syntax or heredoc
- Base64 encoding/decoding

### Root Cause
In the executor, the command construction appears to be:
```
echo '<prompt>' | claude --print ...
```

When `<prompt>` contains single quotes (e.g., `tanh'(S)`), the shell escaping breaks.

### Suggested Fix
Use a heredoc or temp file approach:
```bash
claude --print --dangerously-skip-permissions <<'EOF'
<prompt content here>
EOF
```

Or write prompt to a temp file and use `cat`:
```bash
cat /tmp/prompt-$TASK_ID.txt | claude --print ...
```

---

## Feature Request: Model selection for agents

### Summary
Allow users to specify which model (haiku, sonnet, opus) to use per task or executor, enabling cost optimization.

### Motivation
- **Haiku** is fast and cheap - good for simple tasks like file updates, linting, formatting
- **Sonnet** is balanced - good for most coding tasks
- **Opus** is most capable - needed for complex proofs, architecture decisions

Currently all spawned agents use the default model, which wastes budget on simple tasks.

### Suggested Implementation

**Option 1: Per-executor model config**
```toml
# .workgraph/executors/claude-haiku.toml
[executor]
type = "claude"
command = "claude"
args = ["--print", "--dangerously-skip-permissions", "--model", "haiku"]
```

**Option 2: Per-task model override**
```bash
wg spawn my-task --executor claude --model haiku
```

**Option 3: Task metadata**
```bash
wg add "Simple formatting task" --model haiku
# Then spawn respects the task's model preference
```

### Use Cases
- `update-lean-file` (updating references) → haiku
- `narrative-arc-and` (creative writing) → sonnet or opus
- `formalize-multi-pass` (Lean proofs) → opus

This could easily save 50-80% on agent costs for typical workgraphs with mixed task complexity.

---

## Environment
- workgraph location: ~/workgraph
- OS: Linux 6.8.0-84-generic
- Date: 2026-01-31
