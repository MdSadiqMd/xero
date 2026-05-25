/** @vitest-environment jsdom */

import { render, waitFor } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"

import { EmulatorMissingSdk } from "./emulator-missing-sdk"

const { invokeMock, isTauriMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(() => true),
  listenMock: vi.fn(async () => vi.fn()),
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}))

afterEach(() => {
  vi.clearAllMocks()
})

describe("EmulatorMissingSdk", () => {
  it("auto-provisions iOS when Xcode exists but no simulator runtime is installed", async () => {
    let provisioned = false
    const onProvisioned = vi.fn()

    invokeMock.mockImplementation(async (command: string) => {
      if (command === "emulator_sdk_status") {
        return sdkStatus({
          runtimeCount: provisioned ? 1 : 0,
          deviceCount: provisioned ? 1 : 0,
        })
      }
      if (command === "emulator_ios_provision") {
        provisioned = true
        return undefined
      }
      throw new Error(`Unexpected command: ${command}`)
    })

    render(<EmulatorMissingSdk active onProvisioned={onProvisioned} platform="ios" />)

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("emulator_ios_provision")
    })
    await waitFor(() => {
      expect(onProvisioned).toHaveBeenCalled()
    })
  })
})

function sdkStatus({
  runtimeCount,
  deviceCount,
}: {
  runtimeCount: number
  deviceCount: number
}) {
  return {
    android: {
      present: false,
      sdkRoot: null,
      emulatorPath: null,
      adbPath: null,
      avdmanagerPath: null,
    },
    ios: {
      present: true,
      xcrunPath: "/usr/bin/xcrun",
      simctlPath: "/usr/bin/simctl",
      idbCompanionPresent: false,
      supported: true,
      axPermissionGranted: true,
      screenRecordingPermissionGranted: true,
      helperPresent: true,
      runtimeCount,
      deviceCount,
    },
  }
}
