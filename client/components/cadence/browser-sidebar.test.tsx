/** @vitest-environment jsdom */

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

type ListenerHandle = () => void
type InvokeHandler = (args: Record<string, unknown> | undefined) => unknown

const invokeResponses = new Map<string, InvokeHandler>()
const eventListeners = new Map<string, ((event: { payload: unknown }) => void)[]>()

function resetBridge() {
  invokeResponses.clear()
  eventListeners.clear()
}

function registerInvoke(command: string, handler: InvokeHandler) {
  invokeResponses.set(command, handler)
}

function emitEvent(name: string, payload: unknown) {
  const listeners = eventListeners.get(name) ?? []
  listeners.forEach((listener) => listener({ payload }))
}

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: async (command: string, args?: Record<string, unknown>) => {
    const handler = invokeResponses.get(command)
    if (!handler) return undefined
    return handler(args)
  },
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    eventName: string,
    handler: (event: { payload: unknown }) => void,
  ): Promise<ListenerHandle> => {
    const list = eventListeners.get(eventName) ?? []
    list.push(handler)
    eventListeners.set(eventName, list)
    return () => {
      const existing = eventListeners.get(eventName) ?? []
      eventListeners.set(
        eventName,
        existing.filter((entry) => entry !== handler),
      )
    }
  },
}))

import { BrowserSidebar } from "./browser-sidebar"

// jsdom in this project ships a localStorage object whose methods aren't
// functions; install a minimal in-memory shim so the component's first-run
// check (which reads cookie-import state from storage) has something to call.
function installLocalStorage() {
  const store = new Map<string, string>()
  const shim: Storage = {
    get length() {
      return store.size
    },
    clear() {
      store.clear()
    },
    getItem(key) {
      return store.has(key) ? store.get(key)! : null
    },
    key(index) {
      return Array.from(store.keys())[index] ?? null
    },
    removeItem(key) {
      store.delete(key)
    },
    setItem(key, value) {
      store.set(key, String(value))
    },
  }
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: shim,
  })
  return shim
}

let cookieStorage: Storage | null = null

beforeEach(() => {
  cookieStorage = installLocalStorage()
})

afterEach(() => {
  resetBridge()
  vi.restoreAllMocks()
  cookieStorage?.clear()
})

describe("BrowserSidebar", () => {
  it("hydrates existing tabs when opened", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: "Example",
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])

    render(<BrowserSidebar open />)

    await waitFor(() => {
      const input = screen.getByLabelText("Address") as HTMLInputElement
      expect(input.value).toBe("https://example.com/")
    })
  })

  it("submits a URL and invokes browser_show with the expected shape", async () => {
    registerInvoke("browser_tab_list", async () => [])
    const shownUrls: string[] = []
    registerInvoke("browser_show", async (args) => {
      shownUrls.push(String((args as { url?: string })?.url ?? ""))
      return {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: String((args as { url?: string })?.url ?? ""),
        loading: true,
        canGoBack: false,
        canGoForward: false,
        active: true,
      }
    })

    render(<BrowserSidebar open />)

    const input = await screen.findByLabelText("Address")
    fireEvent.focus(input)
    fireEvent.change(input, { target: { value: "example.com" } })
    const form = input.closest("form")!
    fireEvent.submit(form)

    await waitFor(() => {
      expect(shownUrls).toEqual(["https://example.com"])
    })
  })

  it("enables back and forward buttons whenever a tab is active and dispatches the right command", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])

    const invoked: string[] = []
    registerInvoke("browser_back", async () => {
      invoked.push("back")
      return null
    })
    registerInvoke("browser_forward", async () => {
      invoked.push("forward")
      return null
    })

    render(<BrowserSidebar open />)

    // Wait for hydration so activeTab is set; once it is, both buttons should be
    // clickable (the webview safely no-ops at history endpoints).
    const input = (await screen.findByLabelText("Address")) as HTMLInputElement
    await waitFor(() => expect(input.value).toBe("https://example.com/"))

    const back = await screen.findByLabelText("Back")
    const forward = await screen.findByLabelText("Forward")
    await waitFor(() => expect(back).not.toBeDisabled())
    await waitFor(() => expect(forward).not.toBeDisabled())

    fireEvent.click(back)
    await waitFor(() => expect(invoked).toContain("back"))
    fireEvent.click(forward)
    await waitFor(() => expect(invoked).toContain("forward"))
  })

  it("disables back and forward when no tab is active", async () => {
    registerInvoke("browser_tab_list", async () => [])
    render(<BrowserSidebar open />)
    const back = await screen.findByLabelText("Back")
    const forward = await screen.findByLabelText("Forward")
    expect(back).toBeDisabled()
    expect(forward).toBeDisabled()
  })

  it("exposes the new-tab button as soon as a single tab exists", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])
    const shownUrls: string[] = []
    registerInvoke("browser_show", async (args) => {
      shownUrls.push(String((args as { url?: string })?.url ?? ""))
      return {
        id: "tab-2",
        label: "cadence-browser-tab-2",
        title: null,
        url: String((args as { url?: string })?.url ?? ""),
        loading: true,
        canGoBack: false,
        canGoForward: false,
        active: true,
      }
    })

    render(<BrowserSidebar open />)
    const newTabButton = await screen.findByLabelText("New tab")
    expect(newTabButton).toBeVisible()
    fireEvent.click(newTabButton)
    await waitFor(() => expect(shownUrls.length).toBe(1))
  })

  it("sends newTab=true and no tabId when the + button is clicked so the existing tab is not reused", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])
    let recordedArgs: Record<string, unknown> | null = null
    registerInvoke("browser_show", async (args) => {
      recordedArgs = (args as Record<string, unknown>) ?? null
      return {
        id: "tab-2",
        label: "cadence-browser-tab-2",
        title: null,
        url: String((args as { url?: string })?.url ?? ""),
        loading: true,
        canGoBack: false,
        canGoForward: false,
        active: true,
      }
    })

    render(<BrowserSidebar open />)
    const newTabButton = await screen.findByLabelText("New tab")
    fireEvent.click(newTabButton)
    await waitFor(() => expect(recordedArgs).not.toBeNull())
    expect(recordedArgs!.newTab).toBe(true)
    expect(recordedArgs!.tabId).toBeNull()
  })

  it("applies the resize handle inset to browser_show so the handle stays clickable", async () => {
    registerInvoke("browser_tab_list", async () => [])
    let recordedArgs: Record<string, unknown> | null = null
    registerInvoke("browser_show", async (args) => {
      recordedArgs = (args as Record<string, unknown>) ?? null
      return {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: String((args as { url?: string })?.url ?? ""),
        loading: true,
        canGoBack: false,
        canGoForward: false,
        active: true,
      }
    })

    render(<BrowserSidebar open />)
    const input = await screen.findByLabelText("Address")
    fireEvent.focus(input)
    fireEvent.change(input, { target: { value: "https://example.com" } })
    const form = input.closest("form")!
    fireEvent.submit(form)

    await waitFor(() => expect(recordedArgs).not.toBeNull())
    // The inset is 6px; the webview must start at least that far from the sidebar's left edge.
    expect(Number(recordedArgs!.x)).toBeGreaterThanOrEqual(6)
  })

  it("shows the cookie-import banner once a tab exists and a source is available, then dispatches browser_import_cookies", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])
    registerInvoke("browser_list_cookie_sources", async () => [
      { id: "chrome", label: "Google Chrome", available: true },
      { id: "firefox", label: "Firefox", available: false },
    ])
    const importCalls: Array<Record<string, unknown> | undefined> = []
    registerInvoke("browser_import_cookies", async (args) => {
      importCalls.push(args)
      return { source: "chrome", imported: 42, skipped: 1, domains: 7 }
    })

    render(<BrowserSidebar open />)

    const btn = await screen.findByRole("button", { name: "Google Chrome" })
    expect(btn).toBeVisible()
    // Unavailable source shouldn't render as a button.
    expect(screen.queryByRole("button", { name: "Firefox" })).toBeNull()

    fireEvent.click(btn)
    await waitFor(() => expect(importCalls.length).toBe(1))
    expect(importCalls[0]).toMatchObject({ source: "chrome" })

    // Success summary appears
    await waitFor(() =>
      expect(screen.getByText(/Imported 42 cookies/i)).toBeInTheDocument(),
    )

    // Banner is dismissible and sets the "prompted" flag so it stays closed.
    expect(window.localStorage.getItem("cadence.browser.cookieImportPrompted")).toBe(
      "true",
    )
  })

  it("does not show the cookie-import banner when the prompted flag is already set", async () => {
    window.localStorage.setItem("cadence.browser.cookieImportPrompted", "true")
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])
    registerInvoke("browser_list_cookie_sources", async () => [
      { id: "chrome", label: "Google Chrome", available: true },
    ])

    render(<BrowserSidebar open />)

    // Give the effect a chance to run.
    await screen.findByLabelText("Address")
    // Banner would render a "Google Chrome" action button; the toolbar doesn't.
    expect(screen.queryByRole("button", { name: "Google Chrome" })).toBeNull()
  })

  it("updates the address bar when a load_state event delivers a new URL while unfocused", async () => {
    registerInvoke("browser_tab_list", async () => [
      {
        id: "tab-1",
        label: "cadence-browser",
        title: null,
        url: "https://example.com/",
        loading: false,
        canGoBack: false,
        canGoForward: false,
        active: true,
      },
    ])

    render(<BrowserSidebar open />)
    const input = (await screen.findByLabelText("Address")) as HTMLInputElement
    // Wait for the initial URL so activeTabId is set before emitting load state.
    await waitFor(() => expect(input.value).toBe("https://example.com/"))

    await act(async () => {
      emitEvent("browser:load_state", {
        tabId: "tab-1",
        loading: false,
        url: "https://example.com/changed",
        error: null,
      })
    })

    await waitFor(() => expect(input.value).toBe("https://example.com/changed"))
  })
})
