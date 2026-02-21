= The Task Graph <sec-task-graph>

Work is structure. A project without structure is a list—and lists lie. They hide the fact that you cannot deploy before you test, cannot test before you build, cannot build before you design. A list says "here are things to do." A graph says "here is the order in which reality permits you to do them."

Workgraph models work as a directed graph. Tasks are nodes. Dependencies are edges. The graph is the single source of truth for what exists, what depends on what, and what is available for execution right now. Everything else—the coordinator, the agency, the evolution system—reads from this graph and writes back to it. The graph is not a view of the project. It _is_ the project.

== Tasks as Nodes

A task is the atom of work. It has an identity, a lifecycle, and a body of metadata that guides both human and machine execution. Here is the anatomy:

#figure(
  table(
    columns: (auto, auto),
    align: (left, left),
    stroke: 0.5pt,
    [*Field*], [*Purpose*],
    [`id`], [A slug derived from the title at creation time. The permanent key—used in every edge, every command, every reference. Once set, it never changes.],
    [`title`], [Human-readable name. Can be updated without breaking references.],
    [`description`], [The body: acceptance criteria, context, constraints. What an agent (human or AI) needs to understand the work.],
    [`status`], [Lifecycle state. One of six values—see below.],
    [`estimate`], [Optional cost and hours. Used by budget fitting and forecasting.],
    [`tags`], [Flat labels for filtering and grouping.],
    [`skills`], [Required capabilities—matched against agent capabilities at dispatch time.],
    [`inputs`], [Paths or references the task needs to read.],
    [`deliverables`], [Expected outputs—what the task should produce.],
    [`artifacts`], [Actual outputs recorded after completion.],
    [`exec`], [A shell command for automated execution via the shell executor.],
    [`model`], [Preferred AI model (haiku, sonnet, opus). Overrides coordinator and agent defaults.],
    [`verify`], [Verification criteria—if set, the task requires review before it can be marked done.],
    [`agent`], [Content-hash ID binding an agency agent identity to this task.],
    [`visibility`], [Controls what information crosses organizational boundaries during trace exports. One of `internal` (default—organization only), `public` (sanitized sharing without agent output or logs), or `peer` (richer detail for trusted peers, including evaluations and patterns).],
    [`log`], [Append-only progress entries with timestamps and optional actor attribution.],
  ),
  caption: [Task fields. Every field except `id`, `title`, and `status` is optional.],
) <task-fields>

Tasks are not just descriptions of work—they are self-contained dispatch packets. An agent spawned for a task receives the description, the inputs, the skills, the log history, and the artifacts of completed dependencies. Everything needed to begin work is encoded on the node itself or reachable through its edges.

== Status and Lifecycle

A task moves through six statuses. Most follow the happy path; some take detours.

#figure(
  raw(block: true, lang: none,
"
         ┌──────────────────────────────────────┐
         │              Open                     │
         │   (available for work or re-work)     │
         └──────┬──────────────────▲─────────────┘
                │                  │
           claim│             retry│ / loop re-activation
                │                  │
         ┌──────▼──────────────────┴─────────────┐
         │           InProgress                   │
         │        (agent working)                 │
         └──────┬─────────┬──────────┬───────────┘
                │         │          │
           done │    fail │     abandon│
                │         │          │
         ┌──────▼───┐ ┌──▼──────┐ ┌─▼──────────┐
         │   Done   │ │ Failed  │ │ Abandoned   │
         │ terminal │ │terminal │ │  terminal   │
         └──────────┘ └─────────┘ └─────────────┘

         ┌──────────────────────────────────────┐
         │  Blocked (explicit, rarely used)      │
         └──────────────────────────────────────┘
"),
  caption: [Task state machine. The three terminal statuses share a critical property: they all unblock dependents.],
) <state-machine>

*Open* is the starting state. A task is open when it has been created and is potentially available for work—though it may not yet be _ready_ (a distinction explored below).

*InProgress* means an agent has claimed the task and is working on it. The coordinator sets this atomically before spawning the agent process.

*Done*, *Failed*, and *Abandoned* are the three _terminal_ statuses. A terminal task will not progress further without explicit intervention—retry, manual re-open, or loop re-activation. The crucial design choice: all three terminal statuses unblock dependents. A failed upstream does not freeze the graph. The downstream task gets dispatched and can decide for itself what to do about a failed dependency—inspect the failure reason, skip the work, or adapt.

*Blocked* exists as an explicit status but is rarely used. In practice, blocking is _derived_ from dependencies, not declared. A task whose `blocked_by` list contains non-terminal entries is blocked in the derived sense, regardless of its status field. The explicit `Blocked` status is a manual override for cases where a human wants to freeze a task for reasons outside the graph.

== Terminal Statuses Unblock: A Design Choice

This merits emphasis. In many task systems, a failed dependency blocks everything downstream until a human intervenes. Workgraph takes the opposite stance: failure is information, not obstruction.

When task A fails and task B depends on A, B becomes ready. B's agent receives context from A—the failure reason, the log entries, the artifacts (if any). The agent can then decide: retry the work itself, produce a partial result, or fail explicitly with its own reason. The graph keeps moving.

This works because terminal means "this task has reached an endpoint for this iteration." Done is a successful endpoint. Failed is an unsuccessful one. Abandoned is a deliberate withdrawal. In all three cases, the task is no longer going to change, so dependents can proceed with whatever information is available.

The alternative—frozen pipelines waiting for human intervention—violates the principle that the graph should be self-advancing. If you need a hard stop on failure, model it explicitly: add a guard condition or a verification step. Don't rely on the scheduler to enforce business logic through status propagation.

== Dependencies: `blocked_by` and `blocks`

Dependencies are directed edges. Task B depends on task A means: B cannot be ready until A reaches a terminal status. This is expressed by placing A's ID in B's `blocked_by` list.

#figure(
  raw(block: true, lang: none,
"
    blocked_by edge (authoritative)
    ─────────────────────────────►

    ┌─────────┐  blocked_by  ┌─────────┐  blocked_by  ┌─────────┐
    │ design  │◄─────────────│  build  │◄─────────────│  deploy  │
    └─────────┘              └─────────┘              └─────────┘

    Read as: build is blocked by design. deploy is blocked by build.
    Equivalently: design blocks build. build blocks deploy.
"),
  caption: [Dependency edges. `blocked_by` is authoritative; `blocks` is its computed inverse.],
) <dependency-edges>

The `blocked_by` list is the source of truth. The `blocks` list is its inverse, maintained for bidirectional traversal—if B is blocked by A, then A's `blocks` list includes B. The scheduler never reads `blocks`; it only checks `blocked_by`. The inverse is a convenience index for commands like `wg impact` and `wg bottlenecks` that need to traverse the graph forward from a task to its dependents.

Transitivity works naturally. If C is blocked by B and B is blocked by A, then C cannot be ready while A is non-terminal, because B cannot be ready (and thus cannot become terminal) while A is non-terminal. No transitive closure computation is needed—the scheduler checks each task's immediate blockers, and the chain resolves itself one link at a time.

A subtlety: if a task references a blocker that does not exist in the graph, the missing reference is treated as resolved. This is a fail-open design—a dangling reference does not freeze the graph. The `wg check` command flags these as warnings, but the scheduler proceeds.

== Readiness

A task is _ready_ when four conditions hold simultaneously:

+ *Open status.* The task must be in the `Open` state. Tasks that are in-progress, done, failed, abandoned, or explicitly blocked are never ready.
+ *Not paused.* The task's `paused` flag must be false. Pausing is an explicit hold—the task retains its status and all other state, but the coordinator will not dispatch it.
+ *Past time constraints.* If the task has a `not_before` timestamp, the current time must be past it. If the task has a `ready_after` timestamp (set by loop edge delays), the current time must be past that too. Invalid or missing timestamps are treated as satisfied—they do not block.
+ *All blockers terminal.* Every task ID in the `blocked_by` list must correspond to a task in a terminal status (done, failed, or abandoned). Non-existent blockers are treated as resolved.

These four conditions are evaluated by `ready_tasks()`, the function that the coordinator calls every tick to find work to dispatch. Ready is a precise, computed property—not a flag someone sets. You cannot manually mark a task as ready; you can only create the conditions under which the scheduler derives it.

The `not_before` field enables future scheduling: "do not start this task before next Monday." The `ready_after` field serves a different purpose—it is set automatically by loop edges with delays, creating pacing between loop iterations. Both are checked against the current wall-clock time.

== Loop Edges: Intentional Cycles

Workgraph is a directed graph, not a DAG. This is a deliberate design choice.

Most task systems are acyclic by construction—dependencies flow in one direction, and cycles are errors. This works for projects that execute once: design, build, test, deploy, done. But real work is often iterative. You write a draft, a reviewer reads it, you revise based on feedback, the reviewer reads again. A CI pipeline builds, tests, and if tests fail, loops back to build with fixes. A monitoring system checks, investigates, fixes, verifies, and then checks again.

These patterns are cycles, and they are not bugs. They are the structure of iterative work. Workgraph makes them first-class through _loop edges_.

=== The `loops_to` Mechanism

A loop edge is a conditional back-edge declared via the `loops_to` field on a task. It says: "when I complete, evaluate a condition—if true and iterations remain, re-open the target task upstream."

#figure(
  table(
    columns: (auto, auto),
    align: (left, left),
    stroke: 0.5pt,
    [*Field*], [*Purpose*],
    [`target`], [The task ID to re-activate. Must be upstream (earlier in the dependency chain).],
    [`guard`], [A condition that must be true for the loop to fire. Optional—if absent, the loop fires unconditionally (up to `max_iterations`).],
    [`max_iterations`], [Hard cap on how many times the target can be re-activated by this edge. Mandatory—no unbounded loops.],
    [`delay`], [Optional duration (e.g., `"30s"`, `"5m"`, `"1h"`) to wait before the re-activated target becomes ready. Sets the target's `ready_after` timestamp.],
  ),
  caption: [Loop edge fields. Every loop edge requires a target and a max_iterations cap.],
) <loop-edge-fields>

The critical property: *loop edges are not blocking edges.* They are completely separate from `blocked_by`. They do not appear in the dependency lists. The scheduler never reads them when computing readiness. They exist only as post-completion triggers—evaluated when the source task transitions to done, and ignored at all other times.

This separation is the key insight of the design. The forward dependency chain (via `blocked_by`) remains acyclic and schedulable. The backward loop edge (via `loops_to`) layers iteration on top without disturbing the scheduler.

=== Guards

A guard is a condition on a loop edge that controls whether the loop fires. Three kinds:

- *Always.* The loop fires unconditionally on every completion, up to `max_iterations`. Used for monitoring loops and fixed-iteration patterns.
- *TaskStatus.* The loop fires only if a named task has a specific status. The classic use: "loop back to writing if the review task failed." This is the mechanism for conditional retry.
- *IterationLessThan.* The loop fires only if the target's iteration count is below a threshold. Redundant with `max_iterations` in simple cases, but explicit when you want the guard condition visible in the graph data.

If no guard is specified, the loop behaves as `Always`—it fires on every completion up to the iteration cap.

=== A Review Loop, Step by Step

Consider a three-task review cycle:

#figure(
  raw(block: true, lang: none,
"
    ┌─────────────┐  blocked_by  ┌───────────────┐  blocked_by  ┌───────────────┐
    │ write-draft │◄─────────────│ review-draft  │◄─────────────│ revise-draft  │
    └─────────────┘              └───────────────┘              └───────────────┘
          ▲                                                            │
          │                    loops_to                                 │
          └────────────────────(if review failed, max 5)───────────────┘

    Downstream: ┌─────────┐
                │ publish │  blocked_by revise-draft
                └─────────┘
"),
  caption: [A review loop. Forward edges (blocked_by) are solid. The loop edge is conditional.],
) <review-loop>

The forward chain is acyclic: `write-draft` → `review-draft` → `revise-draft` → `publish`. The loop edge on `revise-draft` points back to `write-draft` with a guard: fire only if `review-draft` has status `Failed`, up to 5 iterations.

Here is the execution:

+ `write-draft` is open with no blockers—it is ready. The coordinator dispatches an agent.
+ The agent completes the draft and calls `wg done write-draft`. The task becomes terminal.
+ `review-draft` has all blockers terminal (just `write-draft`). It becomes ready. The coordinator dispatches a reviewer agent.
+ The reviewer finds problems and calls `wg fail review-draft --reason "Missing section 3"`. The task is now terminal (failed).
+ `revise-draft` has all blockers terminal (`review-draft` is failed—and failed is terminal). It becomes ready. The coordinator dispatches an agent.
+ The agent reads the failure reason from `review-draft`, revises accordingly, and calls `wg done revise-draft`.
+ On completion, the loop edge fires: the guard checks `review-draft`'s status—it is `Failed`. The iteration count on `write-draft` is 0, which is below `max_iterations` (5). The loop fires.
+ `write-draft` is re-opened: status set to `Open`, timestamps cleared, `loop_iteration` incremented to 1. A log entry records: "Re-activated by loop from revise-draft (iteration 1/5)."
+ `review-draft` is also re-opened—it is an intermediate task between the loop target (`write-draft`) and the loop source (`revise-draft`), and it was previously terminal.
+ `revise-draft` itself is re-opened—the source task re-enters the cycle.
+ `write-draft` is now open with no non-terminal blockers. The cycle begins again.

If the reviewer eventually approves (calls `wg done review-draft` instead of `wg fail`), then when `revise-draft` completes, the loop guard checks `review-draft`'s status—it is `Done`, not `Failed`. The guard condition is not met. The loop does not fire. `revise-draft` stays done. `publish` has all blockers terminal. The graph proceeds.

=== Intermediate Re-Opening

When a loop fires and re-opens its target, the system also re-opens intermediate tasks—those on the dependency path between the target and the source that were previously terminal. This ensures the entire cycle is available for re-execution, not just the target.

The source task itself is also re-opened. It was just marked done by the agent, but it is part of the loop and must execute again in the next iteration. The system sets its status back to `Open`, clears its assignment and timestamps, and sets its `loop_iteration` to match the newly re-activated target.

Strictly speaking, intermediate tasks do not need explicit re-opening to be _correct_—the scheduler would not mark them ready anyway, because their blocker (the freshly re-opened target) is no longer terminal. But re-opening them explicitly ensures their status accurately reflects the loop state, and prevents them from appearing as "done" in status reports when they are actually pending re-execution.

=== Bounded Iteration

Every loop edge must specify `max_iterations`. There are no unbounded loops. When the target's `loop_iteration` reaches the cap, the loop edge stops firing, regardless of guard conditions. The task stays done. Downstream work proceeds.

This is a safety property. A guard condition with a logic error could fire indefinitely; `max_iterations` guarantees that every cycle terminates. The cap is per-edge—if multiple loop edges point at the same target, each has its own limit.

=== Early Convergence

The iteration cap is a ceiling, not a target. In practice, iterative work often converges before the maximum is reached—a refine agent determines the output is stable, a review loop approves on the third pass instead of the fifth, a monitoring check finds the system healthy. Running all remaining iterations after convergence wastes compute and delays downstream work.

An agent signals convergence by running `wg done <task-id> --converged`. This marks the task as done and adds a `"converged"` tag. When the loop evaluator runs, it checks the source task for this tag before evaluating any loop edges. If the tag is present, all loop edges from that task are skipped—the loop does not fire, regardless of guard conditions or remaining iterations. Downstream tasks proceed immediately.

The convergence tag is durable but not permanent. Running `wg retry` on a converged task clears the tag along with resetting the task to open, so the loop can fire again if needed. This means convergence is an agent's assertion about _this_ iteration's outcome, not a permanent lock on the loop structure.

The coordinator supports this mechanism in the dispatch cycle: when rendering a prompt for a task that is the source of loop edges, it includes a note about the `--converged` flag, informing the agent that early termination is available. The agent decides—the system does not guess.

=== Loop Delays

A loop edge can specify a `delay`: a human-readable duration like `"30s"`, `"5m"`, `"1h"`, or `"1d"`. When a delayed loop fires, instead of making the target immediately ready, it sets the target's `ready_after` timestamp to `now + delay`. The scheduler will not dispatch the target until the delay has elapsed.

This creates pacing between iterations. A monitoring loop that checks system health every five minutes uses a delay of `"5m"`. A review loop that gives the author time to revise before the next review might use `"1h"`.

== Pause and Resume

Sometimes you need to stop a loop—or any task—without destroying its state. The `paused` flag provides this control.

`wg pause <task>` sets the flag. The task retains its status, its loop iteration count, its log entries—everything. But the scheduler will not dispatch it. It is invisible to `ready_tasks()`.

`wg resume <task>` clears the flag. The task re-enters the readiness calculation. If it meets all four readiness conditions, it becomes available for dispatch on the next coordinator tick.

Pausing is orthogonal to status. You can pause an open task to hold it. You can pause a task mid-loop to halt the cycle without losing iteration state. When you resume, the loop picks up where it left off.

== Emergent Patterns

The dependency graph and loop edges are the only primitives. But from these two mechanisms, several structural patterns emerge naturally.

=== Fan-Out (Map)

One task blocks several children. When the parent completes, all children become ready simultaneously and can execute in parallel.

#figure(
  raw(block: true, lang: none,
"
                  ┌──────────┐
                  │  design  │
                  └────┬─────┘
               ┌───────┼───────┐
               ▼       ▼       ▼
          ┌────────┐ ┌─────┐ ┌───────┐
          │build-ui│ │build│ │build- │
          │        │ │-api │ │worker │
          └────────┘ └─────┘ └───────┘
"),
  caption: [Fan-out: one parent unblocks parallel children.],
) <fan-out>

=== Fan-In (Reduce)

Several tasks block a single aggregator. The aggregator becomes ready only when all of its blockers are terminal.

#figure(
  raw(block: true, lang: none,
"
          ┌────────┐ ┌─────┐ ┌───────┐
          │build-ui│ │build│ │build- │
          │        │ │-api │ │worker │
          └───┬────┘ └──┬──┘ └──┬────┘
              └─────────┼───────┘
                        ▼
                  ┌───────────┐
                  │ integrate │
                  └───────────┘
"),
  caption: [Fan-in: multiple parents must all complete before the child is ready.],
) <fan-in>

Combined, fan-out and fan-in produce the _map/reduce pattern_: a coordinator task fans out parallel work, then an aggregator task fans in the results. This is not a built-in primitive. It arises naturally from the shape of the dependency edges.

=== Pipelines

A linear chain: A blocks B blocks C blocks D. Each task becomes ready only when its single predecessor completes. Pipelines are the simplest dependency structure—a sequence.

=== Review Loops

A forward chain with a loop edge, as described above. The cycle executes repeatedly until a guard condition breaks it or the iteration cap is reached. Review loops are the canonical example of intentional cycles.

=== Trace Functions: Reusable Patterns

When a workflow pattern proves useful—a review loop that consistently produces good results, a map/reduce pipeline tuned for a particular domain—it can be extracted from a completed trace into a reusable template called a _trace function_. The `wg trace extract` command takes a completed task and its subgraph, captures the task structure, dependencies, loop edges, and guards, and parameterizes the variable parts: feature names, file paths, descriptions, and thresholds become named input variables. The result is stored as YAML in `.workgraph/functions/`.

Instantiating a trace function with `wg trace instantiate` reverses the process. It takes a function name and a set of input values, substitutes them into the template, and creates concrete tasks in the graph with proper dependency wiring. The original pattern's structure is preserved—its fan-out topology, its loop bounds, its guard conditions—but applied to new work. Trace functions can also be shared across projects: the `--from` flag accepts a peer name or file path, enabling teams to import proven workflows from one another.

== Graph Analysis

Workgraph provides several analysis tools that read the graph structure and compute derived properties. These are instruments, not concepts—they report on the graph rather than define it.

*Critical path.* The longest dependency chain among active (non-terminal) tasks, measured in estimated hours. The critical path determines the minimum time to completion—no amount of parallelism can shorten it. Tasks on the critical path have zero slack; delays to any of them delay the entire project. `wg critical-path` computes this, skipping cycles to avoid infinite traversals.

*Bottlenecks.* Tasks that transitively block the most downstream work. A bottleneck is not necessarily on the critical path—it might block many short chains rather than one long one. `wg bottlenecks` ranks tasks by the count of transitive dependents, providing recommendations for tasks that should be prioritized.

*Impact.* Given a specific task, what depends on it? `wg impact <task>` traces both direct and transitive dependents, computing the total hours at risk if the task is delayed or fails.

*Cost.* The total estimated cost of a task including all its transitive dependencies, computed with cycle detection to avoid double-counting shared ancestors in diamond patterns.

*Forecast.* Projected completion date based on remaining work, estimated velocity, and dependency structure.

*Visualization.* `wg viz` renders the graph as text. The `--graph` format produces a 2D spatial layout using Unicode box-drawing characters, positioning tasks by their dependency depth—roots at the top, leaf tasks at the bottom. Nodes are color-coded by status and connected by vertical lines that split at fan-out points and merge at fan-in points. The layout algorithm assigns layers via topological sort, then orders nodes within each layer to minimize edge crossings.

These tools share a common pattern: they traverse the graph using `blocked_by` edges (and their inverse), respect the visited-set pattern to handle cycles safely, and report on the structure without modifying it.

== Storage

The graph is stored as JSONL—one JSON object per line, one node per object. A graph file might look like this:

#figure(
  raw(block: true, lang: "jsonl",
"{\"kind\":\"task\",\"id\":\"write-draft\",\"title\":\"Write draft\",\"status\":\"open\",\"loops_to\":[]}\n{\"kind\":\"task\",\"id\":\"review-draft\",\"title\":\"Review draft\",\"status\":\"open\",\"blocked_by\":[\"write-draft\"]}\n{\"kind\":\"task\",\"id\":\"revise-draft\",\"title\":\"Revise\",\"status\":\"open\",\"blocked_by\":[\"review-draft\"],\"loops_to\":[{\"target\":\"write-draft\",\"guard\":{\"TaskStatus\":{\"task\":\"review-draft\",\"status\":\"failed\"}},\"max_iterations\":5}]}\n{\"kind\":\"task\",\"id\":\"publish\",\"title\":\"Publish\",\"status\":\"open\",\"blocked_by\":[\"revise-draft\"]}"
  ),
  caption: [A graph file in JSONL format. Each line is a self-contained node.],
) <jsonl-example>

JSONL has three virtues for this purpose. It is human-readable—you can inspect and edit it with any text editor. It is version-control-friendly—adding or modifying a task changes one line, producing clean diffs. And it supports atomic writes with file locking—concurrent processes cannot corrupt the graph because every write acquires an exclusive lock, rewrites the file, and releases.

The graph file lives at `.workgraph/graph.jsonl` and is the canonical state of the project. There is no database, no server dependency. Everything reads from and writes to this file. The service daemon, when running, holds no state beyond what the file contains—it can be killed and restarted without loss.

Alongside the graph file, the operations log (`operations.jsonl`) records every mutation: task creation, status changes, dependency additions, loop firings, evaluations. This log is the project's trace—its organizational memory. The `wg trace` command queries it. `wg trace export` produces a filtered, shareable snapshot with visibility controls: an `internal` export includes everything, a `public` export sanitizes (omitting agent output and logs), and a `peer` export provides richer detail for trusted collaborators. `wg trace import` ingests a peer's export, enabling cross-boundary knowledge transfer. The graph file tells you where the project _is_. The operations log tells you how it got there.

---

The task graph is the foundation. Dependencies encode the ordering constraints of reality. Loop edges encode the iterative patterns of practice. Readiness is a derived property—the scheduler's answer to "what can happen next?" The coordinator uses this answer to dispatch work, as described in the section on coordination and execution. The agency system uses the graph to record evaluations at each task boundary, as described in the section on evolution.

A well-designed task graph does not just organize work. It makes the structure of the project legible—to humans reviewing progress, to agents receiving dispatch, and to the system itself as it learns from its own history.
