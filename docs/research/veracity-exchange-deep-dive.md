# Veracity Exchange × Workgraph: Deep Dive

## Executive Summary

Veracity Exchange is a marketplace for private participant repositories (datasets, signals, research) that uses internal markets to objectively measure each participant's marginal value contribution. Workgraph is a task coordination system with composable agent identities, provenance logging, and performance evaluation. This report analyzes how these systems could integrate, answering seven specific research questions about protocol mappings, information boundaries, agent definitions as market goods, and minimizing the need for forking.

**Key finding:** The integration is architecturally natural. Workgraph already has the primitives—task provenance, artifact tracking, agent evaluation, and directed dependency graphs—that map onto Veracity's concepts of portfolio positions, scoring, and information flow control. The gap is primarily in metadata richness (tasks need public/private classification, portfolio position mappings) and in post-completion hooks (no plugin system exists to call `run-day` automatically). Both can be addressed with extensions rather than core forks.

---

## 1. What Veracity Exchange Is

### Core Model

Veracity Exchange operates as a data marketplace where participants contribute private repositories containing datasets, signals, or research. The system:

1. **Converts** participant information into daily investment portfolios/forecasts through OpenClaw
2. **Scores** portfolios using objective measures: P&L (profit and loss) and MSE (mean squared error) on predictions
3. **Attributes** value using internal markets that measure each participant's *marginal* contribution—not just whether they were right, but whether their signal added information beyond what the market already had

### Onboarding Protocol

A 6-step interview protocol structures participant onboarding:

| Step | Purpose | Deliverable |
|------|---------|-------------|
| 1. Offer mapping | What's being exchanged, market scope, timing | `candidate_profile.md` |
| 2. Claim decomposition | Break assertions into testable inputs | Part of profile |
| 3. Data inventory | Sources, latency, quality, access constraints | `data_inventory.csv` |
| 4. Signal hypotheses | Translate claims into directional predictions | `edge_hypotheses.md` |
| 5. Risk/failure modes | Map hypothesis breaks, data drift, confounders | Part of hypotheses |
| 6. Packaging | Repo structure and daily execution workflow | `participant_repo_plan.md` |

### API Surface

Two endpoints:

- `GET /veracity/v1/healthz` — connectivity check
- `POST /veracity/v1/run-day` — submit a portfolio (positions with opening/closing prices), receive scoring

### Agent-Native Design

Veracity is explicitly designed for AI agent participation. It ships with:

- `CLAUDE.md` — directs to AGENTS.md for full agent instructions
- `AGENTS.md` — same content as `agent-guide.md`, covering the full interview protocol
- `agent-guide.md` — deployed worker version of the guide
- `interview-template.json` — structured scaffold for the onboarding interview

The deliverables are text-first and deterministic: every claim must link to an observable signal with measurable criteria. This is a good fit for workgraph's evaluation system, which also demands concrete, measurable outcomes.

---

## 2. Research Questions

### 2.1 Replay + Scoring: Provenance → Veracity Portfolio Positions

**Question:** How would wg's provenance system enable replaying a workflow and scoring each sub-unit via Veracity's API? What's the mapping from wg task → veracity portfolio position?

#### Current State of wg Provenance

The provenance system (`src/provenance.rs`) maintains an append-only operation log at `.workgraph/log/operations.jsonl` with automatic rotation at 10MB (compressed to `.jsonl.zst`). Each entry has:

```rust
pub struct OperationEntry {
    pub timestamp: String,      // RFC 3339
    pub op: String,             // "add_task", "done", "fail"
    pub task_id: Option<String>,
    pub actor: Option<String>,
    pub detail: serde_json::Value,
}
```

**Current coverage gap:** Only `add_task`, `done`, and `fail` are logged to the provenance system. Task claims, spawns, retries, evaluations, and artifact recordings only update the task's in-graph `log` field (a per-task append-only list). This means replay from provenance alone is incomplete.

#### The Mapping: Task → Portfolio Position

The conceptual mapping is:

| Workgraph concept | Veracity concept |
|-------------------|------------------|
| Task | Portfolio position |
| Task output/artifacts | Position entry (what was bet on) |
| Task completion timestamp | Position date |
| Upstream dependency outputs | Input signals used to form the position |
| `wg evaluate` score | Internal quality metric (complementary to veracity score) |
| Veracity `run-day` score | Objective external metric (P&L, MSE) |

A workflow that produces a daily forecast would decompose as:

```
data-collection (task) → signal-extraction (task) → forecast-generation (task) → portfolio-submission (task)
```

Each intermediate task produces artifacts that are the "positions" of that sub-unit. The final `portfolio-submission` task calls `POST /veracity/v1/run-day` and records the score.

#### What's Needed for Replay

To enable full replay and per-sub-unit scoring:

1. **Expand provenance coverage.** Every lifecycle event needs to be logged to the op-log, not just add/done/fail. At minimum: `claim`, `spawn`, `artifact`, `evaluate`, `retry`, `unclaim`. This is a straightforward code change—each command already records to the task-level log; it just needs a parallel `provenance::record()` call.

2. **Add input/output hashing to artifacts.** Currently artifacts are bare path strings with no content hash. For replay, each artifact needs a content hash so we can verify that replayed inputs match original inputs. Proposed extension to the `Task` struct:

   ```rust
   pub struct ArtifactRecord {
       pub path: String,
       pub content_hash: Option<String>,  // SHA-256
       pub recorded_at: String,           // RFC 3339
       pub size_bytes: Option<u64>,
   }
   ```

3. **Record veracity scores as a new evaluation dimension.** The existing evaluation system scores on correctness/completeness/efficiency/style. A `veracity_score` dimension (or a separate `ExternalScore` record) would capture the objective market score alongside the LLM-judged quality score. This creates a dual-scoring system: internal quality (did the agent do good work?) and external veracity (did the work produce accurate predictions?).

4. **Replay command.** A `wg replay <workflow-root>` command that reads the provenance log, reconstructs the DAG execution order, and re-runs each task's outputs through the veracity scoring API. This would require:
   - Reading all `OperationEntry` records for tasks in the subgraph
   - Resolving artifacts at each step (using content hashes to verify integrity)
   - Calling `run-day` for scorable positions
   - Producing a per-task scoring report

#### Concrete Protocol Mapping

For `POST /veracity/v1/run-day`, the request body includes portfolio positions with prices. The mapping from wg artifacts:

```
wg task artifacts → parse as structured data → extract positions/prices → format as run-day payload
```

This requires a convention: tasks that produce veracity-scorable output must produce artifacts in a known format (e.g., `portfolio.json` following Veracity's schema). The executor template could enforce this via deliverables:

```bash
wg add "Generate daily forecast" \
  --deliverable "portfolio.json" \
  --tag "veracity:scorable" \
  --skill forecasting
```

---

### 2.2 Public/Private Boundaries

**Question:** How would task definitions and artifacts be classified as public (shareable on the market) or private? What metadata would tasks need?

#### Current Task Metadata

Tasks currently have these relevant fields:
- `tags: Vec<String>` — free-form labels
- `inputs: Vec<String>` — declared input paths
- `deliverables: Vec<String>` — expected output paths
- `artifacts: Vec<String>` — actual produced output paths
- `description: String` — task description (potentially contains sensitive strategy details)

There is **no** arbitrary key-value metadata map (`HashMap<String, Value>`) on the Task struct. Custom metadata must be encoded into existing typed fields.

#### Proposed Classification Scheme

**Option A: Tag-based (minimal change, works today)**

Use tag conventions to classify visibility:

```bash
wg add "Analyze sentiment data" \
  --tag "visibility:public" \
  --tag "veracity:participant-id:abc123"

wg add "Proprietary signal extraction" \
  --tag "visibility:private" \
  --tag "veracity:participant-id:abc123"
```

Advantages: No code changes needed. Tags are already indexed and filterable.
Disadvantages: No enforcement. Tags are free-form strings with no validation.

**Option B: Structured visibility field (recommended)**

Add a first-class visibility field to the Task struct:

```rust
pub enum Visibility {
    Private,            // Never shared externally
    PublicDefinition,   // Task definition shareable, artifacts private
    PublicFull,         // Both definition and artifacts shareable
}

pub struct Task {
    pub visibility: Visibility,  // defaults to Private
    // ...
}
```

This provides compile-time safety and makes the public/private boundary explicit in the data model. The executor and any veracity integration hooks can check `task.visibility` before deciding what to share.

#### Artifact-Level Visibility

Some tasks may have a mix: the methodology (task definition) is public, but specific data artifacts are private. This needs per-artifact classification:

```rust
pub struct ArtifactRecord {
    pub path: String,
    pub visibility: Visibility,
    pub content_hash: Option<String>,
}
```

#### Relationship to wg's Dependency Graph

The dependency graph already creates natural information boundaries:

```
[Public: methodology] → [Private: data processing] → [Public: forecast submission]
```

Tasks marked `Private` should not have their descriptions or artifacts included in the `{{task_context}}` of downstream tasks that are `Public`. This requires a visibility-aware version of `build_task_context()` in `spawn.rs`:

```rust
fn build_task_context(task: &Task, graph: &Graph) -> String {
    for dep_id in &task.blocked_by {
        let dep = graph.get(dep_id);
        if task.visibility.is_public() && dep.visibility.is_private() {
            // Only include sanitized summary, not raw artifacts
            context.push(format!("From {}: [private dependency, summary only]", dep_id));
        } else {
            // Include full context as today
        }
    }
}
```

#### What Would Be Shared on the Market

For Nikete's vision of posting "non-sensitive prompt sections" to a public market:

1. **Task definitions** (title, description, required skills) — shareable if `visibility >= PublicDefinition`
2. **Role definitions** (skills, desired outcomes) — shareable (these are methodological, not data-specific)
3. **Evaluation scores** — shareable as credibility signals
4. **Artifacts** — only if `visibility == PublicFull`
5. **Dependency structure** — the DAG topology itself could be shared as a template, showing how work is organized without revealing content

---

### 2.3 Agent Definitions as Market Goods

**Question:** Could wg agency definitions (roles, motivations, skills) be the things traded on Veracity Exchange?

#### The Natural Fit

This is perhaps the most compelling integration point. Consider:

1. An agent (role + motivation) is assigned to forecasting tasks
2. Those tasks produce veracity-scorable portfolios
3. The agent accumulates a `PerformanceRecord` with objective veracity scores
4. The agent's role definition (its skills, desired outcome, description) becomes a *proven methodology*

The content-hash ID system makes this especially powerful:

- **Immutable identity:** Role `a3f7c21d` always refers to the exact same skill set and desired outcome. Its track record is tied to its identity.
- **Reproducibility:** Anyone who obtains role `a3f7c21d`'s definition can instantiate the same agent and expect similar behavior (modulo the underlying LLM).
- **Verifiable lineage:** The lineage system shows whether a role was evolved from a proven ancestor, adding credibility.

#### What Would Be Traded

| Market good | Veracity Exchange concept | wg representation |
|-------------|--------------------------|-------------------|
| Forecasting methodology | Participant's information product | Role definition (YAML) |
| Behavioral constraints | Quality assurance | Motivation definition (YAML) |
| Proven agent configuration | Participant with track record | Agent = Role + Motivation (YAML) + PerformanceRecord |
| Skill documents | Implementation guides | Skill references (file/URL/inline content) |
| Evolution recipes | Improvement methodology | Evolver skills (`.workgraph/agency/evolver-skills/`) |

#### Scoring Agent Definitions via Veracity

The existing `wg evaluate` scoring (correctness/completeness/efficiency/style) is *internal* quality—LLM-judged. Veracity scores (P&L, MSE) would be *external* quality—market-judged. An agent's full quality profile would combine both:

```yaml
performance:
  task_count: 50
  avg_internal_score: 0.85    # wg evaluate (LLM-judged)
  avg_veracity_score: 0.72    # run-day scoring (market-judged)
  evaluations:
    - task_id: "forecast-2026-02-15"
      internal_score: 0.88
      veracity_score: 0.75
      veracity_pnl: 1234.56
      veracity_mse: 0.0042
```

This dual scoring creates a powerful quality signal: an agent might get high internal scores (clean code, follows conventions) but low veracity scores (bad predictions), or vice versa. The evolution system (`wg evolve`) could optimize for the composite metric.

#### Market Dynamics

1. **Price discovery:** A role definition's market value is determined by its veracity track record. Role `a3f7c21d` with 0.85 avg veracity score over 100 tasks is worth more than role `b4e8f32a` with 0.60 over 10 tasks.

2. **Composition:** Buyers could combine purchased roles with their own motivations, creating new agents. The lineage system tracks this, and the new agent's performance feeds back to verify whether the purchased role transfers well.

3. **Skill markets:** Skills attached to roles (especially `File` and `Url` types) are the actual intellectual property. A role's skills might reference private documents with domain expertise. The role definition (public) points to the skill content (private until purchased).

4. **Evolution as improvement R&D:** The `wg evolve` process becomes a form of R&D investment. Evolved roles with better veracity scores justify higher market prices.

---

### 2.4 Protocol Bridge: Technical Integration

**Question:** What would the integration look like technically?

#### Option 1: Veracity Executor (Recommended)

Create a new executor type `veracity` that extends the existing executor system:

```toml
# .workgraph/executors/veracity.toml
[executor]
type = "veracity"
command = "vx"
args = ["api", "run-day"]
veracity_api_url = "https://api.veracity.exchange"
veracity_credentials = "${VX_API_KEY}"

[executor.portfolio_mapping]
artifact_format = "portfolio.json"   # expected artifact name
position_field = "positions"         # JSON path in artifact

[executor.scoring]
record_as_evaluation = true          # create wg evaluation from vx score
pnl_weight = 0.6
mse_weight = 0.4
```

The executor would:
1. Read the task's `portfolio.json` artifact
2. Format it as a `run-day` request body
3. POST to Veracity's API
4. Record the response score as a wg evaluation
5. Store the raw Veracity response as an artifact

This fits cleanly into the existing executor pattern. The `spawn.rs` wrapper script already handles post-execution status checking; the veracity executor just adds a scoring step.

#### Option 2: Post-Completion Hook

Currently, wg has **no hook system**. The wrapper script generated by `spawn.rs` is hardcoded bash. Adding a hook system would be valuable beyond just Veracity:

```toml
# .workgraph/config.toml
[hooks]
post_completion = [
    { command = "vx api run-day --portfolio {{artifact:portfolio.json}}", when = "tag:veracity:scorable" },
    { command = "notify-slack {{task_id}} {{status}}", when = "always" },
]
```

This is more flexible than a dedicated executor but requires building a hook system from scratch.

#### Option 3: Two-Phase Task Pattern (Works Today)

No code changes required. Use wg's existing dependency system:

```bash
# Phase 1: Generate forecast
wg add "Generate daily forecast for 2026-02-18" \
  --skill forecasting \
  --deliverable "portfolio.json" \
  --tag "veracity:phase:generate"

# Phase 2: Submit and score via shell executor
wg add "Submit forecast to Veracity" \
  --blocked-by "generate-daily-forecast-2026-02-18" \
  --exec "vx api run-day --input .workgraph/artifacts/portfolio.json" \
  --tag "veracity:phase:score"
```

The second task uses the `shell` executor (`--exec`) to call the `vx` CLI. This is the fastest path to a working integration but requires manual wiring for each submission cycle.

#### Recommended Architecture

A layered approach:

1. **Phase 1 (now):** Two-phase task pattern. No code changes. Validates the concept.
2. **Phase 2 (short-term):** Add a post-completion hook system to wg. Benefits Veracity and all other integrations.
3. **Phase 3 (medium-term):** Build a veracity executor that handles portfolio formatting, API calls, and score recording natively.

#### CLI Extension

The `vx` CLI already provides `vx api run-day` and `vx api health`. A wg integration could be a `wg veracity` subcommand or a standalone bridge:

```bash
wg veracity submit <task-id>           # submit task's portfolio artifact to run-day
wg veracity score <task-id>            # fetch and record veracity score
wg veracity status                     # check API health
wg veracity replay <workflow-root>     # replay and score all scorable tasks in subgraph
```

---

### 2.5 Information Flow Control

**Question:** How does wg's dependency graph naturally create information boundaries?

#### Current Information Flow

Workgraph's dependency graph is a DAG with explicit directed edges:

```rust
pub blocked_by: Vec<String>,   // upstream dependencies
pub blocks: Vec<String>,       // downstream dependents (mirror)
```

Information flows downstream through `build_task_context()` in `spawn.rs`: when a task is spawned, it receives context from its direct `blocked_by` dependencies—specifically their artifacts and last 5 log entries.

Key properties:
- **Directed:** Information only flows from upstream to downstream
- **Scoped:** Only *direct* dependencies contribute context (not transitive)
- **Artifact-mediated:** The mechanism is through artifact paths and log entries, not raw task state

#### Mapping to Veracity's Marketplace Model

Nikete's insight is that the DAG structure already *is* an information flow control system. Making it explicit for Veracity:

```
┌─────────────────────┐     ┌──────────────────────┐     ┌───────────────────┐
│  PUBLIC              │     │  PRIVATE               │     │  PUBLIC             │
│  Market data         │────>│  Proprietary signal    │────>│  Portfolio          │
│  collection          │     │  extraction            │     │  submission         │
│                      │     │                        │     │                     │
│  Methodology visible │     │  Artifacts hidden      │     │  Results scored     │
│  on market           │     │  from market           │     │  by veracity        │
└─────────────────────┘     └──────────────────────┘     └───────────────────┘
```

The DAG edges define what each node knows. A public node downstream of a private node can share its *outputs* without revealing the private node's *method*.

#### What Needs to Change

1. **Visibility-aware context building.** As described in §2.2, `build_task_context()` needs to respect visibility boundaries. When a public task depends on a private task, it should receive sanitized context.

2. **Transitive visibility inference.** If a task is public but all its dependencies are private, the task's outputs are effectively derived from private data. The system should warn or enforce:

   ```
   WARNING: Task "portfolio-submission" is public but depends on private task
   "proprietary-signal-extraction". Outputs may leak private information.
   ```

3. **Export control.** A `wg export <task-id> --for-market` command that produces a sanitized view of a task and its public subgraph, suitable for posting to Veracity Exchange's marketplace.

4. **Monitoring what leaves your node.** Nikete mentions "monitor what data leaves your node." This maps to an audit log of all information shared externally:

   ```rust
   pub struct ExportEntry {
       pub timestamp: String,
       pub task_id: String,
       pub destination: String,      // "veracity-market", "peer:alice"
       pub content_type: String,     // "task-definition", "artifact", "score"
       pub content_hash: String,     // what was shared
   }
   ```

   Stored in the provenance log with op `"export"`.

---

### 2.6 Forking vs. Extension

**Question:** What changes truly need wg core changes vs. what can be extensions?

#### What Can Be Done Without Forking

| Capability | Mechanism | Why no fork needed |
|-----------|-----------|-------------------|
| Two-phase task pattern | Existing `--exec` + `--blocked-by` | Works today, no changes |
| Tag-based visibility | Existing `--tag` field | Convention-only, no enforcement |
| Veracity scoring via shell tasks | `--exec "vx api run-day ..."` | Shell executor exists |
| Custom executor config | `.workgraph/executors/veracity.toml` | Executor system is extensible |
| Agent definitions as templates | Export/import YAML files | Standard file operations |
| Skill content from URLs | `--skill "name:https://..."` | URL skill type exists |

#### What Requires Core Changes (but not forking)

These are additive features that could be contributed upstream:

| Feature | Scope | Difficulty |
|---------|-------|-----------|
| Post-completion hooks | New `[hooks]` config section + hook runner in `spawn.rs` wrapper | Medium |
| Structured artifact records | Change `Vec<String>` → `Vec<ArtifactRecord>` in Task struct | Medium (migration needed) |
| Visibility field on tasks | Add `visibility: Visibility` to Task struct | Small |
| Expanded provenance logging | Add `provenance::record()` calls to existing commands | Small |
| External score in evaluations | Add `external_scores: HashMap<String, f64>` to Evaluation | Small |
| Arbitrary task metadata | Add `metadata: HashMap<String, Value>` to Task struct | Small |

#### What Might Require Forking

If Nikete's vision diverges significantly from wg's core assumptions:

| Scenario | Why fork might be needed |
|----------|------------------------|
| Fundamentally different task lifecycle | If veracity tasks have states beyond Open/InProgress/Done/Failed |
| Real-time scoring integration | If tasks need continuous scoring during execution (not just post-completion) |
| Peer-to-peer DAG sharing | If the graph itself needs to be distributed across participants |
| Cryptographic verification | If task outputs need zero-knowledge proofs of correctness |

**Recommendation:** Design interfaces for all the "core changes" items above and propose them as PRs to upstream wg. If accepted, no fork is needed. If the interface design requires changing wg's fundamental assumptions (e.g., tasks are local, graphs are single-owner), then fork specific modules while keeping compatibility with the core task lifecycle.

#### Minimizing Fork Surface

The key principle: **extend, don't modify.** Specifically:

1. **New fields, not changed fields.** Add `visibility`, `external_scores`, `metadata` as new optional fields. Never change the semantics of existing fields.

2. **New executors, not modified executors.** The veracity executor is a new type alongside `claude`, `shell`, and `default`. No changes to existing executor logic.

3. **New commands, not modified commands.** `wg veracity submit`, `wg veracity score`, `wg export` are new subcommands. Existing commands (`wg done`, `wg evaluate`) continue to work unchanged.

4. **Hook system as middleware.** Hooks wrap existing behavior rather than replacing it. `post_completion` hooks run *after* the existing wrapper script logic, not instead of it.

---

### 2.7 Comparison with Veracity's Existing Agent Setup

**Question:** How do Veracity's CLAUDE.md, AGENTS.md, and agent-guide.md relate to wg's agency system?

#### Veracity's Agent System

Veracity's agent setup is **prompt-centric**: static markdown files that instruct a Claude agent on how to conduct the interview protocol. The files are:

| File | Purpose | Content |
|------|---------|---------|
| `CLAUDE.md` | Entry point | Points to AGENTS.md |
| `AGENTS.md` | Full guide | Complete interview protocol, deliverables, quality standards |
| `agent-guide.md` | Deployed version | Same as AGENTS.md (for worker agents) |
| `interview-template.json` | Scaffold | Structured template for interview data |

The agent is expected to:
1. Conduct a 6-step interview
2. Produce 4 deliverables (`candidate_profile.md`, `data_inventory.csv`, `edge_hypotheses.md`, `participant_repo_plan.md`)
3. Follow quality standards (text-first, deterministic, measurable)
4. Use the API (`healthz`, `run-day`) for validation

#### wg's Agency System

Workgraph's agency system is **identity-centric**: agents have composable identities (role + motivation) that shape behavior, with performance tracking and evolution. Key structural differences:

| Dimension | Veracity agents | wg agents |
|-----------|----------------|-----------|
| Identity | Single static role per repo | Composable role + motivation |
| Configuration | Markdown files in repo root | YAML files in `.workgraph/agency/` |
| Behavior shaping | Full prompt in CLAUDE.md | Role/motivation injected into executor prompt |
| Evaluation | Portfolio P&L, MSE (external) | 4-dimension LLM evaluation (internal) |
| Evolution | Manual guide updates | Automated evolution via `wg evolve` |
| Task awareness | Single ongoing task (daily run) | Multi-task DAG with dependencies |
| Skill system | None (all in one guide) | Typed references (name, file, URL, inline) |

#### Overlap and Complementarity

**Overlap:**
- Both define "what the agent should do" (Veracity's guide ≈ wg role's desired outcome + skills)
- Both have quality standards (Veracity's "measurable criteria" ≈ wg's `verify` field and evaluation dimensions)
- Both produce structured deliverables (Veracity's 4 files ≈ wg's `deliverables` and `artifacts`)

**Complementarity (where each system adds what the other lacks):**

| Veracity adds to wg | wg adds to Veracity |
|---------------------|---------------------|
| Objective external scoring (P&L, MSE) | Multi-task DAG decomposition |
| Market-based value attribution | Agent identity composition (role × motivation) |
| Participant onboarding protocol | Automated performance evolution |
| Information marketplace | Information flow control via dependency graph |
| | Content-hash identity for reproducibility |
| | Lineage tracking for evolved agents |

#### Integration Design

The most natural integration maps Veracity's interview protocol to a wg workflow:

```
wg add "Interview: Offer Mapping" --skill veracity-interview --deliverable candidate_profile.md
wg add "Interview: Claim Decomposition" --blocked-by offer-mapping
wg add "Interview: Data Inventory" --blocked-by claim-decomposition --deliverable data_inventory.csv
wg add "Interview: Signal Hypotheses" --blocked-by data-inventory --deliverable edge_hypotheses.md
wg add "Interview: Risk Analysis" --blocked-by signal-hypotheses
wg add "Interview: Packaging" --blocked-by risk-analysis --deliverable participant_repo_plan.md
```

The Veracity `agent-guide.md` content becomes a wg skill:

```bash
wg role add "Veracity Interviewer" \
  --skill "interview-protocol:https://www.veracity.exchange/agent-guide.md" \
  --outcome "Complete participant onboarding with 4 deliverables" \
  --description "Conducts structured interviews to onboard participants to Veracity Exchange"
```

This gives us the best of both worlds:
- Veracity's domain expertise (the interview protocol, scoring) flows in as skill content
- wg's operational machinery (DAG coordination, agent evaluation, evolution) manages execution
- Performance data from both systems feeds into a unified agent quality profile

---

## 3. Proposed Integration Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                     Workgraph Core                            │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Task DAG │  │ Agency   │  │Provenance│  │ Executor │    │
│  │          │  │ System   │  │   Log    │  │ System   │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │              │              │              │          │
│       └──────────────┼──────────────┼──────────────┘          │
│                      │              │                         │
│              ┌───────┴───────┐      │                         │
│              │  Veracity     │      │                         │
│              │  Integration  │      │                         │
│              │  Layer        │◄─────┘                         │
│              └───────┬───────┘                                │
│                      │                                        │
└──────────────────────┼────────────────────────────────────────┘
                       │
              ┌────────┴────────┐
              │  Veracity       │
              │  Exchange API   │
              │  /v1/run-day    │
              └─────────────────┘
```

### Integration Layer Components

1. **Veracity Executor** — handles portfolio submission and scoring
2. **Visibility Module** — enforces public/private boundaries
3. **Score Bridge** — maps Veracity scores to wg evaluations
4. **Export Module** — prepares task/agent data for marketplace posting
5. **Replay Engine** — reconstructs and re-scores historical workflows

### Data Flow for a Daily Forecast Cycle

```
1. Coordinator dispatches "collect-data" task
   → Agent collects market data → artifacts: raw-data.csv
   → Provenance: op="done", detail={artifacts: ["raw-data.csv"]}

2. Coordinator dispatches "extract-signals" task (blocked-by collect-data)
   → Agent reads raw-data.csv from upstream context
   → Produces signals.json → visibility: private
   → Provenance: op="done", detail={artifacts: ["signals.json"]}

3. Coordinator dispatches "generate-forecast" task (blocked-by extract-signals)
   → Agent reads signals.json from upstream context
   → Produces portfolio.json → visibility: public (for scoring)
   → Provenance: op="done", detail={artifacts: ["portfolio.json"]}

4. Post-completion hook (or veracity executor) triggers:
   → Reads portfolio.json
   → POST /veracity/v1/run-day with portfolio data
   → Receives score: {pnl: 1234.56, mse: 0.0042}
   → Records as wg evaluation with external_scores
   → Provenance: op="veracity_score", detail={pnl: 1234.56, mse: 0.0042}

5. wg evaluate runs on each completed task:
   → Internal quality score: 0.88
   → Combined with veracity score for agent performance record
```

---

## 4. Implementation Roadmap

### Phase 1: Proof of Concept (no code changes)

- Use two-phase task patterns with `--exec` for veracity API calls
- Use tags for visibility classification (`--tag visibility:public`)
- Manually run `wg evaluate` and record veracity scores in task logs
- Create a "Veracity Interviewer" role with the agent-guide.md as a URL skill
- **Validates:** concept feasibility, scoring integration, agent-as-market-good idea

### Phase 2: Core Extensions (upstream PRs)

- Add `metadata: HashMap<String, Value>` to Task struct (enables arbitrary veracity metadata without forking)
- Expand provenance logging to all lifecycle events
- Add post-completion hook system
- Add content hashing to artifact records
- Add external score fields to Evaluation struct
- **Validates:** production readiness, information flow control

### Phase 3: Native Integration

- Build veracity executor
- Implement visibility-aware context building
- Build `wg veracity` subcommand suite
- Build replay engine
- Integrate veracity scores into evolution system
- **Delivers:** full integration as described in this report

### Phase 4: Marketplace Features

- Agent definition export/import for market trading
- Peer discovery and veracity exchange network
- Automated portfolio generation from DAG outputs
- Cross-participant workflow composition

---

## 5. Open Questions

1. **Latent payoffs.** Nikete mentions "sub-units may have latent payoffs." How should wg handle evaluations that arrive days or weeks after task completion? The current evaluation system is point-in-time. A deferred evaluation mechanism would need: (a) a `pending_evaluation` state, (b) a polling or webhook system to check for delayed scores, (c) re-computation of agent performance when late scores arrive.

2. **Peer network topology.** "Learn what network of peers to do veracity exchanges with." This implies a social/reputation layer on top of wg. How does peer discovery work? Is it centralized (Veracity Exchange as matchmaker) or decentralized (participants find each other)?

3. **Partial portfolio attribution.** When a workflow has 5 tasks and the final portfolio scores well, how is credit distributed to individual sub-tasks? This is the credit assignment problem. Veracity's internal markets solve it for participants, but intra-workflow attribution is a separate problem. Shapley values over sub-task contribution?

4. **Schema standardization.** What exactly is the schema for `portfolio.json`? The Veracity API accepts "positions with opening/closing prices" but the exact format isn't publicly documented beyond the API endpoint.

5. **Confidentiality of graph structure.** Is the DAG topology itself sensitive? Knowing that someone runs "sentiment analysis → signal extraction → portfolio generation" reveals strategic information even without seeing the data. Should the graph structure itself have visibility controls?

---

## 6. Conclusion

Veracity Exchange and workgraph are complementary systems with a natural integration surface. Veracity provides **objective external scoring** (market P&L, prediction MSE) and a **marketplace for information products**. Workgraph provides **task decomposition** (DAGs), **composable agent identities** (role × motivation), **performance evolution**, and **information flow control** (dependency graph + artifact system).

The integration can be built incrementally: starting with zero-code-change task patterns, progressing through targeted core extensions, and culminating in a native veracity executor with full marketplace features. The critical design principle is **extend, don't fork**: all necessary changes can be expressed as new optional fields, new executor types, and new subcommands without modifying existing behavior.

The most novel insight is that **wg agent definitions are natural market goods** for Veracity Exchange. A role with a proven veracity track record is a concrete, transferable, content-hash-identified methodology with verifiable performance history. This is what Veracity's marketplace is designed to price and trade.
