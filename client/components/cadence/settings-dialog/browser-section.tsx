import { Check, Cookie, Loader2, RefreshCw } from "lucide-react"
import { useEffect } from "react"
import {
  useCookieImport,
  type DetectedBrowser,
} from "@/components/cadence/browser-cookie-import"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { SectionHeader } from "./section-header"

export function BrowserSection() {
  const { browsers, status, refresh, importFrom } = useCookieImport({
    autoLoad: true,
  })

  useEffect(() => {
    if (status.kind !== "success") return
    const t = setTimeout(() => {
      void refresh()
    }, 0)
    return () => clearTimeout(t)
  }, [status, refresh])

  const available = browsers.filter((b) => b.available)
  const unavailable = browsers.filter((b) => !b.available)
  const running = status.kind === "running"

  return (
    <div className="flex flex-col gap-7">
      <SectionHeader
        title="Browser"
        description="Copy cookies from other installed browsers into Cadence's in-app browser so you stay signed in while developing."
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 text-[12px]"
            onClick={() => void refresh()}
            aria-label="Rescan installed browsers"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Rescan
          </Button>
        }
      />

      <section className="flex flex-col gap-3">
        <div>
          <h4 className="text-[12.5px] font-semibold text-foreground">Import cookies</h4>
          <p className="mt-0.5 text-[12px] leading-[1.5] text-muted-foreground">
            Pick a source browser. The first import may prompt once for Keychain access; cookies apply on the next reload.
            The in-app browser must be open at least once.
          </p>
        </div>

        {available.length === 0 ? (
          <p className="rounded-md border border-dashed border-border/60 bg-secondary/20 px-3 py-3 text-center text-[12.5px] text-muted-foreground">
            No supported browsers detected on this machine.
          </p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {available.map((browser) => (
              <BrowserChip
                key={browser.id}
                browser={browser}
                running={running && status.kind === "running" && status.source === browser.id}
                disabled={running}
                onClick={() => void importFrom(browser)}
              />
            ))}
          </div>
        )}

        {status.kind === "success" ? (
          <div className="flex items-start gap-2 rounded-md border border-success/30 bg-success/[0.08] px-3 py-2">
            <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-success" />
            <p className="text-[12.5px] text-foreground/90">
              Imported <span className="font-semibold">{status.result.imported}</span> cookies across{" "}
              <span className="font-semibold">{status.result.domains}</span> domains
              {status.result.skipped > 0 ? ` (${status.result.skipped} skipped)` : ""}.
            </p>
          </div>
        ) : null}
        {status.kind === "error" ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[12.5px] text-destructive">
            {status.message}
          </p>
        ) : null}

        {unavailable.length > 0 ? (
          <p className="text-[11.5px] text-muted-foreground/80">
            <span className="font-medium text-muted-foreground">Not detected:</span>{" "}
            {unavailable.map((b) => b.label).join(", ")}.
          </p>
        ) : null}
      </section>
    </div>
  )
}

interface BrowserChipProps {
  browser: DetectedBrowser
  running: boolean
  disabled: boolean
  onClick: () => void
}

function BrowserChip({ browser, running, disabled, onClick }: BrowserChipProps) {
  return (
    <button
      className={cn(
        "group flex items-center gap-2 rounded-md border bg-background/60 px-3 py-1.5 text-[12.5px] text-foreground transition-all motion-fast",
        "border-border/70 hover:-translate-y-px hover:border-primary/40 hover:bg-background hover:shadow-sm",
        "disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:translate-y-0 disabled:hover:shadow-none",
      )}
      disabled={disabled}
      onClick={onClick}
      type="button"
    >
      {running ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />
      ) : (
        <Cookie className="h-3.5 w-3.5 text-muted-foreground group-hover:text-primary" />
      )}
      <span>{browser.label}</span>
    </button>
  )
}
