import { afterEach, describe, expect, it, vi } from 'vitest'
import { loadData } from './load-data'

function validV2Data() {
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
	}
}

afterEach(() => {
	vi.restoreAllMocks()
})

describe('loadData', () => {
	it('正常系: 有効なv2データを返す', async () => {
		const data = validV2Data()
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue({
				ok: true,
				json: () => Promise.resolve(data),
			}),
		)

		const result = await loadData()
		expect(result.version).toBe(2)
		expect(result.courses).toHaveLength(1)
		expect(result.courses[0].cd).toBe('001')
		expect(result.dicts.semesters).toEqual(['1学期'])
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
