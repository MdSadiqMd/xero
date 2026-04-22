"use client"

import { useCallback, useEffect, useRef, useState } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"
import { ArrowLeft, ArrowRight, RotateCw } from "lucide-react"
import { cn } from "@/lib/utils"

const MIN_WIDTH = 320
const RIGHT_PADDING = 200
const DEFAULT_RATIO = 0.4

interface BrowserSidebarProps {
  open: boolean
}

interface ViewportRect {
  x: number
  y: number
  width: number
  height: number
}

function viewportDefaultWidth() {
  if (typeof window === "undefined") return 640
  return Math.round(window.innerWidth * DEFAULT_RATIO)
}

function viewportMaxWidth() {
  if (typeof window === "undefined") return 1600
  return Math.max(MIN_WIDTH, window.innerWidth - RIGHT_PADDING)
}

function normalizeUrl(input: string): string | null {
  const trimmed = input.trim()
  if (!trimmed) return null
  if (/^https?:\/\//i.test(trimmed)) return trimmed
  if (/^[\w.-]+\.[a-z]{2,}(\/.*)?$/i.test(trimmed)) return `https://${trimmed}`
  const query = encodeURIComponent(trimmed)
  return `https://www.google.com/search?q=${query}`
}

function rectsEqual(a: ViewportRect | null, b: ViewportRect): boolean {
  if (!a) return false
  return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height
}

export function BrowserSidebar({ open }: BrowserSidebarProps) {
  const [width, setWidth] = useState(viewportDefaultWidth)
  const [maxWidth, setMaxWidth] = useState(viewportMaxWidth)
  const [isResizing, setIsResizing] = useState(false)
  const [address, setAddress] = useState("")
  const [currentUrl, setCurrentUrl] = useState<string | null>(null)
  const [navError, setNavError] = useState<string | null>(null)
  const widthRef = useRef(width)
  widthRef.current = width
  const viewportRef = useRef<HTMLDivElement | null>(null)
  const lastSyncedRectRef = useRef<ViewportRect | null>(null)
  const hasWebviewRef = useRef(false)

  useEffect(() => {
    if (typeof window === "undefined") return
    const handleResize = () => {
      const nextMax = viewportMaxWidth()
      setMaxWidth(nextMax)
      setWidth((current) => Math.min(current, nextMax))
    }
    window.addEventListener("resize", handleResize)
    return () => window.removeEventListener("resize", handleResize)
  }, [])

  useEffect(() => {
    if (!open || !isTauri()) return
    if (!hasWebviewRef.current) return

    let rafId = 0
    const tick = () => {
      const node = viewportRef.current
      if (node) {
        const rect = node.getBoundingClientRect()
        const next: ViewportRect = {
          x: Math.round(rect.left),
          y: Math.round(rect.top),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        }
        if (next.width > 0 && next.height > 0 && !rectsEqual(lastSyncedRectRef.current, next)) {
          lastSyncedRectRef.current = next
          void invoke("browser_resize", { ...next }).catch(() => {
            /* swallow — resize is best-effort */
          })
        }
      }
      rafId = requestAnimationFrame(tick)
    }
    rafId = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(rafId)
  }, [open, currentUrl])

  useEffect(() => {
    if (open || !isTauri() || !hasWebviewRef.current) return
    lastSyncedRectRef.current = null
    void invoke("browser_hide").catch(() => {
      /* swallow */
    })
  }, [open])

  const handleResizeStart = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return
      event.preventDefault()
      const startX = event.clientX
      const startWidth = widthRef.current
      const ceiling = viewportMaxWidth()
      setMaxWidth(ceiling)
      setIsResizing(true)

      const previousCursor = document.body.style.cursor
      const previousSelect = document.body.style.userSelect
      document.body.style.cursor = "col-resize"
      document.body.style.userSelect = "none"

      const handleMove = (ev: PointerEvent) => {
        const delta = startX - ev.clientX
        const next = Math.max(MIN_WIDTH, Math.min(ceiling, startWidth + delta))
        setWidth(next)
      }
      const handleUp = () => {
        window.removeEventListener("pointermove", handleMove)
        window.removeEventListener("pointerup", handleUp)
        window.removeEventListener("pointercancel", handleUp)
        document.body.style.cursor = previousCursor
        document.body.style.userSelect = previousSelect
        setIsResizing(false)
      }

      window.addEventListener("pointermove", handleMove)
      window.addEventListener("pointerup", handleUp)
      window.addEventListener("pointercancel", handleUp)
    },
    [],
  )

  const handleResizeKey = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return
    event.preventDefault()
    const step = event.shiftKey ? 32 : 8
    const ceiling = viewportMaxWidth()
    setMaxWidth(ceiling)
    setWidth((current) => {
      const delta = event.key === "ArrowLeft" ? step : -step
      return Math.max(MIN_WIDTH, Math.min(ceiling, current + delta))
    })
  }, [])

  const handleSubmit = useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault()
      const next = normalizeUrl(address)
      if (!next) return
      setNavError(null)
      setCurrentUrl(next)
      setAddress(next)

      if (!isTauri()) return

      const node = viewportRef.current
      if (!node) return
      const rect = node.getBoundingClientRect()
      const payload = {
        url: next,
        x: Math.round(rect.left),
        y: Math.round(rect.top),
        width: Math.max(1, Math.round(rect.width)),
        height: Math.max(1, Math.round(rect.height)),
      }
      lastSyncedRectRef.current = {
        x: payload.x,
        y: payload.y,
        width: payload.width,
        height: payload.height,
      }
      hasWebviewRef.current = true
      void invoke("browser_show", payload).catch((error: unknown) => {
        hasWebviewRef.current = false
        const message =
          typeof error === "object" && error && "message" in error
            ? String((error as { message?: unknown }).message ?? "")
            : String(error)
        setNavError(message || "Failed to open page")
      })
    },
    [address],
  )

  return (
    <aside
      aria-hidden={!open}
      className={cn(
        "relative flex shrink-0 flex-col overflow-hidden border-l border-border/80 bg-sidebar",
        !isResizing && "transition-[width] duration-200 ease-out",
        !open && "border-l-0",
      )}
      inert={!open ? true : undefined}
      style={{ width: open ? width : 0 }}
    >
      <div
        aria-label="Resize browser sidebar"
        aria-orientation="vertical"
        aria-valuemax={maxWidth}
        aria-valuemin={MIN_WIDTH}
        aria-valuenow={width}
        className={cn(
          "absolute inset-y-0 -left-[3px] z-10 w-[6px] cursor-col-resize bg-transparent transition-colors",
          "hover:bg-primary/30",
          isResizing && "bg-primary/40",
        )}
        onKeyDown={handleResizeKey}
        onPointerDown={handleResizeStart}
        role="separator"
        tabIndex={open ? 0 : -1}
      />

      <div className="flex h-10 shrink-0 items-center gap-1 border-b border-border/70 px-2">
        <button
          aria-label="Back"
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-muted-foreground"
          disabled
          type="button"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
        </button>
        <button
          aria-label="Forward"
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-muted-foreground"
          disabled
          type="button"
        >
          <ArrowRight className="h-3.5 w-3.5" />
        </button>
        <button
          aria-label="Reload"
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-muted-foreground"
          disabled={!currentUrl}
          onClick={() => {
            if (!currentUrl || !isTauri()) return
            const node = viewportRef.current
            if (!node) return
            const rect = node.getBoundingClientRect()
            void invoke("browser_show", {
              url: currentUrl,
              x: Math.round(rect.left),
              y: Math.round(rect.top),
              width: Math.max(1, Math.round(rect.width)),
              height: Math.max(1, Math.round(rect.height)),
            }).catch(() => {
              /* swallow */
            })
          }}
          type="button"
        >
          <RotateCw className="h-3.5 w-3.5" />
        </button>

        <form className="ml-1 flex min-w-0 flex-1" onSubmit={handleSubmit}>
          <input
            aria-label="Address"
            className="h-7 w-full min-w-0 rounded-md border border-border/70 bg-background/40 px-2 text-[11.5px] text-foreground placeholder:text-muted-foreground/70 focus:border-primary/50 focus:outline-none"
            onChange={(event) => setAddress(event.target.value)}
            placeholder="Search or enter URL"
            type="text"
            value={address}
          />
        </form>
      </div>

      <div
        ref={viewportRef}
        className="relative flex min-h-0 flex-1 items-center justify-center bg-background/40"
      >
        {navError ? (
          <div className="px-6 text-center text-[11.5px] leading-relaxed text-destructive">
            {navError}
          </div>
        ) : !currentUrl ? (
          <div className="px-6 text-center text-[11.5px] leading-relaxed text-muted-foreground/80">
            Enter a URL to start browsing.
          </div>
        ) : !isTauri() ? (
          <div className="px-6 text-center text-[11.5px] leading-relaxed text-muted-foreground">
            <div className="font-mono text-foreground/85">{currentUrl}</div>
            <div className="mt-2 text-muted-foreground/80">
              Browser engine is only available in the desktop app.
            </div>
          </div>
        ) : null}
      </div>
    </aside>
  )
}
