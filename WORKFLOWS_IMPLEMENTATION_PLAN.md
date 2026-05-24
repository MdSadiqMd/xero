# Xero Workflows Implementation Plan

## Reader And Post-Read Action

This plan is for the engineer implementing Xero's future multi-agent Workflow feature. After reading it, the next useful action should be to start Phase 1 by adding shared Workflow definition contracts, validation, and tests without touching the existing single-agent Stage runtime semantics.

The plan treats GSD as a reference workflow system, not as a product surface to clone. The goal is to make the same class of continuous, self-recovering, artifact-driven work possible inside Xero using Xero's architecture, vocabulary, app-data persistence model, and agent canvas.

## North Star

A user should be able to create a full continuous Workflow by combining agents, gates, routers, artifacts, and recovery lanes on the Xero canvas.

The Workflow should be able to:

- Let users create their own agents and immediately use them as Workflow nodes.
- Let users build Workflows from any mix of custom agents and built-in starter agents.
- Start with a goal and route work through multiple agents.
- Pass typed artifacts from one agent to another.
- Branch with deterministic if/else logic.
- Loop through revise, verify, debug, and gap-closure lanes with explicit limits.
- Pause only when user judgment, manual action, auth, or safety policy requires it.
- Survive app restart and resume from durable state.
- Explain why each routing decision happened.

The user-facing promise is: "Describe or draw how work should move. Xero will coordinate the agents continuously, recover from common failures, and stop with a clear reason when human input is needed."

## Custom Agents Are First-Class

Workflows must not be limited to GSD-like roles such as planner, builder, verifier, reviewer, or debug. Those names are useful examples and starter templates, but they are not required roles in the product model.

The primary authoring flow should be:

1. Create or choose an agent.
2. Drop that agent into a Workflow.
3. Declare what it needs as input.
4. Declare what it produces as output.
5. Connect it to other agents, routers, gates, loops, and recovery lanes.

Every Workflow feature must work with user-created custom agents unless the feature explicitly depends on a specialized artifact type. Built-in agents should be treated as starter content, not privileged infrastructure.

Custom agents should be usable at three levels of structure:

- Status-only: The Workflow can route on success, failure, cancellation, stall, or human pause.
- Text artifact: The agent produces a generic text output that downstream agents can consume.
- Typed artifact: The agent produces structured fields that routers and gates can inspect.

This lets users start quickly with plain custom agents, then add typed outputs only when they need conditional routing or automated gates.

## Terminology Guardrails

Xero already has two concepts that share the same canvas surface. Keep them separate.

- Stage: A gated phase inside a single agent run. Stages enforce per-agent tool allowlists, required checks, retry limits, and stage-to-stage branches. Existing DTO names such as `CustomAgentWorkflowPhaseDto` and `workflowStructure.phases` are legacy wire names. Do not rename them casually.
- Workflow: A multi-agent pipeline where Agent A's output can feed Agent B, gates can route between agents, and workflow-level loops can recover from failure.

User-facing strings must say "Stage" for single-agent phases and "Workflow" only for the top-level canvas surface or the new multi-agent pipeline feature.

## What GSD Teaches Us

The GSD reference system is valuable because it shows how long-running autonomous work stays controllable. These are the behaviors Xero should carry forward.

1. The orchestrator coordinates, agents execute.

   GSD does not rely on one huge prompt to hold the entire process together. An orchestration layer decides which specialist runs next, what context it receives, and how its output is interpreted. Xero Workflows should follow the same split.

2. Durable artifacts are the contract between agents.

   GSD agents communicate through named artifacts such as plans, summaries, verification results, handoffs, debug reports, and gap lists. Xero should formalize that as typed Workflow artifacts instead of relying on loose chat transcript interpretation.

3. Gates are explicit decision points.

   GSD uses pre-flight gates, revision gates, escalation gates, and abort gates. Xero should expose these as Workflow gate or router nodes with inspectable decisions.

4. Loops are bounded.

   GSD revision loops have attempt caps, stall detection, and escalation. Xero should never allow an unbounded Workflow loop.

5. Failure routing is part of the workflow, not an afterthought.

   GSD can send stuck work to a debug lane, send gaps to a planning or refinement lane, send review findings to a fixer, and send manual/auth blockers to a human checkpoint. Xero Workflows should make these lanes first-class without requiring hard-coded agent identities.

6. Human checkpoints are typed.

   GSD separates human verification, human decision, and human action. Xero should do the same because auto-mode can safely handle some decisions, but auth and external manual actions must pause.

7. Dynamic work needs fresh graph evaluation.

   GSD re-reads state after phase execution because new gaps or phases can appear. Xero should evaluate the latest persisted Workflow run state after each node, not assume a static in-memory plan is still correct.

8. Verification is stronger than existence checks.

   GSD distinguishes "file exists" from "implementation is substantive, wired, and functional." Xero artifacts and checking agents should produce structured evidence so gates can route on meaningful results.

## Core Design

### Workflow Definition

A Workflow definition is the reusable graph the user edits.

It contains:

- Metadata: id, name, description, created/updated timestamps.
- Version: immutable definition snapshot used by each run.
- Nodes: agents, gates, routers, merges, checkpoints, and terminals.
- Edges: ordered routing rules from one node to another.
- Artifact contracts: what each node expects and produces.
- Run policy: default model, approval mode, concurrency limit, timeout policy, and recovery defaults.

Definitions should be saved in app-data backed storage, not under `.xero/`.

### Workflow Run

A Workflow run is a durable execution of one Workflow definition version.

It contains:

- Run id, project id, workflow definition version id.
- Status: `queued`, `running`, `paused`, `completed`, `failed`, `cancelled`.
- Current active node runs.
- Immutable artifact records.
- Edge decision records.
- Loop attempt counters.
- Event log records for replay, audit, and UI timeline.

Every run must snapshot the Workflow definition version and agent definition versions it uses. This prevents a running Workflow from changing behavior because the user edits the graph mid-run.

### Node Types

Initial node types:

- `agent`: Launches any Xero agent, including user-created custom agents and built-in starter agents. This node references an agent definition and provides prompt/input bindings.
- `router`: Evaluates deterministic conditions and chooses an outgoing edge.
- `gate`: Evaluates required checks and may pause, revise, escalate, or abort.
- `human_checkpoint`: Pauses for human verification, human decision, or human action.
- `merge`: Waits for multiple incoming branches when parallelism is enabled.
- `terminal`: Ends the Workflow with success, failure, cancelled, or needs-human status.

Do not overload existing Stage nodes for this. Stage nodes remain inside a single agent definition.

### Agent Node Contract

Agent nodes should reference an agent through a neutral `AgentRef`, not through role-specific fields.

The reference should include:

- Agent source: custom or built-in.
- Agent definition id.
- Agent definition version id.
- Optional display label for this Workflow node.
- Optional run overrides, such as model, approval mode, or prompt preface.
- Input bindings from run input or prior artifacts.
- Output contract, either generic text or typed artifact schema.
- Failure classification policy.

No Workflow engine behavior should check for special agent ids like planner, builder, verifier, reviewer, or debug. Templates may use those labels, but the runtime should only see nodes, edges, conditions, artifacts, and agent references.

### Custom Agent Authoring From Workflows

Workflow authoring should make agent creation feel native.

The agent node configuration should support:

- Pick an existing custom or built-in agent.
- Create a new custom agent from this node.
- Edit the selected custom agent through the existing agent authoring experience.
- Return to the Workflow with the new or updated agent selected.
- Define what the node calls the agent's output.
- Choose whether the output is generic text or a typed artifact.

The typed artifact path should be progressive. A user should not need to design a schema just to connect two custom agents. They should need a schema only when they want routers, gates, or loop stall detectors to inspect specific fields.

## Visual Continuity With Agent Authoring

Workflow authoring must feel like the same product surface as agent authoring. It should be aesthetically similar to the current agent canvas, including canvas elements, node cards, edge treatments, selection behavior, palettes, diagnostics, and properties panels.

The Workflow editor may introduce new node types, but it should not introduce a new visual language. A user should feel that they moved from "design one agent" to "compose many agents" inside the same canvas family.

Required visual continuity:

- Canvas shell: Reuse the same React Flow foundation, background treatment, zoom controls, lock/unlock behavior, fit-view behavior, selection focus treatment, and diagnostic placement patterns where practical.
- Node cards: Workflow nodes should share the same card anatomy as agent canvas nodes: compact header, icon or type marker, title, short metadata, connection handles, selected state, hover state, and readable expanded details.
- Edges: Workflow edges should use the same smoothstep and labeled-branch family as the agent canvas. Conditional labels, loop labels, and recovery labels must avoid node overlap and remain readable when selected.
- Properties panel: Selecting a Workflow node or edge should open a properties/details panel that feels like the existing node properties panel: same placement, density, header structure, close behavior, scroll behavior, ShadCN form controls, and validation message style.
- Palette and insertion: Adding Workflow nodes should feel like adding items to the agent canvas, using the same palette/drop-picker interaction patterns where possible.
- Diagnostics: Invalid Workflow graphs should surface diagnostics in the same tone and placement as agent authoring diagnostics.
- Template customization: Starter templates should open into normal editable canvas elements, not a wizard-only or form-only experience.

Workflow-level visuals still need clear semantic distinction from single-agent Stages. Use labels, icons, subtitles, and modest accent differences to communicate "agent node", "router", "gate", "checkpoint", and "terminal" without creating a separate theme.

Implementation preference:

- Extract or reuse shared canvas primitives when the existing agent canvas component is too Stage-specific.
- Avoid duplicating a second properties-panel framework.
- Avoid one-off Workflow-only styling unless the existing canvas grammar cannot represent the interaction.
- Keep node dimensions stable so labels, handles, status badges, and expanded details do not resize the graph unpredictably.

### Edge Types

Initial edge types:

- `success`: Followed when a node succeeds and no more specific condition wins.
- `failure`: Followed when a node fails.
- `conditional`: Followed when a condition expression evaluates true.
- `loop`: Followed only when an explicit loop policy permits another attempt.
- `recovery`: Followed for debug, repair, review-fix, or gap-closure lanes.
- `manual_override`: Followed after a user decision.

Edges are evaluated in priority order. An explicit default else edge is represented as an `always` condition with lowest priority. A node may have at most one default edge.

### Artifact Contracts

Artifacts are the handoff format between nodes.

Each artifact has:

- Artifact id.
- Run id.
- Producer node run id.
- Type, such as `text_output`, `plan`, `implementation_summary`, `verification_result`, `debug_report`, `gap_list`, `review_findings`, `handoff`, `human_decision`, or a user-defined custom type.
- Schema version.
- JSON payload.
- Optional text rendering for UI.
- Creation timestamp.

Agent nodes should declare:

- Required input artifacts.
- Optional input artifacts.
- Produced artifact types.
- Output extraction rules.

The MVP can begin with generic text artifacts plus a small set of built-in artifact schema presets. Those presets should not be the only valid outputs. User-defined artifact types should be allowed once the typed artifact editor exists.

## How Loops Work

Loops are handled by Workflow graph edges plus loop policies. They are not handled by recursive prompts and they are not implicit retries hidden inside an agent node.

Every loop edge must declare:

- `loopKey`: Stable id for the loop counter.
- `maxAttempts`: Hard attempt cap.
- `attemptScope`: Whether the counter is per run, per source node, per target node, or per artifact group.
- `carryoverPolicy`: Which artifacts are passed into the next attempt.
- `resetPolicy`: Whether a successful downstream node resets the loop counter.
- `stallDetector`: Optional rule that detects non-improving attempts.
- `onExhausted`: Target node or terminal state when attempts are exhausted.

No Workflow definition is valid if it contains a cycle without an explicit loop policy. This should be enforced by the definition validator.

### Loop Examples

Quality gate loop:

```text
Work Agent -> Check Agent
Check Agent(status = passed) -> Summary Agent
Check Agent(status = needs_changes) -> Refinement Agent
Refinement Agent -> Work Agent via loop(maxAttempts = 1 or 2)
Loop exhausted -> Human Checkpoint
```

Review loop:

```text
Work Agent -> Review Agent
Review Agent(findings.high_count = 0) -> Summary Agent
Review Agent(findings.high_count > 0) -> Fix Agent
Fix Agent -> Review Agent via loop(maxAttempts = 3, stallDetector = high_count_not_decreasing)
Loop exhausted -> Human Checkpoint
```

Debug recovery loop:

```text
Work Agent(failed or stalled) -> Debug Agent
Debug Agent(resolved = true) -> Work Agent via loop(maxAttempts = 2)
Debug Agent(resolved = false) -> Human Checkpoint
Loop exhausted -> Human Checkpoint
```

### Stall Detection

Stall detection prevents a loop from repeating just because attempts remain.

Initial stall detectors:

- `finding_count_not_decreasing`: Stop if checking or review issue count does not decrease.
- `same_failure_class_repeated`: Stop if the same failure class appears in consecutive attempts.
- `no_artifact_progress`: Stop if no new required artifact is produced in an attempt.
- `runtime_activity_timeout`: Stop if an agent run has no meaningful heartbeat or output for a configured duration.
- `retry_limit_exceeded`: Stop if the underlying agent Stage runtime exceeds its own retry limit.

When a stall detector fires, the node run should be marked `stalled` with a failure class. Routing can then send the run to a debug lane, a human checkpoint, or a terminal.

## How If/Else Routing Works

If/else behavior is handled by router nodes and conditional edges.

Conditions must be deterministic in the MVP. They should evaluate over:

- Node status.
- Artifact fields.
- Failure class.
- Loop attempt count.
- Human decision.
- Checkpoint type.
- Agent output classification fields.

LLM-based classification can be added later as a separate classifier agent node. That classifier must emit a typed artifact. Routing still evaluates deterministic conditions over that artifact.

### Condition DSL MVP

Example:

```json
{
  "kind": "all",
  "conditions": [
    {
      "kind": "node_status",
      "nodeId": "verify",
      "status": "succeeded"
    },
    {
      "kind": "artifact_field_equals",
      "artifactRef": "verify.result",
      "path": "$.status",
      "value": "gaps_found"
    },
    {
      "kind": "loop_attempt_lt",
      "loopKey": "gap_closure",
      "value": 2
    }
  ]
}
```

Initial condition kinds:

- `always`
- `all`
- `any`
- `not`
- `node_status`
- `artifact_exists`
- `artifact_field_equals`
- `artifact_field_in`
- `artifact_field_number_compare`
- `failure_class_is`
- `loop_attempt_lt`
- `loop_attempt_gte`
- `human_decision_is`

Else is not a special branch type. Else is the one lowest-priority edge whose condition is `always`.

### Routing Evaluation

For each completed node:

1. Load latest durable run state.
2. Collect candidate outgoing edges.
3. Sort by priority.
4. Evaluate each edge condition against a read-only state snapshot.
5. Pick the first matching edge.
6. Persist an edge decision record with the condition result and input evidence.
7. Create the next node run or pause if the target is a checkpoint.
8. If no edge matches, pause at an implicit escalation state with a validation warning.

The UI should show the selected edge and why it matched.

## Recovery Lanes

Recovery lanes are reusable Workflow patterns. They are normal nodes and edges, not hidden runtime exceptions.

### Debug Lane

Use when implementation gets stuck, fails repeatedly, or produces unclear errors.

Trigger examples:

- Agent run has no meaningful progress for a configured duration.
- Same tool failure class repeats.
- Underlying Stage retry limit is exceeded.
- A checking agent says root cause is needed.
- A work agent returns a structured `blocked` artifact.

Inputs to Debug:

- Last agent transcript summary.
- Runtime errors and failure class.
- Recent artifacts.
- File diff summary if available.
- Verification or review findings.
- Loop attempt counters.

Outputs from Debug:

- `debug_report`
- `fix_hypothesis`
- `recommended_route`: `retry_work`, `decompose`, `ask_human`, or `abort`

Routes from Debug:

- `retry_work`: Return to the original work agent through a bounded recovery loop.
- `decompose`: Send to planner or node-repair lane.
- `ask_human`: Pause at human checkpoint.
- `abort`: End at failed terminal.

### Gap Closure Lane

Use when verification finds missing requirements or incomplete behavior.

Trigger examples:

- `verification_result.status = gaps_found`
- `verification_result.gaps.length > 0`

The gap closure lane should:

- Pass checking evidence to a planning, refinement, or user-selected gap agent.
- Produce a scoped `gap_closure_plan`.
- Route back to the original work agent or another user-selected agent.
- Limit attempts to prevent endless polishing.

### Code Review Fix Lane

Use when reviewer finds actionable issues.

Trigger examples:

- `review_findings.high_count > 0`
- `review_findings.must_fix = true`

The lane should:

- Route from reviewer to fixer.
- Route fixer back to reviewer.
- Cap attempts, initially at 3.
- Stall if severe finding count does not decrease.

### Node Repair Lane

Use when a single task or node fails verification but can be locally repaired.

Possible decisions:

- `retry`: Run the same node again with failure context.
- `decompose`: Send to planner to split the work.
- `prune`: Mark the failed branch out of scope when allowed by the Workflow.
- `escalate`: Pause for human decision.

This can ship after the basic debug and gap lanes.

### Quota And Runtime Lane

Quota and runtime failures should not blindly retry.

Trigger examples:

- Provider quota exhausted.
- Runtime adapter crash.
- Tool infrastructure unavailable.

Routes:

- Pause until reset.
- Ask user to switch model/provider.
- Retry after a backoff only when the failure class is known transient.

### Human Checkpoint Lane

Human checkpoints have typed reasons:

- `human_verify`: User confirms behavior or accepts evidence.
- `decision`: User chooses between routes.
- `human_action`: User must do something outside Xero, such as logging in or granting access.

Auto-mode may be allowed later for low-risk `human_verify` and first-choice `decision` checkpoints. It must never auto-complete `human_action`.

## Execution Semantics

### State Machine

Workflow run status:

- `queued`
- `running`
- `paused`
- `completed`
- `failed`
- `cancelled`

Workflow node run status:

- `pending`
- `eligible`
- `starting`
- `running`
- `waiting_on_gate`
- `succeeded`
- `failed`
- `stalled`
- `skipped`
- `cancelled`

Terminal status is derived from the terminal node reached.

### Orchestrator Responsibilities

The Workflow orchestrator should:

- Start eligible nodes.
- Launch existing Xero runtime runs for agent nodes.
- Observe runtime completion, failure, or stall.
- Extract and persist produced artifacts.
- Evaluate outgoing edge conditions.
- Enforce loop budgets.
- Pause for human checkpoints.
- Resume after app restart.
- Emit timeline events for the UI.

The frontend should not be the state machine. It should submit commands and render durable state.

### Idempotency

Every node start should use an idempotency key:

```text
workflow_run_id + node_id + attempt_number
```

If the app restarts while starting a node, reconciliation should detect whether a runtime run already exists for that key before launching another one.

### Versioning

Each run snapshots:

- Workflow definition version.
- Agent definition version for every agent node.
- Input artifact ids.
- Run policy.

Editing a Workflow definition creates a new version. Existing runs continue on their original version.

## Data Model

Use the existing app-data database path. Do not write Workflow state into `.xero/`.

Initial tables:

- `workflow_definitions`
- `workflow_definition_versions`
- `workflow_runs`
- `workflow_run_nodes`
- `workflow_run_edges`
- `workflow_artifacts`
- `workflow_gate_decisions`
- `workflow_loop_attempts`
- `workflow_events`

Recommended fields:

```text
workflow_definitions
  id
  project_id
  name
  description
  active_version_id
  created_at
  updated_at

workflow_definition_versions
  id
  workflow_id
  version_number
  definition_json
  created_at

workflow_runs
  id
  project_id
  workflow_version_id
  status
  started_at
  updated_at
  completed_at
  cancellation_reason

workflow_run_nodes
  id
  workflow_run_id
  node_id
  node_type
  status
  attempt_number
  runtime_run_id
  failure_class
  started_at
  updated_at
  completed_at
  idempotency_key

workflow_run_edges
  id
  workflow_run_id
  from_node_id
  to_node_id
  edge_id
  decision_json
  created_at

workflow_artifacts
  id
  workflow_run_id
  producer_node_run_id
  artifact_type
  schema_version
  payload_json
  render_text
  created_at

workflow_gate_decisions
  id
  workflow_run_id
  node_run_id
  checkpoint_type
  decision
  decision_payload_json
  decided_at

workflow_loop_attempts
  id
  workflow_run_id
  loop_key
  attempt_count
  last_node_run_id
  exhausted
  updated_at

workflow_events
  id
  workflow_run_id
  node_run_id
  event_type
  event_json
  created_at
```

Add CHECK constraints for enum-like status fields where practical. Add unique constraints for active idempotency keys and loop counters.

## Code Touchpoints

Start with new files instead of forcing the Workflow concept into existing Stage code.

Existing code to understand:

- `client/src/lib/xero-model/agent-definition.ts`: single-agent Stage schema. Do not repurpose it for multi-agent Workflows.
- `client/src/lib/xero-model/workflow-agents.ts`: existing graph projection and agent detail models for the current canvas.
- `client/src-tauri/src/commands/contracts/workflow_agents.rs`: current Rust DTOs for agent graph and authoring catalog.
- `client/src-tauri/src/commands/start_runtime_run.rs`: entry point for launching one agent runtime run.
- `client/src-tauri/src/runtime/autonomous_orchestrator/`: existing single-agent run persistence and reconcile patterns.
- `client/src-tauri/src/runtime/autonomous_tool_runtime/mod.rs`: current Stage enforcement and retry behavior.
- `client/src-tauri/src/db/migrations.rs`: database migration patterns and built-in agent definitions.
- `client/components/xero/workflow-canvas/`: current canvas implementation using React Flow.

Suggested new modules:

- `client/src/lib/xero-model/workflow-definition.ts`
- `client/src/lib/xero-model/workflow-run.ts`
- `client/src-tauri/src/commands/contracts/workflows.rs`
- `client/src-tauri/src/commands/workflows.rs`
- `client/src-tauri/src/runtime/workflow_orchestrator/`
- `client/src-tauri/src/runtime/workflow_orchestrator/condition_eval.rs`
- `client/src-tauri/src/runtime/workflow_orchestrator/definition_validator.rs`
- `client/src-tauri/src/runtime/workflow_orchestrator/reconcile.rs`
- `client/src-tauri/src/runtime/workflow_orchestrator/artifacts.rs`

Suggested frontend reuse or extraction:

- Shared canvas shell, controls, viewport behavior, and selection focus behavior from the existing agent canvas.
- Shared node card primitives for headers, handles, badges, status, collapsed bodies, and expanded bodies.
- Shared properties/details panel shell and form controls.
- Shared edge label readability behavior for conditional and loop edges.
- Shared diagnostics panel patterns for validation feedback.

## Phased Implementation

### Phase 1: Workflow Contracts And Validation

Goal: Define the Workflow graph shape and make invalid loops impossible.

Scope:

- Add TypeScript schemas for Workflow definitions, nodes, edges, conditions, artifacts, and run policy.
- Add neutral agent references that support custom agents and built-in starter agents.
- Add Rust DTOs that mirror the TypeScript contracts.
- Add a definition validator.
- Validate node ids, edge ids, unique defaults, missing targets, invalid artifact references, and condition shape.
- Detect graph cycles.
- Reject cycles that do not include an explicit loop edge with `maxAttempts`.
- Reject loop edges without an exhaustion route.
- Keep these contracts separate from single-agent Stage contracts.

Acceptance criteria:

- A linear Workflow definition validates.
- A Workflow definition can reference custom agents without using role-specific fields.
- A conditional Workflow with one explicit else edge validates.
- A cycle without loop policy fails validation.
- A loop with max attempts and exhaustion route validates.
- TypeScript and Rust tests cover validation examples.

### Phase 2: App-Data Persistence

Goal: Persist Workflow definitions and runs in the app-data database.

Scope:

- Add migrations for Workflow definition, run, artifact, event, decision, and loop tables.
- Add data access helpers for definitions, versions, runs, nodes, artifacts, events, and loop counters.
- Ensure definitions are versioned and immutable once a run starts.
- Ensure new state never writes to `.xero/`.

Acceptance criteria:

- A Workflow definition can be created, versioned, loaded, and listed.
- A Workflow run can be created from a definition version.
- Artifacts can be inserted and read by run and type.
- Loop counters can increment atomically.
- Scoped database tests cover round trips and constraints.

### Phase 3: Headless Sequential Orchestrator MVP

Goal: Execute a simple linear multi-agent Workflow without UI authoring.

Scope:

- Add Tauri commands to create/list/get/start/cancel Workflow runs.
- Add a headless Workflow orchestrator service.
- Start the first eligible agent node.
- Launch existing Xero runtime runs for custom and built-in agent nodes.
- Observe completion or failure.
- Persist node status and events.
- Route through simple success/failure edges.
- End at terminal node.

Acceptance criteria:

- A fixture definition can run `Custom Agent A -> Custom Agent B -> Terminal`.
- Runtime run ids are linked to Workflow node runs.
- The Workflow run survives app restart via reconcile.
- Cancellation stops future node starts and marks the run cancelled.

### Phase 4: Artifacts And Handoffs

Goal: Make agent-to-agent handoff explicit and inspectable.

Scope:

- Define MVP artifact schemas:
  - `text_output`
  - `task_brief`
  - `plan`
  - `implementation_summary`
  - `verification_result`
  - `debug_report`
  - `gap_list`
  - `review_findings`
  - `human_decision`
- Add safe input binding from artifacts into agent prompts.
- Add output extraction for agent nodes.
- Add generic output support so any custom agent can pass text to a downstream agent.
- Add typed output support for custom agents that need router/gate conditions.
- Persist artifacts as immutable records.
- Show artifacts in run details.

Acceptance criteria:

- Agent B can receive Agent A's produced artifact as input.
- A user-created custom agent can participate with a generic text output.
- A user-created custom agent can opt into a typed output contract for routing.
- Missing required artifacts block node start with a clear error.
- Artifact records show producer, type, schema version, and payload.
- Tests cover artifact reference resolution and missing input behavior.

### Phase 5: Conditional Routing And Gates

Goal: Support if/else routing and user-visible gates.

Scope:

- Implement the condition evaluator in Rust.
- Add router and gate nodes.
- Add priority-ordered edge evaluation.
- Add explicit else edge behavior through `always`.
- Add human checkpoint pause/resume commands.
- Persist edge decision evidence.

Acceptance criteria:

- A router can choose between two custom or built-in agents based on an artifact field.
- A default else route is used when no specific condition matches.
- A node with two default else routes fails validation.
- A human checkpoint pauses the run and can be resumed with a decision.
- Edge decisions are visible in durable run state.

### Phase 6: Bounded Loops And Recovery Lanes

Goal: Support GSD-class recovery without unbounded autonomy.

Scope:

- Implement loop counter persistence and enforcement.
- Implement loop exhaustion routing.
- Add stall detectors:
  - finding count not decreasing
  - same failure class repeated
  - no artifact progress
  - runtime activity timeout
  - underlying Stage retry limit exceeded
- Add reusable lane templates:
  - debug recovery
  - gap closure
  - code review fix
  - node repair
- Let each reusable lane choose custom agents instead of requiring built-in planner, checker, fixer, or debug agents.
- Add failure classification for stalled, quota, runtime, checking gaps, review findings, and human action.

Acceptance criteria:

- A checking-agent gaps loop returns to a work agent until max attempts is reached.
- A code review fix loop stops early if finding counts stop improving.
- A stalled custom work agent can route to a custom or built-in debug agent.
- Debug can route back to the original work agent or pause for human escalation.
- Loop exhaustion is recorded and visible.
- Tests cover loop attempt caps, stall detection, and recovery routing.

### Phase 7: Workflow Canvas Authoring

Goal: Let users create and edit Workflows visually.

Scope:

- Add a Workflow authoring mode separate from agent Stage editing.
- Add node palette for agent, router, gate, human checkpoint, merge, and terminal nodes.
- Add an agent picker that includes custom agents and built-in starter agents.
- Add a create-agent action from an agent node configuration.
- Reuse the existing custom agent authoring flow, then return to the Workflow with the new agent selected.
- Add a swap-agent action so templates can be replaced with user-created agents.
- Use ShadCN components where possible for panels, forms, dialogs, tabs, selects, switches, and validation messages.
- Reuse or extract the existing agent canvas foundations so Workflow authoring is visually consistent with agent authoring.
- Reuse or extract the existing node properties/details panel shell for Workflow node and edge editing.
- Reuse the existing canvas interaction patterns for selection focus, zoom controls, lock controls, diagnostics, palette insertion, and edge labels.
- Add condition editor for edge rules.
- Add loop policy editor for loop edges.
- Add artifact contract editor for agent nodes.
- Add validation diagnostics before save.

UI guardrails:

- Do not add temporary debug or test UI.
- Do not label single-agent phases as Workflow phases.
- Do not ship a Workflow authoring surface that feels like a separate visual product from agent authoring.
- Do not require users to understand JSON for common conditions.
- Keep advanced JSON editing as an optional expert affordance only if needed.

Acceptance criteria:

- A user can create a custom agent from Workflow authoring and immediately connect it.
- A user can create a simple Workflow from existing custom or built-in agents.
- A user can replace a starter-template agent with their own custom agent.
- Workflow node cards, edges, controls, palette behavior, diagnostics, and properties panels are aesthetically aligned with the existing agent authoring canvas.
- Selecting a Workflow node or edge opens a properties/details panel with the same panel grammar as agent authoring.
- A user can add if/else routing from a router node.
- A user can add a bounded loop edge with max attempts and exhaustion route.
- Invalid graphs show actionable validation messages.
- Saved Workflows create immutable versions.

### Phase 8: Run Timeline And Operations

Goal: Make continuous Workflow runs understandable and controllable.

Scope:

- Add Workflow run list and detail view.
- Show active node, completed nodes, artifacts, edge decisions, loop attempts, and pauses.
- Link node runs to underlying agent sessions.
- Add controls:
  - start
  - cancel
  - resume checkpoint
  - retry from node when allowed
  - skip branch when allowed by policy
- Add event timeline from durable `workflow_events`.

Acceptance criteria:

- User can inspect why a Workflow chose a route.
- User can see artifacts passed between agents.
- User can resume a human checkpoint.
- User can cancel a running Workflow.
- Timeline remains accurate after app restart.

### Phase 9: Starter Template Library

Goal: Ship optional starter templates that prove Xero can express GSD-like behavior without hard-coding GSD-like agents as the product model.

Template shape:

```text
Goal Intake
  -> Planning Agent
  -> Work Agent
  -> Check Agent
  -> Router
     if verification passed -> Reviewer
     if gaps found -> Gap Agent -> Work Agent loop
     if stalled -> Debug Agent -> Work Agent loop
  -> Reviewer
  -> Router
     if no high findings -> Summary
     if high findings -> Fixer -> Reviewer loop
  -> Summary
  -> Terminal
```

The template should use Xero language and Xero artifacts. It should not depend on GSD files or commands. Every agent in the template should be swappable with a user-created custom agent.

Acceptance criteria:

- Template can be instantiated into a project Workflow.
- Template can be customized by replacing any starter agent with a custom agent.
- Template includes debug, gap closure, and review-fix lanes.
- Template validates under the same validator as user-created Workflows.
- Template can run end to end with built-in or custom agents once bindings are configured.

### Phase 10: Parallelism And Merge Nodes

Goal: Support safe fan-out/fan-in workflows.

Scope:

- Add parallel eligible node scheduling.
- Add merge nodes with wait policies:
  - wait for all
  - wait for any
  - wait for quorum
  - fail fast
- Add resource conflict policy.
- Add project-level concurrency limit.
- Add branch cancellation semantics.

Acceptance criteria:

- Independent branches can run concurrently.
- Merge waits according to policy.
- Failed branch behavior is explicit.
- Conflicting branches can be serialized by policy.

### Phase 11: Hardening, Observability, And Evals

Goal: Make Workflows reliable enough for long-running use.

Scope:

- Add reconcile tests for app restart at each major transition.
- Add event replay tests.
- Add structured logging around edge evaluation and node starts.
- Add failure classification coverage.
- Add fixture Workflows for regression tests.
- Add export/import for Workflow definitions if useful.
- Add metrics for loop exhaustion, checkpoint pauses, and recovery success.

Acceptance criteria:

- Orchestrator can recover after restart during node start, node run, checkpoint wait, and edge evaluation.
- Duplicate node starts are prevented.
- Recovery decisions are explainable from events.
- Scoped tests cover the shipped template.

## Example: Stuck Work Agent Routes To Debug

This is the concrete case the system must support.

```text
1. A user-created work agent starts its assigned task.
2. The work agent runs longer than the configured activity timeout without producing required artifact progress.
3. Orchestrator marks the work node run as stalled with failure_class = runtime_activity_timeout.
4. Outgoing recovery edge condition matches:

   failure_class_is("runtime_activity_timeout")
   and loop_attempt_lt("debug_recovery", 2)

5. A custom or built-in debug agent starts with the work agent transcript summary, errors, artifacts, diff summary, and loop counters.
6. The debug agent emits debug_report and fix_hypothesis.
7. Router checks the debug agent's recommended_route:
   - retry_work: return to the original work agent through debug_recovery loop
   - decompose: route to a planning or decomposition agent
   - ask_human: pause at Human Checkpoint
   - abort: terminal failed
8. If the work agent stalls again and debug_recovery attempts are exhausted, route to Human Checkpoint.
```

This keeps recovery autonomous when it is likely to help, but bounded and explainable when it does not.

## Test Strategy

Use scoped tests and avoid repo-wide Rust commands unless necessary.

Test layers:

- TypeScript schema tests for Workflow definitions and editor models.
- Rust validator tests for graph structure, cycles, loop policies, and defaults.
- Rust condition evaluator tests for artifact predicates, node status, loop counters, and human decisions.
- Database tests for definition versioning, run persistence, artifacts, decisions, and loop counters.
- Orchestrator unit tests with a fake runtime adapter.
- Reconcile tests for restart scenarios.
- React component tests for authoring forms and validation messages.

Important fixtures:

- Linear two-custom-agent Workflow.
- If/else router Workflow.
- Custom checking-agent gap loop.
- Debug recovery loop.
- Review-fix loop with stall detection.
- Human checkpoint pause/resume.
- Invalid cycle without loop policy.

## Open Design Decisions

These should be resolved during Phase 1 or Phase 2.

1. Artifact extraction source.

   Decide whether MVP agent nodes produce artifacts from final response JSON, structured tool call output, explicit runtime metadata, or a hybrid.

2. Runtime activity heartbeat.

   Decide which runtime events count as meaningful progress for `runtime_activity_timeout`.

3. Auto-mode policy.

   Decide whether Workflow runs initially require all human checkpoints to pause, or whether low-risk human verification can be auto-accepted under an explicit user setting.

4. Agent definition versioning.

   Confirm where custom and built-in agent versions should be snapshotted for Workflow runs.

5. Custom artifact schema authoring.

   Decide how users define typed outputs for custom agents: form-based schema builder, examples-first inference, JSON schema editor, or a hybrid.

6. File/resource conflict detection.

   Decide whether Phase 10 parallelism uses declared resource scopes, inferred file plans, or runtime locking.

## Non-Goals For The First Slice

- Do not implement generic arbitrary code execution inside Workflow conditions.
- Do not allow unbounded loops.
- Do not rename current Stage DTOs.
- Do not migrate legacy `.xero/` state.
- Do not add temporary debug UI.
- Do not hard-code planner, builder, verifier, reviewer, fixer, or debug as required Workflow roles.
- Do not require browser-based verification for the Tauri app.
- Do not ship parallelism before sequential routing, artifacts, and loop safety are solid.

## First Slice Recommendation

Start with Phase 1 and implement the smallest useful vertical slice:

```text
WorkflowDefinition schema
  -> graph validator
  -> condition schema
  -> loop policy validation
  -> TypeScript and Rust tests
```

A good first demo fixture:

```text
Custom Intake Agent -> Custom Work Agent -> Custom Check Agent -> Router
Router passed -> Terminal Success
Router needs_changes -> Custom Work Agent via loop(maxAttempts = 1)
Router else -> Human Checkpoint
```

That slice proves the hardest design constraints early: custom agent references, typed graph definitions, deterministic routing, explicit else behavior, and bounded loops.
