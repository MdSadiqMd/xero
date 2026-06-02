# Issue 48: Workflow Agent Reference Validation And Pinning

## Audit

- `workflow_definition` draft, validate, save, and update currently call the pure structural validator, so Agent Create can persist Workflow agent nodes that reference missing, inactive, stale, or activation-invalid custom agents.
- `definition_validator` verifies graph shape, artifact references, command nodes, state nodes, conditions, and loops, but intentionally accepts unknown custom `AgentRefDto` values in pure tests.
- Workflow execution resolves custom `AgentRefDto` nodes with `resolve_agent_definition_for_run(... Some(definition_id) ...)`, which loads the current version and ignores the pinned Workflow version.
- Built-in Workflow refs carry versions, and built-in runtime descriptors expose supported versions, but Workflow validation does not compare them.
- Agent Create guidance says to validate Workflows, but it does not explicitly require listing/getting agents before composing unknown refs.

## Implementation Plan

1. Keep the pure structural validator available and add a registry-aware validation entry point that appends agent-ref readiness diagnostics.
2. Validate custom refs by loading the definition, checking active lifecycle, loading the requested version, and applying the same activation preflight rules used before runtime startup.
3. Validate built-in refs against the available built-in runtime agent catalog and descriptor versions.
4. Add stable diagnostic codes and paths for agent ref failures, using paths like `nodes.N.agentRef.definitionId` and `nodes.N.agentRef.version`.
5. Add a pinned custom-agent resolver and use it in Workflow execution so runs honor the authored version.
6. Route `workflow_definition` draft/validate/save/update and Tauri Workflow create/update validation through the registry-aware validator.
7. Update Agent Create prompt/tool guidance to list/get agents when refs are not known and validate Workflows before save approval.
8. Add focused Rust tests for missing custom agent, inactive custom agent, missing custom version, stale current-vs-pinned version, valid pinned custom version, invalid built-in version, valid built-in refs, and existing graph-shape behavior.
9. Run scoped formatting and focused tests, one Cargo command at a time.
