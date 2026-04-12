import { describe, expect, it } from 'vitest'
import { validateProcessedData } from './validate'

function validData(overrides: Record<string, unknown> = {}) {
	return {
		version: 1,
		generatedAt: '2026-04-12T00:00:00+09:00',
		totalRaw: 1,
		courses: [
			{
				kogiCd: '001',
				kogiNm: 'テスト',
				tantoKyoin: '教員',
				jikanwariRaw: '',
				slots: [],
				kogiKaikojikiNm: '',
				kogiKubunNm: '',
				sekininBushoNm: '',
				kochiNm: '',
				gakusokuKamokuNm: '',
			},
		],
		semesters: ['1学期'],
		departments: ['理工学部'],
		...overrides,
	}
}

describe('validateProcessedData', () => {
	// --- 正常系 ---
	it('accepts valid processed data', () => {
		const result = validateProcessedData(validData())
		expect(result.ok).toBe(true)
	})

	it('accepts data with empty courses array', () => {
		const result = validateProcessedData(validData({ courses: [], totalRaw: 0 }))
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
		const data = validData()
		delete (data as Record<string, unknown>).version
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('version')
	})

	it('rejects data with non-numeric version', () => {
		const result = validateProcessedData(validData({ version: '1' }))
		expect(result.ok).toBe(false)
	})

	// --- coursesフィールド ---
	it('rejects data without courses field', () => {
		const data = validData()
		delete (data as Record<string, unknown>).courses
		const result = validateProcessedData(data)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('courses')
	})

	it('rejects data with non-array courses', () => {
		const result = validateProcessedData(validData({ courses: 'not an array' }))
		expect(result.ok).toBe(false)
	})

	it('rejects courses with missing kogiCd', () => {
		const result = validateProcessedData(
			validData({
				courses: [{ kogiNm: 'test', tantoKyoin: '' }],
			}),
		)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('kogiCd')
	})

	it('rejects courses with missing kogiNm', () => {
		const result = validateProcessedData(
			validData({
				courses: [{ kogiCd: '001', tantoKyoin: '' }],
			}),
		)
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('kogiNm')
	})

	// --- semesters / departments ---
	it('rejects data without semesters array', () => {
		const result = validateProcessedData(validData({ semesters: 'not array' }))
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('semesters')
	})

	it('rejects data without departments array', () => {
		const result = validateProcessedData(validData({ departments: null }))
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('departments')
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

	it('rejects array (bare array of courses)', () => {
		const result = validateProcessedData([{ kogiCd: '001' }])
		expect(result.ok).toBe(false)
		if (!result.ok) expect(result.error).toContain('version')
	})
})
