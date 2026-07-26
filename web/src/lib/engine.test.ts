import { describe, expect, it } from 'vitest'
import type { Course } from '../types/course'
import type { DatasetManifest } from './dataset'
import {
	assembleGrid,
	dayLabels,
	periodLabels,
	validateInitResult,
	validateWasmGridResult,
	validateWasmPlanResult,
} from './engine'

function view(cd: string): Course {
	return {
		cd,
		nm: `科目${cd}`,
		prof: '教員',
		raw: '',
		slots: [],
		ki: 0,
		kbn: 0,
		dept: 0,
		campus: 0,
		st: '',
	}
}

describe('dayLabels', () => {
	it('is the five weekdays without Saturday', () => {
		expect(dayLabels(5)).toEqual(['月', '火', '水', '木', '金'])
	})

	it('appends 土 when the data has Saturday', () => {
		expect(dayLabels(6)).toEqual(['月', '火', '水', '木', '金', '土'])
	})

	it('retains Sunday when present', () => {
		expect(dayLabels(7)).toEqual(['月', '火', '水', '木', '金', '土', '日'])
	})
})

describe('assembleGrid', () => {
	const days = dayLabels(5)
	const periods = periodLabels(7)
	const views = [view('001'), view('002'), view('003')]

	it('seeds every day×period cell as an empty array', () => {
		const grid = assembleGrid([], views, days, periods)
		expect(grid.size).toBe(days.length * periods.length)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})

	it('resolves course indices into the matching cell', () => {
		const grid = assembleGrid([{ day: 0, period: 1, courses: [0, 2] }], views, days)
		expect(grid.get('月-1')?.map((c) => c.cd)).toEqual(['001', '003'])
		expect(grid.get('火-2')).toEqual([])
	})

	it('maps day index to the right label', () => {
		const grid = assembleGrid([{ day: 4, period: 5, courses: [1] }], views, days)
		expect(grid.get('金-5')?.map((c) => c.cd)).toEqual(['002'])
	})

	it('keeps seventh period cells', () => {
		const grid = assembleGrid([{ day: 0, period: 7, courses: [1] }], views, days, periods)
		expect(grid.get('月-7')?.map((course) => course.cd)).toEqual(['002'])
	})

	it('ignores cells whose day index is out of range', () => {
		const grid = assembleGrid([{ day: 9, period: 1, courses: [0] }], views, days)
		for (const courses of grid.values()) {
			expect(courses).toEqual([])
		}
	})
})

describe('WASM runtime result validation', () => {
	const courses = [view('001'), view('002')]

	it('accepts a coherent query result and rejects count/index tampering', () => {
		const valid = {
			cells: [{ day: 0, period: 1, courses: [0] }],
			total: 2,
			scheduledCount: 1,
			unscheduledCount: 1,
			unscheduled: [1],
			matches: [
				{ i: 0, spans: [{ f: 0, o: 0, l: 1 }] },
				{ i: 1, spans: [{ f: 5, o: 2, l: 1 }] },
			],
		}
		expect(validateWasmGridResult(valid, 2, 5, 7, true)).toEqual(valid)
		expect(() => validateWasmGridResult({ ...valid, total: 1 }, 2, 5, 7, true)).toThrow('整合')
		expect(() =>
			validateWasmGridResult({ ...valid, unscheduled: [99], unscheduledCount: 1 }, 2, 5, 7, true),
		).toThrow('unscheduled')
		expect(() => validateWasmGridResult({ ...valid, injected: true }, 2, 5, 7, true)).toThrow(
			'未知のfield',
		)
	})

	it('accepts a coherent atomic plan and rejects unknown index injection', () => {
		const valid = {
			validCodes: ['001', '002'],
			unknownCodes: ['999'],
			cells: [{ day: 0, period: 1, courses: [0, 1] }],
			unscheduled: [],
			conflicts: [{ day: 0, period: 1, courses: [0, 1] }],
			credits: {
				totalCredits: 4,
				totalCourses: 2,
				uncredited: 0,
				byKubun: [{ key: '専門', credits: 4, count: 2 }],
				byBunrui: [],
				byNen: [],
			},
		}
		expect(validateWasmPlanResult(valid, courses, 5, 7)).toEqual(valid)
		expect(() =>
			validateWasmPlanResult(
				{
					...valid,
					cells: [{ day: 0, period: 1, courses: [2] }],
				},
				courses,
				5,
				7,
			),
		).toThrow('cell')
	})
})

describe('WASM init boundary validation', () => {
	const hash = 'a'.repeat(64)
	const asset = { path: 'asset.aaaaaaaaaaaaaaaa.json', bytes: 1, sha256: hash }
	const manifest: DatasetManifest = {
		schemaVersion: 4,
		appCompatVersion: 4,
		datasetId: hash,
		sourceCommit: '0123456789abcdef0123456789abcdef01234567',
		year: '2026',
		generatedAt: '2026-07-26T00:00:00+09:00',
		basePath: `datasets/${hash}/`,
		counts: {
			courses: 1,
			details: 0,
			detailCoverage: 0,
			scheduledCourses: 1,
			unscheduledCourses: 0,
		},
		range: { minDay: 0, maxDay: 0, minPeriod: 1, maxPeriod: 1 },
		assets: { data: asset, index: asset, details: asset },
	}
	const init = {
		courses: [
			{
				cd: 'ABC001',
				nm: '科目',
				prof: '教員',
				raw: '1学期: 月曜日1時限',
				ki: 0,
				kbn: 0,
				dept: 0,
				campus: 0,
				pat: '4',
			},
		],
		dicts: {
			semesters: ['1学期'],
			departments: ['理工学部'],
			campuses: ['朝倉キャンパス'],
			kubun: ['講義'],
			kaikojiki: ['1学期'],
		},
		generatedAt: manifest.generatedAt,
		year: manifest.year,
		datasetId: manifest.datasetId,
		dayCount: 5,
		maxPeriod: 1,
		searchStatus: 'ready',
	}

	it('accepts a complete typed result and rejects unknown or unsafe course fields', () => {
		expect(validateInitResult(init, manifest)).toEqual(init)
		expect(() =>
			validateInitResult(
				{ ...init, courses: [{ ...init.courses[0], sessionToken: 'private' }] },
				manifest,
			),
		).toThrow('未知のfield')
		expect(() =>
			validateInitResult({ ...init, courses: [{ ...init.courses[0], cd: '../escape' }] }, manifest),
		).toThrow('必須field')
	})

	it('rejects dictionary references and manifest identity/range mismatches', () => {
		expect(() =>
			validateInitResult({ ...init, courses: [{ ...init.courses[0], dept: 1 }] }, manifest),
		).toThrow('辞書参照')
		expect(() =>
			validateInitResult(
				{ ...init, dicts: { ...init.dicts, semesters: ['1学期', '1学期'] } },
				manifest,
			),
		).toThrow('辞書形式')
		expect(() =>
			validateInitResult(
				{ ...init, dicts: { ...init.dicts, departments: ['unsafe\nvalue'] } },
				manifest,
			),
		).toThrow('辞書形式')
		expect(() => validateInitResult({ ...init, maxPeriod: 2 }, manifest)).toThrow('manifestとdata')
		expect(() => validateInitResult({ ...init, injected: true }, manifest)).toThrow('未知のfield')
	})
})
