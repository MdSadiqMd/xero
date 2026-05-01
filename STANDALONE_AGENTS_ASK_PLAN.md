# Standalone Agents And Ask Agent Plan

Date: 2026-05-01

Audience: Xero engineers extending the Agent tab from one general-purpose engineering assistant into many standalone task agents that can later be composed into workflows.

Post-read action: implement an Ask agent that answers questions without repository mutation access, while turning the current owned software-building behavior into an explicit Engineer agent and leaving clean extension points for future task agents.

## Current State

Yes: the current Agent tab is effectively wired as an agentic engineer.

The user-facing Agent surface is a single composer with model, thinking effort, approval mode, auto-compact, and prompt controls. Starting or continuing a conversation calls the owned runtime-run commands, which create or continue an owned agent run for the selected project and agent session.

The backend then:

- builds runtime-run controls from the selected model, thinking effort, approval mode, and plan-mode flag;
- creates an owned agent run with the user prompt;
- builds a prompt that says the model is Xero's owned software-building agent;
- selects an initial tool registry from the prompt;
- lets the provider call tools in a loop;
- records transcript, tool, policy, checkpoint, action-required, and file-change events;
- enforces approval policy and write-observation guards at dispatch time.

The current "approval mode" is not an agent. It only changes safety behavior for commands and operator review. It does not change the agent's job, prompt contract, available tool universe, UI language, completion gates, or persistence model.

The most important current detail for the Ask agent: even a small prompt starts with core tools, and core includes discovery and activation tools. A read-only-looking Ask UI would not be enough unless the backend also prevents tool activation into mutation, command, browser-control, MCP-invoke, device-control, or other effectful capabilities.

## Product Goal

Add standalone agents as a first-class product concept.

Each agent should have:

- a clear task purpose;
- a durable runtime identity;
- a prompt policy;
- a tool authority policy;
- an output contract;
- a project data policy for the durable LanceDB project store;
- enough metadata for a future workflow builder to sequence agents safely.

Initial agents:

- **Ask**: answer in chat. It may use any audited non-mutating tool when needed. It must not mutate repository files, app state, browser/device state, external services, credentials, or process state. It must not ask for operator approval to escape this boundary.
- **Engineer**: the existing agentic software-building behavior, renamed and formalized as one standalone agent among many.

Recommendation: make Ask the default agent for new agent sessions and new terminal-run continuations. Engineer remains one click away for implementation work. Because this app is new, do not add compatibility shims for older local control payloads unless explicitly requested.

## Agent Catalog

Introduce a single agent descriptor registry shared by frontend schemas, Rust contracts, prompt assembly, tool policy, and tests.

Start with this conceptual shape:

```ts
type RuntimeAgentId = "ask" | "engineer"

interface RuntimeAgentDescriptor {
  id: RuntimeAgentId
  label: string
  shortLabel: string
  description: string
  taskPurpose: string
  defaultApprovalMode: "suggest" | "auto_edit" | "yolo"
  allowedApprovalModes: Array<"suggest" | "auto_edit" | "yolo">
  promptPolicy: "ask" | "engineer"
  toolPolicy: "observe_only" | "engineering"
  outputContract: "answer" | "engineering_summary"
  projectDataPolicy: {
    required: true
    recordKinds: Array<"agent_handoff" | "project_fact" | "decision" | "constraint" | "plan" | "finding" | "verification" | "question" | "artifact" | "context_note" | "diagnostic">
    structuredSchemas: string[]
    unstructuredScopes: Array<"answer_note" | "session_summary" | "artifact_excerpt" | "troubleshooting_note">
    memoryCandidateKinds: Array<"project_fact" | "user_preference" | "decision" | "session_summary" | "troubleshooting">
  }
  workflowRole: "interactive" | "workflow_step"
  allowPlanGate: boolean
  allowVerificationGate: boolean
  allowAutoCompact: boolean
}
```

Persist `runtimeAgentId` in runtime-run control snapshots, not as an unrelated UI preference. The selected agent should be part of the durable run contract because it determines prompt assembly, tool authority, output shape, project data capture, and future workflow composition.

The selected agent should be immutable for an active non-terminal run in the first implementation. If the user wants to move from Ask to Engineer, they should start a fresh run in the same agent session after the current run is stopped or completed. This avoids mixing two different system prompts and tool policies inside one provider transcript.

## Ask Agent Contract

Ask is answer-only in observable effect.

Allowed:

- final assistant responses;
- any audited non-mutating tool in the registry;
- transcript and usage persistence required to show the conversation;
- runtime-owned LanceDB writes for redacted project records, including handoff records, retrieval notes, and memory candidates;
- optional session memory/context reads already approved by Xero policy;
- model thinking effort and provider selection.

Not allowed:

- file writes, edits, patches, deletes, renames, directory creation, or notebook edits;
- shell commands, command sessions, process-manager actions, package managers, tests, builds, or dev servers;
- `tool_access` escalation into any non-Ask capability;
- browser control, cookies, storage writes, page navigation, device/emulator control, macOS automation, Solana mutation/simulation/deploy, mutating or unknown-effect MCP tool invocation, skill installation/invocation, or subagents;
- approval prompts that would grant mutation access;
- verification gates that require commands.

Ask v1 should expose all audited non-mutating tools. The list should be generated from tool registry metadata instead of maintained as a small hand-written list. Initial examples include:

- `read`
- `search`
- `find`
- `git_status`
- `git_diff`
- `list`
- `file_hash`
- `code_intel` and `lsp` actions that only inspect code
- context, transcript, storage-overview, and memory-read tools that do not modify state
- read-only network or MCP tools only when their effect classification is explicit and enforced

`tool_access` and `tool_search` may be exposed to Ask only in an agent-filtered form. They must return and activate only audited non-mutating tools for Ask, and dispatch must still reject any forbidden tool even if discovery returns a bad descriptor.

Do not expose shell as "read-only." Even diagnostic commands can run scripts, generate files, contact networks, or mutate caches. Keep command access out of Ask unless a future command runner can prove a specific command is non-mutating under sandboxed execution.

## Backend Enforcement

Ask needs defense in depth at four levels.

### 1. Runtime Contracts

Add `RuntimeAgentIdDto` with at least:

- `ask`
- `engineer`

Thread it through:

- frontend Zod runtime control schemas;
- Rust runtime-run control DTOs;
- active and pending runtime-run control snapshots;
- project-store runtime control JSON records;
- owned agent request structs;
- agent-run persistence and DTOs where run history is shown independently from runtime-run snapshots.

The control schema should reject malformed agent ids. Since compatibility is prohibited, update the base migrations and tests directly rather than maintaining old payload fallbacks.

### 2. Prompt Assembly

Split the current system prompt into common policy plus agent-specific policy.

Common policy keeps instruction hierarchy, repository-instruction handling, prompt-injection posture, redaction, and tool-result trust rules.

Engineer policy keeps the current software-building contract: inspect, plan when needed, edit, run focused verification, and summarize files changed.

Ask policy should say:

- answer the user's question in chat;
- use observe-only tools only when needed to ground the answer;
- do not claim to have changed, run, installed, deployed, opened, or approved anything;
- when implementation is requested while Ask is selected, explain what would need to change and offer a concise plan, but do not perform it;
- do not request approval for mutation tools because Ask cannot escalate.

The tool policy fragment must be generated from the selected agent. Ask should not mention unavailable activation flows or mutation groups.

### 3. Project LanceDB Knowledge Store

All agents should save important information to the per-project LanceDB store. LanceDB should become the durable project knowledge database, not merely a place to stash handoff facts. It should support both structured records that workflows can query deterministically and unstructured text that future retrieval can rank semantically.

There is already a project LanceDB layer under the app-data project directory with an `agent_memories` table. Preserve that direction: project knowledge belongs in the project LanceDB store, not repo-local `.xero/` state and not only in SQLite transcript rows.

Use SQLite for transactional control-plane data where relational invariants matter: projects, sessions, runs, control snapshots, transcript rows, action requests, and lifecycle state. Use LanceDB as the project data plane for knowledge, retrieval, artifacts, workflow context, embeddings, and flexible structured/unstructured records.

Recommended approach:

- Keep user-reviewed long-term prompt memory as the existing `agent_memories` table.
- Add a general-purpose Lance table, tentatively `project_records`, that stores typed project records with both unstructured text and structured JSON payloads.
- Represent handoffs as one `project_records.recordKind` value, such as `agent_handoff`, rather than the whole LanceDB strategy.
- Add specialized Lance tables later only when a domain has a distinct query pattern, lifecycle, or high-volume storage need.
- Let each completed agent run emit one or more project records, plus optional memory candidates when the information should become reviewed long-term prompt context.
- Make LanceDB writes runtime-owned side effects, not model-callable tools. Ask can write project records without gaining repository mutation authority.

`project_records` should be an envelope that can carry structured and unstructured project knowledge:

```ts
interface ProjectRecord {
  recordId: string
  projectId: string
  recordKind: "agent_handoff" | "project_fact" | "decision" | "constraint" | "plan" | "finding" | "verification" | "question" | "artifact" | "context_note" | "diagnostic"
  runtimeAgentId: RuntimeAgentId
  agentSessionId: string | null
  runId: string
  workflowRunId: string | null
  workflowStepId: string | null
  title: string
  summary: string
  text: string
  textHash: string
  contentJson: unknown | null
  contentHash: string | null
  schemaName: string | null
  schemaVersion: number
  importance: "low" | "normal" | "high" | "critical"
  confidence: number | null
  tags: string[]
  sourceItemIds: string[]
  relatedPaths: string[]
  producedArtifactRefs: string[]
  redactionState: "clean" | "redacted" | "blocked"
  visibility: "workflow" | "retrieval" | "memory_candidate" | "diagnostic"
  createdAt: string
  updatedAt: string
  embedding: number[] | null
}
```

Project record capture should happen at runtime boundaries:

- after an agent returns a final answer;
- after an Engineer run records file changes or verification evidence;
- after a review-like agent records findings;
- after a research-like agent records sourced facts;
- when a run pauses with open questions or required approvals.

Ask should generally save:

- concise answer summaries;
- project facts discovered from read-only inspection;
- user-stated constraints or preferences;
- unanswered questions;
- references to files or symbols discussed.

Engineer should generally save:

- decisions made;
- files or subsystems changed;
- verification evidence;
- unresolved blockers;
- follow-up tasks;
- durable troubleshooting facts.

The record envelope should make retrieval and workflow use explicit:

- `text` is the unstructured retrieval surface and can later receive embeddings.
- `contentJson` is the structured workflow surface and should follow a named schema when the agent descriptor declares one.
- metadata fields support deterministic filtering by project, agent, run, workflow, kind, tags, source refs, related paths, importance, and visibility.
- `agent_memories` remains the reviewed prompt-memory table; records in `project_records` are not automatically prompt-visible memory.

Normal prompt injection rules still apply. Project record text, structured payloads, and memory candidates must be redacted, bounded as untrusted lower-priority context, linked to source run ids/items, and excluded from automatic prompt injection unless the active context policy explicitly selects them. If a project record also looks like long-term memory, create a reviewable memory candidate rather than silently enabling it.

### 4. Tool Policy

Add an agent-aware capability policy object and pass it to tool registry construction, tool activation, and dispatch.

Every tool descriptor should carry an explicit effect classification. Ask allows tools classified as non-mutating and denies tools that mutate, may mutate, require approval to be safe, or have an unknown effect. The classification should describe observable effects, not intent: a tool is not Ask-safe merely because it can be used for diagnosis.

Required checks:

- Initial registry selection intersects prompt-selected tools with the selected agent allowlist and the non-mutating effect class.
- Dynamic activation cannot grant tools outside the selected agent allowlist or outside the non-mutating effect class.
- Deferred tool catalog results are filtered by selected agent and effect class.
- Dispatch rejects any tool call whose decoded request is outside the selected agent, even if a bad registry state somehow exposed the descriptor.
- Policy-denied Ask calls should fail as terminal agent-boundary violations, not as approval-required actions.

This is the core safety invariant:

> An agent's tool authority is an allowlist, not a prompt convention.

## Frontend UX

Use ShadCN primitives for the agent picker.

Place a compact agent selector in the composer control row near model and thinking effort. A segmented control is appropriate while there are only two or three agents; a dropdown can replace it when the agent catalog grows.

Initial labels:

- Ask
- Engineer

Ask UI behavior:

- hide or disable the approval-mode selector, because approval mode does not apply when mutation and commands are unavailable;
- keep model and thinking selectors;
- keep auto-compact only if it remains a transcript-only operation;
- use Ask-specific placeholder copy, such as "Ask about this project...";
- show the active agent on the current run and recent session items where it helps users understand history.

Engineer UI behavior:

- keep the existing approval selector;
- keep existing prompt, action-required, checkpoint, and approval surfaces;
- rename user-facing copy away from generic "agent" where the distinction matters.

During an active non-terminal run, disable agent switching with a tooltip that says the selected agent is fixed for the current run. After completion, the next prompt may start a fresh run with the selected agent.

## Workflow Composition

The long-term goal is not just a picker in chat. Xero should eventually have a catalog of standalone agents for specific tasks, and workflows should string those agents together.

Likely future agents:

- **Architect**: produce design plans and tradeoffs, maybe read-only plus diagrams/docs.
- **Review**: inspect diffs and produce findings, no edits by default.
- **Debug**: inspect, run scoped commands, propose or optionally apply fixes.
- **Autonomous Engineer**: long-running engineering with stronger plan/approval gates.
- **Research**: web/search/MCP read-only with citations.
- **Release**: prepare release notes, version checks, and packaging steps under explicit approval.

Design each agent descriptor so a future workflow engine can ask:

- what inputs this agent accepts;
- what outputs it produces;
- what project records it writes, including handoff records;
- what memory candidates it may propose;
- which tools it can use;
- whether it may mutate state;
- whether it requires human approval before execution;
- whether its output can safely feed another agent.

Future agents should be data additions to the agent registry plus targeted policy tests. They should not require new one-off booleans sprinkled through UI and runtime code.

The workflow engine should use project LanceDB as its context bus:

- upstream agents write typed project records, including handoff records;
- the workflow planner selects relevant records by project, workflow run, agent id, kind, tags, embeddings, source refs, related paths, and structured schema;
- downstream agents receive only selected, redacted records as lower-priority context;
- workflow completion can promote selected records into reviewable long-term memory candidates.

## Implementation Slices

### Slice 1: Shared Agent Types

- Add shared TypeScript and Rust agent id enums.
- Add `runtimeAgentId` to runtime control input, active snapshot, pending snapshot, and mapped view models.
- Add labels, default descriptors, and project data policies in one frontend/backend-shared helper.
- Update schema tests for valid agents, invalid agents, and default control construction.

### Slice 2: Runtime Persistence And Run Creation

- Persist `runtimeAgentId` in runtime-run control JSON.
- Include the selected agent in owned agent request structs.
- Make the selected agent immutable while an owned run is active.
- Ensure runtime-run updates can queue prompts without changing the selected agent.
- Update runtime-run tests that construct active/pending controls.

### Slice 3: Project LanceDB Knowledge Store

- Add a project LanceDB table for typed structured/unstructured project records, tentatively `project_records`.
- Keep existing `agent_memories` as the user-reviewed long-term memory table.
- Add typed Rust records, DTOs, redaction checks, and list/query helpers for project records.
- Support deterministic metadata filters and unstructured text fields that can later be embedded.
- Store handoffs as `agent_handoff` records rather than a separate handoff-only concept unless profiling shows a dedicated table is needed.
- Add deduplication by source run, kind, text/content hash, and related refs so repeated continuations do not spam workflow context.
- Add tests proving project records are stored under the project app-data Lance directory and never under repo-local `.xero/`.

### Slice 4: Agent Project Data Capture

- Add a runtime-owned project-record extraction step at final answer, pause, and completion boundaries.
- Make every agent descriptor declare which project record kinds, structured schemas, unstructured scopes, and memory candidate kinds it may emit.
- Write Ask project records without exposing any model-callable mutation tools.
- Create reviewable `agent_memories` candidates only for information that should become long-term prompt context.
- Add tests for Ask and Engineer project record emission, redaction, provenance, structured JSON validation, and deduplication.

### Slice 5: Prompt Compiler

- Add agent-specific prompt fragments.
- Rename current base behavior to Engineer policy.
- Add Ask policy.
- Add prompt compiler tests proving Ask prompt does not call itself a software-building agent and does not mention mutation/approval escalation.

### Slice 6: Tool Registry And Dispatch Policy

- Add agent-aware allowlists and explicit non-mutating/mutating/unknown effect classifications.
- Build Ask registry from all audited non-mutating tools.
- Keep `tool_access` and `tool_search` available only if they are filtered to Ask-safe tools and cannot activate mutating or unknown-effect tools.
- Filter dynamic activation and deferred catalog by selected agent and effect class.
- Add dispatch-level denial for forbidden agent/risk-class combinations.
- Add Rust tests proving Ask exposes every audited non-mutating tool and cannot expose or dispatch mutation, command, browser-control, unsafe MCP invoke, skill, subagent, emulator, macOS automation, or Solana mutation tools, even through explicit `tool:` prompt directives.

### Slice 7: Provider Loop Gates

- Make plan and verification gates agent-aware.
- Ask should not require command-based verification.
- Ask should fail closed if the provider returns forbidden tool calls.
- Add tests for Ask final-response flow and forbidden-tool failure behavior.

### Slice 8: Composer UI

- Add the agent selector with ShadCN controls.
- Hide approval controls in Ask.
- Disable agent switching during active non-terminal runs.
- Pass `runtimeAgentId` into `startRuntimeRun`.
- Preserve current Engineer behavior when Engineer is selected.
- Add focused component and state-hook tests.

### Slice 9: Stream And History Projection

- Surface active agent in `AgentPaneView`.
- Include selected agent in runtime stream/tool-registry snapshots so debugging an Ask run shows its active capability policy.
- Show project-record status where useful, especially when a workflow step is waiting on an upstream agent's output.
- Add a small agent label to session/run history only if it clarifies mixed-agent sessions.

### Slice 10: Documentation And Verification

- Update user-facing docs once behavior ships.
- Run focused frontend tests for runtime schemas, state projections, and composer UI.
- Run focused Rust tests for runtime control persistence, LanceDB project-record persistence, project data capture, prompt compiler, tool registry, dispatch policy, and provider loop gates.
- Run one Cargo command at a time.

## Acceptance Criteria

- New sessions can start with the Ask agent.
- The Engineer agent preserves today's agentic engineering behavior.
- Ask exposes all audited non-mutating tools, not only a small hard-coded read list.
- Ask can answer questions without exposing mutation, command, process, browser-control, emulator, Solana mutation, mutating or unknown-effect MCP invoke, skill, or subagent tools.
- Ask cannot activate forbidden tools through `tool_access`, `tool_search`, prompt directives, dynamic MCP routes, or provider-produced tool calls.
- Ask does not create file-change records.
- Ask does not create approval requests for mutation escalation.
- Ask does create redacted project LanceDB records for important answer facts, questions, discussed references, and handoff summaries.
- Ask's system prompt never identifies the agent as a software-building executor.
- Every completed agent run writes at least one project record or a diagnostic explaining why no important information was captured.
- The selected agent is visible enough that users understand whether they are asking or delegating engineering work.
- Adding a future standalone agent requires extending the agent registry and tests, not inventing a second control system.
- A future workflow builder can inspect each agent's purpose, inputs, outputs, project data policy, prompt-memory review policy, tool policy, mutation policy, and approval requirements.
- Workflow steps can pass selected LanceDB records from upstream agents to downstream agents as redacted lower-priority context.
- The project LanceDB store can hold both structured workflow/query records and unstructured retrieval text without making either automatically trusted prompt instructions.

## Open Decisions

- Which tools count as non-mutating? Recommendation: audit and classify every tool by observable side effects, allow all non-mutating tools for Ask, and deny unknown-effect tools until classified.
- Should Ask be the default for all new sessions? Recommendation: yes, because it is safer and makes Engineer an intentional choice.
- Should the selected agent live on agent sessions as a remembered default? Recommendation: not in v1. Keep the agent run-scoped first, then add session defaults if users repeatedly switch agents.
- Should web search exist in Ask? Recommendation: not in v1. Add a separate Research agent or an explicit Ask+Web capability after network-read UX is settled.
- Should `project_records` be one generic table or several specialized tables? Recommendation: start with one generic envelope plus `agent_memories`, then add specialized Lance tables only when a domain has a distinct query or retention need.
- Should workflow records require user approval before downstream agents can read them? Recommendation: no for workflow-scoped records, yes for long-term prompt memory. Records should be redacted, provenance-linked, and lower-priority; promotion into always-available memory should remain review-gated.
