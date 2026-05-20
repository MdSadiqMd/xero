import Link from "next/link"
import { Github, ShieldCheck } from "lucide-react"
import { TuiInstall } from "@/components/landing/tui-install"
import { siteConfig, tuiInstallCommand, tuiPowerShellInstallCommand } from "@/lib/site"

const installTargets = [
  { id: "unix", label: "macOS / Linux", command: tuiInstallCommand },
  { id: "windows", label: "Windows", command: tuiPowerShellInstallCommand },
]

export function CTA() {
  return (
    <section id="download" className="relative isolate overflow-hidden">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 -z-10 bg-grid [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)] opacity-30"
      />
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 -z-10 bg-radial-fade"
      />
      <div className="mx-auto w-full max-w-5xl px-4 py-24 text-center sm:px-6 lg:px-8 lg:py-32">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-primary">
          Xero TUI · The terminal edition
        </p>
        <h2 className="mx-auto mt-3 max-w-3xl font-sans text-3xl font-medium tracking-tight text-balance sm:text-5xl lg:text-6xl">
          Or run Xero in your terminal. <br className="hidden sm:block" />
          <span className="text-muted-foreground">Bring your own keys.</span>
        </h2>
        <p className="mx-auto mt-5 max-w-lg text-pretty text-muted-foreground">
          A full terminal interface for the same agents, workflows, and keys as
          the desktop app, without leaving your shell. Use it on its own or
          alongside the app.
        </p>

        <div className="mx-auto mt-10 max-w-2xl">
          <TuiInstall targets={installTargets} />

          <div className="mt-5 flex flex-wrap items-center justify-center gap-x-5 gap-y-2 text-xs text-muted-foreground/80">
            <span className="inline-flex items-center gap-1.5">
              <ShieldCheck className="h-3.5 w-3.5 text-primary" />
              SHA-256 checksummed builds
            </span>
            <Link
              href="/install.sh"
              className="underline-offset-4 transition-colors hover:text-foreground hover:underline"
            >
              View install script
            </Link>
            <Link
              href={siteConfig.githubUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 underline-offset-4 transition-colors hover:text-foreground hover:underline"
            >
              <Github className="h-3.5 w-3.5" />
              Source on GitHub
            </Link>
          </div>
        </div>
      </div>
    </section>
  )
}
