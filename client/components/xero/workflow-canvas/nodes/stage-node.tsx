'use client'

import { memo } from 'react'
import { Handle, Position, type NodeProps } from '@xyflow/react'
import { CheckSquare, Flag, GitBranch, ListChecks } from 'lucide-react'

import { Badge } from '@/components/ui/badge'

import type { StageFlowNode } from '../build-agent-graph'
import { humanizeIdentifier } from '../build-agent-graph'
import { CanvasNodeCard } from './canvas-node-card'

const STAGE_HANDLE_CLASS = '!bg-amber-500'

function gateLabel(check: StageFlowNode['data']['phase']['requiredChecks'] extends
  | readonly (infer G)[]
  | undefined
  ? G
  : never): string {
  if (check.kind === 'todo_completed') {
    return `todo: ${check.todoId}`
  }
  const count = check.minCount && check.minCount > 1 ? ` × ${check.minCount}` : ''
  return `tool: ${check.toolName}${count}`
}

export const StageNode = memo(function StageNode({
  data,
}: NodeProps<StageFlowNode>) {
  const { phase, isStart } = data
  const requiredChecks = phase.requiredChecks ?? []

  return (
    <>
      {/* Both handles on the left edge so stage→stage edges between
          vertically-stacked cards bow out cleanly on one side instead of
          spiralling around the card. React Flow needs unique ids when two
          handles share an axis. */}
      <Handle
        id="in"
        type="target"
        position={Position.Left}
        className={STAGE_HANDLE_CLASS}
      />
      <Handle
        id="out"
        type="source"
        position={Position.Left}
        className={STAGE_HANDLE_CLASS}
      />
      <CanvasNodeCard
        title={phase.title || humanizeIdentifier(phase.id)}
        subtitle={phase.id}
        icon={GitBranch}
        tone="amber"
        iconClassName="bg-amber-500/12 text-amber-500 ring-amber-500/30"
        testId="stage-node"
        attributes={{ 'data-phase-id': phase.id }}
        detail={phase.description || undefined}
        badges={
          isStart ? (
            <Badge
              variant="outline"
              className="h-5 px-1.5 text-[9.5px] font-medium border-amber-500/40 bg-amber-500/12 text-amber-700 dark:text-amber-300"
            >
              <Flag className="mr-0.5 h-2.5 w-2.5" aria-hidden="true" /> start
            </Badge>
          ) : null
        }
        chips={
          requiredChecks.length > 0 ? (
            <>
              <ListChecks
                className="h-3 w-3 shrink-0 text-muted-foreground/70"
                aria-hidden="true"
              />
              {requiredChecks.map((check, index) => (
                <Badge
                  key={`${check.kind}:${index}`}
                  variant="outline"
                  className="h-5 px-1.5 text-[9.5px] font-medium border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
                >
                  <CheckSquare className="mr-0.5 h-2.5 w-2.5" aria-hidden="true" />
                  {gateLabel(check)}
                </Badge>
              ))}
            </>
          ) : null
        }
        footer={
          phase.retryLimit !== undefined ? (
            <div className="text-[10px] text-muted-foreground/80">
              retry limit: {phase.retryLimit}
            </div>
          ) : null
        }
      />
    </>
  )
})
