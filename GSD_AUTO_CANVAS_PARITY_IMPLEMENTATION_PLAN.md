# GSD Auto Canvas Parity Implementation Plan

Temporary phased plan for expanding Xero's Workflow canvas and runtime until it can natively model the GSD `/auto` workflow end to end.

## North Star

Xero's canvas should be general-purpose enough to model GSD `/auto` as a first-class Workflow definition, without hard-coding Xero into only being GSD.

The target is not "copy GSD prompts into a node." The target is that the Workflow graph model, runtime, persistence layer, and UI are expressive enough to represent the same lifecycle GSD provides:

- start or continue a milestone
- discover incomplete delivery phases
- discuss, plan, execute, review, verify, and close gaps per phase
- re-read the roadmap after each phase and handle inserted phases
- audit the milestone
- complete/archive the milestone
- start the next milestone
- preserve durable project state across runs

GSD becomes the proving workload. If Xero can model GSD `/auto` cleanly, the same primitives should support many other advanced delivery workflows.

## Terminology Guardrails

- **Workflow** means the canvas-authored multi-agent graph.
- **Stages** mean gated phases inside a single agent run.
- **Delivery phase** means a project planning unit similar to a GSD roadmap phase. Do not label these as agent Stages in user-facing canvas UI.
- **Milestone** means an evolving product scope/version boundary containing delivery phases and requirements.

## Current Gap Summary

Xero already has useful graph primitives: agent nodes, routers, gates, human checkpoints, merge nodes, terminal nodes, typed artifacts, bounded loops, run snapshots, retries, skips, and resume.

The missing pieces for GSD `/auto` parity are mostly higher-order orchestration primitives:

- durable project/milestone/requirement/roadmap state
- dynamic iteration over a data set of delivery phases
- graph nodes that can read/write structured project state, not only pass run-local artifacts
- strict artifact schemas and prompt-enforced output contracts
- native subprocess/tool/test verification nodes
- background/parallel agent execution with dependency joins
- milestone lifecycle nodes: new milestone, audit, complete, archive, next milestone
- human decision nodes with typed choices and resume payloads
- canvas affordances for loops, collections, state reads/writes, and lifecycle templates

## Design Principles

1. Keep the Workflow engine generic. Add primitives such as collections, state stores, schema contracts, and typed human checkpoints; build GSD as a template on top.
2. Store durable project state in OS app-data, not `.xero/`.
3. Avoid backwards compatibility glue for stale local state. This is a new app; wipe incompatible app-data during development when needed.
4. Make graph behavior inspectable. Every route, state write, verification result, and retry decision should have an event trail.
5. Prefer strict contracts over agent hope. If routing depends on JSON fields, enforce schemas and inject exact final-output instructions.
6. Keep repo-mutating nodes serialized by default unless an isolation story is explicitly configured.
7. Support reusable workflow subgraphs so GSD-like plans are not one giant unmaintainable canvas.

## Phase 0: Baseline Audit And Test Harness

Goal: lock down the current Workflow behavior before expanding it.

Tasks:

- Add focused tests around existing Workflow validation, run creation, routing, loop exhaustion, checkpoint resume, retry, skip, and artifact extraction.
- Add a fixture Workflow that resembles the current Continuous Delivery template and assert its graph validates.
- Add regression tests for required input failure and JSON extraction failure.
- Add an internal "GSD parity fixture" as data only: milestone with three delivery phases, one inserted decimal phase, one gap closure, one human verification, and milestone close.

Acceptance criteria:

- Existing Continuous Delivery template validates in tests.
- Runtime tests prove current loops are bounded.
- A failing JSON artifact produces an actionable failure state.
- No UI changes yet.

## Phase 1: Strict Artifact Contracts

Goal: make typed handoffs dependable enough for routing and verification.

Tasks:

- Extend artifact contracts from "json_object/json_array/generic_text" to full JSON Schema validation.
- Add per-node final response contract generation from the output contract.
- Store validation diagnostics with the artifact extraction event.
- Add render path validation so UI previews do not silently blank.
- Add template contracts for:
  - task brief
  - delivery plan
  - implementation summary
  - verification result
  - review findings
  - gap list
  - debug report
  - human decision
  - milestone audit

Acceptance criteria:

- A node that declares `verification_result.status` can only produce allowed statuses.
- A router condition cannot reference an artifact field that the schema does not allow.
- The Continuous Delivery template is updated to emit strict JSON for routing artifacts.
- Tests cover valid JSON, invalid JSON, wrong shape, and missing required fields.

## Phase 2: Durable Delivery State Store

Goal: add the native app-data equivalent of GSD's durable planning files.

Tasks:

- Add app-data tables or records for:
  - delivery projects
  - milestones
  - requirements
  - delivery phases
  - phase context
  - phase plans
  - phase summaries
  - verification evidence
  - deferred items/seeds
  - milestone archives
- Add command contracts for reading/writing this state.
- Add state event records so Workflow runs can explain what changed.
- Add import/export JSON for debugging and future portability.
- Keep this separate from legacy `.xero/` repo-local state.

Acceptance criteria:

- A Workflow run can create a milestone, add requirements, add delivery phases, and mark phases complete.
- State survives app restart and new Workflow runs.
- State can be wiped clean for a project during development.
- No `.xero/` writes are introduced.

## Phase 3: State Nodes For The Canvas

Goal: let the canvas read and write durable delivery state without custom code per template.

New generic node types or node capabilities:

- **State read**: query durable project/milestone/phase/requirement records.
- **State write**: create/update/archive records with schema validation.
- **State patch**: apply controlled updates to existing records.
- **State query**: filter/sort collections, such as incomplete delivery phases.
- **State checkpoint**: pause when state has blockers or unresolved human decisions.

Tasks:

- Add state bindings alongside run input and artifact bindings.
- Add condition predicates for state fields and collection counts.
- Add idempotency keys for state writes so retries do not duplicate milestones or phases.
- Show state reads/writes in the run event timeline.

Acceptance criteria:

- A Workflow can query "all incomplete delivery phases for the current milestone."
- A Workflow can mark a delivery phase complete and then re-query the remaining phases.
- Retrying a state-writing node does not create duplicate records.

## Phase 4: Collection Iteration And Dynamic Graph Expansion

Goal: model GSD `/auto` phase iteration natively.

Tasks:

- Add a collection loop primitive:
  - source collection binding
  - item variable binding
  - sort key
  - completion condition
  - per-item subgraph
  - after-item re-query option
- Support dynamic insertion: after each phase, the loop can re-read roadmap state and include newly inserted phases.
- Add loop controls:
  - `--from` equivalent
  - `--to` equivalent
  - `--only` equivalent
  - max item count guard
  - max total runtime guard
- Add UI representation for collection loops without making the canvas unreadable.

Acceptance criteria:

- A test Workflow processes phases 1, 2, and 3.
- If phase 2 inserts phase 2.1, the Workflow catches and runs 2.1 before phase 3.
- A single-phase run processes only the requested phase and skips milestone close.
- Loop decisions are visible in the event timeline.

## Phase 5: Reusable Subgraphs

Goal: avoid turning GSD parity into one enormous graph.

Tasks:

- Add named subgraph definitions with inputs, outputs, and local nodes.
- Allow a Workflow node to invoke a subgraph.
- Version subgraphs with the parent Workflow snapshot at run start.
- Add subgraph templates:
  - milestone intake
  - smart discuss
  - phase planning
  - phase execution
  - code review and fix
  - verification routing
  - gap closure
  - UI review
  - milestone audit
  - milestone completion

Acceptance criteria:

- The GSD Auto template can be read as top-level lifecycle nodes rather than hundreds of flat nodes.
- A subgraph can be reused by multiple Workflow templates.
- Run inspection can drill from top-level node into subgraph node runs.

## Phase 6: Tool, Command, And Test Nodes

Goal: make verification and state discovery executable, not just agent-described.

Tasks:

- Add a generic command node with:
  - command allowlist
  - working directory binding
  - timeout
  - output capture
  - structured parser
  - success/failure mapping
- Add first-class test/check nodes for common project checks:
  - package script
  - Cargo check/test, one command at a time
  - TypeScript/Vitest
  - lint/format scoped runs
- Add artifact extraction from command output.
- Add approval policy for command nodes where needed.

Acceptance criteria:

- Verification can depend on actual check results, not only an agent's prose.
- Failed checks route to debug/gap closure with captured output.
- Command output is stored as evidence.

## Phase 7: Parallelism, Isolation, And Joins

Goal: support GSD's "parallel waves" where safe, while protecting repos by default.

Tasks:

- Add dependency-aware parallel groups.
- Add join/merge policies that understand successful, failed, skipped, and human-needed branches.
- Add resource scopes for repo-mutating nodes.
- Add optional worktree isolation per node or per parallel branch.
- Default to serialized execution for nodes that mutate the same repository scope.

Acceptance criteria:

- Independent planning/research nodes can run concurrently.
- Repo-mutating execution nodes serialize unless isolated.
- Merge nodes can summarize mixed branch outcomes.
- Cancellation propagates cleanly across a parallel group.

## Phase 8: Human Decision And UAT Nodes

Goal: model the human gates GSD uses without ad hoc chat prompts.

Tasks:

- Add typed human checkpoint schemas:
  - approve/adjust
  - choose one
  - choose many
  - freeform correction
  - manual verification result
- Store checkpoint decisions as durable artifacts and state events.
- Allow a checkpoint to update delivery state directly.
- Add resume payload validation.
- Add a compact UI for pending decisions in the Workflow run inspector.

Acceptance criteria:

- A phase with `human_needed` can pause, present verification items, resume as passed or gaps found, and route accordingly.
- A milestone intake checkpoint can revise the milestone summary before state is written.
- Decisions remain inspectable after run completion.

## Phase 9: Native GSD Auto Template

Goal: build the proving template that models GSD `/auto` in Xero.

Top-level graph:

1. Load delivery project state.
2. If no milestone exists, run milestone intake.
3. Discover incomplete delivery phases.
4. Iterate incomplete phases:
   - smart discuss
   - optional UI design contract
   - plan
   - execute
   - code review/fix
   - verify
   - route passed, human-needed, or gaps-found
   - optional UI review
   - mark phase complete or deferred
   - re-query roadmap
5. If run is partial or single-phase, summarize and stop.
6. Audit milestone.
7. Route audit passed, tech debt, or gaps found.
8. Complete/archive milestone.
9. Offer next milestone.

Tasks:

- Implement the template using only generic Workflow primitives from prior phases.
- Add seed data/template examples.
- Add tests that simulate:
  - all phases pass
  - gap closure succeeds
  - gap closure fails and asks human
  - inserted phase is picked up
  - single-phase mode skips lifecycle
  - milestone audit fails on unsatisfied requirement
  - milestone completes and archive is written

Acceptance criteria:

- The native Xero template can perform the same lifecycle shape as GSD `/auto`.
- The template does not require hard-coded GSD branches in the runtime.
- A user can run the template again to continue the same project or start the next milestone.

## Phase 10: Canvas Authoring UX

Goal: make the new power usable by humans.

Tasks:

- Add a Workflow palette for:
  - state nodes
  - collection loops
  - subgraphs
  - command/check nodes
  - human checkpoints
  - lifecycle templates
- Add property panels for schemas, bindings, loop filters, command parsers, and checkpoint options.
- Add validation diagnostics directly on nodes and edges.
- Add a run preview that explains:
  - required initial inputs
  - state reads/writes
  - loops
  - human checkpoints
  - possible terminal states
- Fix start-run UX so required inputs are collected before run creation.

Acceptance criteria:

- A user cannot start the GSD Auto template with an empty required goal.
- The canvas explains why a route did or did not fire.
- Editing a schema or binding shows validation feedback before save.
- The GSD Auto template is authorable and inspectable without reading code.

## Phase 11: Observability And Recovery

Goal: make long-running Workflows debuggable and resumable.

Tasks:

- Add run-level progress metrics.
- Add per-node event logs with state diffs and artifact validation results.
- Add "resume from failed node" and "resume from next incomplete delivery phase."
- Add "explain current blocker" output.
- Add export bundle for failed Workflow runs.
- Add stale-running-node detection and recovery.

Acceptance criteria:

- A failed milestone run can be resumed without restarting completed phases.
- The user can see exactly which requirement, phase, node, or check blocked progress.
- Support can inspect a run bundle without app-data spelunking.

## Phase 12: Hardening And Template Generalization

Goal: prove these primitives are not GSD-only.

Tasks:

- Build at least two non-GSD advanced templates using the same primitives:
  - release train
  - bug triage and fix loop
  - dependency upgrade campaign
  - security audit and remediation
- Remove any GSD-specific assumptions from runtime primitives.
- Add migration/wipe guidance for development app-data changes.
- Add performance tests for large milestones and large event histories.

Acceptance criteria:

- GSD Auto is just one template in a broader Workflow system.
- Runtime primitives remain domain-neutral.
- Large runs remain inspectable and responsive.

## Implementation Order

Recommended first slice:

1. Strict artifact contracts.
2. Durable delivery state.
3. State read/write nodes.
4. Required-input start-run UX.
5. Collection loop over incomplete delivery phases.

This first slice unlocks the core difference between "run this graph once" and "continue evolving a project over many runs."

Recommended second slice:

1. Reusable subgraphs.
2. Command/check nodes.
3. Human/UAT checkpoints.
4. GSD Auto template v1.

Recommended third slice:

1. Parallelism and isolation.
2. Milestone audit and completion hardening.
3. Run recovery/observability.
4. Non-GSD templates proving generality.

## Risks

- **Canvas complexity:** collection loops and subgraphs can make the graph hard to understand. Mitigate with collapsed subgraphs and run preview.
- **Schema friction:** strict contracts can annoy users if the agent misses the shape. Mitigate with generated final-response contracts and repair prompts.
- **State duplication:** durable state and run artifacts can drift. Mitigate with state events and idempotent writes.
- **Repo mutation conflicts:** parallelism can corrupt work if isolation is weak. Mitigate by serializing shared resource scopes by default.
- **Overfitting to GSD:** keep primitives generic and make GSD a template, not runtime behavior.

## Done Definition

This initiative is done when a fresh project can use the Xero canvas to:

1. create a milestone from a goal,
2. generate requirements and delivery phases,
3. run all incomplete phases,
4. recover through review/debug/gap loops,
5. pause for human verification when needed,
6. re-read the roadmap and pick up inserted phases,
7. audit requirements coverage,
8. complete/archive the milestone,
9. start or continue the next milestone,
10. and do all of that using generic Workflow primitives visible on the canvas.

