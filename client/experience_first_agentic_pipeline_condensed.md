# Experience-First Agentic Delivery Pipeline (Condensed)
## Rolling milestone planning, distributed experience assembly, brownfield onboarding, smart model routing, and fault containment

## Purpose

This design describes a SQLite-backed, experience-first, fresh-agent delivery pipeline. It gets to a believable product experience early, uses that experience to size and partition work into milestones, and deeply plans only the active milestone. It is designed for three entry modes: greenfield starts, brownfield onboarding, and continuation of an existing milestone horizon.

The architecture keeps durable memory in SQLite, keeps code in the repository, routes work to specialized fresh agents with minimal context, validates every output before acceptance, tests against fixtures instead of live systems, and requires refactoring after each passing slice. It replaces whole-project upfront planning and monolithic mock generation with rolling milestone planning plus distributed experience-envelope assembly.

## 1. Core principles

1. **Experience before internals.** Shape the visible user experience early; harden internals after the product promise is concrete.
2. **Fresh agent per work unit.** Agents are disposable; durable state lives outside the session.
3. **SQLite is orchestration memory.** Store state, milestones, decisions, blockers, digests, routing outcomes, provenance, and validation results in SQLite.
4. **Minimal context, maximal clarity.** Each agent gets only the smallest useful packet for its task.
5. **Thin vertical slices.** Build end-to-end value, protection, or enabling work instead of broad technical layers.
6. **Fixture-first testing.** Automated work must use fixtures, fakes, stubs, or deterministic adapters rather than live dependencies.
7. **Mandatory refactoring.** Every slice is refactored after it passes its required tests.
8. **Locked decisions.** Approved experience, preservation, and technical contracts are binding at their stated fidelity.
9. **Autonomous continuation.** The orchestrator continues while meaningful unblocked work exists and pauses only for real stop conditions.
10. **Brownfield awareness.** Existing systems change the entry path and require preservation contracts.
11. **Milestones are first-class units.** The product is long-lived; a milestone is the current bounded delivery increment.
12. **Model routing is orchestration policy.** Capability selection, fallbacks, budgets, and escalation rules are durable policy, not prompt trivia.
13. **Model output is untrusted until validated.** Outputs are proposals that must pass structural, semantic, scope, and policy checks.
14. **Preserve stable knowledge and plan only the delta.** Keep a coarse program map, a concrete active milestone, and a precise current work unit.
15. **Secrets are not ordinary records.** SQLite stores secret metadata and secure references, not plaintext credentials.
16. **Experience assembly is distributed and fidelity is selective.** Build shell, spine, clusters, and states separately; fully deepen only the surfaces needed now.

## 2. High-level operating model

### 2.1 Thin orchestrator

The orchestrator does not solve the project itself. Its job is to:

- read program, milestone, blocker, routing, and readiness state,
- decide the next best work unit,
- assemble a minimal context packet,
- choose a model capability profile,
- launch a specialist agent,
- stage and validate output,
- accept only valid output into durable state or the repository,
- update blockers, digests, dependencies, and milestone progress,
- continue until only true stop conditions remain.

### 2.2 Nested scopes and planning depth

The system works across three scopes:

- **Program:** long-lived product memory, stable constraints, program core, approved experience envelope, and coarse milestone horizon.
- **Active milestone:** the currently promoted increment with a concrete contract, blockers, readiness state, and definition of done.
- **Work unit:** the smallest routable item, such as a research task, screen spec, mock shard, slice, repair, or verifier run.

Planning depth should remain explicit:

- **Program:** coarse direction plus approved experience envelope.
- **Future milestones:** shell-level intent, dependencies, key risks, and promotion conditions.
- **Active milestone:** deep experience detail when required, technical derivation, and slice planning.
- **Current work unit:** precise acceptance criteria and allowed scope.

### 2.3 Entry modes

- **Greenfield start:** new product; run ideation and experience shaping before deep partitioning.
- **Brownfield start:** existing product or repository; map reality first, then shape the next increment against observed constraints.
- **Milestone continuation:** reuse program core and horizon; treat the new request as a delta and repartition only if needed.

### 2.4 Default control loop

1. Read program state, active milestone state, milestone horizon, locks, blockers, readiness, and open work.
2. Refresh only the repository or brownfield digests relevant to the next decision.
3. If no approved experience envelope exists, or the current one is too coarse, deepen only the necessary experience work.
4. If no active milestone exists, or the current horizon is no longer credible, run milestone sizing and partitioning.
5. Keep future milestones shallow until activated.
6. Infer missing prerequisites and dependencies.
7. Skip blocked work when other useful work exists.
8. Batch missing user inputs.
9. Assemble minimal context.
10. Route to a model profile.
11. Stage, validate, and accept only conforming output.
12. Recompute blockers, dependencies, and milestone eligibility.
13. Continue automatically until no meaningful unblocked work remains.

### 2.5 Stop conditions

Pause only when there is no other valuable unblocked work and one of the following is true:

- a required user choice is unresolved,
- approval or sign-off is mandatory,
- a credential, permission, or account is truly required,
- a compliance decision must come from the user,
- output repeatedly fails validation and cannot be repaired safely,
- the active milestone boundary is too unclear to infer responsibly,
- or a hard external blocker prevents further safe progress.

### 2.6 Major phases

- **Phase B:** Brownfield reconnaissance and knowledge seeding
- **Phase 0:** Guided ideation or program shaping
- **Phase 1:** Structured intake, brownfield refresh, and execution readiness
- **Phase 2:** Program framing and horizon setup
- **Phase 3:** Experience discovery
- **Phase 4:** Targeted experience research
- **Phase 5:** Interaction architecture, impact mapping, and mock decomposition planning
- **Phase 6:** Distributed experience-envelope assembly
- **Phase 7:** UX review and lock
- **Phase 8:** Scope sizing, milestone partitioning, and activation
- **Phase 9:** Active-milestone technical derivation, targeted mock deepening, and delta impact analysis
- **Phase 10:** Active-milestone slice planning and rolling schedule
- **Phase 11:** Autonomous slice execution loop
- **Phase 12:** Milestone hardening, release readiness, and continuation planning

## 3. Milestones, experience depth, and continuation

### 3.1 Milestone 1 is the first committed subset

Milestone 1 is the first operational subset that makes the product promise real. It is not a commitment to exhaustively plan or build the entire program in one pass.

### 3.2 Experience envelope before deep partitioning

For greenfield work, the system should first lock a believable experience envelope: global shell, primary journey, shared interaction rules, shell-level coverage of major later capabilities, and deeper state coverage only where milestone boundaries depend on it. The goal is not a fully detailed whole-product mock; it is enough experience clarity to size and split work responsibly.

Brownfield and continuation work can often use a narrower delta envelope, but the same rule applies: do not deeply decompose work until the changed experience is concrete enough to reason about.

### 3.3 Milestone count is discovered, not predeclared

Milestone count is derived from:

- breadth of user journeys,
- integration density and volatility,
- edge-case and state complexity,
- preservation risk in existing surfaces,
- migration cliffs or irreversible changes,
- testability and release safety,
- and how much learning is still needed.

Small, low-risk scopes may collapse into one milestone. Larger or riskier scopes should expand into an ordered set of milestone shells.

### 3.4 Phase-skipping and fidelity rules

- Run ideation fully for greenfield work; run it narrowly for brownfield shaping; skip most of it for concrete continuations.
- Experience simulation is required when visible behavior changes, targeted when only a small area changes, and skippable only when behavior is already fully specified.
- Research should answer the next real decision, not map the entire future roadmap.
- Milestone partitioning is mandatory after UX lock when true scope is still unclear.
- Brownfield refresh is heavy at onboarding and incremental later.
- Technical derivation and slice planning are deep only for the active milestone.
- Later surfaces remain shell-level unless early fidelity is necessary for safe partitioning or implementation.

### 3.5 Milestone close-out and carry-forward

Every milestone close-out should:

- audit success against the milestone contract,
- extract discovered follow-on work and dependencies,
- recut the horizon when learnings changed sequencing,
- update stable program truth when the milestone changed it,
- and prepare the next activation if continuation makes sense.

Carry-forward items should be classified as:

- **future milestone shells** for likely next increments,
- **future seeds** for ideas tied to later conditions,
- **backlog items** for known but inactive work,
- **threads/investigations** for cross-milestone knowledge.

## 4. Phase-by-phase design

### Phase B — Brownfield reconnaissance and knowledge seeding

**Objective:** Build a trustworthy picture of an existing product, repository, and document set before planning.

**Key work:** Map repository topology, ingest docs and decisions, inventory runtime and dependencies, infer behaviors and contracts, map tests and fixtures, detect hotspots and weakly tested zones, and seed trust-scored knowledge.

**Key outputs:** brownfield snapshot, repository topology, dependency and runtime inventories, behavior map, test landscape, hotspots, brownfield risks, knowledge seeds, entry recommendation.

**Exit gate:** The system can answer what exists, what must be preserved, what is risky, what technologies and integrations are already present, and whether to proceed with full shaping, narrow shaping, or direct intake.

### Phase 0 — Guided ideation or program shaping

**Objective:** Turn a blank-page request or continuation request into a coherent product direction and first-value experience target.

**Key work:** Frame the problem, identify impacted users and contexts, define preservation boundaries for brownfield work, clarify success and constraints, explore candidate directions when needed, build a coarse capability map, articulate an experience thesis, and separate “now” from “later.”

**Key outputs:** program brief, milestone brief, problem statement, target users, constraints, success metrics, scope and preservation boundaries, capability map, selected direction, experience thesis, open questions, locked decisions.

**Exit gate:** There is a coherent objective, success criteria, a coarse capability map, a preservation boundary when relevant, and enough clarity to design an early experience artifact.

### Phase 1 — Structured intake, brownfield refresh, and execution readiness

**Objective:** Convert intent into a structured requirement profile, readiness model, and early dependency plan.

**Key work:** Capture product and team context, technology preferences, environment and deployment assumptions, integration inventory, credential and access needs, scope and complexity signals, model policy constraints, user-input batches, and any needed brownfield refresh.

**Key outputs:** structured requirement profile, technology and deployment preferences, integration requirements, credential and access requirements, brownfield constraints, scope and complexity signals, planning-horizon hints, model policy preferences, input manifest, readiness check, blockers, user input request batch.

**Exit gate:** Near-term requirements are classified as known, inferred, must-ask-now, needed-later, or optional, and the system has explicit signals for later milestone sizing.

### Phase 2 — Program framing and horizon setup

**Objective:** Create stable program memory and a provisional planning horizon.

**Key work:** Distill shaping and readiness outputs into compact cores, initialize state, define terminology and non-goals, summarize risks and blockers, and create a provisional milestone frame when needed.

**Key outputs:** program core, milestone core, milestone horizon policy, glossary, non-goals, program and milestone state, risk register, readiness summary.

**Exit gate:** Later agents can understand the product direction, current request, and planning horizon from the cores alone.

### Phase 3 — Experience discovery

**Objective:** Define the journeys, moments of value, setup touchpoints, changed journeys, preserved journeys, and failure conditions that the early experience must represent.

**Key work:** Map journeys and journey deltas, capture jobs-to-be-done, identify onboarding and setup touchpoints, enumerate failure modes, and lock preserved experience constraints for brownfield work.

**Key outputs:** journeys, journey deltas, job statements, moments of value, failure modes, onboarding flow, setup touchpoints, permission hints, preserved experience constraints.

**Exit gate:** The system has explicit primary outcomes, setup or permission touchpoints, changed and preserved journey segments, and key boundary conditions.

### Phase 4 — Targeted experience research

**Objective:** Research only the patterns, edge cases, and risks needed to justify the current design and next milestone decision.

**Key work:** Pattern research, comparable-product review, accessibility and inclusion guidance, edge-case enumeration, and brownfield conflict detection.

**Key outputs:** pattern findings, UX recommendations, accessibility requirements, edge-case sets, risk notes, brownfield conflict notes.

**Exit gate:** The design has enough pattern and edge-case support to move into interaction architecture without pretending future milestones have already been fully researched.

### Phase 5 — Interaction architecture, impact mapping, and mock decomposition planning

**Objective:** Turn the approved direction into a screen system, interaction model, impact map, and build graph that can be assembled through bounded agents.

**Key work:** Define routes or flows, screen purposes, view models, state matrices, interaction rules, impact maps, compatibility guards, experience build graphs, surface clusters, fidelity tiers, and shared UI contract outlines.

**Key outputs:** route map, screen specs, view models, state matrix, interaction rules, impact map, compatibility guards, experience build graph, surface clusters, fidelity tiers, shared UI contract outline, permission and visibility rules.

**Exit gate:** Each important changed surface has a purpose, entry and exit points, actions, data needs, state coverage, preservation expectations, impact mapping, cluster assignment, and a fidelity target.

### Phase 6 — Distributed experience-envelope assembly

**Objective:** Build an early believable experience through multiple bounded work units rather than one giant prototype run.

**Key work:** Confirm the build graph; build the global shell and navigation scaffold; define shared UI and state contracts; assemble the experience spine; build surface-cluster shards; inject loading, empty, error, permission, disconnected, and degraded states; stitch shards into one navigable envelope; verify coherence; selectively deepen only the surfaces needed now.

**Key outputs:** experience envelope, build graph, shell surfaces, shared UI contract, mock shards, state packs, stitch results, coherence report, mock gaps, fidelity assignments.

**Exit gate:** The user can navigate a coherent experience artifact, the core value loop is believable, major capabilities are at least represented as shells, likely early-milestone surfaces are detailed enough, and any undeepened areas are intentionally tagged as shell-level or deferred.

### Phase 7 — UX review and lock

**Objective:** Turn feedback on the experience envelope into binding contracts while making explicit what is locked at detailed fidelity versus shell-level direction.

**Key work:** Orchestrate review, synthesize feedback, lock approved experience contracts, check preservation boundaries, and triage gaps or change requests.

**Key outputs:** UX feedback, approved program experience contract, approved milestone experience contract, preservation contract, locked decisions, change requests, active-surface priorities.

**Exit gate:** Approved experience and preservation contracts are durable and binding at their stated fidelity.

### Phase 8 — Scope sizing, milestone partitioning, and activation

**Objective:** Use the approved experience envelope and current constraints to decide whether the work fits in one milestone or several, create an ordered horizon, and activate only the next milestone for deep planning.

**Key work:** Assess scope, partition around user value steps, dependency cliffs, uncertainty concentrations, preservation risks, irreversible changes, and release safety; order milestones by risk and value; write promotion conditions; select the active milestone; identify active versus deferred surfaces.

**Key outputs:** scope assessment, milestone shells, milestone order, planning horizon, milestone activation, promotion conditions, deferred capabilities, milestone dependencies, active surface set, deferred surface shells.

**Exit gate:** The active milestone has a crisp contract, future milestones exist only as coarse shells, deferred work is explicitly recorded, and any required active-surface deepening is scheduled.

### Phase 9 — Active-milestone technical derivation, targeted mock deepening, and delta impact analysis

**Objective:** Derive the technical shape only for the active milestone and deepen only the surfaces that are still too coarse for safe implementation.

**Key work:** Analyze feasibility, define domain entities and contracts, write policy and validation rules, map integration and credential boundaries, analyze delta impact on the existing system, plan migrations and rollback, write future-boundary notes, and deepen active milestone mock detail when needed.

**Key outputs:** technical shape, domain entities, API and event contracts, validation and policy rules, integration boundaries, credential binding specs, delta impact map, migration plan, rollback plan, future-boundary notes, active surface contract, milestone mock delta, latency budget.

**Exit gate:** The system knows what must be built now, what existing structures must change, which integrations are real versus fixture-backed, what migration protections are needed, and what future work should stay shallow.

### Phase 10 — Active-milestone slice planning and rolling schedule

**Objective:** Break the active milestone into thin end-to-end slices without deeply planning future milestones.

**Key work:** Create feature, migration, preservation, enablement, and hardening slices; map dependencies; write acceptance criteria; plan tests and fixtures; set execution priority; classify blockers; assign routing classes and model-route hints; enforce horizon guards.

**Key outputs:** slices, slice plan, dependency edges, acceptance criteria, test matrix, fixture plan, execution priority, blocker strategy, waves, routing class, model-route hints, milestone rollover hints.

**Exit gate:** Each slice has a single purpose, acceptance criteria, required tests, required fixtures, allowed file scope, dependency position, blocker classification, and routing class.

### Phase 11 — Autonomous slice execution loop

**Objective:** Execute the active milestone through disciplined, repeatable slice-level work.

**Loop structure:**

1. **Model routing:** choose primary model capability, verifier profile, budget, fallback chain, and escalation conditions.
2. **Test design:** expand the slice into explicit test cases and fixture requirements.
3. **Implementation:** build only the assigned slice, stay inside the allowed scope, and use fixture-backed boundaries.
4. **Staging and validation:** stage output and check structure, schema, references, scope, preservation, and policy compliance.
5. **Test execution:** write missing tests if needed and run the required matrix.
6. **Mandatory refactoring:** improve internal structure while preserving behavior and contracts.
7. **Regression verification:** rerun relevant checks and confirm acceptance criteria, UX conformance, and preservation compliance.
8. **State update:** close the slice, update dependencies and blockers, store digests and routing outcomes.
9. **Continue or pause:** continue automatically when eligible work exists; batch requests when all paths are blocked; advance to milestone hardening when the active milestone is complete.

**Key outputs:** model route decisions, test cases, implementation summary, staged output, validation result, repair request or quarantine item, test results, refactor summary, verification result, slice status update, run digest, blocker update, routing outcome, next action.

### Phase 12 — Milestone hardening, release readiness, and continuation planning

**Objective:** Run broader milestone checks, decide readiness, refresh the horizon, and prepare continuation.

**Key work:** Verify wave-level behavior, audit accessibility and performance, check security and policy, assess live readiness, audit milestone completion, synthesize seeds and backlog, update the milestone horizon, summarize release readiness.

**Key outputs:** wave verification, accessibility audit, performance note, integration readiness, release readiness, milestone audit, future seeds, backlog candidates, milestone horizon update, next milestone options, program digest.

**Exit gate:** The system knows whether the milestone met its definition of done, what unresolved risks remain, what live validations are still pending, what future work surfaced, and whether another milestone should activate now.

## 5. SQLite-native data model

### 5.1 Storage strategy

Use SQLite for orchestration state and typed records, with the repository remaining the source of truth for code.

**Control/state tables:** projects, program state, milestones, milestone horizons, milestone state, work units, agent runs, dependencies, locks, input requirements, stop conditions.

**Knowledge and brownfield tables:** codebase snapshots, document ingestions, dependency inventories, behavior maps, test landscapes, hotspots, knowledge seeds.

**Readiness/access tables:** credential requirements, credential bindings, access requirements, user preferences, integration targets, deployment targets.

**Routing tables:** model policies, model assignments, fallback events, escalation events, routing outcomes.

**Experience-assembly tables:** experience surfaces, build graphs, surface clusters, mock shards, state packs, stitch runs, coherence reports.

**Validation tables:** staged outputs, validation runs, repair runs, quarantine items, acceptance journal.

**Code/test/trace tables:** source assets, test cases, test results, fixture sets, verification results, refactor cycles, trace links, decision links, context links, blocker links.

### 5.2 Generic record envelope

The generic record store should preserve, at minimum:

- project scope,
- milestone scope,
- record type and key,
- version,
- status and lock state,
- trust level,
- schema version,
- tags,
- structured payload,
- human-readable summary,
- provenance,
- supersession link,
- creation timestamp.

This keeps the system flexible without forcing a schema migration for every new record type.

### 5.3 Important record families

Representative record types include:

- program and milestone briefs/cores,
- experience envelopes and build graphs,
- surface clusters, mock shards, and state packs,
- brownfield snapshots and knowledge seeds,
- milestone shells, planning horizons, and activation records,
- preservation contracts and active surface contracts,
- technical shape, delta impact maps, migration and rollback plans,
- slice plans, test matrices, fixture scenarios,
- model route decisions,
- staged outputs, validation results, and quarantine items,
- implementation, refactor, and verification summaries,
- future seeds, backlog candidates, milestone horizon updates,
- user-input request batches and run digests.

### 5.4 Versioning, trust, and continuity

Every important output should be versioned and never silently overwritten. When a locked record changes, the old version remains, the new one supersedes it, and downstream links can detect whether replanning is required.

Useful trust and acceptance states:

- **observed:** directly detected from repository, docs, or execution,
- **inferred:** model-derived but not yet validated,
- **validated:** passed structural and semantic checks,
- **locked:** approved and binding,
- **superseded:** replaced by a later accepted version,
- **quarantined:** rejected from normal flow.

### 5.5 Special storage rules

- **Code:** keep source in the repository; SQLite stores references, checksums, summaries, dependencies, and ownership.
- **Secrets:** keep only provider, owner, scope, readiness, secure reference, expiration, and validation timestamp in SQLite.
- **Blockers:** store blocker type, severity, owner, earliest affected phase, grouped request key, unblock action, alternative-work status, surfaced status, and blocker class.
- **Milestone continuity:** treat milestone shells, planning horizons, promotion conditions, future seeds, backlog candidates, thread references, next milestone options, and milestone summaries as first-class records.

## 6. Context engineering

### 6.1 Context layers

A well-formed context packet is assembled from layers:

- **Program core:** stable direction, experience-envelope summary, horizon summary, glossary, locked high-level decisions, stable constraints.
- **Milestone core:** active milestone objective, scope, success metrics, preserved behaviors, blockers, active surfaces.
- **Phase contract:** records specific to the current phase, such as brownfield findings, UX specs, technical contracts, or verification criteria.
- **Current work unit:** the exact slice, shard, investigation, or validation task.
- **Relevant history digests:** short prior summaries ranked by relevance, dependency relation, and recency.
- **Brownfield/preservation state:** hotspots, preserved behaviors, compatibility guards, impacted legacy behavior.
- **Readiness/model policy:** technology preferences, access and credential status, routing policy, budgets, compliance constraints.
- **Local source neighborhood:** touched files, nearby interfaces, related tests, fixtures, one-hop dependencies.
- **Output contract/trust policy:** what the agent may emit and what validation level its output must satisfy.

### 6.2 Assembly rules

1. Prefer stable cores over raw history.
2. Load preservation constraints early for brownfield work.
3. Filter by milestone and work-unit tags.
4. Use digest-first recall instead of large transcripts.
5. Include readiness and routing data only when they affect the task.
6. Limit code context to the local neighborhood.
7. Keep future milestones as short shells, not full plans.
8. Never materialize deep future plans until a milestone is active.
9. Batch missing user inputs.
10. Schema-validate all outputs.
11. Carry trust levels forward.

### 6.3 Minimal context by agent family

- **Brownfield mappers:** repository structure, docs, deployment hints, and the specific area being refreshed.
- **Shapers/partitioners:** program core, current request, constraints, scope signals, risks, active-surface priorities, and current product state.
- **Research agents:** milestone core, journey or question, user type, and scope constraints.
- **Screen/build-graph/UX agents:** journeys, UX recommendations, accessibility rules, preservation constraints, impacted surfaces, shell or cluster boundaries.
- **Shell/cluster/state-pack builders:** only the assigned shell, cluster, or state scope plus shared UI and state contracts.
- **Stitchers/coherence verifiers:** shell contract, cluster outputs, navigation graph, cross-cluster rules, fidelity tags.
- **Contract writers/technical derivation agents:** active milestone contract, active surfaces, domain assumptions, preservation contracts, integration requirements, technology preferences.
- **Implementers/testers/refactorers:** slice plan, acceptance criteria, fixture plan, local file neighborhood, recent digests, and relevant blockers or readiness state.
- **Verifiers:** acceptance criteria, milestone contract, preservation contract, test results, and locked decisions.
- **Repair agents:** rejected output, validation failures, output contract, and the smallest context needed to repair shape or scope.

## 7. Smart model routing

### 7.1 Objective and routing dimensions

Route each work unit to the best-fit capability profile rather than forcing one model to do everything. Consider:

- work type and modality,
- need for long-context synthesis,
- structured-output reliability,
- UX or UI judgment,
- precise code editing,
- tool use,
- latency and cost tolerance,
- compliance and provider restrictions,
- recent failure history on similar tasks.

### 7.2 Capability profiles by work class

- **Intake/extraction/classification:** low-cost, fast, structure-reliable models.
- **Research/synthesis:** models strong at broad comparison and long-context summarization.
- **UX architecture/mock decomposition:** models strong at interface reasoning, clustering, and state coverage.
- **Mock shard building/UI stitching:** UI-strong builders plus a separate consistency-focused verifier.
- **Architecture/contract derivation:** models strong at systems reasoning and constraint consistency.
- **Focused implementation/repair:** code-editing specialists for narrow, local changes.
- **Refactoring:** models strong at local structural improvement and behavior preservation.
- **Verification/policy checking:** skeptical verifier profiles, ideally independent from the implementer.
- **Output repair/normalization:** cheap deterministic-leaning profiles first, escalating only when necessary.
- **Summarization/digest creation:** lower-cost models unless the digest is strategically cross-cutting.

### 7.3 Policy lifecycle

- **Capture:** collect provider restrictions, compliance limits, cost ceilings, and preferred model families during intake.
- **Assign:** record the chosen capability profile, fallback chain, budget, and rationale before each agent run.
- **Validate:** link outcomes and validation failures back to the route that produced them.
- **Learn:** refine routing policy over time using success rates, failure modes, and cost patterns.

### 7.4 Escalation, fallback, and independence

Start cheaper for low-risk, repetitive, or highly structured work. Escalate for high ambiguity, high risk, repeated schema failures, contradictory outputs, broad risky diffs, preservation violations, or repeated test failures. Downshift when patterns stabilize.

Implementation and verification should not share the same cognitive lane when it is practical to separate them. Different model families, routes, or at least distinct verifier framing reduce correlated blind spots.

### 7.5 Policy versus configuration

**Policy** should define work classes, capability requirements, escalation rules, independence rules, and acceptance expectations.

**Configuration** should define provider/model IDs, per-environment overrides, cost ceilings, enterprise restrictions, and temporary runtime availability.

## 8. Output hardening and fault containment

### 8.1 Core rule

Treat every model output as a proposal. Nothing mutates durable state or repository code until it is staged, validated, and accepted.

### 8.2 Acceptance pipeline

1. Stage raw output.
2. Canonicalize obvious harmless formatting issues.
3. Validate syntax and schema.
4. Validate references and semantics.
5. Check scope, preservation, and policy.
6. Accept atomically if valid.
7. Otherwise repair or quarantine.

### 8.3 Validation layers

- **Structural validation:** parseability and shape.
- **Contract validation:** conformance to the output contract.
- **Semantic validation:** real and coherent references, files, and identifiers.
- **Scope validation:** milestone and file-scope compliance.
- **Preservation validation:** no violation of locked preservation contracts.
- **Policy validation:** no security, compliance, or operational violations.

### 8.4 Protecting state and repository mutations

To protect SQLite:

- never write unvalidated output into accepted records,
- use append-only staging tables,
- accept through atomic transactions,
- version schemas and records,
- keep idempotent run identifiers,
- journal acceptance decisions for replay and recovery.

To protect the repository:

- require allowed file scope per work unit,
- check existence and target boundaries before mutation,
- keep pre- and post-acceptance checksums or summaries,
- independently validate destructive operations,
- require tests or preservation checks before merge-worthy acceptance.

### 8.5 Repair, quarantine, recovery, and observability

Repair should escalate gradually:

1. deterministic normalization,
2. narrow self-repair using explicit validation errors,
3. alternate-model repair,
4. quarantine if still unsafe.

Quarantine records should state what failed, why, what it affected, what repair was attempted, and what remediation work should follow.

Recovery should always restart from the last accepted durable state, not from partially written artifacts.

Track at least validation failure rates by work class, quarantine rates by model route, common repair reasons, schema drift, preservation violations, shard compatibility failures, and expensive or failure-prone phases.

### 8.6 Distributed mock compatibility

Mock shards should declare the routes they own, the shared components they depend on, the state or fixture contracts they expect, and the shell version they target. The stitcher should reject incompatible shards, missing state obligations, broken route links, or shared-component drift instead of presenting an incoherent experience envelope.

## 9. Testing, fixtures, and refactoring

### 9.1 Fixture-first testing

No work unit is complete until it is covered by the required tests, and automated tests must not rely on live external services.

Every external dependency should sit behind an interface or adapter boundary, including AI providers, payment systems, email services, analytics, search, remote APIs, databases, and third-party auth. Tests and prototype work use fixtures, fakes, stubs, or deterministic adapters.

### 9.2 Missing-credential and brownfield rules

Missing live credentials should not block experience-envelope work, slice implementation, or automated tests when a fixture-backed boundary exists. Credentials become blockers only for work that truly requires live access, such as deployment, provisioning, or real end-to-end validation.

Brownfield work must test both the new milestone behavior and a preservation suite for risky adjacent existing behavior.

### 9.3 LLM-specific fixture scenarios

If the product uses an AI model, tests should cover fixture-backed scenarios such as:

- ideal result,
- low-confidence result,
- malformed output,
- refusal,
- timeout,
- partial tool output,
- hallucination-like answer,
- rate-limit-like failure.

### 9.4 Test categories and completion rule

Useful categories include unit, component, integration with fixture-backed boundaries, contract, migration, preservation, regression, and multi-state scenario tests.

A slice only closes when required tests exist, required tests pass, important states are covered by fixtures, preservation checks pass where relevant, post-refactor verification is green, and any live-readiness blockers are recorded honestly.

### 9.5 Mandatory refactoring

Refactoring runs after a slice is functionally correct and green under required tests.

Allowed work includes simplifying, extracting, renaming, reorganizing local structure, improving adapter boundaries, reducing duplication, improving fixture and test reuse, and making future slices easier.

Refactoring may not alter user-visible behavior, break preservation contracts, widen scope into unrelated modules, or silently redesign the system. If a broader refactor is best, the agent should emit a refactor candidate for intentional later scheduling.

Every refactor is followed by regression verification.

## 10. Representative lifecycle patterns

### 10.1 Brownfield improvement

When starting from an existing product, the system first maps the codebase, existing integrations, tests, and hotspots. It then runs narrow shaping around the requested improvement, designs only the affected journeys, builds only the affected experience surfaces, partitions the work into milestones if needed, and adds preservation slices wherever the surrounding system is fragile.

### 10.2 Later milestone continuation

When a product already has program core and milestone history, the system reuses that memory, runs a targeted brownfield refresh for only the relevant areas, skips broad ideation, locks only the changed experience, partitions the new request if needed, and deeply plans only the next active increment.

### 10.3 Large greenfield program

For a broad new product, the system defines journeys and surface clusters, builds a shell and experience spine first, keeps later capabilities at shell fidelity, gets the outer experience reviewed, partitions the program into milestones, and deepens only the surfaces in the active milestone before technical derivation and slice planning.

## 11. Practical implementation guidance

### 11.1 Highest-leverage optimizations

1. Make rolling-horizon planning a first-class service.
2. Put milestone partitioning after UX lock for greenfield work.
3. Separate the program-level experience envelope from active-milestone deep detail.
4. Replace monolithic prototype generation with explicit experience build graphs.
5. Make shell, shard, state-pack, and stitch passes first-class work types.
6. Cache shared UI contracts and design primitives early.
7. Build one engine with three entry modes rather than three separate pipelines.
8. Separate stable program memory, active milestone memory, and future milestone shells from day one.
9. Add cached brownfield digests plus incremental refresh.
10. Make preservation contracts explicit.
11. Treat model routing as a policy layer, not a hardcoded switch.
12. Build validation, acceptance, and quarantine early.
13. Budget research to the next real decision.
14. Use fidelity tiers so later surfaces can stay intentionally coarse.
15. Use risk-tiered verification.
16. Promote milestone shells, future seeds, backlog items, and refactor candidates to first-class outputs.
17. Tune the scheduler to work around blockers.

### 11.2 Practical build order

1. Build the milestone-aware SQLite schema, horizon state, experience tables, and acceptance journal.
2. Build brownfield mapping and trust-scored knowledge seeding.
3. Build the entry router and phase-skipping rules.
4. Build readiness capture, credential tracking, scope signals, and model policy capture.
5. Build program core, experience-envelope, build-graph, and context-packer services.
6. Build experience-side agents and distributed mock assembly.
7. Build milestone partitioning and activation logic.
8. Build active-milestone deepening, technical derivation, delta impact, and preservation contracts.
9. Build active-milestone slice planning, scheduling, and model routing.
10. Build the execution loop with validation, repair, testing, refactoring, and verification.
11. Build milestone hardening, horizon refresh, and continuation planning.

## 12. Final operating rules

1. Every agent is fresh; durable memory lives in SQLite.
2. Every handoff is structured and versioned.
3. Map existing codebases before planning against them.
4. Show an early experience envelope before hardening deep implementation plans.
5. Build mocks through distributed shell, shard, state-pack, and stitch work.
6. Discover milestone count from scope; do not assume one milestone.
7. Partition deeply only after experience lock when true scope is still uncertain.
8. Deeply plan only the active milestone.
9. Keep future milestones as coarse shells until activated.
10. Keep program core, experience envelope, active milestone core, and horizon state separate.
11. Use preservation contracts for brownfield safety.
12. Make model selection explicit, stored, and revisable.
13. Validate all outputs before accepting them into state or the repository.
14. Continue automatically while meaningful unblocked work exists.
15. Batch missing user inputs and access needs.
16. Store secrets as secure references, not plaintext.
17. Deepen only the active surface set when more mock detail is needed.
18. Break active milestones into thin slices.
19. Test every slice with fixtures rather than live dependencies.
20. Refactor every slice after it works.
21. Preserve traceability across decisions, records, code changes, and blockers.
22. Treat approved experience, shell-level direction, and preservation contracts as binding at their stated fidelity.
23. Repair or quarantine corrupt output; never force it into accepted state.
24. Use milestone close-out to refresh the continuation surface and recut the horizon if needed.

## Summary

This architecture is a SQLite-native, experience-first, fresh-agent delivery system for greenfield starts, brownfield onboarding, and milestone continuation. It gets to a credible product experience early, uses that experience to size and order milestones, deeply plans only the active milestone, and executes that milestone through thin slices with model routing, validator-first acceptance, fixture-backed testing, mandatory refactoring, and milestone-aware continuation. The result is a system that can keep evolving a product without over-planning the future, overloading a single context window, or letting malformed model output corrupt durable state.
