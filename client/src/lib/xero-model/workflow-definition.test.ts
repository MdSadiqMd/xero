import { describe, expect, it } from 'vitest'

import {
  validateWorkflowDefinition,
  type WorkflowDefinitionDto,
} from './workflow-definition'
import {
  WORKFLOW_TEMPLATE_LIBRARY,
  instantiateBlankWorkflow,
  instantiateWorkflowTemplate,
} from './workflow-templates'

const builtInAgentRef = {
  kind: 'built_in',
  runtimeAgentId: 'engineer',
  version: 2,
} as const

function linearWorkflow(): WorkflowDefinitionDto {
  return {
    schema: 'xero.workflow_definition.v1',
    id: 'workflow-linear',
    projectId: 'project-1',
    name: 'Linear workflow',
    description: '',
    version: 1,
    startNodeId: 'agent-a',
    nodes: [
      {
        id: 'agent-a',
        type: 'agent',
        title: 'Agent A',
        description: '',
        position: { x: 0, y: 0 },
        agentRef: builtInAgentRef,
        inputBindings: [],
        outputContract: {
          artifactType: 'text_output',
          schemaVersion: 1,
          extraction: 'generic_text',
          required: true,
        },
        failurePolicy: {
          quotaFailureClasses: [],
          transientFailureClasses: [],
        },
        resourceScopes: [],
      },
      {
        id: 'agent-b',
        type: 'agent',
        title: 'Agent B',
        description: '',
        position: { x: 320, y: 0 },
        agentRef: {
          kind: 'custom',
          definitionId: 'project-agent',
          version: 1,
        },
        inputBindings: [
          {
            source: 'artifact',
            name: 'handoff',
            required: true,
            artifactRef: 'agent-a.text_output',
          },
        ],
        outputContract: {
          artifactType: 'implementation_summary',
          schemaVersion: 1,
          extraction: 'generic_text',
          required: true,
        },
        failurePolicy: {
          quotaFailureClasses: [],
          transientFailureClasses: [],
        },
        resourceScopes: [],
      },
      {
        id: 'done',
        type: 'terminal',
        title: 'Done',
        description: '',
        position: { x: 640, y: 0 },
        terminalStatus: 'success',
      },
    ],
    edges: [
      {
        id: 'edge-a-b',
        fromNodeId: 'agent-a',
        toNodeId: 'agent-b',
        type: 'success',
        label: '',
        priority: 10,
        condition: { kind: 'node_status', nodeId: 'agent-a', status: 'succeeded' },
      },
      {
        id: 'edge-b-done',
        fromNodeId: 'agent-b',
        toNodeId: 'done',
        type: 'success',
        label: '',
        priority: 10,
        condition: { kind: 'always' },
      },
    ],
    artifactContracts: [],
    runPolicy: {
      concurrencyLimit: 1,
      resourceConflictPolicy: {
        mode: 'serialize_conflicts',
        defaultScopes: [],
      },
      recoveryDefaults: {
        debugMaxAttempts: 2,
        gapClosureMaxAttempts: 2,
        reviewFixMaxAttempts: 3,
      },
    },
  }
}

describe('validateWorkflowDefinition', () => {
  it('accepts a linear workflow with a custom downstream agent', () => {
    const report = validateWorkflowDefinition(linearWorkflow())

    expect(report).toEqual({ status: 'valid', diagnostics: [] })
  })

  it('accepts a conditional router with one explicit else edge', () => {
    const workflow = linearWorkflow()
    workflow.nodes.splice(2, 0, {
      id: 'router',
      type: 'router',
      title: 'Route',
      description: '',
      position: { x: 640, y: 0 },
    })
    workflow.edges = [
      {
        id: 'edge-a-router',
        fromNodeId: 'agent-a',
        toNodeId: 'router',
        type: 'success',
        label: '',
        priority: 10,
        condition: { kind: 'always' },
      },
      {
        id: 'edge-router-agent-b',
        fromNodeId: 'router',
        toNodeId: 'agent-b',
        type: 'conditional',
        label: 'needs work',
        priority: 10,
        condition: {
          kind: 'artifact_field_equals',
          artifactRef: 'agent-a.text_output',
          path: '$.status',
          value: 'needs_changes',
        },
      },
      {
        id: 'edge-router-done',
        fromNodeId: 'router',
        toNodeId: 'done',
        type: 'conditional',
        label: 'else',
        priority: 999,
        condition: { kind: 'always' },
      },
      {
        id: 'edge-b-done',
        fromNodeId: 'agent-b',
        toNodeId: 'done',
        type: 'success',
        label: '',
        priority: 10,
        condition: { kind: 'always' },
      },
    ]

    expect(validateWorkflowDefinition(workflow).status).toBe('valid')
  })

  it('rejects a cycle without an explicit loop policy', () => {
    const workflow = linearWorkflow()
    workflow.edges.push({
      id: 'edge-b-a',
      fromNodeId: 'agent-b',
      toNodeId: 'agent-a',
      type: 'conditional',
      label: 'retry',
      priority: 20,
      condition: { kind: 'always' },
    })

    const report = validateWorkflowDefinition(workflow)

    expect(report.status).toBe('invalid')
    expect(report.diagnostics.map((diagnostic) => diagnostic.code)).toContain(
      'cycle_without_loop_policy',
    )
  })

  it('accepts a bounded loop with an exhaustion target', () => {
    const workflow = linearWorkflow()
    workflow.nodes.push({
      id: 'human',
      type: 'human_checkpoint',
      title: 'Human review',
      description: '',
      position: { x: 320, y: 240 },
      checkpointType: 'decision',
      prompt: 'Choose the next route.',
      decisionOptions: ['retry', 'stop'],
    })
    workflow.edges.push({
      id: 'edge-b-a',
      fromNodeId: 'agent-b',
      toNodeId: 'agent-a',
      type: 'loop',
      label: 'retry',
      priority: 20,
      condition: { kind: 'loop_attempt_lt', loopKey: 'agent_retry', value: 2 },
      loopPolicy: {
        loopKey: 'agent_retry',
        maxAttempts: 2,
        attemptScope: 'run',
        carryoverPolicy: 'all',
        selectedArtifactRefs: [],
        resetPolicy: 'never',
        onExhausted: 'human',
      },
    })

    expect(validateWorkflowDefinition(workflow).status).toBe('valid')
  })

  it('rejects two default else edges from the same node', () => {
    const workflow = linearWorkflow()
    workflow.edges.push({
      id: 'edge-b-other-default',
      fromNodeId: 'agent-b',
      toNodeId: 'done',
      type: 'conditional',
      label: 'else again',
      priority: 999,
      condition: { kind: 'always' },
    })

    const report = validateWorkflowDefinition(workflow)

    expect(report.status).toBe('invalid')
    expect(report.diagnostics.map((diagnostic) => diagnostic.code)).toContain(
      'duplicate_default_edge',
    )
  })

  it('accepts every starter workflow template', () => {
    for (const template of WORKFLOW_TEMPLATE_LIBRARY) {
      const report = validateWorkflowDefinition(
        instantiateWorkflowTemplate({
          projectId: 'project-1',
          templateId: template.id,
        }),
      )

      expect(report, template.id).toEqual({ status: 'valid', diagnostics: [] })
    }
  })

  it('keeps blank workflow drafts empty until the user adds a start node', () => {
    const draft = instantiateBlankWorkflow({
      projectId: 'project-1',
    })
    const report = validateWorkflowDefinition(
      draft,
    )

    expect(draft.nodes).toEqual([])
    expect(report.status).toBe('invalid')
    expect(report.diagnostics).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code: 'schema_invalid', path: 'startNodeId' }),
        expect.objectContaining({ code: 'schema_invalid', path: 'nodes' }),
      ]),
    )
  })
})
