import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { isTauriMock, tauriWindowMock, invokeMock, listenMock, openUrlMock } = vi.hoisted(() => ({
  isTauriMock: vi.fn(() => false),
  tauriWindowMock: {
    close: vi.fn(),
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    startDragging: vi.fn(),
  },
  invokeMock: vi.fn(async () => ({
    android: { present: false },
    ios: { present: false, supported: false },
  })),
  listenMock: vi.fn(async () => () => undefined),
  openUrlMock: vi.fn(async () => undefined),
}))

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: isTauriMock,
  invoke: invokeMock,
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => tauriWindowMock,
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: openUrlMock,
}))

import { CadenceShell } from './shell'

describe('CadenceShell', () => {
  beforeEach(() => {
    isTauriMock.mockReturnValue(false)
    tauriWindowMock.close.mockReset()
    tauriWindowMock.minimize.mockReset()
    tauriWindowMock.toggleMaximize.mockReset()
    tauriWindowMock.startDragging.mockReset()
    invokeMock.mockReset()
    invokeMock.mockResolvedValue({
      android: { present: false },
      ios: { present: false, supported: false },
    })
    listenMock.mockReset()
    listenMock.mockResolvedValue(() => undefined)
    openUrlMock.mockReset()
    openUrlMock.mockResolvedValue(undefined)
  })

  it.each(['macos', 'windows'] as const)('renders the sidebar toggle in the %s titlebar', (platform) => {
    const onToggleSidebar = vi.fn()

    render(
      <CadenceShell
        activeView="phases"
        onToggleSidebar={onToggleSidebar}
        onViewChange={() => undefined}
        platformOverride={platform}
      >
        <div>Body</div>
      </CadenceShell>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Collapse project sidebar' }))

    expect(onToggleSidebar).toHaveBeenCalledTimes(1)
    expect(screen.getByRole('navigation')).toBeVisible()
  })

  it.each(['macos', 'windows'] as const)('toggles the arcade from the %s titlebar', (platform) => {
    const onToggleGames = vi.fn()

    const { rerender } = render(
      <CadenceShell
        activeView="phases"
        onToggleGames={onToggleGames}
        onViewChange={() => undefined}
        platformOverride={platform}
      >
        <div>Body</div>
      </CadenceShell>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Open arcade' }))
    expect(onToggleGames).toHaveBeenCalledTimes(1)

    rerender(
      <CadenceShell
        activeView="phases"
        gamesOpen
        onToggleGames={onToggleGames}
        onViewChange={() => undefined}
        platformOverride={platform}
      >
        <div>Body</div>
      </CadenceShell>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Close arcade' }))
    expect(onToggleGames).toHaveBeenCalledTimes(2)
  })

  it.each(['macos', 'windows'] as const)('toggles the Android emulator from the %s titlebar', (platform) => {
    const onToggleAndroid = vi.fn()

    const { rerender } = render(
      <CadenceShell
        activeView="phases"
        onToggleAndroid={onToggleAndroid}
        onViewChange={() => undefined}
        platformOverride={platform}
      >
        <div>Body</div>
      </CadenceShell>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Open Android emulator' }))
    expect(onToggleAndroid).toHaveBeenCalledTimes(1)

    rerender(
      <CadenceShell
        activeView="phases"
        androidOpen
        onToggleAndroid={onToggleAndroid}
        onViewChange={() => undefined}
        platformOverride={platform}
      >
        <div>Body</div>
      </CadenceShell>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Close Android emulator' }))
    expect(onToggleAndroid).toHaveBeenCalledTimes(2)
  })

  it('flips the iOS button to an Install Xcode CTA when Xcode is missing', async () => {
    isTauriMock.mockReturnValue(true)
    invokeMock.mockResolvedValue({
      android: { present: true },
      ios: { present: false, supported: true },
    })
    const onToggleIos = vi.fn()

    render(
      <CadenceShell
        activeView="phases"
        onToggleIos={onToggleIos}
        onViewChange={() => undefined}
        platformOverride="macos"
      >
        <div>Body</div>
      </CadenceShell>,
    )

    const ctaBtn = await screen.findByRole('button', { name: 'Install Xcode' })
    fireEvent.click(ctaBtn)
    await waitFor(() =>
      expect(openUrlMock).toHaveBeenCalledWith('https://apps.apple.com/app/xcode/id497799835'),
    )
    // Clicking the CTA never toggles the iOS sidebar — opening an
    // empty panel would just repeat the same "Install Xcode" message.
    expect(onToggleIos).not.toHaveBeenCalled()
    expect(screen.queryByRole('button', { name: /Open iOS simulator/ })).toBeNull()
  })

  it('tints the Android button amber when the SDK is absent', async () => {
    isTauriMock.mockReturnValue(true)
    invokeMock.mockResolvedValue({
      android: { present: false },
      ios: { present: true, supported: true },
    })

    render(
      <CadenceShell
        activeView="phases"
        onToggleAndroid={vi.fn()}
        onViewChange={() => undefined}
        platformOverride="macos"
      >
        <div>Body</div>
      </CadenceShell>,
    )

    const btn = await screen.findByRole('button', { name: 'Open Android emulator' })
    await waitFor(() =>
      expect(btn.getAttribute('title')).toMatch(/Android SDK not installed/),
    )
  })

  it('renders the iOS button only on macOS', () => {
    const onToggleIos = vi.fn()

    const { rerender } = render(
      <CadenceShell
        activeView="phases"
        onToggleIos={onToggleIos}
        onViewChange={() => undefined}
        platformOverride="macos"
      >
        <div>Body</div>
      </CadenceShell>,
    )

    expect(screen.getByRole('button', { name: 'Open iOS simulator' })).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Open iOS simulator' }))
    expect(onToggleIos).toHaveBeenCalledTimes(1)

    for (const platform of ['windows', 'linux'] as const) {
      rerender(
        <CadenceShell
          activeView="phases"
          onToggleIos={onToggleIos}
          onViewChange={() => undefined}
          platformOverride={platform}
        >
          <div>Body</div>
        </CadenceShell>,
      )
      expect(screen.queryByRole('button', { name: /iOS simulator/ })).toBeNull()
    }
  })

  it.each(['macos', 'windows'] as const)('keeps titlebar controls out of the drag strip in %s', (platform) => {
    isTauriMock.mockReturnValue(true)

    const { container } = render(
      <CadenceShell activeView="phases" onOpenSettings={() => undefined} onViewChange={() => undefined} platformOverride={platform}>
        <div>Body</div>
      </CadenceShell>,
    )

    const header = container.querySelector('header')
    expect(header).not.toHaveAttribute('data-tauri-drag-region')

    fireEvent.mouseDown(screen.getByRole('button', { name: 'Settings' }), { button: 0, detail: 2 })

    expect(tauriWindowMock.toggleMaximize).not.toHaveBeenCalled()
    expect(tauriWindowMock.startDragging).not.toHaveBeenCalled()
  })

  it.each(['macos', 'windows'] as const)('preserves drag strip gestures in %s', async (platform) => {
    isTauriMock.mockReturnValue(true)

    const { container } = render(
      <CadenceShell activeView="phases" onViewChange={() => undefined} platformOverride={platform}>
        <div>Body</div>
      </CadenceShell>,
    )

    const dragRegion = container.querySelector('[data-tauri-drag-region]')
    expect(dragRegion).toBeInstanceOf(HTMLElement)

    fireEvent.mouseDown(dragRegion as HTMLElement, { button: 0, detail: 1 })
    await waitFor(() => expect(tauriWindowMock.startDragging).toHaveBeenCalledTimes(1))

    fireEvent.mouseDown(dragRegion as HTMLElement, { button: 0, detail: 2 })
    await waitFor(() => expect(tauriWindowMock.toggleMaximize).toHaveBeenCalledTimes(1))
  })
})
