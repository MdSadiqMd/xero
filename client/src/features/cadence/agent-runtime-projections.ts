import type {
  AutonomousUnitArtifactView,
  AutonomousUnitHistoryEntryView,
  AutonomousUnitStatusDto,
  AutonomousWorkflowContextState,
  NotificationBrokerActionView,
  NotificationBrokerView,
  OperatorApprovalView,
  PlanningLifecycleStageView,
  PlanningLifecycleView,
  ResumeHistoryEntryView,
  RuntimeStreamActionRequiredItemView,
  WorkflowHandoffPackageView,
} from '@/src/lib/cadence-model'

export const MAX_RECENT_AUTONOMOUS_UNITS = 5
const MAX_RECENT_AUTONOMOUS_EVIDENCE_PREVIEWS = 2

export type RecentAutonomousUnitWorkflowState = AutonomousWorkflowContextState | 'unlinked'

export interface RecentAutonomousUnitEvidencePreview {
  artifactId: string
  artifactKindLabel: string
  statusLabel: string
  summary: string
  updatedAt: string
}

export interface RecentAutonomousUnitCardView {
  unitId: string
  sequence: number
  sequenceLabel: string
  kindLabel: string
  status: AutonomousUnitStatusDto
  statusLabel: string
  summary: string
  boundaryId: string | null
  updatedAt: string
  latestAttemptOnlyLabel: string
  latestAttemptLabel: string
  latestAttemptStatusLabel: string
  latestAttemptUpdatedAt: string | null
  latestAttemptSummary: string
  workflowState: RecentAutonomousUnitWorkflowState
  workflowStateLabel: string
  workflowNodeLabel: string
  workflowLinkageLabel: string
  workflowDetail: string
  evidenceCount: number
  evidenceStateLabel: string
  evidenceSummary: string
  latestEvidenceAt: string | null
  evidencePreviews: RecentAutonomousUnitEvidencePreview[]
}

export interface RecentAutonomousUnitsProjectionView {
  items: RecentAutonomousUnitCardView[]
  totalCount: number
  visibleCount: number
  hiddenCount: number
  isTruncated: boolean
  windowLabel: string
  latestAttemptOnlyCopy: string
  emptyTitle: string
  emptyBody: string
}

interface RecentAutonomousUnitWorkflowContext {
  state: RecentAutonomousUnitWorkflowState
  stateLabel: string
  nodeLabel: string
  linkageLabel: string
  detail: string
}

interface WorkflowProjectionMaps {
  stageByNodeId: Map<string, PlanningLifecycleStageView>
  handoffByTransitionId: Map<string, WorkflowHandoffPackageView>
  pendingApprovalByNodeId: Map<string, OperatorApprovalView>
}

function getTimestampMs(value: string | null | undefined): number {
  if (typeof value !== 'string' || value.trim().length === 0) {
    return 0
  }

  const parsed = Date.parse(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function normalizeText(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function sortByNewest<T>(items: readonly T[], getTimestamp: (item: T) => string | null | undefined): T[] {
  return [...items]
    .map((item, index) => ({ item, index }))
    .sort((left, right) => {
      const leftTime = getTimestampMs(getTimestamp(left.item))
      const rightTime = getTimestampMs(getTimestamp(right.item))
      if (leftTime === rightTime) {
        return left.index - right.index
      }

      return rightTime - leftTime
    })
    .map(({ item }) => item)
}

function getWorkflowStateLabel(state: RecentAutonomousUnitWorkflowState): string {
  switch (state) {
    case 'ready':
      return 'In sync'
    case 'awaiting_snapshot':
      return 'Snapshot lag'
    case 'awaiting_handoff':
      return 'Handoff pending'
    case 'unlinked':
      return 'Linkage pending'
  }
}

function buildWorkflowProjectionMaps(options: {
  lifecycle: PlanningLifecycleView
  handoffPackages: WorkflowHandoffPackageView[]
  approvalRequests: OperatorApprovalView[]
}): WorkflowProjectionMaps {
  const stageByNodeId = new Map<string, PlanningLifecycleStageView>()
  for (const stage of options.lifecycle.stages) {
    const nodeId = normalizeText(stage.nodeId)
    if (!nodeId || stageByNodeId.has(nodeId)) {
      continue
    }

    stageByNodeId.set(nodeId, stage)
  }

  const handoffByTransitionId = new Map<string, WorkflowHandoffPackageView>()
  for (const handoff of sortByNewest(options.handoffPackages, (pkg) => pkg.createdAt)) {
    const handoffTransitionId = normalizeText(handoff.handoffTransitionId)
    if (!handoffTransitionId || handoffByTransitionId.has(handoffTransitionId)) {
      continue
    }

    handoffByTransitionId.set(handoffTransitionId, handoff)
  }

  const pendingApprovalByNodeId = new Map<string, OperatorApprovalView>()
  for (const approval of sortByNewest(options.approvalRequests, (item) => item.updatedAt || item.createdAt)) {
    const gateNodeId = normalizeText(approval.gateNodeId)
    if (!approval.isPending || !gateNodeId || pendingApprovalByNodeId.has(gateNodeId)) {
      continue
    }

    pendingApprovalByNodeId.set(gateNodeId, approval)
  }

  return {
    stageByNodeId,
    handoffByTransitionId,
    pendingApprovalByNodeId,
  }
}

function getRecentUnitWorkflowContext(options: {
  entry: AutonomousUnitHistoryEntryView
  lifecycle: PlanningLifecycleView
  maps: WorkflowProjectionMaps
}): RecentAutonomousUnitWorkflowContext {
  const attemptLinkage = options.entry.latestAttempt?.workflowLinkage ?? null
  const unitLinkage = options.entry.unit.workflowLinkage ?? null
  const linkage = attemptLinkage ?? unitLinkage

  if (!linkage) {
    return {
      state: 'unlinked',
      stateLabel: getWorkflowStateLabel('unlinked'),
      nodeLabel: 'Not linked',
      linkageLabel: 'Workflow linkage missing',
      detail: 'Cadence has not persisted workflow-node and handoff linkage for this unit yet.',
    }
  }

  const linkedStage = options.maps.stageByNodeId.get(linkage.workflowNodeId) ?? null
  const activeLifecycleStage = options.lifecycle.activeStage
  const handoff = options.maps.handoffByTransitionId.get(linkage.handoffTransitionId) ?? null
  const pendingApproval = options.maps.pendingApprovalByNodeId.get(linkage.workflowNodeId) ?? null
  const activeStageMismatch = Boolean(activeLifecycleStage && activeLifecycleStage.nodeId !== linkage.workflowNodeId)
  const handoffHashMismatch = Boolean(handoff && handoff.packageHash !== linkage.handoffPackageHash)

  let state: RecentAutonomousUnitWorkflowState
  let detail: string

  if (!linkedStage) {
    state = 'awaiting_snapshot'
    detail =
      'Cadence has persisted workflow linkage for this unit, but the selected project snapshot has not exposed the linked lifecycle node yet.'
  } else if (activeStageMismatch) {
    state = 'awaiting_snapshot'
    detail = `Cadence is keeping lifecycle progression anchored to snapshot truth while the linked node \`${linkedStage.stageLabel}\` waits for the active lifecycle stage to catch up.`
  } else if (handoffHashMismatch) {
    state = 'awaiting_snapshot'
    detail =
      'Cadence found the linked handoff transition in the selected project snapshot, but the persisted handoff hash has not caught up to this unit yet.'
  } else if (!handoff) {
    state = 'awaiting_handoff'
    detail =
      'Cadence has persisted workflow linkage for this unit, but the linked handoff package is not visible in the selected project snapshot yet.'
  } else {
    state = 'ready'
    detail = 'Lifecycle stage, autonomous linkage, and handoff package all agree on backend truth for this unit.'
  }

  if (pendingApproval) {
    detail = `${detail} Pending approval \`${pendingApproval.title}\` is still blocking continuation at this linked node.`
  }

  return {
    state,
    stateLabel: getWorkflowStateLabel(state),
    nodeLabel: linkedStage?.stageLabel ?? linkage.workflowNodeId,
    linkageLabel: attemptLinkage ? 'Attempt linkage' : 'Unit linkage',
    detail,
  }
}

function collectEvidenceByUnitId(options: {
  history: AutonomousUnitHistoryEntryView[]
  recentArtifacts: AutonomousUnitArtifactView[]
}): Map<string, AutonomousUnitArtifactView[]> {
  const validUnitIds = new Set(
    options.history
      .map((entry) => normalizeText(entry.unit.unitId))
      .filter((unitId): unitId is string => Boolean(unitId)),
  )

  const evidenceByUnitId = new Map<string, AutonomousUnitArtifactView[]>()
  const appendEvidence = (artifact: AutonomousUnitArtifactView) => {
    const unitId = normalizeText(artifact.unitId)
    const artifactId = normalizeText(artifact.artifactId)
    if (!unitId || !artifactId || !validUnitIds.has(unitId)) {
      return
    }

    const existing = evidenceByUnitId.get(unitId)
    if (existing) {
      existing.push(artifact)
      return
    }

    evidenceByUnitId.set(unitId, [artifact])
  }

  for (const entry of options.history) {
    for (const artifact of entry.artifacts ?? []) {
      appendEvidence(artifact)
    }
  }

  for (const artifact of options.recentArtifacts) {
    appendEvidence(artifact)
  }

  for (const [unitId, artifacts] of evidenceByUnitId.entries()) {
    const deduped = new Map<string, AutonomousUnitArtifactView>()
    for (const artifact of sortByNewest(artifacts, (item) => item.updatedAt || item.createdAt)) {
      const artifactId = normalizeText(artifact.artifactId)
      if (!artifactId || deduped.has(artifactId)) {
        continue
      }

      deduped.set(artifactId, artifact)
    }

    evidenceByUnitId.set(unitId, [...deduped.values()])
  }

  return evidenceByUnitId
}

function getEvidenceProjection(options: {
  entry: AutonomousUnitHistoryEntryView
  evidenceByUnitId: Map<string, AutonomousUnitArtifactView[]>
}): Pick<RecentAutonomousUnitCardView, 'evidenceCount' | 'evidenceStateLabel' | 'evidenceSummary' | 'latestEvidenceAt' | 'evidencePreviews'> {
  const unitId = normalizeText(options.entry.unit.unitId)
  const latestAttemptId = normalizeText(options.entry.latestAttempt?.attemptId)
  const matchedEvidence = unitId ? options.evidenceByUnitId.get(unitId) ?? [] : []
  const filteredEvidence = matchedEvidence.filter((artifact) => {
    if (!latestAttemptId) {
      return true
    }

    return artifact.attemptId === latestAttemptId
  })
  const evidenceCount = filteredEvidence.length
  const evidencePreviews = filteredEvidence.slice(0, MAX_RECENT_AUTONOMOUS_EVIDENCE_PREVIEWS).map((artifact) => ({
    artifactId: artifact.artifactId,
    artifactKindLabel: artifact.artifactKindLabel,
    statusLabel: artifact.statusLabel,
    summary: artifact.summary,
    updatedAt: artifact.updatedAt || artifact.createdAt,
  }))

  if (evidenceCount === 0) {
    return {
      evidenceCount: 0,
      evidenceStateLabel: 'No durable evidence in bounded window',
      evidenceSummary: latestAttemptId
        ? 'Cadence has not retained a matching artifact for the latest attempt inside the bounded evidence window.'
        : 'Cadence has not retained a matching artifact for this unit inside the bounded evidence window.',
      latestEvidenceAt: null,
      evidencePreviews: [],
    }
  }

  return {
    evidenceCount,
    evidenceStateLabel: `${evidenceCount} recent evidence row${evidenceCount === 1 ? '' : 's'}`,
    evidenceSummary:
      evidenceCount === 1
        ? 'Showing the latest durable evidence row linked to this unit.'
        : 'Showing the newest durable evidence rows linked to this unit.',
    latestEvidenceAt: filteredEvidence[0]?.updatedAt || filteredEvidence[0]?.createdAt || null,
    evidencePreviews,
  }
}

function getLatestAttemptProjection(entry: AutonomousUnitHistoryEntryView): Pick<
  RecentAutonomousUnitCardView,
  'latestAttemptLabel' | 'latestAttemptStatusLabel' | 'latestAttemptUpdatedAt' | 'latestAttemptSummary'
> {
  const latestAttempt = entry.latestAttempt
  if (!latestAttempt) {
    return {
      latestAttemptLabel: 'Latest attempt unavailable',
      latestAttemptStatusLabel: 'Not recorded',
      latestAttemptUpdatedAt: null,
      latestAttemptSummary: 'Cadence has not persisted a latest-attempt row for this unit yet.',
    }
  }

  return {
    latestAttemptLabel: `Attempt #${latestAttempt.attemptNumber}`,
    latestAttemptStatusLabel: latestAttempt.statusLabel,
    latestAttemptUpdatedAt: latestAttempt.updatedAt,
    latestAttemptSummary: `Latest durable attempt is ${latestAttempt.statusLabel.toLowerCase()} for child session ${latestAttempt.childSessionId}.`,
  }
}

function compareRecentHistoryEntries(
  left: AutonomousUnitHistoryEntryView,
  right: AutonomousUnitHistoryEntryView,
): number {
  const leftTime = getTimestampMs(left.latestAttempt?.updatedAt || left.unit.updatedAt || left.unit.finishedAt || left.unit.startedAt)
  const rightTime = getTimestampMs(right.latestAttempt?.updatedAt || right.unit.updatedAt || right.unit.finishedAt || right.unit.startedAt)

  if (leftTime !== rightTime) {
    return rightTime - leftTime
  }

  return right.unit.sequence - left.unit.sequence
}

export function projectRecentAutonomousUnits(options: {
  autonomousHistory: AutonomousUnitHistoryEntryView[]
  autonomousRecentArtifacts: AutonomousUnitArtifactView[]
  lifecycle: PlanningLifecycleView
  handoffPackages: WorkflowHandoffPackageView[]
  approvalRequests: OperatorApprovalView[]
  limit?: number
}): RecentAutonomousUnitsProjectionView {
  const limit = Math.max(1, options.limit ?? MAX_RECENT_AUTONOMOUS_UNITS)
  const history = options.autonomousHistory
    .filter((entry) => Boolean(normalizeText(entry.unit.unitId)))
    .sort(compareRecentHistoryEntries)
  const visibleHistory = history.slice(0, limit)
  const maps = buildWorkflowProjectionMaps({
    lifecycle: options.lifecycle,
    handoffPackages: options.handoffPackages,
    approvalRequests: options.approvalRequests,
  })
  const evidenceByUnitId = collectEvidenceByUnitId({
    history,
    recentArtifacts: options.autonomousRecentArtifacts,
  })

  const items = visibleHistory.map((entry) => {
    const workflow = getRecentUnitWorkflowContext({
      entry,
      lifecycle: options.lifecycle,
      maps,
    })
    const evidence = getEvidenceProjection({
      entry,
      evidenceByUnitId,
    })
    const latestAttempt = getLatestAttemptProjection(entry)

    return {
      unitId: entry.unit.unitId,
      sequence: entry.unit.sequence,
      sequenceLabel: entry.unit.sequence > 0 ? `#${entry.unit.sequence}` : 'Not observed',
      kindLabel: entry.unit.kindLabel,
      status: entry.unit.status,
      statusLabel: entry.unit.statusLabel,
      summary: entry.unit.summary,
      boundaryId: entry.unit.boundaryId,
      updatedAt: entry.unit.updatedAt,
      latestAttemptOnlyLabel: 'Only the latest attempt is shown for this unit.',
      latestAttemptLabel: latestAttempt.latestAttemptLabel,
      latestAttemptStatusLabel: latestAttempt.latestAttemptStatusLabel,
      latestAttemptUpdatedAt: latestAttempt.latestAttemptUpdatedAt,
      latestAttemptSummary: latestAttempt.latestAttemptSummary,
      workflowState: workflow.state,
      workflowStateLabel: workflow.stateLabel,
      workflowNodeLabel: workflow.nodeLabel,
      workflowLinkageLabel: workflow.linkageLabel,
      workflowDetail: workflow.detail,
      evidenceCount: evidence.evidenceCount,
      evidenceStateLabel: evidence.evidenceStateLabel,
      evidenceSummary: evidence.evidenceSummary,
      latestEvidenceAt: evidence.latestEvidenceAt,
      evidencePreviews: evidence.evidencePreviews,
    } satisfies RecentAutonomousUnitCardView
  })

  const totalCount = history.length
  const visibleCount = items.length
  const hiddenCount = Math.max(0, totalCount - visibleCount)
  const isTruncated = hiddenCount > 0

  return {
    items,
    totalCount,
    visibleCount,
    hiddenCount,
    isTruncated,
    windowLabel: isTruncated
      ? `Showing ${visibleCount} of ${totalCount} durable units in the bounded recent-history window.`
      : visibleCount > 0
        ? `Showing ${visibleCount} durable unit${visibleCount === 1 ? '' : 's'} from the recent-history window.`
        : 'No durable recent units are available yet.',
    latestAttemptOnlyCopy: 'Only the latest durable attempt per unit is shown here.',
    emptyTitle: 'No recent autonomous units recorded',
    emptyBody: 'Cadence has not persisted a bounded autonomous unit history for this project yet.',
  }
}

export const MAX_CHECKPOINT_CONTROL_LOOPS = 6
const MAX_CHECKPOINT_CONTROL_LOOP_EVIDENCE_PREVIEWS = 3
const MAX_CHECKPOINT_CONTROL_LOOP_BROKER_ROUTE_PREVIEWS = 3

export type CheckpointControlLoopTruthSource =
  | 'live_and_durable'
  | 'live_hint_only'
  | 'durable_only'
  | 'recovered_durable'

export interface CheckpointControlLoopEvidencePreview {
  artifactId: string
  artifactKindLabel: string
  statusLabel: string
  summary: string
  updatedAt: string
}

export interface CheckpointControlLoopBrokerRoutePreview {
  routeId: string
  statusLabel: string
  detail: string
  updatedAt: string
}

export interface CheckpointControlLoopCardView {
  key: string
  actionId: string
  boundaryId: string | null
  title: string
  detail: string
  gateLinkageLabel: string | null
  truthSource: CheckpointControlLoopTruthSource
  truthSourceLabel: string
  truthSourceDetail: string
  liveActionRequired: RuntimeStreamActionRequiredItemView | null
  liveStateLabel: string
  liveStateDetail: string
  liveUpdatedAt: string | null
  approval: OperatorApprovalView | null
  durableStateLabel: string
  durableStateDetail: string
  durableUpdatedAt: string | null
  latestResume: ResumeHistoryEntryView | null
  resumeStateLabel: string
  resumeDetail: string
  resumeUpdatedAt: string | null
  brokerAction: NotificationBrokerActionView | null
  brokerStateLabel: string
  brokerStateDetail: string
  brokerLatestUpdatedAt: string | null
  brokerRoutePreviews: CheckpointControlLoopBrokerRoutePreview[]
  evidenceCount: number
  evidenceStateLabel: string
  evidenceSummary: string
  latestEvidenceAt: string | null
  evidencePreviews: CheckpointControlLoopEvidencePreview[]
  sortTimestamp: string | null
}

export interface CheckpointControlLoopProjectionView {
  items: CheckpointControlLoopCardView[]
  totalCount: number
  visibleCount: number
  hiddenCount: number
  isTruncated: boolean
  windowLabel: string
  emptyTitle: string
  emptyBody: string
  missingEvidenceCount: number
  liveHintOnlyCount: number
  durableOnlyCount: number
  recoveredCount: number
}

interface CheckpointControlLoopCardAccumulator {
  key: string
  actionId: string
  boundaryId: string | null
  liveActionRequired: RuntimeStreamActionRequiredItemView | null
  approval: OperatorApprovalView | null
  latestResume: ResumeHistoryEntryView | null
  brokerAction: NotificationBrokerActionView | null
  evidence: AutonomousUnitArtifactView[]
}

function createCheckpointControlLoopKey(actionId: string, boundaryId: string | null): string {
  return `${actionId}::${boundaryId ?? 'pending-boundary'}`
}

function extractRuntimeBoundaryIdFromActionId(actionId: string, actionType: string): string | null {
  const normalizedActionId = actionId.trim()
  const normalizedActionType = actionType.trim()
  if (
    normalizedActionId.length === 0 ||
    normalizedActionType.length === 0 ||
    !normalizedActionId.includes(':run:') ||
    !normalizedActionId.includes(':boundary:')
  ) {
    return null
  }

  const boundaryMarker = ':boundary:'
  const boundaryMarkerIndex = normalizedActionId.indexOf(boundaryMarker)
  if (boundaryMarkerIndex < 0) {
    return null
  }

  const boundaryAndAction = normalizedActionId.slice(boundaryMarkerIndex + boundaryMarker.length)
  const actionSuffix = `:${normalizedActionType}`
  if (!boundaryAndAction.endsWith(actionSuffix)) {
    return null
  }

  const boundaryId = boundaryAndAction.slice(0, -actionSuffix.length).trim()
  return boundaryId.length > 0 ? boundaryId : null
}

function formatCheckpointGateLinkage(approval: OperatorApprovalView | null): string | null {
  if (!approval?.gateNodeId || !approval.gateKey) {
    return null
  }

  const transition =
    approval.transitionFromNodeId && approval.transitionToNodeId && approval.transitionKind
      ? `${approval.transitionFromNodeId} → ${approval.transitionToNodeId} (${approval.transitionKind})`
      : null

  return transition
    ? `${approval.gateNodeId} · ${approval.gateKey} · ${transition}`
    : `${approval.gateNodeId} · ${approval.gateKey}`
}

function getUniqueCheckpointArtifacts(options: {
  autonomousHistory: AutonomousUnitHistoryEntryView[]
  autonomousRecentArtifacts: AutonomousUnitArtifactView[]
}): AutonomousUnitArtifactView[] {
  const deduped = new Map<string, AutonomousUnitArtifactView>()

  const appendArtifact = (artifact: AutonomousUnitArtifactView) => {
    const artifactId = normalizeText(artifact.artifactId)
    if (!artifactId || deduped.has(artifactId)) {
      return
    }

    deduped.set(artifactId, artifact)
  }

  for (const entry of options.autonomousHistory) {
    for (const artifact of entry.artifacts ?? []) {
      appendArtifact(artifact)
    }
  }

  for (const artifact of options.autonomousRecentArtifacts) {
    appendArtifact(artifact)
  }

  return sortByNewest([...deduped.values()], (artifact) => artifact.updatedAt || artifact.createdAt)
}

function getSingleCardForAction(options: {
  actionId: string
  cardsByKey: Map<string, CheckpointControlLoopCardAccumulator>
  keysByActionId: Map<string, string[]>
}): CheckpointControlLoopCardAccumulator | null {
  const keys = options.keysByActionId.get(options.actionId) ?? []
  if (keys.length !== 1) {
    return null
  }

  return options.cardsByKey.get(keys[0]) ?? null
}

function createCheckpointControlLoopCardAccumulator(actionId: string, boundaryId: string | null): CheckpointControlLoopCardAccumulator {
  return {
    key: createCheckpointControlLoopKey(actionId, boundaryId),
    actionId,
    boundaryId,
    liveActionRequired: null,
    approval: null,
    latestResume: null,
    brokerAction: null,
    evidence: [],
  }
}

function upsertCheckpointControlLoopCard(options: {
  actionId: string | null | undefined
  boundaryId?: string | null
  allowCreateWithoutBoundary: boolean
  cardsByKey: Map<string, CheckpointControlLoopCardAccumulator>
  keysByActionId: Map<string, string[]>
}): CheckpointControlLoopCardAccumulator | null {
  const actionId = normalizeText(options.actionId)
  if (!actionId) {
    return null
  }

  const boundaryId = normalizeText(options.boundaryId)
  if (boundaryId) {
    const key = createCheckpointControlLoopKey(actionId, boundaryId)
    const existing = options.cardsByKey.get(key)
    if (existing) {
      return existing
    }

    const created = createCheckpointControlLoopCardAccumulator(actionId, boundaryId)
    options.cardsByKey.set(key, created)
    const existingKeys = options.keysByActionId.get(actionId)
    if (existingKeys) {
      existingKeys.push(key)
    } else {
      options.keysByActionId.set(actionId, [key])
    }
    return created
  }

  const existing = getSingleCardForAction({
    actionId,
    cardsByKey: options.cardsByKey,
    keysByActionId: options.keysByActionId,
  })
  if (existing) {
    return existing
  }

  if (!options.allowCreateWithoutBoundary) {
    return null
  }

  const key = createCheckpointControlLoopKey(actionId, null)
  const created = options.cardsByKey.get(key) ?? createCheckpointControlLoopCardAccumulator(actionId, null)
  options.cardsByKey.set(key, created)
  const existingKeys = options.keysByActionId.get(actionId)
  if (existingKeys) {
    if (!existingKeys.includes(key)) {
      existingKeys.push(key)
    }
  } else {
    options.keysByActionId.set(actionId, [key])
  }
  return created
}

function appendCheckpointEvidence(card: CheckpointControlLoopCardAccumulator, artifact: AutonomousUnitArtifactView): void {
  const artifactId = normalizeText(artifact.artifactId)
  if (!artifactId) {
    return
  }

  if (card.evidence.some((entry) => entry.artifactId === artifactId)) {
    return
  }

  card.evidence.push(artifact)
  card.evidence.sort(
    (left, right) =>
      getTimestampMs(right.updatedAt || right.createdAt) - getTimestampMs(left.updatedAt || left.createdAt),
  )
}

function getCheckpointTruthSource(card: CheckpointControlLoopCardAccumulator): {
  truthSource: CheckpointControlLoopTruthSource
  truthSourceLabel: string
  truthSourceDetail: string
} {
  const hasDurableState = Boolean(card.approval || card.latestResume || card.evidence.length > 0)

  if (card.liveActionRequired && hasDurableState) {
    return {
      truthSource: 'live_and_durable',
      truthSourceLabel: 'Live + durable',
      truthSourceDetail:
        'The live action-required row still matches durable approval, resume, or evidence records for this boundary.',
    }
  }

  if (card.liveActionRequired) {
    return {
      truthSource: 'live_hint_only',
      truthSourceLabel: 'Live hint only',
      truthSourceDetail:
        'Cadence is showing the live action-required row while waiting for durable approval or evidence rows to persist.',
    }
  }

  if (card.approval) {
    return {
      truthSource: 'durable_only',
      truthSourceLabel: 'Durable only',
      truthSourceDetail:
        'The live row has cleared or is unavailable, so this card is anchored to durable approval and resume truth.',
    }
  }

  return {
    truthSource: 'recovered_durable',
    truthSourceLabel: 'Recovered durable state',
    truthSourceDetail:
      'Cadence recovered this boundary from recent resume or evidence history after the live row disappeared.',
  }
}

function getCheckpointLiveState(card: CheckpointControlLoopCardAccumulator): {
  liveStateLabel: string
  liveStateDetail: string
  liveUpdatedAt: string | null
} {
  if (card.liveActionRequired) {
    return {
      liveStateLabel: 'Live action required',
      liveStateDetail: normalizeText(card.liveActionRequired.detail) ?? 'The live runtime stream still reports this checkpoint boundary as blocked.',
      liveUpdatedAt: card.liveActionRequired.createdAt,
    }
  }

  if (card.latestResume?.status === 'started') {
    return {
      liveStateLabel: 'Live row cleared',
      liveStateDetail:
        'The live action-required row is no longer present for this boundary, so Cadence is showing the latest durable resume outcome instead.',
      liveUpdatedAt: card.latestResume.createdAt,
    }
  }

  if (card.approval?.isPending) {
    return {
      liveStateLabel: 'Live row unavailable',
      liveStateDetail:
        'The selected project snapshot still shows this checkpoint as pending even though the live stream no longer has a matching row.',
      liveUpdatedAt: card.approval.updatedAt || card.approval.createdAt,
    }
  }

  return {
    liveStateLabel: 'No live row',
    liveStateDetail:
      'No current action-required row is visible for this checkpoint in the bounded live stream window.',
    liveUpdatedAt: null,
  }
}

function getCheckpointDurableState(card: CheckpointControlLoopCardAccumulator): {
  durableStateLabel: string
  durableStateDetail: string
  durableUpdatedAt: string | null
} {
  if (card.approval) {
    return {
      durableStateLabel: card.approval.statusLabel,
      durableStateDetail: normalizeText(card.approval.detail) ?? 'Durable approval state is persisted for this action.',
      durableUpdatedAt: card.approval.updatedAt || card.approval.createdAt,
    }
  }

  if (card.latestResume?.status === 'started') {
    return {
      durableStateLabel: 'Approval cleared from durable snapshot',
      durableStateDetail:
        'Cadence has already cleared the durable approval row for this action and is relying on the persisted resume outcome.',
      durableUpdatedAt: card.latestResume.createdAt,
    }
  }

  if (card.liveActionRequired) {
    return {
      durableStateLabel: 'Durable approval pending refresh',
      durableStateDetail:
        'The live action-required row arrived before the selected-project snapshot persisted a matching durable approval row.',
      durableUpdatedAt: null,
    }
  }

  return {
    durableStateLabel: 'Durable approval missing',
    durableStateDetail:
      'Cadence could not find a durable approval row for this action inside the current selected-project snapshot.',
    durableUpdatedAt: null,
  }
}

function getCheckpointResumeState(card: CheckpointControlLoopCardAccumulator): {
  resumeStateLabel: string
  resumeDetail: string
  resumeUpdatedAt: string | null
} {
  if (card.latestResume?.status === 'failed') {
    return {
      resumeStateLabel: card.latestResume.statusLabel,
      resumeDetail: normalizeText(card.latestResume.summary) ?? 'The latest durable resume attempt failed for this action.',
      resumeUpdatedAt: card.latestResume.createdAt,
    }
  }

  if (card.latestResume?.status === 'started') {
    return {
      resumeStateLabel: card.latestResume.statusLabel,
      resumeDetail: normalizeText(card.latestResume.summary) ?? 'The latest durable resume attempt started for this action.',
      resumeUpdatedAt: card.latestResume.createdAt,
    }
  }

  if (card.approval?.isPending) {
    return {
      resumeStateLabel: 'Waiting on approval',
      resumeDetail: 'Cadence is waiting for operator input before this action can resume the run.',
      resumeUpdatedAt: card.approval.updatedAt || card.approval.createdAt,
    }
  }

  if (card.approval?.canResume) {
    return {
      resumeStateLabel: 'Ready to resume',
      resumeDetail: 'The durable approval is resolved, but no matching resume history row has been recorded yet.',
      resumeUpdatedAt: card.approval.updatedAt || card.approval.createdAt,
    }
  }

  return {
    resumeStateLabel: 'No durable resume',
    resumeDetail: 'Cadence has not recorded a durable resume outcome for this action yet.',
    resumeUpdatedAt: null,
  }
}

function getCheckpointBrokerState(card: CheckpointControlLoopCardAccumulator): {
  brokerStateLabel: string
  brokerStateDetail: string
  brokerLatestUpdatedAt: string | null
  brokerRoutePreviews: CheckpointControlLoopBrokerRoutePreview[]
} {
  const brokerAction = card.brokerAction
  if (!brokerAction) {
    return {
      brokerStateLabel: 'Broker diagnostics unavailable',
      brokerStateDetail: 'No notification broker fan-out rows were retained for this action in the bounded dispatch window.',
      brokerLatestUpdatedAt: null,
      brokerRoutePreviews: [],
    }
  }

  const routePreviews = brokerAction.dispatches.slice(0, MAX_CHECKPOINT_CONTROL_LOOP_BROKER_ROUTE_PREVIEWS).map((dispatch) => ({
    routeId: dispatch.routeId,
    statusLabel: dispatch.statusLabel,
    detail: dispatch.lastErrorMessage ?? dispatch.lastErrorCode ?? `Attempted ${dispatch.attemptCount} time${dispatch.attemptCount === 1 ? '' : 's'}.`,
    updatedAt: dispatch.updatedAt || dispatch.createdAt,
  }))

  if (brokerAction.hasFailures) {
    return {
      brokerStateLabel: `${brokerAction.failedCount} broker failure${brokerAction.failedCount === 1 ? '' : 's'}`,
      brokerStateDetail: `${brokerAction.dispatchCount} dispatch row${brokerAction.dispatchCount === 1 ? '' : 's'} remain visible for this action, and at least one route delivery failed.`,
      brokerLatestUpdatedAt: brokerAction.latestUpdatedAt,
      brokerRoutePreviews: routePreviews,
    }
  }

  if (brokerAction.hasPending) {
    return {
      brokerStateLabel: `${brokerAction.pendingCount} route${brokerAction.pendingCount === 1 ? '' : 's'} pending`,
      brokerStateDetail: `${brokerAction.dispatchCount} dispatch row${brokerAction.dispatchCount === 1 ? '' : 's'} are still waiting for broker delivery or operator claim updates.`,
      brokerLatestUpdatedAt: brokerAction.latestUpdatedAt,
      brokerRoutePreviews: routePreviews,
    }
  }

  return {
    brokerStateLabel: `${brokerAction.dispatchCount} route dispatch${brokerAction.dispatchCount === 1 ? '' : 'es'}`,
    brokerStateDetail: brokerAction.hasClaimed
      ? 'Remote route delivery and at least one operator claim were recorded for this action.'
      : 'Remote route fan-out completed without failed dispatches in the bounded broker window.',
    brokerLatestUpdatedAt: brokerAction.latestUpdatedAt,
    brokerRoutePreviews: routePreviews,
  }
}

function getCheckpointEvidenceState(card: CheckpointControlLoopCardAccumulator): {
  evidenceCount: number
  evidenceStateLabel: string
  evidenceSummary: string
  latestEvidenceAt: string | null
  evidencePreviews: CheckpointControlLoopEvidencePreview[]
} {
  const evidence = card.evidence
  const evidenceCount = evidence.length
  if (evidenceCount === 0) {
    return {
      evidenceCount: 0,
      evidenceStateLabel: 'No durable evidence in bounded window',
      evidenceSummary:
        'Cadence did not retain a matching tool result, verification row, or policy denial for this action in the bounded evidence window.',
      latestEvidenceAt: null,
      evidencePreviews: [],
    }
  }

  return {
    evidenceCount,
    evidenceStateLabel: `${evidenceCount} durable evidence row${evidenceCount === 1 ? '' : 's'}`,
    evidenceSummary:
      evidenceCount === 1
        ? 'Showing the latest durable evidence row linked to this action.'
        : 'Showing the newest durable evidence rows linked to this action.',
    latestEvidenceAt: evidence[0]?.updatedAt || evidence[0]?.createdAt || null,
    evidencePreviews: evidence.slice(0, MAX_CHECKPOINT_CONTROL_LOOP_EVIDENCE_PREVIEWS).map((artifact) => ({
      artifactId: artifact.artifactId,
      artifactKindLabel: artifact.artifactKindLabel,
      statusLabel: artifact.statusLabel,
      summary: artifact.summary,
      updatedAt: artifact.updatedAt || artifact.createdAt,
    })),
  }
}

function getCheckpointControlLoopTitle(card: CheckpointControlLoopCardAccumulator): string {
  return (
    normalizeText(card.approval?.title) ??
    normalizeText(card.liveActionRequired?.title) ??
    normalizeText(card.latestResume?.summary) ??
    normalizeText(card.evidence[0]?.summary) ??
    'Checkpoint action'
  )
}

function getCheckpointControlLoopDetail(card: CheckpointControlLoopCardAccumulator): string {
  return (
    normalizeText(card.approval?.detail) ??
    normalizeText(card.liveActionRequired?.detail) ??
    normalizeText(card.latestResume?.summary) ??
    normalizeText(card.evidence[0]?.summary) ??
    'Cadence is tracking this checkpoint boundary from the selected-project snapshot and bounded runtime evidence window.'
  )
}

function getCheckpointSortTimestamp(card: CheckpointControlLoopCardAccumulator): string | null {
  const candidates = [
    card.liveActionRequired?.createdAt,
    card.approval?.updatedAt,
    card.approval?.createdAt,
    card.latestResume?.createdAt,
    card.brokerAction?.latestUpdatedAt,
    card.evidence[0]?.updatedAt,
    card.evidence[0]?.createdAt,
  ]

  let selected: string | null = null
  let selectedTime = 0
  for (const candidate of candidates) {
    const candidateTime = getTimestampMs(candidate)
    if (candidateTime > selectedTime) {
      selected = candidate ?? null
      selectedTime = candidateTime
    }
  }

  return selected
}

export function projectCheckpointControlLoops(options: {
  actionRequiredItems: RuntimeStreamActionRequiredItemView[]
  approvalRequests: OperatorApprovalView[]
  resumeHistory: ResumeHistoryEntryView[]
  notificationBroker: NotificationBrokerView
  autonomousHistory: AutonomousUnitHistoryEntryView[]
  autonomousRecentArtifacts: AutonomousUnitArtifactView[]
  limit?: number
}): CheckpointControlLoopProjectionView {
  const limit = Math.max(1, options.limit ?? MAX_CHECKPOINT_CONTROL_LOOPS)
  const cardsByKey = new Map<string, CheckpointControlLoopCardAccumulator>()
  const keysByActionId = new Map<string, string[]>()

  const allArtifacts = getUniqueCheckpointArtifacts({
    autonomousHistory: options.autonomousHistory,
    autonomousRecentArtifacts: options.autonomousRecentArtifacts,
  })

  for (const artifact of allArtifacts) {
    const actionId = normalizeText(artifact.actionId)
    if (!actionId) {
      continue
    }

    const card = upsertCheckpointControlLoopCard({
      actionId,
      boundaryId: artifact.boundaryId,
      allowCreateWithoutBoundary: true,
      cardsByKey,
      keysByActionId,
    })

    if (card) {
      appendCheckpointEvidence(card, artifact)
    }
  }

  for (const liveActionRequired of sortByNewest(options.actionRequiredItems, (item) => item.createdAt)) {
    const card = upsertCheckpointControlLoopCard({
      actionId: liveActionRequired.actionId,
      boundaryId: liveActionRequired.boundaryId,
      allowCreateWithoutBoundary: false,
      cardsByKey,
      keysByActionId,
    })

    if (!card || card.liveActionRequired) {
      continue
    }

    card.liveActionRequired = liveActionRequired
  }

  for (const approval of sortByNewest(options.approvalRequests, (item) => item.updatedAt || item.createdAt)) {
    const card = upsertCheckpointControlLoopCard({
      actionId: approval.actionId,
      boundaryId: extractRuntimeBoundaryIdFromActionId(approval.actionId, approval.actionType),
      allowCreateWithoutBoundary: true,
      cardsByKey,
      keysByActionId,
    })

    if (!card || card.approval) {
      continue
    }

    card.approval = approval
  }

  for (const resumeEntry of sortByNewest(options.resumeHistory, (entry) => entry.createdAt)) {
    const card = upsertCheckpointControlLoopCard({
      actionId: resumeEntry.sourceActionId,
      allowCreateWithoutBoundary: true,
      cardsByKey,
      keysByActionId,
    })

    if (!card || card.latestResume) {
      continue
    }

    card.latestResume = resumeEntry
  }

  for (const brokerAction of options.notificationBroker.actions) {
    const card = upsertCheckpointControlLoopCard({
      actionId: brokerAction.actionId,
      allowCreateWithoutBoundary: false,
      cardsByKey,
      keysByActionId,
    })

    if (!card || card.brokerAction) {
      continue
    }

    card.brokerAction = brokerAction
  }

  const items = [...cardsByKey.values()]
    .map((card) => {
      const truthSource = getCheckpointTruthSource(card)
      const liveState = getCheckpointLiveState(card)
      const durableState = getCheckpointDurableState(card)
      const resumeState = getCheckpointResumeState(card)
      const brokerState = getCheckpointBrokerState(card)
      const evidenceState = getCheckpointEvidenceState(card)
      const sortTimestamp = getCheckpointSortTimestamp(card)

      return {
        key: card.key,
        actionId: card.actionId,
        boundaryId: card.boundaryId,
        title: getCheckpointControlLoopTitle(card),
        detail: getCheckpointControlLoopDetail(card),
        gateLinkageLabel: formatCheckpointGateLinkage(card.approval),
        truthSource: truthSource.truthSource,
        truthSourceLabel: truthSource.truthSourceLabel,
        truthSourceDetail: truthSource.truthSourceDetail,
        liveActionRequired: card.liveActionRequired,
        liveStateLabel: liveState.liveStateLabel,
        liveStateDetail: liveState.liveStateDetail,
        liveUpdatedAt: liveState.liveUpdatedAt,
        approval: card.approval,
        durableStateLabel: durableState.durableStateLabel,
        durableStateDetail: durableState.durableStateDetail,
        durableUpdatedAt: durableState.durableUpdatedAt,
        latestResume: card.latestResume,
        resumeStateLabel: resumeState.resumeStateLabel,
        resumeDetail: resumeState.resumeDetail,
        resumeUpdatedAt: resumeState.resumeUpdatedAt,
        brokerAction: card.brokerAction,
        brokerStateLabel: brokerState.brokerStateLabel,
        brokerStateDetail: brokerState.brokerStateDetail,
        brokerLatestUpdatedAt: brokerState.brokerLatestUpdatedAt,
        brokerRoutePreviews: brokerState.brokerRoutePreviews,
        evidenceCount: evidenceState.evidenceCount,
        evidenceStateLabel: evidenceState.evidenceStateLabel,
        evidenceSummary: evidenceState.evidenceSummary,
        latestEvidenceAt: evidenceState.latestEvidenceAt,
        evidencePreviews: evidenceState.evidencePreviews,
        sortTimestamp,
      } satisfies CheckpointControlLoopCardView
    })
    .sort((left, right) => {
      const byTimestamp = getTimestampMs(right.sortTimestamp) - getTimestampMs(left.sortTimestamp)
      if (byTimestamp !== 0) {
        return byTimestamp
      }

      return left.actionId.localeCompare(right.actionId)
    })

  const totalCount = items.length
  const visibleItems = items.slice(0, limit)
  const visibleCount = visibleItems.length
  const hiddenCount = Math.max(0, totalCount - visibleCount)
  const isTruncated = hiddenCount > 0

  return {
    items: visibleItems,
    totalCount,
    visibleCount,
    hiddenCount,
    isTruncated,
    windowLabel: isTruncated
      ? `Showing ${visibleCount} of ${totalCount} checkpoint actions in the bounded control-loop window.`
      : visibleCount > 0
        ? `Showing ${visibleCount} checkpoint action${visibleCount === 1 ? '' : 's'} from the bounded control-loop window.`
        : 'No checkpoint actions are visible in the bounded control-loop window.',
    emptyTitle: 'No checkpoint control loops recorded',
    emptyBody:
      'Cadence has not observed a live or durable checkpoint boundary for this project yet. Waiting boundaries, resume outcomes, and broker fan-out will appear here once recorded.',
    missingEvidenceCount: visibleItems.filter((item) => item.evidenceCount === 0).length,
    liveHintOnlyCount: visibleItems.filter((item) => item.truthSource === 'live_hint_only').length,
    durableOnlyCount: visibleItems.filter((item) => item.truthSource === 'durable_only').length,
    recoveredCount: visibleItems.filter((item) => item.truthSource === 'recovered_durable').length,
  }
}
