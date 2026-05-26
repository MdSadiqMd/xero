import { describe, expect, it } from "vitest"

import {
  extractBrowserSupportedDevServerUrls,
  isBrowserSupportedDevServerUrl,
  makeBrowserLaunchTarget,
} from "./browser-launch-targets"

describe("browser launch targets", () => {
  it("extracts local dev-server URLs from terminal output", () => {
    expect(
      extractBrowserSupportedDevServerUrls(
        "\u001b[32mVITE\u001b[0m ready\n  Local: http://localhost:5173/\n  API: http://127.0.0.1:4000/docs",
      ),
    ).toEqual(["http://localhost:5173/", "http://127.0.0.1:4000/docs"])
  })

  it("rejects non-local browser URLs for project launch targets", () => {
    expect(isBrowserSupportedDevServerUrl("https://example.com")).toBe(false)
    expect(isBrowserSupportedDevServerUrl("http://localhost:3000")).toBe(true)
  })

  it("builds stable project browser targets", () => {
    const target = makeBrowserLaunchTarget({
      label: "web",
      url: "http://localhost:5173/",
      source: "vite",
    })

    expect(target).toMatchObject({
      id: "browser-app:http://localhost:5173/",
      label: "web",
      url: "http://localhost:5173/",
      source: "vite",
    })
  })
})
