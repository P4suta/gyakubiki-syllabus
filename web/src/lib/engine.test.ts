import { describe, expect, it } from 'vitest'
import type { Course } from '../types/course'
import { assembleGrid, dayLabels, PERIODS } from './engine'

function view(cd: string): Course {
	return {
		cd,
		nm: `科目${cd}`,
		prof: '教員',
		raw: '',
		slots: [],
		ki: 0,
		kbn: 0,
		dept: 0,
		campus: 0,
		st: '',
	}
}

describe('dayLabels', () => {
	it('is the five weekdays without Saturday', () => {
		expect(dayLabels(false)).toEqual(['月', '火', '水', '木', '金'])
	})

	it('appends 土 when the data has Saturday', () => {
		expect(dayLabels(true)).toEqual(['月', '火', '水', '木', '金', '土'])
	})
})

describe('assembleGrid', () => {
	const days = dayLabels(false)
	const views = [view('001'), view('002'), view('003')]

	it('seeds every day×period cell as an empty array', () => {
		const grid = assembleGrid([], views, days)
		expect(grid.size).toBe(days.length * PERIODS.length)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('resolves course indices into the matching cell', () => {
		const grid = assembleGrid([{ day: 0, period: 1, courses: [0, 2] }], views, days)
		expect(grid.get('月-1')?.map((c) => c.cd)).toEqual(['001', '003'])
		expect(grid.get('火-2')).toEqual([])
	})

	it('maps day index to the right label', () => {
		const grid = assembleGrid([{ day: 4, period: 5, courses: [1] }], views, days)
		expect(grid.get('金-5')?.map((c) => c.cd)).toEqual(['002'])
	})

	it('ignores cells whose day index is out of range', () => {
		const grid = assembleGrid([{ day: 9, period: 1, courses: [0] }], views, days)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})
})
