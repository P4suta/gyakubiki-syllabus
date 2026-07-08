import { describe, expect, it } from 'vitest'
import { COLORS } from './colors'

// Palette *harmony* invariants. The redesign placed the 10 tiles on a shared
// OKLCH ring (equal lightness & chroma per theme) so they read as one calm
// family; these lock that so a future ad-hoc tint that breaks the uniformity
// fails here, not just by eye. sRGB→OKLCH is inlined (test-only, like contrast).

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

const light = COLORS.map((c) => c.light)
const dark = COLORS.map((c) => c.dark)

describe('palette harmony (OKLCH uniformity)', () => {
	it('all light tiles sit in one pale lightness band', () => {
		for (const t of light) {
			const { L } = srgbToOklch(t.bg)
			expect(L, `${t.bg} lightness`).toBeGreaterThan(0.93)
			expect(L, `${t.bg} lightness`).toBeLessThan(0.98)
		}
	})

	it('all dark tiles sit in one deep lightness band', () => {
		for (const t of dark) {
			const { L } = srgbToOklch(t.bg)
			expect(L, `${t.bg} lightness`).toBeGreaterThan(0.22)
			expect(L, `${t.bg} lightness`).toBeLessThan(0.33)
		}
	})

	it('tile backgrounds stay low-chroma (pastel), never garish', () => {
		for (const t of [...light, ...dark]) {
			expect(srgbToOklch(t.bg).C, `${t.bg} chroma`).toBeLessThan(0.075)
		}
	})

	it('the lightness spread across tiles is tight (a single family)', () => {
		const spread = (arr: typeof light) => {
			const ls = arr.map((t) => srgbToOklch(t.bg).L)
			return Math.max(...ls) - Math.min(...ls)
		}
		expect(spread(light), 'light bg lightness spread').toBeLessThan(0.05)
		expect(spread(dark), 'dark bg lightness spread').toBeLessThan(0.06)
	})

	it('every tile shares one ink per theme (name/meta text is consistent)', () => {
		expect(new Set(light.map((t) => t.text)).size).toBe(1)
		expect(new Set(dark.map((t) => t.text)).size).toBe(1)
	})

	it('the 10 hues are distinct (no accidental duplicate tints)', () => {
		expect(new Set(light.map((t) => t.bg)).size).toBe(10)
		expect(new Set(dark.map((t) => t.bg)).size).toBe(10)
	})
})
