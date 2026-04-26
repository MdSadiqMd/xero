import { useCallback, useEffect, useRef, useState } from 'react'
import { useReducedMotion, type Transition } from 'motion/react'

const SIDEBAR_REVEAL_EASE: [number, number, number, number] = [0.22, 1, 0.36, 1]

export const SIDEBAR_WIDTH_SPRING: Transition = {
  type: 'spring',
  stiffness: 520,
  damping: 48,
  mass: 0.78,
}

export const SIDEBAR_LAYOUT_SPRING: Transition = {
  type: 'spring',
  stiffness: 620,
  damping: 52,
  mass: 0.82,
}

export const SIDEBAR_REVEAL_TRANSITION: Transition = {
  duration: 0.16,
  ease: SIDEBAR_REVEAL_EASE,
}

export const SIDEBAR_INSTANT_TRANSITION: Transition = { duration: 0 }

export function useSidebarMotion(isResizing = false) {
  const shouldReduceMotion = useReducedMotion()

  return {
    contentTransition: shouldReduceMotion
      ? SIDEBAR_INSTANT_TRANSITION
      : SIDEBAR_REVEAL_TRANSITION,
    layoutTransition: shouldReduceMotion
      ? SIDEBAR_INSTANT_TRANSITION
      : SIDEBAR_LAYOUT_SPRING,
    widthTransition: isResizing || shouldReduceMotion
      ? SIDEBAR_INSTANT_TRANSITION
      : SIDEBAR_WIDTH_SPRING,
  }
}

export function useDeferredSidebarActivation(open: boolean) {
  const [active, setActive] = useState(open)
  const activationFramesRef = useRef<number[]>([])

  const cancelActivationFrames = useCallback(() => {
    if (typeof window === 'undefined') return
    for (const frame of activationFramesRef.current) {
      window.cancelAnimationFrame(frame)
    }
    activationFramesRef.current = []
  }, [])

  useEffect(() => {
    if (!open) {
      cancelActivationFrames()
      setActive(false)
    }
  }, [cancelActivationFrames, open])

  useEffect(() => cancelActivationFrames, [cancelActivationFrames])

  const activateAfterAnimation = useCallback(() => {
    if (!open) return
    cancelActivationFrames()
    if (typeof window === 'undefined') {
      setActive(true)
      return
    }
    const firstFrame = window.requestAnimationFrame(() => {
      const secondFrame = window.requestAnimationFrame(() => {
        activationFramesRef.current = []
        setActive(true)
      })
      activationFramesRef.current = [secondFrame]
    })
    activationFramesRef.current = [firstFrame]
  }, [cancelActivationFrames, open])

  return { activateAfterAnimation, active }
}
