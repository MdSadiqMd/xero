"use client"

import { useEffect, useState } from "react"
import { Check, Copy } from "lucide-react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

export type InstallTarget = {
  id: string
  label: string
  command: string
}

export function TuiInstall({ targets }: { targets: InstallTarget[] }) {
  const [active, setActive] = useState(targets[0]?.id)
  const [copied, setCopied] = useState(false)

  const current = targets.find((target) => target.id === active) ?? targets[0]

  useEffect(() => {
    if (!copied) {
      return
    }

    const timeout = window.setTimeout(() => setCopied(false), 1600)
    return () => window.clearTimeout(timeout)
  }, [copied])

  async function copyCommand() {
    await navigator.clipboard.writeText(current.command)
    setCopied(true)
  }

  return (
    <Tabs
      value={active}
      onValueChange={(value) => {
        setActive(value)
        setCopied(false)
      }}
      className="gap-0 overflow-hidden rounded-xl border border-border/70 bg-secondary/20 text-left shadow-[0_30px_70px_-40px_black] backdrop-blur"
    >
      <div className="flex items-center justify-between gap-3 border-b border-border/60 bg-secondary/30 px-3 py-2">
        <div className="flex min-w-0 items-center gap-3">
          <div aria-hidden className="hidden items-center gap-1.5 pl-1 sm:flex">
            <span className="h-2.5 w-2.5 rounded-full bg-border" />
            <span className="h-2.5 w-2.5 rounded-full bg-border" />
            <span className="h-2.5 w-2.5 rounded-full bg-border" />
          </div>
          <TabsList className="h-auto gap-1 bg-transparent p-0">
            {targets.map((target) => (
              <TabsTrigger
                key={target.id}
                value={target.id}
                className="rounded-md px-2.5 py-1 text-xs text-muted-foreground data-[state=active]:bg-primary/15 data-[state=active]:text-foreground data-[state=active]:shadow-none dark:data-[state=active]:bg-primary/15"
              >
                {target.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </div>
        <button
          type="button"
          onClick={copyCommand}
          aria-label={`Copy ${current.label} install command`}
          className="inline-flex h-7 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        >
          {copied ? <Check className="h-3.5 w-3.5 text-primary" /> : <Copy className="h-3.5 w-3.5" />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      {targets.map((target) => (
        <TabsContent key={target.id} value={target.id} className="mt-0">
          <pre className="overflow-x-auto px-4 py-4 font-mono text-[13px] leading-relaxed text-foreground sm:text-sm">
            <code>
              <span className="select-none text-primary">$ </span>
              {target.command}
            </code>
          </pre>
        </TabsContent>
      ))}
    </Tabs>
  )
}
