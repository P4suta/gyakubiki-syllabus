import { describe, expect, it } from 'vitest'
import { validateCourseDetail } from './details'

describe('validateCourseDetail', () => {
	it('accepts the complete public allowlist shape', () => {
		const value = {
			cd: 'ABC123',
			summary: '概要',
			delivery: { mode: 'online', raw: 'オンライン', isMedia: true },
			eval: { rows: [{ item: '試験', weight: 100, type: 'exam' }], note: '補足' },
			goals: ['到達目標'],
			plan: [{ n: 1, text: '導入', kind: 'start' }],
			officeHour: [{ name: '教員', day: '月', time: '12:00', place: '研究室' }],
			extra: [{ label: '講義副題', text: '副題' }],
			textbookInfo: { isNone: false, sections: [{ label: '教科書', lines: ['書名'] }] },
			prepInfo: { hours: 2, yoshu: '予習', fukushu: '復習' },
		}
		expect(validateCourseDetail(value)).toEqual(value)
	})

	it('rejects crawler or newly introduced fields', () => {
		expect(() => validateCourseDetail({ cd: 'ABC123', lastUpdate: 'secret' })).toThrow(
			'未知のfield',
		)
		expect(() => validateCourseDetail({ cd: 'ABC123', sessionToken: 'secret' })).toThrow(
			'未知のfield',
		)
	})

	it('rejects malformed nested values and unsafe codes', () => {
		expect(() => validateCourseDetail({ cd: 'ABC123', plan: [{ n: '1', text: '導入' }] })).toThrow(
			'plan.n',
		)
		expect(() => validateCourseDetail({ cd: '../secret' })).toThrow('科目コード')
		expect(() => validateCourseDetail({ cd: 'ABC123', delivery: { mode: 'telepathy' } })).toThrow(
			'delivery.mode',
		)
	})
})
