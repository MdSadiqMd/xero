'use client'

import { memo } from 'react'
import { Handle, Position, type NodeProps } from '@xyflow/react'

import type { DbGroupFrameFlowNode } from '../build-agent-graph'

export const DbGroupFrameNode = memo(function DbGroupFrameNode({
  width,
  height,
}: NodeProps<DbGroupFrameFlowNode>) {
  const style: React.CSSProperties = {}
  if (typeof width === 'number') style.width = width
  if (typeof height === 'number') style.height = height

  return (
    <div className="agent-db-group-frame" style={style}>
      <Handle type="target" position={Position.Left} className="!bg-emerald-500" />
    </div>
  )
})
