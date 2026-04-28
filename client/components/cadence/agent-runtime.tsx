"use client"

import { useMemo } from 'react'

import type {
  AgentPaneView,
  AgentProviderModelView,
} from '@/src/features/cadence/use-cadence-desktop-state'
import type {
  RuntimeRunView,
  RuntimeAutoCompactPreferenceDto,
  RuntimeSessionView,
  UpsertNotificationRouteRequestDto,
} from '@/src/lib/cadence-model'
import {
  getRuntimeRunThinkingEffortLabel,
  type RuntimeRunControlInputDto,
} from '@/src/lib/cadence-model'

import {
  createEmptyCheckpointControlLoop,
  getCheckpointControlLoopCoverageAlertMeta,
  getCheckpointControlLoopRecoveryAlertMeta,
} from './agent-runtime/checkpoint-control-loop-helpers'
import { CheckpointControlLoopSection } from './agent-runtime/checkpoint-control-loop-section'
import {
  getComposerApprovalOptions,
  getComposerModelGroups,
  getComposerModelOption,
  getComposerPlaceholder,
  getComposerThinkingOptions,
  getSelectedProviderId,
  isSelectedProviderReadyForSession,
} from './agent-runtime/composer-helpers'
import { ComposerDock } from './agent-runtime/composer-dock'
import { EmptySessionState } from './agent-runtime/empty-session-state'
import {
  getStreamRunId,
  hasUsableRuntimeRunId,
} from './agent-runtime/runtime-stream-helpers'
import { SetupEmptyState } from './agent-runtime/setup-empty-state'
import { useAgentRuntimeController } from './agent-runtime/use-agent-runtime-controller'
import type { SpeechDictationAdapter } from './agent-runtime/use-speech-dictation'

interface AgentRuntimeProps {
  agent: AgentPaneView
  onOpenSettings?: () => void
  onOpenDiagnostics?: () => void
  onStartLogin?: (options?: { profileId?: string | null }) => Promise<RuntimeSessionView | null>
  onStartAutonomousRun?: () => Promise<unknown>
  onInspectAutonomousRun?: () => Promise<unknown>
  onCancelAutonomousRun?: (runId: string) => Promise<unknown>
  onStartRuntimeRun?: (options?: {
    controls?: RuntimeRunControlInputDto | null
    prompt?: string | null
  }) => Promise<RuntimeRunView | null>
  onUpdateRuntimeRunControls?: (request?: {
    controls?: RuntimeRunControlInputDto | null
    prompt?: string | null
    autoCompact?: RuntimeAutoCompactPreferenceDto | null
  }) => Promise<RuntimeRunView | null>
  onStartRuntimeSession?: (options?: { providerProfileId?: string | null }) => Promise<RuntimeSessionView | null>
  onStopRuntimeRun?: (runId: string) => Promise<RuntimeRunView | null>
  onSubmitManualCallback?: (flowId: string, manualInput: string) => Promise<RuntimeSessionView | null>
  onLogout?: () => Promise<RuntimeSessionView | null>
  onRetryStream?: () => Promise<void>
  onResolveOperatorAction?: (
    actionId: string,
    decision: 'approve' | 'reject',
    options?: { userAnswer?: string | null },
  ) => Promise<unknown>
  onResumeOperatorRun?: (actionId: string, options?: { userAnswer?: string | null }) => Promise<unknown>
  onRefreshNotificationRoutes?: (options?: { force?: boolean }) => Promise<unknown>
  onUpsertNotificationRoute?: (
    request: Omit<UpsertNotificationRouteRequestDto, 'projectId'>,
  ) => Promise<unknown>
  desktopAdapter?: SpeechDictationAdapter
}

const EMPTY_ACTION_REQUIRED_ITEMS: NonNullable<AgentPaneView['actionRequiredItems']> = []

export function AgentRuntime({
  agent,
  onOpenSettings,
  onOpenDiagnostics,
  onStartRuntimeRun,
  onUpdateRuntimeRunControls,
  onStopRuntimeRun,
  onStartRuntimeSession,
  onResolveOperatorAction,
  onResumeOperatorRun,
  desktopAdapter,
}: AgentRuntimeProps) {
  const runtimeSession = agent.runtimeSession ?? null
  const runtimeRun = agent.runtimeRun ?? null
  const renderableRuntimeRun = hasUsableRuntimeRunId(runtimeRun) ? runtimeRun : null
  const hasIncompleteRuntimeRunPayload = Boolean(runtimeRun && !renderableRuntimeRun)
  const runtimeStream = agent.runtimeStream ?? null
  const streamStatus = agent.runtimeStreamStatus ?? runtimeStream?.status ?? 'idle'
  const runtimeStreamItems = agent.runtimeStreamItems ?? runtimeStream?.items ?? []
  const activityItems = agent.activityItems ?? runtimeStream?.activityItems ?? []
  const skillItems = agent.skillItems ?? runtimeStream?.skillItems ?? []
  const actionRequiredItems = agent.actionRequiredItems ?? runtimeStream?.actionRequired ?? EMPTY_ACTION_REQUIRED_ITEMS
  const transcriptItems = runtimeStream?.transcriptItems ?? []
  const toolCalls = runtimeStream?.toolCalls ?? []
  const streamIssue = agent.runtimeStreamError ?? runtimeStream?.lastIssue ?? null

  const selectedProviderId = getSelectedProviderId(agent, runtimeSession)
  const selectedModelId = agent.selectedModelId?.trim() || null
  // Phase 3.5 follow-up: when `composerModelOptions` (the union of catalogs
  // across credentialed providers) is non-empty, use it as the picker's
  // source of truth. The legacy single-profile catalog stays as a fallback
  // for the transitional period.
  const composerModelOptionsView = useMemo<AgentProviderModelView[]>(() => {
    const unionOptions = agent.composerModelOptions ?? []
    if (unionOptions.length === 0) {
      return agent.providerModelCatalog.models
    }
    return unionOptions.map((option) => ({
      selectionKey: option.selectionKey,
      profileId: null,
      profileLabel: null,
      providerId: option.providerId,
      providerLabel: option.providerLabel,
      modelId: option.modelId,
      label: option.modelId,
      displayName: option.displayName,
      groupId: option.providerId,
      groupLabel: option.providerLabel,
      availability: 'available',
      availabilityLabel: 'Available',
      thinkingSupported: option.thinking.supported,
      thinkingEffortOptions: option.thinkingEffortOptions,
      defaultThinkingEffort: option.defaultThinkingEffort,
    }))
  }, [agent.composerModelOptions, agent.providerModelCatalog.models])
  const availableModels = composerModelOptionsView
  const openrouterApiKeyConfigured = agent.openrouterApiKeyConfigured ?? false
  const providerMismatch = agent.providerMismatch ?? false
  const hasRepositoryBinding = Boolean(agent.repositoryPath?.trim())
  // Phase 3.5 follow-up: prefer `agentRuntimeBlocked` (credentials-driven)
  // over the legacy `selectedProviderReadyForSession` projection when the
  // composer's union catalog is non-empty. While the legacy profile path
  // still feeds the picker (no credentials configured), keep the legacy
  // mismatch / readiness logic untouched — flipping prematurely would
  // produce misleading "no providers" empty states for users with legacy
  // data only.
  const useCredentialsTruth = (agent.composerModelOptions ?? []).length > 0
  const selectedProviderReadyForSession = useCredentialsTruth
    ? !agent.agentRuntimeBlocked
    : isSelectedProviderReadyForSession({
        selectedProviderId,
        selectedProfileReadiness: agent.selectedProfileReadiness ?? null,
        openrouterApiKeyConfigured,
      })
  const canMutateRuntimeRun = useCredentialsTruth ? !agent.agentRuntimeBlocked : !providerMismatch
  const canStartRuntimeSession = Boolean(
    canMutateRuntimeRun &&
      hasRepositoryBinding &&
      typeof onStartRuntimeSession === 'function' &&
      selectedProviderReadyForSession &&
      (!runtimeSession?.isAuthenticated || runtimeSession.providerId !== selectedProviderId),
  )
  const canStartRuntimeRun = Boolean(
    canMutateRuntimeRun &&
      hasRepositoryBinding &&
      typeof onStartRuntimeRun === 'function' &&
      runtimeSession?.isAuthenticated,
  )
  const canStopRuntimeRun = Boolean(
    hasRepositoryBinding && renderableRuntimeRun && !renderableRuntimeRun.isTerminal && typeof onStopRuntimeRun === 'function',
  )

  const controller = useAgentRuntimeController({
    projectId: agent.project.id,
    selectedModelSelectionKey: agent.selectedModelSelectionKey ?? agent.selectedModelOption?.selectionKey ?? selectedModelId,
    selectedThinkingEffort: agent.selectedThinkingEffort,
    selectedApprovalMode: agent.selectedApprovalMode,
    selectedPrompt: agent.selectedPrompt,
    availableModels,
    approvalRequests: agent.approvalRequests,
    operatorActionStatus: agent.operatorActionStatus,
    pendingOperatorActionId: agent.pendingOperatorActionId,
    pendingRuntimeRunAction: agent.pendingRuntimeRunAction,
    renderableRuntimeRun,
    runtimeRunPendingControls: agent.runtimeRunPendingControls,
    runtimeStream,
    runtimeStreamItems,
    runtimeRunActionStatus: agent.runtimeRunActionStatus,
    runtimeRunActionError: agent.runtimeRunActionError,
    canStartRuntimeRun,
    canStartRuntimeSession,
    canStopRuntimeRun,
    actionRequiredItems,
    dictationAdapter: desktopAdapter,
    dictationScopeKey: `${agent.project.id}:${agent.project.selectedAgentSessionId ?? 'none'}`,
    onStartRuntimeRun,
    onStartRuntimeSession,
    onUpdateRuntimeRunControls: canMutateRuntimeRun ? onUpdateRuntimeRunControls : undefined,
    onStopRuntimeRun,
    onResolveOperatorAction,
    onResumeOperatorRun,
  })

  const selectedComposerModel = useMemo(
    () => getComposerModelOption(availableModels, controller.composerModelId),
    [availableModels, controller.composerModelId],
  )
  const composerModelGroups = useMemo(
    () => getComposerModelGroups(availableModels, controller.composerModelId),
    [availableModels, controller.composerModelId],
  )
  const composerThinkingOptions = useMemo(
    () => getComposerThinkingOptions(selectedComposerModel),
    [selectedComposerModel],
  )
  const composerApprovalOptions = useMemo(() => getComposerApprovalOptions(), [])
  const composerThinkingPlaceholder = controller.composerThinkingEffort
    ? getRuntimeRunThinkingEffortLabel(controller.composerThinkingEffort)
    : controller.composerModelId
      ? 'Thinking unavailable'
      : 'Choose model'
  const streamRunId = getStreamRunId(runtimeStream, renderableRuntimeRun)
  const checkpointControlLoop = agent.checkpointControlLoop ?? createEmptyCheckpointControlLoop()
  const checkpointControlLoopRecoveryAlert = getCheckpointControlLoopRecoveryAlertMeta({
    controlLoop: checkpointControlLoop,
    trustSnapshot: {
      syncState: agent.trustSnapshot?.syncState ?? 'unavailable',
      syncReason:
        agent.trustSnapshot?.syncReason ??
        'Cadence has not projected notification sync trust for this project yet.',
    },
    autonomousRunErrorMessage: agent.autonomousRunErrorMessage,
    notificationSyncPollingActive: agent.notificationSyncPollingActive ?? false,
    notificationSyncPollingActionId: agent.notificationSyncPollingActionId ?? null,
    notificationSyncPollingBoundaryId: agent.notificationSyncPollingBoundaryId ?? null,
  })
  const checkpointControlLoopCoverageAlert = getCheckpointControlLoopCoverageAlertMeta(checkpointControlLoop)
  const showCheckpointControlLoopSection =
    checkpointControlLoop.items.length > 0 ||
    Boolean(checkpointControlLoopRecoveryAlert) ||
    Boolean(checkpointControlLoopCoverageAlert)
  // When credentials truth is in play, the legacy mismatch banner cannot
  // fire — chosen model fully determines the provider. Force `providerMismatch`
  // to false in the placeholder lookup so the rebind copy is suppressed.
  const composerPlaceholder = getComposerPlaceholder(
    runtimeSession,
    streamStatus,
    renderableRuntimeRun,
    streamRunId,
    {
      selectedProviderId,
      selectedProfileReadiness: agent.selectedProfileReadiness ?? null,
      openrouterApiKeyConfigured,
      providerMismatch: useCredentialsTruth ? false : providerMismatch,
    },
  )
  const showAgentSetupEmptyState = useCredentialsTruth
    ? Boolean(
        agent.agentRuntimeBlocked &&
          (!runtimeSession || runtimeSession.isSignedOut || runtimeSession.phase === 'idle'),
      )
    : Boolean(
        !providerMismatch &&
          !selectedProviderReadyForSession &&
          (!runtimeSession || runtimeSession.isSignedOut || runtimeSession.phase === 'idle'),
      )
  const hasSessionActivity = Boolean(
    hasIncompleteRuntimeRunPayload ||
      renderableRuntimeRun ||
      controller.recentRunReplacement ||
      streamIssue ||
      transcriptItems.length > 0 ||
      activityItems.length > 0 ||
      toolCalls.length > 0 ||
      skillItems.length > 0 ||
      actionRequiredItems.length > 0 ||
      runtimeStream?.completion ||
      runtimeStream?.failure,
  )
  const promptInputLabel = controller.promptInputAvailable ? 'Agent input' : 'Agent input unavailable'
  const sendButtonLabel = controller.promptInputAvailable ? 'Send message' : 'Send message unavailable'
  const isProviderLoggedIn = Boolean(
    selectedProviderReadyForSession ||
      runtimeSession?.isAuthenticated,
  )
  const showEmptySessionState = Boolean(
    !showAgentSetupEmptyState &&
      (useCredentialsTruth ? !agent.agentRuntimeBlocked : !providerMismatch) &&
      isProviderLoggedIn &&
      !hasSessionActivity,
  )
  const projectLabel =
    agent.project.repository?.displayName ?? agent.project.name ?? 'this project'

  return (
    <div className="flex min-h-0 min-w-0 flex-1">
      <div className="flex min-w-0 flex-1 flex-col">
        <div
          className={
            showAgentSetupEmptyState || showEmptySessionState
              ? 'flex flex-1 items-center justify-center overflow-y-auto scrollbar-thin px-6 py-5'
              : 'flex-1 overflow-y-auto scrollbar-thin px-4 py-4'
          }
        >
          {showAgentSetupEmptyState ? (
            <SetupEmptyState onOpenSettings={onOpenSettings} />
          ) : showEmptySessionState ? (
            <EmptySessionState
              projectLabel={projectLabel}
              onSelectSuggestion={(prompt) => {
                controller.handleDraftPromptChange(prompt)
                controller.promptInputRef.current?.focus()
              }}
            />
          ) : (
            <div className="mx-auto flex max-w-4xl flex-col gap-4">
              {showCheckpointControlLoopSection ? (
                <CheckpointControlLoopSection
                  checkpointControlLoop={checkpointControlLoop}
                  pendingApprovalCount={agent.pendingApprovalCount ?? 0}
                  operatorActionError={agent.operatorActionError ?? null}
                  operatorActionStatus={agent.operatorActionStatus}
                  pendingOperatorActionId={agent.pendingOperatorActionId ?? null}
                  pendingOperatorIntent={controller.pendingOperatorIntent}
                  operatorAnswers={controller.operatorAnswers}
                  checkpointControlLoopRecoveryAlert={checkpointControlLoopRecoveryAlert}
                  checkpointControlLoopCoverageAlert={checkpointControlLoopCoverageAlert}
                  onOperatorAnswerChange={controller.handleOperatorAnswerChange}
                  onResolveOperatorAction={controller.handleResolveOperatorAction}
                  onResumeOperatorRun={controller.handleResumeOperatorRun}
                  onResumeLiveActionRequired={controller.handleResumeLiveActionRequired}
                />
              ) : null}
            </div>
          )}
        </div>

        <ComposerDock
          composerApprovalMode={controller.composerApprovalMode}
          composerApprovalOptions={composerApprovalOptions}
          autoCompactEnabled={controller.autoCompactEnabled}
          composerModelGroups={composerModelGroups}
          composerModelId={controller.composerModelId}
          composerThinkingLevel={controller.composerThinkingEffort}
          composerThinkingOptions={composerThinkingOptions}
          composerThinkingPlaceholder={composerThinkingPlaceholder}
          controlsDisabled={controller.areControlsDisabled}
          dictation={controller.dictation}
          draftPrompt={controller.draftPrompt}
          isPromptDisabled={controller.isPromptDisabled}
          isSendDisabled={!controller.canSubmitPrompt}
          onComposerApprovalModeChange={controller.handleComposerApprovalModeChange}
          onAutoCompactEnabledChange={controller.handleAutoCompactEnabledChange}
          onComposerModelChange={controller.handleComposerModelChange}
          onComposerThinkingLevelChange={controller.handleComposerThinkingLevelChange}
          onDraftPromptChange={controller.handleDraftPromptChange}
          onSubmitDraftPrompt={() => void controller.handleSubmitDraftPrompt()}
          pendingRuntimeRunAction={agent.pendingRuntimeRunAction ?? null}
          placeholder={composerPlaceholder}
          promptInputRef={controller.promptInputRef}
          promptInputLabel={promptInputLabel}
          runtimeSessionBindInFlight={controller.runtimeSessionBindInFlight}
          runtimeRunActionError={controller.runtimeRunActionError}
          runtimeRunActionErrorTitle={controller.runtimeRunActionErrorTitle}
          runtimeRunActionStatus={agent.runtimeRunActionStatus}
          sendButtonLabel={sendButtonLabel}
          onOpenDiagnostics={onOpenDiagnostics}
        />
      </div>
    </div>
  )
}
