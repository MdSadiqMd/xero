import { Copy, Plus, Sparkles } from 'lucide-react'

import type {
  AgentRefDto,
  WorkflowAgentSummaryDto,
} from '@/src/lib/xero-model/workflow-agents'

import { AgentTemplatePicker } from './agent-template-picker'
import {
  CreateEntityDialog,
  type CreateEntityDialogView,
} from './create-entity-dialog'

interface CreateAgentDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  view: CreateEntityDialogView
  onSetView: (view: CreateEntityDialogView) => void
  canStartBlank: boolean
  canPickTemplate: boolean
  templates: WorkflowAgentSummaryDto[]
  templatesLoading: boolean
  templatesError: Error | null
  onStartBlank: () => void
  onPickTemplate: (ref: AgentRefDto) => void
}

export function CreateAgentDialog({
  open,
  onOpenChange,
  view,
  onSetView,
  canStartBlank,
  canPickTemplate,
  templates,
  templatesLoading,
  templatesError,
  onStartBlank,
  onPickTemplate,
}: CreateAgentDialogProps) {
  return (
    <CreateEntityDialog
      open={open}
      onOpenChange={onOpenChange}
      view={view}
      onSetView={onSetView}
      title="Create agent"
      icon={<Sparkles className="h-4 w-4" />}
      choiceDescription="Start from scratch or copy an existing agent as a template."
      templatesDescription="Templates open on the canvas with “(copy)” appended so you can edit freely."
      footerNote="Agents become reusable building blocks across workflows."
      blankChoice={{
        icon: <Plus className="h-4 w-4" />,
        title: 'New agent',
        description: 'Open the canvas with an empty agent header.',
        disabled: !canStartBlank,
        onClick: onStartBlank,
      }}
      templateChoice={
        canPickTemplate
          ? {
              icon: <Copy className="h-4 w-4" />,
              title: 'From template',
              description: 'Copy a built-in or saved agent and tweak it.',
              onClick: () => onSetView('templates'),
            }
          : undefined
      }
      templatesContent={
        <AgentTemplatePicker
          agents={templates}
          loading={templatesLoading}
          error={templatesError}
          onSelectTemplate={onPickTemplate}
          onStartBlank={onStartBlank}
          headless
          hideStartBlank
          className="max-w-none gap-0 rounded-lg border-0 bg-transparent p-0 shadow-none"
        />
      }
    />
  )
}
