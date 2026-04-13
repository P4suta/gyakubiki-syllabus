import { describe, expect, it } from 'vitest'
import type { Course } from '../types/course'
import { CourseIndex, normalize } from './course-index'

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

describe('normalize', () => {
	it('converts full-width spaces to half-width', () => {
		expect(normalize('山田\u3000太郎')).toBe('山田 太郎')
	})

	it('lowercases ascii characters', () => {
		expect(normalize('English')).toBe('english')
	})

	it('handles empty string', () => {
		expect(normalize('')).toBe('')
	})
})

describe('CourseIndex', () => {
	const courses: Course[] = [
		makeCourse({
			kogiCd: '001',
			kogiNm: '微分積分学',
			sekininBushoNm: '理工学部',
			tantoKyoin: '山田 太郎',
			slots: [{ semester: '1学期', day: '月', period: 1 }],
		}),
		makeCourse({
			kogiCd: '002',
			kogiNm: '政治学概論',
			sekininBushoNm: '人文社会科学部',
			tantoKyoin: '小川 寛貴',
			slots: [{ semester: '2学期', day: '火', period: 2 }],
		}),
		makeCourse({
			kogiCd: '003',
			kogiNm: '哲学概論',
			sekininBushoNm: '人文社会科学部',
			tantoKyoin: '佐藤 哲也',
			slots: [{ semester: '通年', day: '金', period: 5 }],
		}),
	]

	// --- 全件 ---
	it('returns all when no filters', () => {
		const idx = new CourseIndex(courses)
		expect(idx.filter('all', 'all', '')).toHaveLength(3)
	})

	// --- 学期フィルタ ---
	it('filters by semester: 1学期', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('1学期', 'all', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['001', '003'])
	})

	it('filters by semester: 2学期', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('2学期', 'all', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['002', '003'])
	})

	it('通年 courses appear in all semester filters', () => {
		const idx = new CourseIndex(courses)
		const result1 = idx.filter('1学期', 'all', '')
		const result2 = idx.filter('2学期', 'all', '')
		expect(result1.some((c) => c.kogiCd === '003')).toBe(true)
		expect(result2.some((c) => c.kogiCd === '003')).toBe(true)
	})

	// --- 部署フィルタ ---
	it('filters by department', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('all', '理工学部', '')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('001')
	})

	it('returns empty for nonexistent department', () => {
		const idx = new CourseIndex(courses)
		expect(idx.filter('all', '医学部', '')).toHaveLength(0)
	})

	// --- テキスト検索 ---
	it('searches by course name', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('all', 'all', '微分')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('001')
	})

	it('searches by instructor name', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('all', 'all', '小川')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('002')
	})

	it('searches by kogiCd', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('all', 'all', '003')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('003')
	})

	it('search is case insensitive', () => {
		const withEnglish = [
			makeCourse({
				kogiCd: '004',
				kogiNm: 'English Communication',
				tantoKyoin: 'Smith, John',
			}),
		]
		const idx = new CourseIndex(withEnglish)
		expect(idx.filter('all', 'all', 'english')).toHaveLength(1)
		expect(idx.filter('all', 'all', 'SMITH')).toHaveLength(1)
	})

	it('searches in fukudai', () => {
		const withFukudai = [
			makeCourse({ kogiCd: '005', kogiNm: 'テスト', fukudai: '副題テスト' }),
		]
		const idx = new CourseIndex(withFukudai)
		expect(idx.filter('all', 'all', '副題')).toHaveLength(1)
	})

	it('handles undefined fukudai gracefully', () => {
		const withoutFukudai = [
			makeCourse({ kogiCd: '005', kogiNm: 'テスト', fukudai: undefined }),
		]
		const idx = new CourseIndex(withoutFukudai)
		expect(idx.filter('all', 'all', 'テスト')).toHaveLength(1)
	})

	// --- 全角/半角スペース正規化 ---
	it('matches instructor with half-width space when data has full-width', () => {
		const c = [makeCourse({ kogiCd: '010', tantoKyoin: '山田\u3000太郎' })]
		const idx = new CourseIndex(c)
		expect(idx.filter('all', 'all', '山田 太郎')).toHaveLength(1)
	})

	it('matches instructor with full-width space when data has half-width', () => {
		const c = [makeCourse({ kogiCd: '010', tantoKyoin: '山田 太郎' })]
		const idx = new CourseIndex(c)
		expect(idx.filter('all', 'all', '山田\u3000太郎')).toHaveLength(1)
	})

	// --- 複合フィルタ ---
	it('combines semester and department filters', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('2学期', '人文社会科学部', '')
		expect(result.map((c) => c.kogiCd)).toEqual(['002', '003'])
	})

	it('combines all three filters', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('2学期', '人文社会科学部', '政治')
		expect(result).toHaveLength(1)
		expect(result[0].kogiCd).toBe('002')
	})

	// --- エッジケース ---
	it('handles empty courses array', () => {
		const idx = new CourseIndex([])
		expect(idx.filter('all', 'all', '')).toEqual([])
	})

	it('handles empty search after whitespace', () => {
		const idx = new CourseIndex(courses)
		expect(idx.filter('all', 'all', '')).toHaveLength(3)
	})

	it('handles course with empty slots array', () => {
		const noSlots = [makeCourse({ slots: [] })]
		const idx = new CourseIndex(noSlots)
		// With 'all' semester, no semester filter → should include it
		expect(idx.filter('all', 'all', '')).toHaveLength(1)
		// With specific semester, no matching slots → filtered out
		expect(idx.filter('1学期', 'all', '')).toHaveLength(0)
	})

	it('does not crash with special regex characters in search', () => {
		const idx = new CourseIndex(courses)
		expect(idx.filter('all', 'all', '[.*+?]')).toHaveLength(0)
	})

	// --- searchText フィールド ---
	it('uses pre-computed searchText when available', () => {
		const withSearchText = [
			makeCourse({
				kogiCd: '001',
				kogiNm: '微分積分学',
				searchText: '微分積分学 山田 太郎 001 理工学部',
			}),
		]
		const idx = new CourseIndex(withSearchText)
		expect(idx.filter('all', 'all', '微分')).toHaveLength(1)
		expect(idx.filter('all', 'all', '山田')).toHaveLength(1)
	})

	it('falls back to runtime computation when searchText is missing', () => {
		const withoutSearchText = [
			makeCourse({
				kogiCd: '001',
				kogiNm: '微分積分学',
				tantoKyoin: '山田 太郎',
				searchText: undefined,
			}),
		]
		const idx = new CourseIndex(withoutSearchText)
		expect(idx.filter('all', 'all', '微分')).toHaveLength(1)
		expect(idx.filter('all', 'all', '山田')).toHaveLength(1)
	})

	// --- department + semester の交差 ---
	it('department filter with semester narrows correctly', () => {
		const idx = new CourseIndex(courses)
		// 理工学部 has only course 001 (1学期)
		// With semester=2学期, 001 should not appear
		expect(idx.filter('2学期', '理工学部', '')).toHaveLength(0)
	})

	it('semester=all with department still works', () => {
		const idx = new CourseIndex(courses)
		const result = idx.filter('all', '人文社会科学部', '')
		expect(result).toHaveLength(2)
		expect(result.map((c) => c.kogiCd)).toEqual(['002', '003'])
	})
})
