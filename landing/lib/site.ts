const DEFAULT_SITE_URL = "https://xeroshell.com"
const DEFAULT_CLOUD_URL = "https://cloud.xeroshell.com"

function normalizeSiteUrl(url: string) {
  return url.replace(/\/+$/, "")
}

export const siteConfig = {
  name: "Xero",
  legalName: "Xero Labs",
  url: normalizeSiteUrl(process.env.NEXT_PUBLIC_SITE_URL ?? DEFAULT_SITE_URL),
  cloudUrl: normalizeSiteUrl(process.env.NEXT_PUBLIC_CLOUD_URL ?? DEFAULT_CLOUD_URL),
  title: "Xero | Agentic coding studio for desktop developers",
  description:
    "Xero is a local-first desktop app for building custom coding agents, visual workflows, and production software with your own model provider keys.",
  githubUrl: "https://github.com/hyperpush-org/xero",
  supportEmail: "team@xeroshell.com",
  keywords: [
    "Xero",
    "agentic coding",
    "AI coding agent",
    "desktop developer tools",
    "local-first AI",
    "Tauri app",
    "workflow automation",
    "software development",
    "OpenRouter",
    "Claude",
    "OpenAI",
    "xAI",
    "Grok",
  ],
} as const

export const siteDomain = new URL(siteConfig.url).hostname

export function absoluteUrl(path = "/") {
  if (/^https?:\/\//.test(path)) {
    return path
  }

  return `${siteConfig.url}${path.startsWith("/") ? path : `/${path}`}`
}

export function mailto(subject?: string) {
  const query = subject ? `?subject=${encodeURIComponent(subject)}` : ""
  return `mailto:${siteConfig.supportEmail}${query}`
}

export const desktopRelease = {
  version: "0.1.11",
  tag: "v0.1.11",
  releaseUrl: `${siteConfig.githubUrl}/releases/latest`,
} as const

export const desktopDownloads = [
  {
    id: "macos-apple-silicon",
    platform: "macOS",
    label: "Apple Silicon",
    detail: "M-series Macs",
    format: "DMG",
    size: "215 MB",
    href: "/download/macos-apple-silicon",
    recommended: true,
  },
  {
    id: "macos-intel",
    platform: "macOS",
    label: "Intel",
    detail: "x86_64 Macs",
    format: "DMG",
    size: "233 MB",
    href: "/download/macos-intel",
    recommended: false,
  },
  {
    id: "windows",
    platform: "Windows",
    label: "x64 installer",
    detail: "Windows 10 or newer",
    format: "EXE",
    size: "127 MB",
    href: "/download/windows",
    recommended: false,
  },
  {
    id: "linux",
    platform: "Linux",
    label: "x86_64 AppImage",
    detail: "Portable desktop build",
    format: "AppImage",
    size: "294 MB",
    href: "/download/linux",
    recommended: false,
  },
] as const

export const tuiInstallCommand = `curl -fsSL ${absoluteUrl("/install.sh")} | sh`
export const tuiPowerShellInstallCommand = `irm ${absoluteUrl("/install.ps1")} | iex`
