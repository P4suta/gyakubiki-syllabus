import { describe, expect, it } from 'vitest'
import type { CourseV2, Dictionaries } from '../types/course'
import { buildGrid, countUnique, DAYS, PERIODS } from './grid'

const DICTS: Dictionaries = {
	semesters: ['1学期', '2学期', '通年'],
	departments: ['理工学部'],
	campuses: ['朝倉キャンパス'],
	kubun: ['講義'],
	kaikojiki: ['1学期', '2学期', '通年'],
}

function makeCourse(overrides: Partial<CourseV2> = {}): CourseV2 {
	return {
		cd: '001',
		nm: 'テスト講義',
		prof: '教員',
		raw: '',
		slots: [],
		ki: 0,
		kbn: 0,
		dept: 0,
		campus: 0,
		st: '',
		...overrides,
	}
}

describe('buildGrid', () => {
	it('initializes all cells as empty arrays', () => {
		const grid = buildGrid([], 'all', DICTS)
		expect(grid.size).toBe(DAYS.length * PERIODS.length)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('places course in correct cell', () => {
		const course = makeCourse({
			slots: [{ s: 0, d: 0, p: 1 }], // 1学期, 月, 1
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(grid.get('月-1')).toHaveLength(1)
		expect(grid.get('月-1')![0].cd).toBe('001')
	})

	it('places course with multiple slots in multiple cells', () => {
		const course = makeCourse({
			slots: [
				{ s: 0, d: 0, p: 1 }, // 1学期, 月, 1
				{ s: 0, d: 2, p: 3 }, // 1学期, 水, 3
			],
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(grid.get('月-1')).toHaveLength(1)
		expect(grid.get('水-3')).toHaveLength(1)
	})

	it('filters by semester', () => {
		const course = makeCourse({
			slots: [{ s: 1, d: 1, p: 2 }], // 2学期, 火, 2
		})
		const grid = buildGrid([course], '1学期', DICTS)
		expect(grid.get('火-2')).toHaveLength(0)
	})

	it('shows 通年 courses in any semester filter', () => {
		const course = makeCourse({
			slots: [{ s: 2, d: 4, p: 5 }], // 通年, 金, 5
		})
		const grid1 = buildGrid([course], '1学期', DICTS)
		const grid2 = buildGrid([course], '2学期', DICTS)
		expect(grid1.get('金-5')).toHaveLength(1)
		expect(grid2.get('金-5')).toHaveLength(1)
	})

	it('deduplicates same cd in same cell', () => {
		const course = makeCourse({
			slots: [
				{ s: 0, d: 0, p: 1 }, // 1学期, 月, 1
				{ s: 2, d: 0, p: 1 }, // 通年, 月, 1
			],
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(grid.get('月-1')).toHaveLength(1)
	})

	it('handles courses with no slots gracefully', () => {
		const course = makeCourse({ slots: [] })
		const grid = buildGrid([course], 'all', DICTS)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('ignores slots with day index 6 (日) which is not in grid', () => {
		const course = makeCourse({
			slots: [{ s: 0, d: 6, p: 1 }], // 1学期, 日, 1
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(grid.has('日-1')).toBe(false)
	})

	it('ignores slots with period outside 1-6', () => {
		const course = makeCourse({
			slots: [{ s: 0, d: 0, p: 7 }], // 1学期, 月, 7
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(grid.has('月-7')).toBe(false)
	})
})

describe('countUnique', () => {
	it('returns 0 for empty grid', () => {
		const grid = buildGrid([], 'all', DICTS)
		expect(countUnique(grid)).toBe(0)
	})

	it('counts unique courses across cells', () => {
		const c1 = makeCourse({
			cd: '001',
			slots: [
				{ s: 0, d: 0, p: 1 },
				{ s: 0, d: 2, p: 3 },
			],
		})
		const c2 = makeCourse({
			cd: '002',
			slots: [{ s: 0, d: 1, p: 2 }],
		})
		const grid = buildGrid([c1, c2], 'all', DICTS)
		expect(countUnique(grid)).toBe(2)
	})

	it('does not double-count courses in multiple cells', () => {
		const course = makeCourse({
			cd: '001',
			slots: [
				{ s: 0, d: 0, p: 1 },
				{ s: 0, d: 1, p: 2 },
				{ s: 0, d: 2, p: 3 },
			],
		})
		const grid = buildGrid([course], 'all', DICTS)
		expect(countUnique(grid)).toBe(1)
	})
})
