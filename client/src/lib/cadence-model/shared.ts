import { z } from 'zod'
import type { PhaseStep } from '@/components/cadence/data'

export const PHASE_STEPS = ['discuss', 'plan', 'execute', 'verify', 'ship'] as const satisfies readonly PhaseStep[]
export const STEP_INDEX = new Map(PHASE_STEPS.map((step, index) => [step, index]))

export const changeKindSchema = z.enum([
  'added',
  'modified',
  'deleted',
  'renamed',
  'copied',
  'type_change',
  'conflicted',
])

export const phaseStatusSchema = z.enum(['complete', 'active', 'pending', 'blocked'])
export const phaseStepSchema = z.enum(PHASE_STEPS)
export const nullableTextSchema = z.string().nullable().optional()
export const nonEmptyOptionalTextSchema = z.string().trim().min(1).nullable().optional()
export const isoTimestampSchema = z.string().datetime({ offset: true })
export const optionalIsoTimestampSchema = isoTimestampSchema.nullable().optional()

export function sortByNewest<T>(
  items: readonly T[],
  getTimestamp: (item: T) => string | null | undefined,
): T[] {
  return [...items]
    .map((item, index) => ({ item, index }))
    .sort((left, right) => {
      const leftTime = Date.parse(getTimestamp(left.item) ?? '')
      const rightTime = Date.parse(getTimestamp(right.item) ?? '')
      const normalizedLeftTime = Number.isFinite(leftTime) ? leftTime : 0
      const normalizedRightTime = Number.isFinite(rightTime) ? rightTime : 0

      if (normalizedLeftTime === normalizedRightTime) {
        return left.index - right.index
      }

      return normalizedRightTime - normalizedLeftTime
    })
    .map(({ item }) => item)
}

export function safePercent(completed: number, total: number): number {
  if (!Number.isFinite(total) || total <= 0) {
    return 0
  }

  const ratio = completed / total
  if (!Number.isFinite(ratio) || ratio <= 0) {
    return 0
  }

  return Math.max(0, Math.min(100, Math.round(ratio * 100)))
}

export function normalizeText(value: string | null | undefined, fallback: string): string {
  if (typeof value !== 'string') {
    return fallback
  }

  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : fallback
}

export function normalizeOptionalText(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}
