import { describe, expect, it } from 'vitest'
import { getColor } from './colors'

describe('calendar-style golden angle colors', () => {
	it('returns a valid CourseColor', () => {
		const c = getColor('25001')
		expect(c).toHaveProperty('bg')
		expect(c).toHaveProperty('border')
		expect(c).toHaveProperty('text')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
		expect(c.border).toMatch(/^#[0-9a-f]{6}$/)
		expect(c.text).toBe('#1d1d1f')
	})

	it('is deterministic', () => {
		const a = getColor('25001')
		const b = getColor('25001')
		expect(a).toEqual(b)
	})

	it('produces good hue variety across different codes', () => {
		const codes = Array.from({ length: 20 }, (_, i) => `CODE${i}`)
		const bgs = new Set(codes.map((c) => getColor(c).bg))
		// With golden angle, 20 codes should produce at least 15 distinct colors
		expect(bgs.size).toBeGreaterThanOrEqual(15)
	})

	it('different codes produce different colors', () => {
		const codes = ['001', '002', '003', '010', '020', '030']
		const bgs = new Set(codes.map((c) => getColor(c).bg))
		expect(bgs.size).toBeGreaterThan(1)
	})

	it('handles empty string', () => {
		const c = getColor('')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles very long string', () => {
		const c = getColor('a'.repeat(10000))
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles Japanese characters', () => {
		const c = getColor('講義コード')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})
})
