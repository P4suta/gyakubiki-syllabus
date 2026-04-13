import { afterEach, describe, expect, it, vi } from 'vitest'
import { loadData } from './load-data'

function validData() {
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
	}
}

afterEach(() => {
	vi.restoreAllMocks()
})

describe('loadData', () => {
	it('正常系: 有効なデータを返す', async () => {
		const data = validData()
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue({
				ok: true,
				json: () => Promise.resolve(data),
			}),
		)

		const result = await loadData()
		expect(result.version).toBe(1)
		expect(result.courses).toHaveLength(1)
		expect(result.courses[0].kogiCd).toBe('001')
	})

	it('異常系: HTTPエラー時にthrowする', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue({
				ok: false,
				status: 404,
			}),
		)

		await expect(loadData()).rejects.toThrow('HTTP 404')
	})

	it('異常系: バリデーション失敗時にthrowする', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue({
				ok: true,
				json: () => Promise.resolve({ invalid: true }),
			}),
		)

		await expect(loadData()).rejects.toThrow()
	})
})
