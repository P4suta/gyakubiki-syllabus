import { describe, expect, it } from 'vitest'
import { COLORS } from './colors'

// Palette *harmony* invariants for the "macaron" tiles. The generator (derivation
// script) places 10 hues 36° apart from rose (350°), each held at ONE chroma per
// theme (equal vividness) but the LIGHTEST lightness that still carries it — so
// hue is even and chroma uniform while lightness varies per hue by design (yellow
// light, blue a touch deeper). Each tile's ink is its own hue-tinted tone, solved
// for AA on its bg (locked by colors.test.ts). These lock the family so a future
// ad-hoc tint that breaks the ring fails here, not just by eye. sRGB→OKLCH is
// inlined (test-only, like contrast).

function srgbToOklch(hex: string): { L: number; C: number; h: number } {
	const [r, g, b] = [0, 2, 4].map((i) => {
		const s = parseInt(hex.slice(1 + i, 3 + i), 16) / 255
		return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
	})
	const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
	const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
	const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
	const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
	const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
	const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
	return { L, C: Math.hypot(a, bb), h: ((Math.atan2(bb, a) * 180) / Math.PI + 360) % 360 }
}

// Shortest angular distance between two hues, in degrees.
function hueDist(a: number, b: number): number {
	const d = Math.abs(a - b) % 360
	return d > 180 ? 360 - d : d
}

const light = COLORS.map((c) => c.light)
const dark = COLORS.map((c) => c.dark)

describe('palette harmony (macaron: even hues · one chroma · per-hue lightness)', () => {
	it('the 10 tile hues are 36° apart, starting at rose (~350°)', () => {
		for (const tiles of [light, dark]) {
			tiles.forEach((t, i) => {
				const want = (350 + i * 36) % 360
				expect(hueDist(srgbToOklch(t.bg).h, want), `tile ${i} bg hue vs ${want}°`).toBeLessThan(4)
			})
		}
	})

	it('tile backgrounds hold one chroma per theme (equal vividness)', () => {
		const spread = (arr: typeof light) => {
			const cs = arr.map((t) => srgbToOklch(t.bg).C)
			return Math.max(...cs) - Math.min(...cs)
		}
		expect(spread(light), 'light bg chroma spread').toBeLessThan(0.01)
		expect(spread(dark), 'dark bg chroma spread').toBeLessThan(0.01)
	})

	it('tile backgrounds stay low-chroma (pastel), never garish', () => {
		for (const t of [...light, ...dark]) {
			expect(srgbToOklch(t.bg).C, `${t.bg} chroma`).toBeLessThan(0.075)
		}
	})

	it('light tiles read pale, dark tiles read deep (a family; lightness is per-hue)', () => {
		for (const t of light) {
			const { L } = srgbToOklch(t.bg)
			expect(L, `${t.bg} lightness`).toBeGreaterThan(0.84)
			expect(L, `${t.bg} lightness`).toBeLessThan(0.98)
		}
		for (const t of dark) {
			const { L } = srgbToOklch(t.bg)
			expect(L, `${t.bg} lightness`).toBeGreaterThan(0.2)
			expect(L, `${t.bg} lightness`).toBeLessThan(0.37)
		}
	})

	it("each tile's accent ink is its own hue, not a shared grey", () => {
		// The old palette shared one ink across tiles; macaron gives each tile a
		// hue-tinted ink. Lock that the vivid accent stays on the tile's hue.
		for (const tiles of [light, dark]) {
			for (const t of tiles) {
				expect(
					hueDist(srgbToOklch(t.accentText).h, srgbToOklch(t.bg).h),
					`${t.accentText} accent hue vs bg ${t.bg}`,
				).toBeLessThan(12)
			}
		}
	})

	it('the 10 hues are distinct (no accidental duplicate tints)', () => {
		expect(new Set(light.map((t) => t.bg)).size).toBe(10)
		expect(new Set(dark.map((t) => t.bg)).size).toBe(10)
	})
})
