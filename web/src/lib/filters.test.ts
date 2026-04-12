import { describe, expect, it } from 'vitest'
import type { Course } from '../types/course'
import { filterCourses } from './filters'

function makeCourse(overrides: Partial<Course> = {}): Course {
	return {
		kogiCd: '001',
		kogiNm: 'テスト講義',
		tantoKyoin: '教員 太郎',
		jikanwariRaw: '',
		slots: [{ semester: '1学期', day: '月', period: 1 }],
		kogiKaikojikiNm: '',
		kogiKubunNm: '',
		sekininBushoNm: '理工学部',
		kochiNm: '',
		gakusokuKamokuNm: '',
		...overrides,
	}
}

describe('filterCourses', () => {
	const courses: Course[] = [
		makeCourse({ kogiCd: '001', kogiNm: '微分積分学', sekininBushoNm: '理工学部', tantoKyoin: '山田 太郎', slots: [{ semester: '1学期', day: '月', period: 1 }] }),
		makeCourse({ kogiCd: '002', kogiNm: '政治学概論', sekininBushoNm: '人文社会科学部', tantoKyoin: '小川 寛貴', slots: [{ semester: '2学期', day: '火', period: 2 }] }),
		makeCourse({ kogiCd: '003', kogiNm: '哲学概論', sekininBushoNm: '人文社会科学部', tantoKyoin: '佐藤 哲也', slots: [{ semester: '通年', day: '金', period: 5 }] }),
	]

	// --- 全件 ---
	it('returns all when no filters', () => {
		expect(filterCourses(courses, 'all', 'all', '')).toHaveLength(3)
	})

	// --- 学期フィルタ ---
	it('filters by semester: 1学期', () => {
		const result = filterCourses(courses, '1学期', 'all', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['001', '003'])
	})

	it('filters by semester: 2学期', () => {
		const result = filterCourses(courses, '2学期', 'all', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['002', '003'])
	})

	it('通年 courses appear in all semester filters', () => {
		const result1 = filterCourses(courses, '1学期', 'all', '')
		const result2 = filterCourses(courses, '2学期', 'all', '')
		expect(result1.some((c) => c.kogiCd === '003')).toBe(true)
		expect(result2.some((c) => c.kogiCd === '003')).toBe(true)
	})

	// --- 部署フィルタ ---
	it('filters by department', () => {
		const result = filterCourses(courses, 'all', '理工学部', '')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('001')
	})

	it('returns empty for nonexistent department', () => {
		expect(filterCourses(courses, 'all', '医学部', '')).toHaveLength(0)
	})

	// --- テキスト検索 ---
	it('searches by course name', () => {
		const result = filterCourses(courses, 'all', 'all', '微分')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('001')
	})

	it('searches by instructor name', () => {
		const result = filterCourses(courses, 'all', 'all', '小川')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('002')
	})

	it('searches by kogiCd', () => {
		const result = filterCourses(courses, 'all', 'all', '003')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('003')
	})

	it('search is case insensitive', () => {
		const withEnglish = [
			makeCourse({ kogiCd: '004', kogiNm: 'English Communication', tantoKyoin: 'Smith, John' }),
		]
		expect(filterCourses(withEnglish, 'all', 'all', 'english')).toHaveLength(1)
		expect(filterCourses(withEnglish, 'all', 'all', 'SMITH')).toHaveLength(1)
	})

	it('searches in fukudai', () => {
		const withFukudai = [
			makeCourse({ kogiCd: '005', kogiNm: 'テスト', fukudai: '副題テスト' }),
		]
		expect(filterCourses(withFukudai, 'all', 'all', '副題')).toHaveLength(1)
	})

	it('handles undefined fukudai gracefully', () => {
		const withoutFukudai = [
			makeCourse({ kogiCd: '005', kogiNm: 'テスト', fukudai: undefined }),
		]
		expect(filterCourses(withoutFukudai, 'all', 'all', 'テスト')).toHaveLength(1)
	})

	// --- 全角/半角スペース正規化 ---
	it('matches instructor with half-width space when data has full-width', () => {
		const courses = [
			makeCourse({ kogiCd: '010', tantoKyoin: '山田\u3000太郎' }), // full-width space
		]
		expect(filterCourses(courses, 'all', 'all', '山田 太郎')).toHaveLength(1)
	})

	it('matches instructor with full-width space when data has half-width', () => {
		const courses = [
			makeCourse({ kogiCd: '010', tantoKyoin: '山田 太郎' }), // half-width space
		]
		expect(filterCourses(courses, 'all', 'all', '山田\u3000太郎')).toHaveLength(1)
	})

	// --- 複合フィルタ ---
	it('combines semester and department filters', () => {
		const result = filterCourses(courses, '2学期', '人文社会科学部', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['002', '003'])
	})

	it('combines all three filters', () => {
		const result = filterCourses(courses, '2学期', '人文社会科学部', '政治')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('002')
	})

	// --- エッジケース ---
	it('handles empty courses array', () => {
		expect(filterCourses([], 'all', 'all', '')).toEqual([])
	})

	it('handles empty search after whitespace', () => {
		// empty string search should match all (search is not trimmed, but "" is falsy)
		expect(filterCourses(courses, 'all', 'all', '')).toHaveLength(3)
	})

	it('handles course with empty slots array', () => {
		const noSlots = [makeCourse({ slots: [] })]
		// With 'all' semester, no semester filter applied → should include it
		expect(filterCourses(noSlots, 'all', 'all', '')).toHaveLength(1)
		// With specific semester, slots.some returns false → filtered out
		expect(filterCourses(noSlots, '1学期', 'all', '')).toHaveLength(0)
	})

	it('does not crash with special regex characters in search', () => {
		// String.includes is used, not regex, so special chars are fine
		expect(filterCourses(courses, 'all', 'all', '[.*+?]')).toHaveLength(0)
	})
})
