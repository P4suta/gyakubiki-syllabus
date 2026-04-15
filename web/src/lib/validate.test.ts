import { describe, expect, it } from 'vitest'
import { validateProcessedData } from './validate'

function validV2Data(overrides: Record<string, unknown> = {}) {
	return {
		version: 2,
		generatedAt: '2026-04-16T00:00:00+09:00',
		totalRaw: 1,
		dicts: {
			semesters: ['1学期'],
			departments: ['理工学部'],
			campuses: ['朝倉キャンパス'],
			kubun: ['講義'],
			kaikojiki: ['1学期'],
		},
		indices: {
			semester: {},
			department: {},
			campus: {},
		},
		courses: [
			{
				cd: '001',
				nm: 'テスト',
				prof: '教員',
				raw: '',
				slots: [],
				ki: 0,
				kbn: 0,
				dept: 0,
				campus: 0,
				st: '',
			},
		],
		...overrides,
	}
}

describe('validateProcessedData', () => {
	// --- 正常系 ---
	it('accepts valid v2 data', () => {
		const result = validateProcessedData(validV2Data())
		expect(result.ok).toBe(true)
	})

	it('accepts data with empty courses array', () => {
		const result = validateProcessedData(validV2Data({ courses: [], totalRaw: 0 }))
		expect(result.ok).toBe(true)
	})

	// --- 未処理APIレスポンスの検出 ---
	it('rejects raw API response with selectKogiDtoList', () => {
		const result = validateProcessedData({
			selectKogiDtoList: [{ kogiCd: '001' }],
		})
		expect(result.ok).toBe(false)
		if (!result.ok) {
			expect(result.error).toContain('syllabus-cli convert')
			expect(result.error).toContain('APIレスポンス')
		}
	})

	// --- バージョンフィールド ---
	it('rejects data without version field', () => {
		const data = validV2Data()
		delete (data as Record<string, unknown>).version
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('version')
	})

	it('rejects data with non-numeric version', () => {
		const result = validateProcessedData(validV2Data({ version: '2' }))
		expect(result.ok).toBe(false)
	})

	it('rejects v1 data', () => {
		const result = validateProcessedData(validV2Data({ version: 1 }))
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('version 2')
	})

	// --- coursesフィールド ---
	it('rejects data without courses field', () => {
		const data = validV2Data()
		delete (data as Record<string, unknown>).courses
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('courses')
	})

	it('rejects data with non-array courses', () => {
		const result = validateProcessedData(validV2Data({ courses: 'not an array' }))
		expect(result.ok).toBe(false)
	})

	it('rejects courses with missing cd', () => {
		const result = validateProcessedData(
			validV2Data({
				courses: [{ nm: 'test', prof: '' }],
			}),
		)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('cd')
	})

	it('rejects courses with missing nm', () => {
		const result = validateProcessedData(
			validV2Data({
				courses: [{ cd: '001', prof: '' }],
			}),
		)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('nm')
	})

	// --- dicts フィールド ---
	it('rejects data without dicts field', () => {
		const data = validV2Data()
		delete (data as Record<string, unknown>).dicts
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('dicts')
	})

	it('rejects dicts with missing semesters', () => {
		const data = validV2Data()
		delete (data.dicts as Record<string, unknown>).semesters
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('dicts.semesters')
	})

	it('rejects dicts with missing campuses', () => {
		const data = validV2Data()
		delete (data.dicts as Record<string, unknown>).campuses
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('dicts.campuses')
	})

	// --- indices フィールド ---
	it('rejects data without indices field', () => {
		const data = validV2Data()
		delete (data as Record<string, unknown>).indices
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('indices')
	})

	// --- null / undefined / 非オブジェクト ---
	it('rejects null', () => {
		const result = validateProcessedData(null)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('空')
	})

	it('rejects undefined', () => {
		const result = validateProcessedData(undefined)
		expect(result.ok).toBe(false)
	})

	it('rejects plain string', () => {
		const result = validateProcessedData('hello')
		expect(result.ok).toBe(false)
	})

	it('rejects number', () => {
		const result = validateProcessedData(42)
		expect(result.ok).toBe(false)
	})
})
