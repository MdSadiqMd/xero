import { Cpu, Download, ExternalLink, Laptop, Monitor, Terminal } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { desktopDownloads, desktopRelease } from "@/lib/site"

const downloadIcons = {
  "macos-apple-silicon": Cpu,
  "macos-intel": Laptop,
  windows: Monitor,
  linux: Terminal,
} as const

export function DesktopDownloads() {
  return (
    <div className="grid gap-3 md:grid-cols-2">
      {desktopDownloads.map((download) => {
        const Icon = downloadIcons[download.id]

        return (
          <article
            key={download.id}
            className="group flex min-h-44 flex-col justify-between rounded-lg border border-border/70 bg-secondary/20 p-4 text-left shadow-[0_24px_60px_-48px_black] transition-colors hover:border-primary/50 hover:bg-secondary/35"
          >
            <div className="flex items-start justify-between gap-4">
              <div className="flex min-w-0 items-start gap-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border/70 bg-background/70 text-primary">
                  <Icon className="h-5 w-5" />
                </div>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="text-base font-medium tracking-tight text-foreground">
                      {download.platform}
                    </h3>
                    {download.recommended ? (
                      <Badge
                        variant="outline"
                        className="border-primary/40 bg-primary/10 text-primary"
                      >
                        Recommended
                      </Badge>
                    ) : null}
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{download.label}</p>
                  <p className="mt-2 text-xs text-muted-foreground/70">{download.detail}</p>
                </div>
              </div>
              <div className="shrink-0 text-right font-mono text-[11px] uppercase tracking-[0.12em] text-muted-foreground/70">
                <div>{download.format}</div>
                <div className="mt-1 normal-case tracking-normal">{download.size}</div>
              </div>
            </div>

            <Button
              asChild
              className="mt-5 h-10 w-full gap-2 bg-primary text-primary-foreground hover:bg-primary/90"
            >
              <a
                href={download.href}
                aria-label={`Download Xero for ${download.platform} ${download.label}`}
              >
                <Download className="h-4 w-4" />
                Download
              </a>
            </Button>
          </article>
        )
      })}

      <a
        href={desktopRelease.releaseUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="flex items-center justify-between gap-4 rounded-lg border border-dashed border-border/70 bg-background/30 px-4 py-3 text-sm text-muted-foreground transition-colors hover:border-primary/40 hover:text-foreground md:col-span-2"
      >
        <span>All release assets, updater archives, and checksums</span>
        <span className="inline-flex shrink-0 items-center gap-1.5 text-primary">
          GitHub release
          <ExternalLink className="h-3.5 w-3.5" />
        </span>
      </a>
    </div>
  )
}
