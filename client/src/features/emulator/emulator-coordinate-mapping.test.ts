import { describe, expect, it } from "vitest"
import { computeNormalizedCoords } from "@/components/cadence/emulator-sidebar"

// Common device aspect ratios for iPhone / iPad simulators.
const IPHONE_15_PRO = { w: 1179, h: 2556 } // 9:19.5-ish
const IPHONE_SE = { w: 750, h: 1334 } // 9:16
const IPAD_PRO_11 = { w: 1668, h: 2388 } // ~0.7:1

describe("computeNormalizedCoords", () => {
  it("maps center of a perfectly-matched container to (0.5, 0.5)", () => {
    const rect = { left: 0, top: 0, width: 300, height: 650 }
    const result = computeNormalizedCoords(
      150, 325, rect,
      IPHONE_15_PRO.w, IPHONE_15_PRO.h,
    )
    expect(result.x).toBeCloseTo(0.5, 1)
    expect(result.y).toBeCloseTo(0.5, 1)
  })

  it("maps top-left of a matched container to (0, 0)", () => {
    const rect = { left: 50, top: 100, width: 300, height: 650 }
    const result = computeNormalizedCoords(
      50, 100, rect,
      IPHONE_15_PRO.w, IPHONE_15_PRO.h,
    )
    expect(result.x).toBeCloseTo(0, 5)
    expect(result.y).toBeCloseTo(0, 5)
  })

  it("maps bottom-right of a matched container to (1, 1)", () => {
    const rect = { left: 50, top: 100, width: 300, height: 650 }
    const result = computeNormalizedCoords(
      350, 750, rect,
      IPHONE_15_PRO.w, IPHONE_15_PRO.h,
    )
    expect(result.x).toBeCloseTo(1, 1)
    expect(result.y).toBeCloseTo(1, 1)
  })

  describe("letterboxed (bars left/right)", () => {
    // Container is 400×400 (1:1), image is 1179×2556 (tall).
    // object-contain will show the image at width = 400*(1179/2556) ≈ 184.5,
    // centered with ~107.75 bars on each side.
    const rect = { left: 0, top: 0, width: 400, height: 400 }
    const imgW = IPHONE_15_PRO.w
    const imgH = IPHONE_15_PRO.h

    it("maps center to (0.5, 0.5)", () => {
      const result = computeNormalizedCoords(200, 200, rect, imgW, imgH)
      expect(result.x).toBeCloseTo(0.5, 1)
      expect(result.y).toBeCloseTo(0.5, 1)
    })

    it("a click in the left letterbox bar clamps to x=0", () => {
      // The image starts at ~107.75, so clicking at x=50 is in the bar.
      const result = computeNormalizedCoords(50, 200, rect, imgW, imgH)
      expect(result.x).toBe(0)
      expect(result.y).toBeCloseTo(0.5, 1)
    })

    it("a click in the right letterbox bar clamps to x=1", () => {
      const result = computeNormalizedCoords(380, 200, rect, imgW, imgH)
      expect(result.x).toBe(1)
    })
  })

  describe("pillarboxed (bars top/bottom)", () => {
    // Container is 400×400 (1:1), image is landscape 2556×1179.
    // object-contain width fills 400, height = 400*(1179/2556) ≈ 184.5,
    // centered with ~107.75 bars top and bottom.
    const rect = { left: 0, top: 0, width: 400, height: 400 }

    it("maps center to (0.5, 0.5)", () => {
      const result = computeNormalizedCoords(200, 200, rect, 2556, 1179)
      expect(result.x).toBeCloseTo(0.5, 1)
      expect(result.y).toBeCloseTo(0.5, 1)
    })

    it("a click in the top pillar bar clamps to y=0", () => {
      const result = computeNormalizedCoords(200, 50, rect, 2556, 1179)
      expect(result.y).toBe(0)
    })

    it("a click in the bottom pillar bar clamps to y=1", () => {
      const result = computeNormalizedCoords(200, 380, rect, 2556, 1179)
      expect(result.y).toBe(1)
    })
  })

  it("falls back to raw container mapping when image dimensions are 0", () => {
    const rect = { left: 100, top: 200, width: 300, height: 600 }
    const result = computeNormalizedCoords(250, 500, rect, 0, 0)
    expect(result.x).toBeCloseTo(0.5, 5)
    expect(result.y).toBeCloseTo(0.5, 5)
  })

  it("accounts for container offset in the viewport", () => {
    // Sidebar is at x=800, y=50 (right side of screen).
    const rect = { left: 800, top: 50, width: 300, height: 650 }
    const result = computeNormalizedCoords(
      950, 375, rect,
      IPHONE_15_PRO.w, IPHONE_15_PRO.h,
    )
    expect(result.x).toBeCloseTo(0.5, 1)
    expect(result.y).toBeCloseTo(0.5, 1)
  })

  it("handles iPhone SE aspect ratio", () => {
    const rect = { left: 0, top: 0, width: 200, height: 400 }
    const result = computeNormalizedCoords(100, 200, rect, IPHONE_SE.w, IPHONE_SE.h)
    expect(result.x).toBeCloseTo(0.5, 1)
    expect(result.y).toBeCloseTo(0.5, 1)
  })

  it("handles iPad Pro 11 aspect ratio", () => {
    const rect = { left: 0, top: 0, width: 300, height: 400 }
    const result = computeNormalizedCoords(150, 200, rect, IPAD_PRO_11.w, IPAD_PRO_11.h)
    expect(result.x).toBeCloseTo(0.5, 1)
    expect(result.y).toBeCloseTo(0.5, 1)
  })

  describe("regression: tap offset bug", () => {
    // The original bug: container had a slightly different aspect ratio
    // than the image (due to rounded corners eating a few pixels of
    // computed height), so object-cover would crop the top, causing
    // taps to register ~1 icon row above the intended target.
    //
    // With object-contain + proper coordinate mapping, taps at any
    // Y position should map accurately regardless of aspect mismatch.

    it("tap at 75% Y maps to ~0.75 even with slight aspect mismatch", () => {
      // Container slightly wider than image aspect → image has small
      // left/right bars.
      const containerW = 320
      const containerH = 690
      const rect = { left: 40, top: 80, width: containerW, height: containerH }
      const tapY = 80 + containerH * 0.75

      const result = computeNormalizedCoords(
        40 + containerW / 2, tapY, rect,
        IPHONE_15_PRO.w, IPHONE_15_PRO.h,
      )
      // Y should be very close to 0.75, not shifted up toward 0.70
      expect(result.y).toBeCloseTo(0.75, 1)
    })

    it("tap at 25% Y maps to ~0.25 even with aspect mismatch", () => {
      const containerW = 320
      const containerH = 690
      const rect = { left: 40, top: 80, width: containerW, height: containerH }
      const tapY = 80 + containerH * 0.25

      const result = computeNormalizedCoords(
        40 + containerW / 2, tapY, rect,
        IPHONE_15_PRO.w, IPHONE_15_PRO.h,
      )
      expect(result.y).toBeCloseTo(0.25, 1)
    })
  })

  it("clamps values outside the image area to 0..1", () => {
    const rect = { left: 100, top: 100, width: 300, height: 600 }
    const above = computeNormalizedCoords(250, 50, rect, IPHONE_15_PRO.w, IPHONE_15_PRO.h)
    const below = computeNormalizedCoords(250, 800, rect, IPHONE_15_PRO.w, IPHONE_15_PRO.h)
    const left = computeNormalizedCoords(50, 400, rect, IPHONE_15_PRO.w, IPHONE_15_PRO.h)
    const right = computeNormalizedCoords(450, 400, rect, IPHONE_15_PRO.w, IPHONE_15_PRO.h)

    expect(above.y).toBe(0)
    expect(below.y).toBe(1)
    expect(left.x).toBe(0)
    expect(right.x).toBe(1)
  })
})
