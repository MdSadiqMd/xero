
# Outcome-First Agentic Delivery Pipeline (Condensed)
## Rolling milestone planning, modality-aware evidence assembly, brownfield onboarding, smart model routing, and fault containment

## Purpose

This design describes a SQLite-backed, outcome-first, fresh-agent delivery pipeline. It makes the intended **development outcome** concrete early, uses that concrete artifact to size and partition work into milestones, and deeply plans only the active milestone. It is designed for three entry modes: greenfield starts, brownfield onboarding, and continuation of an existing milestone horizon.

The crucial change from the earlier experience-first model is that the early artifact is no longer assumed to be visual. For a UI-heavy product it may still be an experience envelope, but for other development work it may be an API contract bundle, a compiler corpus and diagnostics pack, a library example set, a docs information architecture, or an infrastructure rollout pack.

This workflow is intentionally bounded to **development-related work**: applications, APIs, services, CLIs, compilers, toolchains, libraries, SDKs, docs, data or schema work, migrations, quality or security work, and infrastructure or platform changes.

## 1. Core principles

1. **Reviewable outcome before deep internals.** Make the milestone concrete early in the strongest modality available.
2. **Milestone contract before deep partitioning.** Do not deeply decompose work until “done” is explicit.
3. **Evidence packs are modality-aware.** UI packs are one subtype, not the universal abstraction.
4. **Scenario packs are mandatory.** Capture happy paths, failures, edge cases, degraded cases, compatibility cases, migrations, and examples.
5. **Stability contracts matter.** Brownfield preservation is one subtype of a broader stability model.
6. **Fresh agent per work unit.** Durable memory lives outside the agent.
7. **SQLite is orchestration memory.** Store state, milestones, contracts, blockers, provenance, and validation in SQLite.
8. **Minimal context, maximal clarity.** Each agent gets only the smallest useful packet.
9. **Thin vertical slices.** Build end-to-end value, protection, or enabling work.
10. **Fixture-first testing.** Use fixtures, fakes, stubs, and deterministic adapters instead of live dependencies.
11. **Mandatory refactoring.** Every slice is refactored after it passes required tests.
12. **Locked contracts.** Approved contracts are binding at their stated fidelity.
13. **Autonomous continuation.** Continue while meaningful unblocked work exists.
14. **Milestones are first-class units.** The program is long-lived; the active milestone is the bounded delivery increment.
15. **Model routing is orchestration policy.** Capability selection and fallbacks are durable policy, not prompt trivia.
16. **Model output is untrusted until validated.** Stage, validate, accept, or quarantine.
17. **Plan only the delta.** Keep a coarse program map, a concrete active milestone, and a precise current work unit.
18. **Secrets are not ordinary records.** Store secure references and readiness metadata, not plaintext secrets.
19. **Evidence assembly is distributed and fidelity is selective.** Build scaffold, backbone, clusters, and scenario packs separately; deepen only the targets needed now.

## 2. High-level operating model

### 2.1 Core planning artifacts

- **Milestone contract:** what must be true when the milestone is done.
- **Evidence pack:** the smallest reviewable artifact set that makes the milestone concrete enough to review, partition, and implement safely.
- **Scenario pack:** the cases that matter for design and verification.
- **Stability contract:** what must remain true while other things change.
- **Validation profile:** how the milestone will later be proven correct.

### 2.2 Nested scopes and planning depth

- **Program:** long-lived product memory, stable constraints, approved outcome summary, coarse milestone horizon.
- **Active milestone:** current bounded increment with a concrete contract, blockers, readiness state, and definition of done.
- **Work unit:** the smallest routable task.

Planning depth:

- **program** = coarse core plus approved evidence-pack summary,
- **future milestones** = shell-level intent, dependencies, risks, and promotion conditions,
- **active milestone** = deep contract, targeted evidence detail, technical derivation, and slices,
- **current work unit** = exact task with precise acceptance criteria.

### 2.3 Work classification

Before choosing an evidence-pack shape, classify the work across these axes:

- **work class:** feature, migration, refactor, docs, quality, perf, security, infra, or research;
- **primary modality:** UI, API or service, CLI or toolchain, compiler, library or SDK, docs, infra or ops, data or schema, or internal code;
- **primary consumers:** end users, developers, operators, maintainers, writers, or internal systems;
- **proof style:** human review, contract tests, golden tests, differential tests, linting, dry runs, benchmarks, or rehearsals.

### 2.4 Entry modes

- **Greenfield start:** shape the outcome before deep partitioning.
- **Brownfield start:** map reality first, then shape the next increment against observed constraints.
- **Milestone continuation:** reuse program core and horizon; treat the new request as a delta.

### 2.5 Default control loop

1. Read program state, active milestone state, horizon, locks, blockers, readiness, and open work.
2. Refresh only the repository or brownfield digests relevant to the next decision.
3. If no approved evidence pack exists, or the current one is too coarse, deepen only the necessary evidence work.
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

### 2.6 Stop conditions

Pause only when there is no other valuable unblocked work and one of the following is true:

- a required user choice is unresolved,
- approval or sign-off is mandatory,
- a credential, permission, or account is truly required,
- a compliance decision must come from the user,
- output repeatedly fails validation and cannot be repaired safely,
- the active milestone boundary is too unclear to infer responsibly,
- or a hard external blocker prevents further safe progress.

### 2.7 Major phases

- **Phase B:** Brownfield reconnaissance and knowledge seeding
- **Phase 0:** Guided ideation or program shaping
- **Phase 1:** Structured intake, brownfield refresh, work classification, and execution readiness
- **Phase 2:** Program framing and horizon setup
- **Phase 3:** Outcome and consumer discovery
- **Phase 4:** Targeted decision research
- **Phase 5:** Contract and artifact architecture
- **Phase 6:** Distributed evidence-pack assembly
- **Phase 7:** Review and contract lock
- **Phase 8:** Scope sizing, milestone partitioning, and activation
- **Phase 9:** Active-milestone technical derivation, targeted evidence deepening, and delta impact analysis
- **Phase 10:** Active-milestone slice planning and rolling schedule
- **Phase 11:** Autonomous slice execution loop
- **Phase 12:** Milestone hardening, release readiness, and continuation planning

## 3. Milestones, evidence depth, and continuation

### 3.1 Milestone 1 is the first committed subset

Milestone 1 is the first operational subset that makes the program promise real. It is not a commitment to exhaustively plan or build the entire program in one pass.

### 3.2 Evidence pack before deep partitioning

For greenfield work, the system should first lock a believable evidence pack: shared scaffold where relevant, primary path or backbone, shared primitives, shell-level representation of major later capabilities, and deeper scenario coverage only where milestone boundaries depend on it. The goal is not a fully detailed whole-program artifact. It is enough clarity to size and split work responsibly.

Brownfield and continuation work can often use a narrower delta pack, but the same rule applies: do not deeply decompose work until the changed outcome is concrete enough to reason about.

### 3.3 Evidence-pack shapes by modality

| Modality | Typical early evidence pack | Main verification |
|---|---|---|
| UI/full-stack | shell, primary journey, scenario packs, shell-level later surfaces | coherence review, scenario coverage, UI tests |
| API/service | endpoint contracts, request/response examples, auth/error matrix, sequence flows | contract tests, compatibility checks |
| Compiler/toolchain | corpus slice, CLI transcripts, diagnostics catalog, AST/IR snapshots, benchmark baseline | golden tests, differential tests, perf checks |
| Library/SDK | public API contract, examples, recipes, compatibility matrix | example execution, API/type compatibility |
| Documentation | audience map, IA, outline, glossary, sample pages, executable examples | lint, link checks, example execution, completeness review |
| Infra/platform | topology diff, rollout/rollback plan, runbook pack, dry-run evidence | policy checks, rehearsals, safety verification |
| Data/schema | schema diff, migration plan, compatibility matrix, fixture data | migration tests, data validation, compatibility checks |

### 3.4 Milestone count is discovered, not predeclared

Milestone count is derived from:

- breadth of user or consumer paths,
- integration density and volatility,
- edge-case and scenario complexity,
- stability risk in existing surfaces or contracts,
- migration cliffs or irreversible changes,
- performance sensitivity,
- testability and release safety,
- documentation coverage burden,
- and how much learning is still needed.

### 3.5 Phase-skipping and fidelity rules

- Run ideation fully for greenfield work, narrowly for brownfield shaping, and minimally for concrete continuations.
- Evidence simulation is required when visible behavior, contracts, docs experience, or operational meaning changes; targeted when only a small area changes; skippable only when behavior is already fully specified.
- Research should answer the next real decision, not map the entire future roadmap.
- Milestone partitioning is mandatory after contract lock when true scope is still unclear.
- Brownfield refresh is heavy at onboarding and incremental later.
- Technical derivation and slice planning are deep only for the active milestone.
- Later targets remain shell-level or outline-level unless early fidelity is necessary for safe partitioning or implementation.

## 4. Phase-by-phase design

### Phase B — Brownfield reconnaissance and knowledge seeding
**Objective:** Build a trustworthy picture of an existing product, repository, and document set before planning.  
**Key work:** Map repository topology, ingest docs and decisions, inventory runtime and dependencies, infer behaviors and contracts, map tests and fixtures, detect hotspots and weakly tested zones, and seed trust-scored knowledge.  
**Key outputs:** `brownfield_snapshot`, `repo_topology`, `dependency_inventory`, `behavior_contract_guess`, `artifact_surface_map`, `test_landscape`, `hotspot`, `brownfield_risk`, `knowledge_seed`.  
**Exit gate:** The system can answer what exists, what must be preserved or kept compatible, what is risky, and whether to proceed with full shaping, narrow shaping, or direct intake.

### Phase 0 — Guided ideation or program shaping
**Objective:** Turn a blank-page request or continuation request into a coherent product direction and first-value outcome target.  
**Key work:** Frame the problem, identify primary consumers and contexts, define stability boundaries for brownfield work, clarify success and constraints, explore candidate directions when needed, build a coarse capability map, articulate an outcome thesis, and separate “now” from “later.”  
**Key outputs:** `program_brief`, `milestone_brief`, `problem_statement`, `primary_consumers`, `constraints`, `success_metric`, `scope_boundary`, `stability_boundary`, `capability_map`, `selected_direction`, `outcome_thesis`.  
**Exit gate:** There is a coherent objective, success criteria, a coarse capability map, a stability boundary when relevant, and enough clarity to design an early evidence artifact.

### Phase 1 — Structured intake, brownfield refresh, work classification, and execution readiness
**Objective:** Convert intent into a structured requirement profile, readiness model, classification record, and early dependency plan.  
**Key work:** Capture product and team context, technology preferences, environment and deployment assumptions, integration inventory, credential and access needs, scope and complexity signals, work class and modality, model policy constraints, user-input batches, and any needed brownfield refresh.  
**Key outputs:** `structured_requirement_profile`, `technology_preference`, `deployment_preference`, `integration_requirement`, `credential_requirement`, `work_class`, `primary_modality`, `proof_style`, `validation_profile_hint`, `readiness_check`, `blocking_dependency`, `user_input_request_batch`.  
**Exit gate:** Near-term requirements are classified as known, inferred, must-ask-now, needed-later, or optional, and the system has explicit classification and validation signals for milestone sizing.

### Phase 2 — Program framing and horizon setup
**Objective:** Create stable program memory and a provisional planning horizon.  
**Key work:** Distill shaping and readiness outputs into compact cores, initialize state, define terminology and non-goals, summarize risks and blockers, and create a provisional milestone frame when needed.  
**Key outputs:** `program_core`, `milestone_core`, `milestone_horizon_policy`, `glossary`, `non_goals`, `program_state`, `milestone_state`, `risk_register`, `readiness_summary`.  
**Exit gate:** Later agents can understand the product direction, current request, and planning horizon from the cores alone.

### Phase 3 — Outcome and consumer discovery
**Objective:** Define the consumer paths, moments of value, setup touchpoints, preserved paths, and failure conditions that the early evidence pack must represent.  
**Key work:** Map usage paths and path deltas, capture jobs-to-be-done, identify setup and onboarding touchpoints, enumerate failure modes, and lock preserved constraints for brownfield work.  
**Key outputs:** `usage_path`, `path_delta`, `job_statement`, `moment_of_value`, `failure_mode`, `setup_touchpoint`, `stability_constraint`.  
**Exit gate:** The system has explicit primary outcomes, setup or permission touchpoints, changed and preserved path segments, and key boundary conditions.

### Phase 4 — Targeted decision research
**Objective:** Research only the patterns, edge cases, and risks needed to justify the current design and next milestone decision.  
**Key work:** Pattern research, comparable-system review, accessibility or compatibility guidance, edge-case enumeration, and brownfield conflict detection.  
**Key outputs:** `pattern_finding`, `recommendation`, `accessibility_requirement`, `edge_case_set`, `risk_note`, `brownfield_conflict_note`, `compatibility_note`.  
**Exit gate:** The design has enough support to move into contract and artifact architecture without pretending future milestones have already been fully researched.

### Phase 5 — Contract and artifact architecture
**Objective:** Turn the approved direction into a milestone-contract outline, artifact graph, scenario model, impact map, and build decomposition that can be assembled through bounded agents.  
**Key work:** Define contract boundaries, artifact specs, behavior models, scenario matrices, impact maps, stability guards, artifact clusters, fidelity tiers, and shared contract outlines.  
**Key outputs:** `milestone_contract_outline`, `artifact_graph`, `artifact_spec`, `behavior_model`, `scenario_matrix`, `impact_map`, `stability_guard`, `artifact_cluster`, `fidelity_tier`, `shared_contract_outline`.  
**Exit gate:** Each important changed target has a purpose, inputs/outputs, scenario coverage, stability expectations, cluster assignment, and a fidelity target.

### Phase 6 — Distributed evidence-pack assembly
**Objective:** Build an early believable evidence pack through multiple bounded work units rather than one giant prototype run.  
**Key work:** Confirm the artifact graph; build shared scaffold and shared contracts; define the backbone; build artifact-cluster shards; inject scenario packs; assemble everything into one reviewable evidence pack; verify consistency; selectively deepen only the targets needed now.  
**Key outputs:** `evidence_pack`, `artifact_graph`, `artifact_cluster`, `artifact_shard`, `scenario_pack`, `assembly_result`, `consistency_report`, `evidence_gap`, `shared_contract`, `scaffold_fragment`.  
**Exit gate:** Reviewers can inspect a coherent evidence pack, the core value loop is believable, major capabilities are represented at least as shells or outlines, and undeepened areas are intentionally tagged rather than silently missing.

### Phase 7 — Review and contract lock
**Objective:** Turn feedback on the evidence pack into binding contracts while making explicit what is locked at detailed fidelity versus shell-level direction.  
**Key work:** Orchestrate review, synthesize feedback, lock approved program and milestone contracts, check stability boundaries, and triage gaps or change requests.  
**Key outputs:** `review_feedback`, `approved_program_contract`, `approved_milestone_contract`, `stability_contract`, `locked_decision`, `change_request`, `active_target_priority`, `validation_profile`.  
**Exit gate:** Approved contracts and stability boundaries are durable and binding at their stated fidelity.

### Phase 8 — Scope sizing, milestone partitioning, and activation
**Objective:** Use the approved contract and current constraints to decide whether the work fits in one milestone or several, create an ordered horizon, and activate only the next milestone for deep planning.  
**Key work:** Assess scope, partition around value steps, dependency cliffs, uncertainty concentrations, stability risks, irreversible changes, performance sensitivity, and release safety; order milestones by risk and value; write promotion conditions; select the active milestone; identify active versus deferred targets.  
**Key outputs:** `scope_assessment`, `milestone_shell`, `milestone_order`, `planning_horizon`, `milestone_activation`, `promotion_condition`, `deferred_capability`, `milestone_dependency`, `active_target_set`, `deferred_target_shell`.  
**Exit gate:** The active milestone has a crisp contract, future milestones exist only as coarse shells, deferred work is explicitly recorded, and any required active-target deepening is scheduled.

### Phase 9 — Active-milestone technical derivation, targeted evidence deepening, and delta impact analysis
**Objective:** Derive the technical shape only for the active milestone and deepen only the targets that are still too coarse for safe implementation.  
**Key work:** Analyze feasibility, define domain entities and contracts, write policy and validation rules, map integration and credential boundaries, analyze delta impact on the existing system, plan migrations and rollback, write future-boundary notes, and deepen active-target detail when needed.  
**Key outputs:** `technical_shape`, `domain_entity`, `api_contract`, `event_contract`, `validation_rule`, `policy_rule`, `integration_boundary`, `credential_binding_spec`, `delta_impact_map`, `migration_plan`, `rollback_plan`, `future_boundary_note`, `active_target_contract`, `milestone_evidence_delta`, `performance_budget`.  
**Exit gate:** The system knows what must be built now, what existing structures must change, which integrations are real versus fixture-backed, what protections are needed, and what future work should stay shallow.

### Phase 10 — Active-milestone slice planning and rolling schedule
**Objective:** Break the active milestone into thin end-to-end slices without deeply planning future milestones.  
**Key work:** Create feature, migration, stability, enablement, hardening, and documentation slices; map dependencies; write acceptance criteria; plan tests and fixtures; set execution priority; classify blockers; assign routing classes and model-route hints.  
**Key outputs:** `slice`, `slice_plan`, `dependency_edge`, `acceptance_criteria`, `test_matrix`, `fixture_plan`, `execution_priority`, `blocker_strategy`, `wave`, `routing_class`, `model_route_hint`, `milestone_rollover_hint`.  
**Exit gate:** Each slice has a single purpose, acceptance criteria, required tests, required fixtures, allowed file scope, dependency position, blocker classification, and routing class.

### Phase 11 — Autonomous slice execution loop
**Objective:** Execute the active milestone through disciplined, repeatable slice-level work.  
**Loop structure:** model routing -> test design -> implementation -> staging and validation -> test execution -> mandatory refactoring -> regression verification -> state update -> continue or pause.  
**Key outputs:** model-route decisions, test cases, implementation summary, staged output, validation result, repair request or quarantine item, test results, refactor summary, verification result, slice status update, run digest, blocker update, routing outcome, next action.

### Phase 12 — Milestone hardening, release readiness, and continuation planning
**Objective:** Run broader milestone checks, decide readiness, refresh the horizon, and prepare continuation.  
**Key work:** Verify wave-level behavior, audit accessibility and performance where relevant, check security and policy, assess live readiness, audit milestone completion, synthesize seeds and backlog, update the milestone horizon, summarize release readiness.  
**Key outputs:** `wave_verification`, `accessibility_audit`, `performance_note`, `integration_readiness`, `release_readiness`, `milestone_audit`, `future_seed`, `backlog_candidate`, `milestone_horizon_update`, `next_milestone_option`, `program_digest`.  
**Exit gate:** The system knows whether the milestone met its definition of done, what unresolved risks remain, what live validations are still pending, what future work surfaced, and whether another milestone should activate now.

## 5. SQLite-native data model

### 5.1 Storage strategy

Use SQLite for orchestration state and typed records, with the repository remaining the source of truth for code and source-controlled artifacts.

**Control/state tables:** projects, program state, milestones, milestone horizons, milestone state, work units, agent runs, dependencies, locks, input requirements, stop conditions.

**Knowledge tables:** codebase snapshots, document ingestions, dependency inventories, behavior maps, artifact surface maps, test landscapes, hotspots, knowledge seeds.

**Readiness/access tables:** credential requirements, credential bindings, access requirements, user preferences, integration targets, deployment targets.

**Routing tables:** model policies, model assignments, fallback events, escalation events, routing outcomes.

**Contract/evidence tables:** review targets, artifact graphs, artifact clusters, artifact shards, scenario packs, shared contracts, assembly runs, consistency reports, fidelity assignments.

**Validation tables:** staged outputs, validation runs, repair runs, quarantine items, acceptance journal.

**Code/test/trace tables:** source assets, test cases, test results, fixture sets, verification results, refactor cycles, benchmark results, example runs, trace links, decision links, context links, blocker links.

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

### 5.3 Important record families

Representative record types include:

- program and milestone briefs or cores,
- evidence packs and artifact graphs,
- artifact clusters, shards, and scenario packs,
- brownfield snapshots and knowledge seeds,
- milestone shells, planning horizons, and activation records,
- stability contracts and active-target contracts,
- technical shape, delta impact maps, migration and rollback plans,
- slice plans, test matrices, fixture scenarios,
- model-route decisions,
- staged outputs, validation results, and quarantine items,
- implementation, refactor, and verification summaries,
- future seeds, backlog candidates, and horizon updates.

UI-specific or modality-specific subtypes can still exist, such as `screen_spec`, `endpoint_contract_bundle`, `cli_transcript`, `diagnostic_catalog`, `docs_outline`, or `runbook_fragment`.

## 6. Context engineering

### 6.1 Context layers

A well-formed context packet is assembled from layers:

- **program core:** stable direction, evidence-pack summary, horizon summary, glossary, locked high-level decisions, stable constraints;
- **milestone core:** active milestone objective, scope, success metrics, stability obligations, blockers, active targets;
- **phase contract:** records specific to the current phase;
- **current work unit:** the exact slice, shard, investigation, or validation task;
- **relevant history digests:** short prior summaries ranked by relevance, dependency relation, and recency;
- **brownfield/stability state:** hotspots, preserved behaviors, compatibility guards, impacted legacy behavior;
- **readiness/model policy:** technology preferences, access and credential status, routing policy, budgets, compliance constraints;
- **local source neighborhood:** touched files, nearby interfaces, related tests, fixtures, one-hop dependencies;
- **output contract/trust policy:** what the agent may emit and what validation level its output must satisfy.

### 6.2 Assembly rules

1. Prefer stable cores over raw history.
2. Load stability constraints early for brownfield work.
3. Filter by milestone and work-unit tags.
4. Use digest-first recall instead of large transcripts.
5. Include readiness and routing data only when they affect the task.
6. Limit source context to the local neighborhood.
7. Keep future milestones as short shells, not full plans.
8. Never materialize deep future plans until a milestone is active.
9. Batch missing user inputs.
10. Schema-validate all outputs.
11. Carry trust levels forward.

### 6.3 Minimal context by agent family

- **Brownfield mappers:** repository structure, docs, deployment hints, and the specific area being refreshed.
- **Shapers/partitioners:** program core, current request, constraints, scope signals, risks, active-target priorities, and current product state.
- **Classification agents:** brief, constraints, primary consumers, and brownfield hints.
- **Research agents:** milestone core, target question, consumer type, and scope constraints.
- **Artifact/contract agents:** usage paths, recommendations, stability constraints, impacted targets, and scaffold or cluster boundaries.
- **Scaffold/cluster/scenario builders:** only the assigned scope plus shared contracts.
- **Assemblers/consistency verifiers:** scaffold contract, cluster outputs, cross-cluster rules, and fidelity tags.
- **Contract writers/technical derivation agents:** active milestone contract, active targets, domain assumptions, stability contracts, integration requirements, technology preferences.
- **Implementers/testers/refactorers:** slice plan, acceptance criteria, fixture plan, local file neighborhood, recent digests, and relevant blockers or readiness state.
- **Verifiers:** acceptance criteria, milestone contract, stability contract, validation profile, test results, and locked decisions.
- **Repair agents:** rejected output, validation failures, output contract, and the smallest context needed to repair shape or scope.

## 7. Smart model routing

### 7.1 Objective and routing dimensions

Route each work unit to the best-fit capability profile rather than forcing one model to do everything. Consider:

- work type and modality,
- need for long-context synthesis,
- structured-output reliability,
- interface or interaction judgment,
- precise code editing,
- formal or contractual consistency,
- tool use,
- latency and cost tolerance,
- compliance and provider restrictions,
- recent failure history on similar tasks.

### 7.2 Capability profiles by work class

- **Intake/extraction/classification:** low-cost, fast, structure-reliable models.
- **Research/synthesis:** models strong at broad comparison and long-context summarization.
- **Contract and artifact architecture:** models strong at decomposition, consistency, and scenario coverage.
- **Evidence assembly:** builders suited to the chosen modality plus a separate consistency-focused verifier.
- **Architecture/technical derivation:** models strong at systems reasoning and constraint consistency.
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

Start cheaper for low-risk, repetitive, or highly structured work. Escalate for high ambiguity, high risk, repeated schema failures, contradictory outputs, broad risky diffs, stability violations, or repeated test failures. Downshift when patterns stabilize.

Implementation and verification should not share the same cognitive lane when practical. Different model families, routes, or at least distinct verifier framing reduce correlated blind spots.

## 8. Output hardening and fault containment

### 8.1 Core rule

Treat every model output as a proposal. Nothing mutates durable state or source-controlled artifacts until it is staged, validated, and accepted.

### 8.2 Acceptance pipeline

1. Stage raw output.
2. Canonicalize obvious harmless formatting issues.
3. Validate syntax and schema.
4. Validate references and semantics.
5. Check scope, stability, and policy.
6. Accept atomically if valid.
7. Otherwise repair or quarantine.

### 8.3 Validation layers

- **Structural validation:** parseability and shape.
- **Contract validation:** conformance to the output contract.
- **Semantic validation:** real and coherent references, files, dependencies, and identifiers.
- **Scope validation:** milestone and file-scope compliance.
- **Stability validation:** no violation of locked stability contracts.
- **Policy validation:** no security, compliance, or operational violations.

### 8.4 Protecting state and source mutations

To protect SQLite:

- never write unvalidated output into accepted records,
- use append-only staging tables,
- accept through atomic transactions,
- version schemas and records,
- keep idempotent run identifiers,
- journal acceptance decisions for replay and recovery.

To protect the repository and source artifacts:

- require allowed file scope per work unit,
- check existence and target boundaries before mutation,
- keep pre- and post-acceptance checksums or summaries,
- independently validate destructive operations,
- require tests or stability checks before merge-worthy acceptance.

### 8.5 Repair, quarantine, recovery, and observability

Repair should escalate gradually:

1. deterministic normalization,
2. narrow self-repair using explicit validation errors,
3. alternate-model repair,
4. quarantine if still unsafe.

Recovery should always restart from the last accepted durable state, not from partially written artifacts.

Track at least validation failure rates by work class, quarantine rates by model route, common repair reasons, schema drift, stability violations, artifact compatibility failures, and expensive or failure-prone phases.

### 8.6 Distributed artifact compatibility

Artifact shards should declare the targets they own, the shared fragments they depend on, the scenario obligations they expect, and the pack version they target. The assembler should reject incompatible shards, missing scenario obligations, broken links, or shared-fragment drift instead of presenting an incoherent evidence pack.

## 9. Testing, fixtures, and refactoring

### 9.1 Fixture-first testing

No work unit is complete until it is covered by the required tests, and automated tests must not rely on live external services.

Every external dependency should sit behind an interface or adapter boundary, including AI providers, payment systems, email services, analytics, search, remote APIs, databases, third-party auth, external toolchains, docs publishing services, and cloud control planes.

### 9.2 Missing-credential and brownfield rules

Missing live credentials should not block evidence-pack work, slice implementation, or automated tests when a fixture-backed boundary exists. Credentials become blockers only for work that truly requires live access, such as deployment, provisioning, live integration validation, or explicitly real end-to-end checks.

Brownfield work must test both the new milestone behavior and a stability suite for risky adjacent existing behavior.

### 9.3 Modality-specific validation examples

- **UI:** loading, empty, error, permission, disconnected, and degraded states.
- **API/service:** contracts, auth, errors, compatibility, versioning.
- **Compiler/CLI:** corpora, diagnostics, generated outputs, regression baselines, benchmark budgets.
- **Library/SDK:** example execution, public API compatibility, type expectations.
- **Documentation:** linting, broken links, terminology consistency, runnable examples, coverage matrix checks.
- **Infra/platform:** policy checks, dry runs, rollout safety, rollback paths, runbook correctness.
- **AI-integrated products:** ideal result, low-confidence result, malformed output, refusal, timeout, partial tool output, hallucination-like output, rate-limit-like failure.

### 9.4 Test categories and completion rule

Useful categories include unit, component, integration with fixture-backed boundaries, contract, migration, stability, regression, scenario, example execution, golden, differential, and benchmark tests.

A slice only closes when required tests exist, required tests pass, important scenarios are covered by fixtures, stability checks pass where relevant, post-refactor verification is green, and any live-readiness blockers are recorded honestly.

### 9.5 Mandatory refactoring

Refactoring runs after a slice is functionally correct and green under required tests.

Allowed work includes simplifying, extracting, renaming, reorganizing local structure, improving adapter boundaries, reducing duplication, improving fixture and test reuse, and making future slices easier.

Refactoring may not alter approved behavior, break stability contracts, widen scope into unrelated modules, or silently redesign the system. If a broader refactor is best, the agent should emit a `refactor_candidate` for intentional later scheduling.

## 10. Representative lifecycle patterns

### 10.1 Compiler milestone
A compiler milestone should not be forced through visual-first language. Its evidence pack might contain a grammar slice, source corpus, expected diagnostics, CLI transcripts, AST or IR snapshots, and benchmark baselines. Its validation profile might rely on golden tests, differential tests, regression corpora, and perf thresholds.

### 10.2 Documentation-only milestone
A docs-only milestone becomes first-class. Its evidence pack might contain an audience map, information architecture, outline, glossary, representative pages, runnable examples, and a coverage matrix. Its validation profile might rely on linting, broken-link checks, terminology consistency, and example execution.

### 10.3 Brownfield improvement
When starting from an existing product, the system first maps the codebase, existing integrations, tests, hotspots, and preserved constraints. It then runs narrow shaping around the requested improvement, designs only the affected paths, builds only the affected evidence targets, partitions the work into milestones if needed, and adds stability slices wherever the surrounding system is fragile.

### 10.4 Later milestone continuation
When a product already has program core and milestone history, the system reuses that memory, runs a targeted brownfield refresh for only the relevant areas, skips broad ideation, locks only the changed outcome, partitions the new request if needed, and deeply plans only the next active increment.

### 10.5 Large greenfield program
For a broad new product, the system defines consumer paths and artifact clusters, builds a shared scaffold and backbone first, keeps later capabilities at shell fidelity, gets the outer evidence pack reviewed, partitions the program into milestones, and deepens only the active targets before technical derivation and slice planning.

## 11. Practical implementation guidance

### 11.1 Highest-leverage optimizations

1. Make rolling-horizon planning a first-class service.
2. Put milestone partitioning after contract lock.
3. Separate the program-level evidence pack from active-milestone deep detail.
4. Add work classification and validation-profile selection early.
5. Replace monolithic prototype generation with explicit artifact graphs.
6. Make scaffold, shard, scenario-pack, and assembly passes first-class work types.
7. Cache shared contracts and common structural primitives early.
8. Build one engine with three entry modes rather than three separate pipelines.
9. Separate stable program memory, active milestone memory, and future milestone shells from day one.
10. Add cached brownfield digests plus incremental refresh.
11. Make stability contracts explicit.
12. Treat model routing as a policy layer, not a hardcoded switch.
13. Build validation, acceptance, and quarantine early.
14. Budget research to the next real decision.
15. Use fidelity tiers so later targets can stay intentionally coarse.
16. Use risk-tiered verification.
17. Promote milestone shells, future seeds, backlog items, and refactor candidates to first-class outputs.
18. Tune the scheduler to work around blockers.

### 11.2 Practical build order

1. Build the milestone-aware SQLite schema, horizon state, evidence tables, and acceptance journal.
2. Build brownfield mapping and trust-scored knowledge seeding.
3. Build the entry router and phase-skipping rules.
4. Build readiness capture, classification, scope signals, and model policy capture.
5. Build program core, contract and evidence-pack services, artifact-graph services, and the context packer.
6. Build discovery, research, contract, and distributed evidence-assembly agents.
7. Build milestone partitioning and activation logic.
8. Build active-milestone deepening, technical derivation, delta impact, and stability contracts.
9. Build active-milestone slice planning, scheduling, and model routing.
10. Build the execution loop with validation, repair, testing, refactoring, and verification.
11. Build milestone hardening, horizon refresh, and continuation planning.

## 12. Final operating rules

1. Every agent is fresh; durable memory lives in SQLite.
2. Every handoff is structured and versioned.
3. Map existing codebases before planning against them.
4. Make the intended development outcome concrete before deep implementation planning hardens.
5. Early evidence is modality-aware; a UI mock is one valid form, not the only form.
6. Build early artifacts through distributed scaffold, shard, scenario-pack, and assembly work.
7. Discover milestone count from scope; do not assume one milestone.
8. Partition deeply only after contract lock when true scope is still uncertain.
9. Deeply plan only the active milestone.
10. Keep future milestones as coarse shells until activated.
11. Keep program core, evidence-pack summary, active milestone core, and horizon state separate.
12. Use stability contracts for brownfield and compatibility safety.
13. Make model selection explicit, stored, and revisable.
14. Validate all outputs before accepting them into state or source control.
15. Continue automatically while meaningful unblocked work exists.
16. Batch missing user inputs and access needs.
17. Store secrets as secure references, not plaintext.
18. Deepen only the active target set when more detail is needed.
19. Break active milestones into thin slices.
20. Test every slice with fixtures rather than live dependencies.
21. Refactor every slice after it works.
22. Preserve traceability across decisions, records, changes, and blockers.
23. Treat approved contracts, shell-level direction, and stability constraints as binding at their stated fidelity.
24. Repair or quarantine corrupt output; never force it into accepted state.
25. Use milestone close-out to refresh the continuation surface and recut the horizon if needed.
26. Development is the hard boundary; this is not a universal workflow for non-development domains.

## Summary

This architecture is a SQLite-native, outcome-first, fresh-agent delivery system for greenfield starts, brownfield onboarding, and milestone continuation. It gets to a credible review artifact early, uses that artifact to size and order milestones, deeply plans only the active milestone, and executes that milestone through thin slices with model routing, validator-first acceptance, fixture-backed testing, mandatory refactoring, and milestone-aware continuation.

The crucial generalization is that the early artifact is now **modality-aware** rather than assumed to be visual. That one change turns the workflow from “great for UI-heavy work” into “good for most development work,” including compilers, APIs, libraries, infra, migrations, and documentation-only milestones.
