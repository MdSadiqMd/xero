# Mailbox Gate Optimization Plan

## Reader And Goal

This plan is for an internal Xero engineer implementing follow-up improvements to the concurrent-agent mailbox mutation gate.

After reading this, the engineer should be able to implement lower-token mailbox checks without weakening the invariant that concurrent same-project agent runs must inspect relevant mailbox state before mutating project files.

## Current Behavior

When a project has only one active agent run, repository mutations proceed normally.

When another same-project run is active, repository write tools and non-read-only shell commands require mailbox-check evidence before mutation. A successful `agent_coordination` `read_inbox` call records a high-water mark for mailbox items relevant to that run. Later mutations reuse that evidence until a newer relevant mailbox item arrives.

This means an agent editing ten files one after another should not need to read the mailbox before every file edit. It should read once, continue mutating, and only be forced to read again if newer relevant mailbox state appears.

Today, the expensive part is that `read_inbox` returns the full visible inbox page rather than supporting narrower reads based on what the agent is about to do.

## Existing Data Shape

Mailbox items already carry enough structure to support scoped reads:

- sender session/run/role metadata
- optional target session/run/role metadata
- item type
- title and body
- related paths
- priority
- status
- created and expiry timestamps
- acknowledgement state for the current run

The `agent_coordination` tool schema already accepts `path` and `paths`, but the current `read_inbox` behavior does not use them to filter inbox reads.

## Invariants To Preserve

- Single active run behavior stays unchanged.
- Concurrent same-project runs still require mailbox awareness before project-changing mutations.
- A mailbox check remains valid across multiple mutations until newer relevant mailbox state arrives.
- A later relevant mailbox delivery stales prior evidence.
- The guard remains central runtime/tool policy, not prompt-only or UI-only.
- Mailbox state remains temporary OS app-data runtime state, not legacy repo-local state.
- Mailbox content remains advisory and never overrides user instructions, tool policy, or current file evidence.

## Proposed Improvements

### 1. Path-Scoped `read_inbox`

Teach `agent_coordination` `read_inbox` to honor `path` and `paths`.

When paths are provided, return only open, unexpired, unacknowledged inbox items whose `related_paths` overlap any requested path. The overlap rule should match the reservation conflict behavior: exact file overlap and directory-prefix overlap should both count.

This gives agents a cheap pattern:

1. Compute planned mutation paths.
2. Call `agent_coordination/read_inbox` with those paths.
3. Review only mailbox items related to the files being changed.
4. Mutate until newer relevant mailbox state arrives.

### 2. Path-Scoped Evidence

Current evidence is run-wide. If path-scoped reads are added, evidence should be path-aware too.

Suggested model:

- Keep existing run-wide evidence for unfiltered `read_inbox`.
- Add scoped evidence entries keyed by normalized path or by a digest of the requested path set.
- Store the latest relevant mailbox high-water mark for that scope.
- During mutation, compare the mutation paths against the freshest matching evidence.

Simple version:

- If the last check was unfiltered, it satisfies all paths.
- If the last check was scoped, it satisfies mutations only when every mutation path is covered by the scoped check.
- If no matching evidence exists, deny with the existing mailbox-check policy code.

### 3. Lightweight Inbox Status Action

Add an `agent_coordination` action such as `check_inbox_status`.

It should return metadata only:

- active sibling count
- whether the current run has valid mailbox evidence
- whether evidence is stale
- count of relevant open items
- highest relevant mailbox high-water mark
- optionally counts by priority and item type

It should not return mailbox bodies.

This lets an agent cheaply ask, “Do I need to spend tokens reading mailbox content?” before pulling full items.

### 4. Filtered Full Reads

Extend `read_inbox` with optional filters:

- `paths`
- `itemTypes`
- `priorityAtLeast`
- `sinceLastCheck`
- `limit`

Recommended first filters are `paths` and `sinceLastCheck`; they solve the main token problem without making the API too broad.

`sinceLastCheck` should return only relevant mailbox items newer than the evidence already recorded for the run or scope.

### 5. Better Denial Guidance

Keep the stable code:

`policy_requires_mailbox_check_before_mutation`

Improve the guidance payload/message so the agent knows the cheapest retry:

- If mutation paths are known: tell it to call `agent_coordination/read_inbox` with those paths.
- If paths are unknown or the tool is a broad shell command: tell it to call unfiltered `read_inbox`.
- If a status-only action exists: tell it to call `check_inbox_status` first when appropriate.

### 6. Batch-Friendly Agent Guidance

Update tool descriptions and runtime guidance to encourage batching:

- Prefer one mailbox check before a planned batch of related edits.
- Prefer path-scoped inbox reads for the intended write set.
- Prefer `patch` or `fs_transaction` for coherent multi-file changes when appropriate.
- Do not re-read the mailbox between every file write unless the policy says evidence is stale.

This is guidance only; the policy high-water mark remains the source of truth.

## Suggested Implementation Order

1. Add path overlap helpers for mailbox related paths, reusing the reservation overlap semantics where possible.
2. Add path filtering to inbox queries.
3. Record whether inbox-check evidence was unfiltered or path-scoped.
4. Update mutation gate evaluation to use scoped evidence when mutation paths are known.
5. Add `check_inbox_status` as a metadata-only action.
6. Add filtered `read_inbox` options beyond paths only if tests show the path filter is not enough.
7. Update tool descriptions and denial guidance.

## Test Plan

Add focused Rust tests for:

- Unfiltered `read_inbox` still satisfies later mutations for any path.
- Path-scoped `read_inbox` satisfies mutation for an overlapping file.
- Path-scoped `read_inbox` does not satisfy mutation for an unrelated file.
- A later mailbox item on an overlapping path stales scoped evidence.
- A later mailbox item on an unrelated path does not stale scoped evidence for the checked paths.
- Directory-prefix overlap works for path-scoped evidence.
- `check_inbox_status` returns counts/high-water metadata without mailbox bodies.
- Denial guidance recommends path-scoped `read_inbox` when mutation paths are known.

## Open Design Questions

- Should a scoped check cover exact paths only, or should checking `src/` cover all files under `src/`? Recommended: match reservation overlap semantics.
- Should scoped evidence be keyed by each normalized path individually, or by a normalized path-set digest? Recommended: start with individual normalized paths if mutation tools expose paths cleanly.
- Should shell command mutations always require unfiltered evidence, or can path candidates from command arguments be used? Recommended: start conservative with unfiltered evidence for broad shell commands.
- Should acknowledged mailbox items affect high-water freshness? Recommended: no. The mutation gate should care about delivery awareness, while acknowledgement remains a separate semantic action.

## Acceptance Criteria

- Agents editing many files can usually do one mailbox read per coherent batch, not one per file.
- Agents can fetch mailbox content scoped to intended mutation paths.
- Agents can check whether mailbox evidence is fresh without fetching mailbox bodies.
- New mailbox items stale evidence only when they are relevant to the evidence scope.
- Existing tests for single-session bypass, concurrent denial, post-read allow, and stale evidence continue to pass.
