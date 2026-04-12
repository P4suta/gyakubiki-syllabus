import { describe, expect, it } from 'vitest'
import type { Course } from '../types/course'
import { buildGrid, countUnique, DAYS, PERIODS } from './grid'

function makeCourse(overrides: Partial<Course> = {}): Course {
	return {
		kogiCd: '001',
		kogiNm: 'テスト講義',
		tantoKyoin: '教員',
		jikanwariRaw: '',
		slots: [],
		kogiKaikojikiNm: '',
		kogiKubunNm: '',
		sekininBushoNm: '',
		kochiNm: '',
		gakusokuKamokuNm: '',
		...overrides,
	}
}

describe('buildGrid', () => {
	it('initializes all cells as empty arrays', () => {
		const grid = buildGrid([], 'all')
		expect(grid.size).toBe(DAYS.length * PERIODS.length)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('places course in correct cell', () => {
		const course = makeCourse({
			slots: [{ semester: '1学期', day: '月', period: 1 }],
		})
		const grid = buildGrid([course], 'all')
		expect(grid.get('月-1')).toHaveLength(1)
		expect(grid.get('月-1')![0].kogiCd).toBe('001')
	})

	it('places course with multiple slots in multiple cells', () => {
		const course = makeCourse({
			slots: [
				{ semester: '1学期', day: '月', period: 1 },
				{ semester: '1学期', day: '水', period: 3 },
			],
		})
		const grid = buildGrid([course], 'all')
		expect(grid.get('月-1')).toHaveLength(1)
		expect(grid.get('水-3')).toHaveLength(1)
	})

	it('filters by semester', () => {
		const course = makeCourse({
			slots: [{ semester: '2学期', day: '火', period: 2 }],
		})
		const grid = buildGrid([course], '1学期')
		expect(grid.get('火-2')).toHaveLength(0)
	})

	it('shows 通年 courses in any semester filter', () => {
		const course = makeCourse({
			slots: [{ semester: '通年', day: '金', period: 5 }],
		})
		const grid1 = buildGrid([course], '1学期')
		const grid2 = buildGrid([course], '2学期')
		expect(grid1.get('金-5')).toHaveLength(1)
		expect(grid2.get('金-5')).toHaveLength(1)
	})

	it('deduplicates same kogiCd in same cell', () => {
		const course = makeCourse({
			slots: [
				{ semester: '1学期', day: '月', period: 1 },
				{ semester: '通年', day: '月', period: 1 },
			],
		})
		const grid = buildGrid([course], 'all')
		expect(grid.get('月-1')).toHaveLength(1)
	})

	it('handles courses with no slots gracefully', () => {
		const course = makeCourse({ slots: [] })
		const grid = buildGrid([course], 'all')
		// Should not crash, all cells empty
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('ignores slots with day not in DAYS (e.g. 日)', () => {
		const course = makeCourse({
			slots: [{ semester: '1学期', day: '日', period: 1 }],
		})
		const grid = buildGrid([course], 'all')
		// 日 is not in DAYS, so no cell for it — course is just not placed
		expect(grid.has('日-1')).toBe(false)
	})

	it('ignores slots with period outside 1-6', () => {
		const course = makeCourse({
			slots: [{ semester: '1学期', day: '月', period: 7 }],
		})
		const grid = buildGrid([course], 'all')
		expect(grid.has('月-7')).toBe(false)
	})
})

describe('countUnique', () => {
	it('returns 0 for empty grid', () => {
		const grid = buildGrid([], 'all')
		expect(countUnique(grid)).toBe(0)
	})

	it('counts unique courses across cells', () => {
		const c1 = makeCourse({
			kogiCd: '001',
			slots: [
				{ semester: '1学期', day: '月', period: 1 },
				{ semester: '1学期', day: '水', period: 3 },
			],
		})
		const c2 = makeCourse({
			kogiCd: '002',
			slots: [{ semester: '1学期', day: '火', period: 2 }],
		})
		const grid = buildGrid([c1, c2], 'all')
		expect(countUnique(grid)).toBe(2)
	})

	it('does not double-count courses in multiple cells', () => {
		const course = makeCourse({
			kogiCd: '001',
			slots: [
				{ semester: '1学期', day: '月', period: 1 },
				{ semester: '1学期', day: '火', period: 2 },
				{ semester: '1学期', day: '水', period: 3 },
			],
		})
		const grid = buildGrid([course], 'all')
		expect(countUnique(grid)).toBe(1)
	})
})
