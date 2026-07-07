import { describe, expect, it } from 'vitest'
import { defaultSemester, seasonSemester } from './semester'

describe('seasonSemester', () => {
	it('returns 1学期 while it is in session (Apr–Jul)', () => {
		for (const m of [4, 5, 6, 7]) expect(seasonSemester(m)).toBe('1学期')
	})

	it('returns 2学期 while it is in session (Oct–Jan)', () => {
		for (const m of [10, 11, 12, 1]) expect(seasonSemester(m)).toBe('2学期')
	})

	it('returns null in the exam/break shoulder months (Feb, Mar, Aug, Sep)', () => {
		for (const m of [2, 3, 8, 9]) expect(seasonSemester(m)).toBeNull()
	})
})

describe('defaultSemester', () => {
	const dict = ['1学期', '1学期前半', '2学期', '2学期前半', '通年']

	it('selects the in-session term when the dataset has it', () => {
		expect(defaultSemester(dict, new Date(2026, 5, 1))).toBe('1学期') // June
		expect(defaultSemester(dict, new Date(2026, 10, 1))).toBe('2学期') // November
	})

	it('falls back to 全て in shoulder months', () => {
		expect(defaultSemester(dict, new Date(2026, 8, 1))).toBe('all') // September
		expect(defaultSemester(dict, new Date(2026, 1, 1))).toBe('all') // February
	})

	it('falls back to 全て when the term is absent from the data', () => {
		expect(defaultSemester(['通年'], new Date(2026, 5, 1))).toBe('all')
		expect(defaultSemester([], new Date(2026, 5, 1))).toBe('all')
	})
})
