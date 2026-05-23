import { NextResponse } from "next/server"

const releasePageUrl = "https://github.com/hyperpush-org/xero/releases/latest"
const releaseApiUrl = "https://api.github.com/repos/hyperpush-org/xero/releases/latest"

const assetPatterns = {
  "macos-apple-silicon": /^Xero_.*_aarch64_macos-aarch64\.dmg$/,
  "macos-intel": /^Xero_.*_x64_macos-x86_64\.dmg$/,
  windows: /^Xero_.*_x64-setup\.exe$/,
  linux: /^Xero_.*_amd64\.AppImage$/,
} as const

type GitHubRelease = {
  html_url?: string
  assets?: Array<{
    name: string
    browser_download_url?: string
  }>
}

export const revalidate = 300

function redirectTo(url: string) {
  const response = NextResponse.redirect(url, 302)
  response.headers.set("Cache-Control", "public, max-age=300, s-maxage=300")
  return response
}

export async function GET(
  _request: Request,
  context: { params: Promise<{ target: string }> },
) {
  const { target } = await context.params

  if (target === "release") {
    return redirectTo(releasePageUrl)
  }

  const pattern = assetPatterns[target as keyof typeof assetPatterns]
  if (!pattern) {
    return NextResponse.json({ error: "Unknown download target" }, { status: 404 })
  }

  try {
    const response = await fetch(releaseApiUrl, {
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "xero-landing",
      },
      next: { revalidate },
    })

    if (!response.ok) {
      return redirectTo(releasePageUrl)
    }

    const release = (await response.json()) as GitHubRelease
    const asset = release.assets?.find((candidate) => pattern.test(candidate.name))

    return redirectTo(asset?.browser_download_url ?? release.html_url ?? releasePageUrl)
  } catch {
    return redirectTo(releasePageUrl)
  }
}
