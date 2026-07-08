import { describe, expect, it } from 'vitest'
import { COLORS, getColor, type Tint } from './colors'
import { contrastRatio } from './contrast'
import { EVAL_KIND } from './syllabus-icons'

const AA = 4.5
const AA_GRAPHIC = 3 // WCAG non-text contrast for graphical objects (arcs, dots)
const HEX6 = /^#[0-9a-f]{6}$/

describe('getColor', () => {
	it('returns light and dark tints, each with bg/border/text/accentText', () => {
		const color = getColor('25001')
		for (const tint of [color.light, color.dark]) {
			expect(tint).toHaveProperty('bg')
			expect(tint).toHaveProperty('border')
			expect(tint).toHaveProperty('text')
			expect(tint).toHaveProperty('accentText')
			expect(tint.bg).toMatch(HEX6)
		}
	})

	it('returns same color for same kogiCd (deterministic)', () => {
		expect(getColor('25001')).toEqual(getColor('25001'))
	})

	it('returns different colors for different codes (generally)', () => {
		const codes = ['001', '002', '003', '010', '020', '030']
		const unique = new Set(codes.map((c) => getColor(c).light.bg))
		// Not all will be unique (only 10 colors), but should have some variety.
		expect(unique.size).toBeGreaterThan(1)
	})

	it('handles empty / very long / non-ASCII input without crashing', () => {
		expect(getColor('').light).toHaveProperty('bg')
		expect(getColor('a'.repeat(10000)).light).toHaveProperty('bg')
		expect(getColor('講義コード').dark).toHaveProperty('bg')
	})
})

describe('course palette contrast (WCAG AA)', () => {
	// The card's meta line renders `accentText` as text on `bg`; the course name
	// renders `text` on `bg`. Both must clear 4.5:1 in *both* themes, or the tile
	// fails Lighthouse. Every value is a lowercase 6-digit hex (getColor's format).
	const themes: Array<[string, (c: (typeof COLORS)[number]) => Tint]> = [
		['light', (c) => c.light],
		['dark', (c) => c.dark],
	]
	for (const [name, pick] of themes) {
		for (const c of COLORS) {
			const t = pick(c)
			it(`${name}: accentText ${t.accentText} on ${t.bg} ≥ ${AA}:1`, () => {
				expect(contrastRatio(t.accentText, t.bg)).toBeGreaterThanOrEqual(AA)
			})
			it(`${name}: text ${t.text} on ${t.bg} ≥ ${AA}:1`, () => {
				expect(contrastRatio(t.text, t.bg)).toBeGreaterThanOrEqual(AA)
			})
			it(`${name}: ${t.bg} uses lowercase hex6`, () => {
				for (const v of [t.bg, t.border, t.text, t.accentText]) expect(v).toMatch(HEX6)
			})
		}
	}
})

describe('eval palette contrast (graphical, WCAG 3:1)', () => {
	// The donut arcs and legend dots are graphical objects on the modal surface
	// (elevated: white in light, deep slate in dark). Each must clear 3:1 there,
	// in both themes, and be a lowercase hex6. See lib/syllabus-icons.ts.
	const surfaces: Array<['light' | 'dark', string]> = [
		['light', '#ffffff'], // --color-surface-elevated (light)
		['dark', '#1f232b'], // --color-surface-elevated (dark)
	]
	for (const [theme, surface] of surfaces) {
		for (const [key, style] of Object.entries(EVAL_KIND)) {
			const c = style.color[theme]
			it(`${theme}: ${key} ${c} on ${surface} ≥ ${AA_GRAPHIC}:1`, () => {
				expect(c).toMatch(HEX6)
				expect(contrastRatio(c, surface)).toBeGreaterThanOrEqual(AA_GRAPHIC)
			})
		}
	}
})
