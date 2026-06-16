import type { RuntimeAgentIdDto } from '@/src/lib/xero-model'
import { getRuntimeAgentLabel } from '@/src/lib/xero-model'
import type { ConversationTurn } from '@xero/ui/components/transcript/conversation-section'
import type { RoutingSuggestionDecision } from '@xero/ui/components/transcript/routing-suggestion-card'

type RoutingResolutionMode = 'manual' | 'automatic'

export type RoutingResolutionRecord = {
  acceptedTarget: RuntimeAgentIdDto | null
  acceptedTargetAgentDefinitionId: string | null
  acceptedTargetLabel: string | null
  routingResolutionMode: RoutingResolutionMode | null
}

export interface InternalRoutingContinuationDecision {
  kind: 'accept' | 'decline'
  targetLabel: string
}

const ROUTING_DECLINE_CONTINUATION_PREFIX =
  'The user chose to stay with the current Agent instead of switching to '
const ROUTING_DECLINE_CONTINUATION_BODY =
  'Continue the original request now. Do not stop at another routing recommendation for this same request.'
const ROUTING_ACCEPT_CONTINUATION_PREFIX =
  'The user accepted the routing suggestion to switch to '
const ROUTING_ACCEPT_CONTINUATION_BODY =
  'Continue the original request now in this same session.'

function normalizeRoutingContinuationText(text: string): string {
  return text.trim().replace(/\s+/g, ' ')
}

function parseRoutingContinuationTargetLabel(
  normalizedText: string,
  prefix: string,
  continuationBody: string,
): string | null {
  if (!normalizedText.startsWith(prefix)) {
    return null
  }

  const targetWithRest = normalizedText.slice(prefix.length)
  const continuationStart = targetWithRest.indexOf(`. ${continuationBody}`)
  if (continuationStart < 0) {
    return null
  }
  const targetLabel = targetWithRest.slice(0, continuationStart).trim()
  return targetLabel.length > 0 ? targetLabel : null
}

export function parseInternalRoutingContinuationPromptText(
  text: string,
): InternalRoutingContinuationDecision | null {
  const normalized = normalizeRoutingContinuationText(text)
  if (!normalized) return null

  const declinedTargetLabel = parseRoutingContinuationTargetLabel(
    normalized,
    ROUTING_DECLINE_CONTINUATION_PREFIX,
    ROUTING_DECLINE_CONTINUATION_BODY,
  )
  if (declinedTargetLabel) {
    return {
      kind: 'decline',
      targetLabel: declinedTargetLabel,
    }
  }

  const acceptedTargetLabel = parseRoutingContinuationTargetLabel(
    normalized,
    ROUTING_ACCEPT_CONTINUATION_PREFIX,
    ROUTING_ACCEPT_CONTINUATION_BODY,
  )
  if (acceptedTargetLabel) {
    return {
      kind: 'accept',
      targetLabel: acceptedTargetLabel,
    }
  }

  return null
}

export function isInternalRoutingContinuationPromptText(text: string): boolean {
  return parseInternalRoutingContinuationPromptText(text) !== null
}

export function isInternalRoutingContinuationTurn(turn: ConversationTurn): boolean {
  return (
    turn.kind === 'message' &&
    turn.role === 'user' &&
    isInternalRoutingContinuationPromptText(turn.text)
  )
}

export function filterInternalRoutingContinuationTurns(
  turns: readonly ConversationTurn[],
): ConversationTurn[] {
  const filteredTurns = turns.filter((turn) => !isInternalRoutingContinuationTurn(turn))
  return filteredTurns.length === turns.length ? turns.slice() : filteredTurns
}

export function getRoutingDecisionTargetLabel(
  decision: Pick<
    RoutingSuggestionDecision,
    'targetAgentId' | 'targetAgentDefinitionId' | 'targetLabel'
  >,
): string {
  return (
    decision.targetLabel?.trim() ||
    (decision.targetAgentDefinitionId ? 'the suggested custom agent' : getRuntimeAgentLabel(decision.targetAgentId))
  )
}

function getRoutingTurnTargetLabel(
  turn: Extract<ConversationTurn, { kind: 'routing_suggestion' }>,
): string {
  return (
    turn.targetLabel?.trim() ||
    (turn.targetAgentDefinitionId ? 'the suggested custom agent' : getRuntimeAgentLabel(turn.targetAgentId))
  )
}

function routingContinuationMatchesTurn(
  decision: InternalRoutingContinuationDecision,
  turn: Extract<ConversationTurn, { kind: 'routing_suggestion' }>,
): boolean {
  return (
    normalizeRoutingContinuationText(decision.targetLabel).toLocaleLowerCase() ===
    normalizeRoutingContinuationText(getRoutingTurnTargetLabel(turn)).toLocaleLowerCase()
  )
}

function resolveRoutingTurnFromContinuation(
  turn: Extract<ConversationTurn, { kind: 'routing_suggestion' }>,
  decision: InternalRoutingContinuationDecision,
): Extract<ConversationTurn, { kind: 'routing_suggestion' }> {
  if (decision.kind === 'decline') {
    return {
      ...turn,
      isResolved: true,
      acceptedTarget: null,
      acceptedTargetAgentDefinitionId: null,
      acceptedTargetLabel: null,
      routingResolutionMode: 'manual',
    }
  }

  return {
    ...turn,
    isResolved: true,
    acceptedTarget: turn.targetAgentId,
    acceptedTargetAgentDefinitionId: turn.targetAgentDefinitionId,
    acceptedTargetLabel: turn.targetLabel ?? decision.targetLabel,
    routingResolutionMode: 'manual',
  }
}

export function applyRoutingContinuationDecision(
  turns: readonly ConversationTurn[],
  decision: InternalRoutingContinuationDecision,
  beforeIndex: number,
): ConversationTurn[] {
  for (let candidateIndex = beforeIndex - 1; candidateIndex >= 0; candidateIndex -= 1) {
    const candidate = turns[candidateIndex]
    if (candidate.kind !== 'routing_suggestion') {
      continue
    }
    if (!routingContinuationMatchesTurn(decision, candidate)) {
      continue
    }

    const nextTurns = turns.slice()
    nextTurns[candidateIndex] = resolveRoutingTurnFromContinuation(candidate, decision)
    return nextTurns
  }

  return turns.slice()
}

export function applyPersistedRoutingContinuationResolutions(
  turns: readonly ConversationTurn[],
): ConversationTurn[] {
  let nextTurns = turns.slice()

  for (let index = 0; index < turns.length; index += 1) {
    const turn = turns[index]
    if (turn.kind !== 'message' || turn.role !== 'user') {
      continue
    }

    const decision = parseInternalRoutingContinuationPromptText(turn.text)
    if (!decision) {
      continue
    }

    nextTurns = applyRoutingContinuationDecision(nextTurns, decision, index)
  }

  return nextTurns
}

export function cleanPersistedRoutingContinuationTurns(
  turns: readonly ConversationTurn[],
): ConversationTurn[] {
  return filterInternalRoutingContinuationTurns(
    applyPersistedRoutingContinuationResolutions(turns),
  )
}

export function getRoutingResolutionForDecision(
  decision: RoutingSuggestionDecision,
): RoutingResolutionRecord {
  if (decision.kind === 'decline') {
    return {
      acceptedTarget: null,
      acceptedTargetAgentDefinitionId: null,
      acceptedTargetLabel: null,
      routingResolutionMode: decision.resolutionMode ?? 'manual',
    }
  }

  return {
    acceptedTarget: decision.targetAgentId,
    acceptedTargetAgentDefinitionId: decision.targetAgentDefinitionId ?? null,
    acceptedTargetLabel: decision.targetLabel ?? null,
    routingResolutionMode: decision.resolutionMode ?? 'manual',
  }
}
