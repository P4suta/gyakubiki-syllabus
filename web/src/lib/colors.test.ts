import { describe, expect, it } from 'vitest'
import { getColor, initDeptColors } from './colors'

const SAMPLE_DEPTS = [
	'人文社会科学部',
	'全学開設科目',
	'共通教育',
	'医学部 医学科',
	'医学部 看護学科',
	'地域協働学部',
	'教育学部',
	'理工学部',
	'総合人間自然科学研究科（修士課程） 理工学専攻',
	'農林海洋科学部',
]

describe('hybrid department colors', () => {
	initDeptColors(SAMPLE_DEPTS)

	it('returns a valid CourseColor', () => {
		const c = getColor(0, '25001')
		expect(c).toHaveProperty('bg')
		expect(c).toHaveProperty('border')
		expect(c).toHaveProperty('text')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
		expect(c.border).toMatch(/^#[0-9a-f]{6}$/)
		expect(c.text).toBe('#1d1d1f')
	})

	it('is deterministic for same dept + code', () => {
		const a = getColor(7, 'CS101')
		const b = getColor(7, 'CS101')
		expect(a).toEqual(b)
	})

	it('same dept, different codes produce different bg colors', () => {
		const codes = ['A001', 'A002', 'A003', 'B010', 'C020', 'D030']
		const bgs = new Set(codes.map((c) => getColor(7, c).bg))
		expect(bgs.size).toBeGreaterThan(1)
	})

	it('different depts produce different hue families', () => {
		const c0 = getColor(0, 'X001') // 人文社会科学部 — orange
		const c7 = getColor(7, 'X001') // 理工学部 — blue
		expect(c0.bg).not.toBe(c7.bg)
	})

	it('sub-departments share parent hue family', () => {
		const c3 = getColor(3, 'M001')
		const c4 = getColor(4, 'M001')
		expect(c3.bg).toMatch(/^#[0-9a-f]{6}$/)
		expect(c4.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles out-of-range dept index', () => {
		const c = getColor(999, 'X001')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles empty course code', () => {
		const c = getColor(0, '')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles very long course code', () => {
		const c = getColor(0, 'a'.repeat(10000))
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('handles Japanese course code', () => {
		const c = getColor(0, '講義コード')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})
})
