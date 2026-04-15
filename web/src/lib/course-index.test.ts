import { describe, expect, it } from 'vitest'
import type { CourseV2, Dictionaries, IndicesMap } from '../types/course'
import { CourseIndex, normalize } from './course-index'
import { BitSet } from './bitset'

const DICTS: Dictionaries = {
	semesters: ['1学期', '2学期', '通年'],
	departments: ['人文社会科学部', '理工学部'],
	campuses: ['朝倉キャンパス', '物部キャンパス'],
	kubun: ['講義', '演習'],
	kaikojiki: ['1学期', '2学期', '通年'],
}

function makeCourse(overrides: Partial<CourseV2> = {}): CourseV2 {
	return {
		cd: '001',
		nm: 'テスト講義',
		prof: '教員 太郎',
		raw: '1学期: 月曜日１時限',
		slots: [{ s: 0, d: 0, p: 1 }], // 1学期, 月, 1
		ki: 0,
		kbn: 0,
		dept: 1,    // 理工学部
		campus: 0,  // 朝倉キャンパス
		st: 'テスト講義 教員 太郎 001 理工学部',
		...overrides,
	}
}

// Build minimal indices for test data
function buildTestIndices(courses: CourseV2[], dicts: Dictionaries): IndicesMap {
	const n = courses.length
	const numWords = Math.ceil(n / 32) || 1

	const semester: Record<string, string> = {}
	const department: Record<string, string> = {}
	const campus: Record<string, string> = {}

	// Semester bitsets
	const tsuunenIdx = dicts.semesters.indexOf('通年')
	const tsuunenCourses: number[] = []

	for (let si = 0; si < dicts.semesters.length; si++) {
		const words = new Uint32Array(numWords)
		for (let ci = 0; ci < courses.length; ci++) {
			for (const slot of courses[ci].slots) {
				if (slot.s === si) {
					words[ci >>> 5] |= 1 << (ci & 31)
				}
				if (slot.s === tsuunenIdx && !tsuunenCourses.includes(ci)) {
					tsuunenCourses.push(ci)
				}
			}
		}
		semester[String(si)] = encodeBitset(words)
	}

	// Propagate 通年 to all non-通年 semester bitsets
	for (const key of Object.keys(semester)) {
		if (Number(key) === tsuunenIdx) continue
		const bs = BitSet.fromBase64(semester[key])
		const words = new Uint32Array(bs.words)
		for (const ci of tsuunenCourses) {
			words[ci >>> 5] |= 1 << (ci & 31)
		}
		semester[key] = encodeBitset(words)
	}

	// Department bitsets
	for (let di = 0; di < dicts.departments.length; di++) {
		const words = new Uint32Array(numWords)
		for (let ci = 0; ci < courses.length; ci++) {
			if (courses[ci].dept === di) {
				words[ci >>> 5] |= 1 << (ci & 31)
			}
		}
		department[String(di)] = encodeBitset(words)
	}

	// Campus bitsets
	for (let cai = 0; cai < dicts.campuses.length; cai++) {
		const words = new Uint32Array(numWords)
		for (let ci = 0; ci < courses.length; ci++) {
			if (courses[ci].campus === cai) {
				words[ci >>> 5] |= 1 << (ci & 31)
			}
		}
		campus[String(cai)] = encodeBitset(words)
	}

	return { semester, department, campus }
}

function encodeBitset(words: Uint32Array): string {
	const bytes = new Uint8Array(words.buffer)
	return btoa(String.fromCharCode(...bytes))
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
	const courses: CourseV2[] = [
		makeCourse({
			cd: '001',
			nm: '微分積分学',
			prof: '山田 太郎',
			slots: [{ s: 0, d: 0, p: 1 }], // 1学期, 月, 1
			dept: 1,    // 理工学部
			campus: 0,  // 朝倉キャンパス
			st: '微分積分学 山田 太郎 001 理工学部',
		}),
		makeCourse({
			cd: '002',
			nm: '政治学概論',
			prof: '小川 寛貴',
			slots: [{ s: 1, d: 1, p: 2 }], // 2学期, 火, 2
			dept: 0,    // 人文社会科学部
			campus: 1,  // 物部キャンパス
			st: '政治学概論 小川 寛貴 002 人文社会科学部',
		}),
		makeCourse({
			cd: '003',
			nm: '哲学概論',
			prof: '佐藤 哲也',
			slots: [{ s: 2, d: 4, p: 5 }], // 通年, 金, 5
			dept: 0,    // 人文社会科学部
			campus: 0,  // 朝倉キャンパス
			st: '哲学概論 佐藤 哲也 003 人文社会科学部',
		}),
	]
	const indices = buildTestIndices(courses, DICTS)

	// --- 全件 ---
	it('returns all when no filters', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		expect(idx.filter('all', 'all', 'all', '')).toHaveLength(3)
	})

	// --- 学期フィルタ ---
	it('filters by semester: 1学期', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('1学期', 'all', 'all', '')
		expect(result.map((c) => c.cd)).toEqual(['001', '003'])
	})

	it('filters by semester: 2学期', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('2学期', 'all', 'all', '')
		expect(result.map((c) => c.cd)).toEqual(['002', '003'])
	})

	it('通年 courses appear in all semester filters', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result1 = idx.filter('1学期', 'all', 'all', '')
		const result2 = idx.filter('2学期', 'all', 'all', '')
		expect(result1.some((c) => c.cd === '003')).toBe(true)
		expect(result2.some((c) => c.cd === '003')).toBe(true)
	})

	// --- 部署フィルタ ---
	it('filters by department', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', '理工学部', 'all', '')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('001')
	})

	it('returns empty for nonexistent department', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		expect(idx.filter('all', '医学部', 'all', '')).toHaveLength(0)
	})

	// --- キャンパスフィルタ ---
	it('filters by campus', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', 'all', '朝倉キャンパス', '')
		expect(result).toHaveLength(2)
		expect(result.map((c) => c.cd)).toEqual(['001', '003'])
	})

	it('filters by campus: 物部', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', 'all', '物部キャンパス', '')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('002')
	})

	it('returns empty for nonexistent campus', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		expect(idx.filter('all', 'all', '岡豊キャンパス', '')).toHaveLength(0)
	})

	// --- テキスト検索 ---
	it('searches by course name', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', 'all', 'all', '微分')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('001')
	})

	it('searches by instructor name', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', 'all', 'all', '小川')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('002')
	})

	it('searches by kogiCd', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', 'all', 'all', '003')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('003')
	})

	it('search is case insensitive', () => {
		const c = [makeCourse({
			cd: '004',
			nm: 'English Communication',
			prof: 'Smith, John',
			st: 'english communication smith, john 004 理工学部',
		})]
		const i = buildTestIndices(c, DICTS)
		const idx = new CourseIndex(c, DICTS, i)
		expect(idx.filter('all', 'all', 'all', 'english')).toHaveLength(1)
		expect(idx.filter('all', 'all', 'all', 'SMITH')).toHaveLength(1)
	})

	// --- 複合フィルタ ---
	it('combines semester and department filters', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('2学期', '人文社会科学部', 'all', '')
		expect(result.map((c) => c.cd)).toEqual(['002', '003'])
	})

	it('combines all four filters', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('2学期', '人文社会科学部', '物部キャンパス', '')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('002')
	})

	it('combines campus with semester', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('1学期', 'all', '朝倉キャンパス', '')
		expect(result.map((c) => c.cd)).toEqual(['001', '003'])
	})

	it('combines campus with department and search', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', '人文社会科学部', '朝倉キャンパス', '哲学')
		expect(result).toHaveLength(1)
		expect(result[0].cd).toBe('003')
	})

	// --- エッジケース ---
	it('handles empty courses array', () => {
		const empty: IndicesMap = { semester: {}, department: {}, campus: {} }
		const idx = new CourseIndex([], DICTS, empty)
		expect(idx.filter('all', 'all', 'all', '')).toEqual([])
	})

	it('handles course with empty slots array', () => {
		const noSlots = [makeCourse({ slots: [] })]
		const i = buildTestIndices(noSlots, DICTS)
		const idx = new CourseIndex(noSlots, DICTS, i)
		expect(idx.filter('all', 'all', 'all', '')).toHaveLength(1)
		// With specific semester, no matching slots → filtered out via bitset
		expect(idx.filter('1学期', 'all', 'all', '')).toHaveLength(0)
	})

	it('does not crash with special regex characters in search', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		expect(idx.filter('all', 'all', 'all', '[.*+?]')).toHaveLength(0)
	})

	// --- department + semester の交差 ---
	it('department filter with semester narrows correctly', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		expect(idx.filter('2学期', '理工学部', 'all', '')).toHaveLength(0)
	})

	it('semester=all with department still works', () => {
		const idx = new CourseIndex(courses, DICTS, indices)
		const result = idx.filter('all', '人文社会科学部', 'all', '')
		expect(result).toHaveLength(2)
		expect(result.map((c) => c.cd)).toEqual(['002', '003'])
	})
})
