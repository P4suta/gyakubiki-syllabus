import { describe, expect, it } from 'vitest'
import { COLORS, getColor } from './colors'
import { contrastRatio } from './contrast'

const AA = 4.5

describe('getColor', () => {
	it('returns a color object with bg, border, text', () => {
		const color = getColor('25001')
		expect(color).toHaveProperty('bg')
		expect(color).toHaveProperty('border')
		expect(color).toHaveProperty('text')
		expect(color.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('returns same color for same kogiCd (deterministic)', () => {
		const a = getColor('25001')
		const b = getColor('25001')
		expect(a).toEqual(b)
	})

	it('returns different colors for different codes (generally)', () => {
		const codes = ['001', '002', '003', '010', '020', '030']
		const colors = codes.map(getColor)
		const unique = new Set(colors.map((c) => c.bg))
		// Not all will be unique (only 10 colors), but should have some variety
		expect(unique.size).toBeGreaterThan(1)
	})

	it('handles empty string without crashing', () => {
		const color = getColor('')
		expect(color).toHaveProperty('bg')
	})

	it('handles very long string without crashing', () => {
		const color = getColor('a'.repeat(10000))
		expect(color).toHaveProperty('bg')
	})

	it('handles Japanese characters', () => {
		const color = getColor('講義コード')
		expect(color).toHaveProperty('bg')
	})
})

describe('course palette contrast (WCAG AA)', () => {
	// The card's meta line renders `accentText` as text on `bg`; the course name
	// renders `text` on `bg`. Both must clear 4.5:1, or the tile fails Lighthouse.
	for (const c of COLORS) {
		it(`accentText ${c.accentText} on ${c.bg} ≥ ${AA}:1`, () => {
			expect(contrastRatio(c.accentText, c.bg)).toBeGreaterThanOrEqual(AA)
		})
		it(`text ${c.text} on ${c.bg} ≥ ${AA}:1`, () => {
			expect(contrastRatio(c.text, c.bg)).toBeGreaterThanOrEqual(AA)
		})
	}
})
