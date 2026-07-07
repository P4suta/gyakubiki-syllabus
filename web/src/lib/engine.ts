import initWasm, { SyllabusEngine as WasmEngine } from '../wasm/syllabus.js'
import type { Course, Dictionaries } from '../types/course'

/** A timetable cell key, `"<day label>-<period>"` (e.g. `"月-1"`). */
export type GridKey = `${string}-${number}`

/** Periods shown in the grid (1–6); fixed, unlike the day columns. */
export const PERIODS = [1, 2, 3, 4, 5, 6] as const

const WEEKDAYS = ['月', '火', '水', '木', '金'] as const

/** Day column labels, widened with 土 only when the data needs it. */
export function dayLabels(hasSaturday: boolean): readonly string[] {
	return hasSaturday ? [...WEEKDAYS, '土'] : [...WEEKDAYS]
}

/** One populated cell as returned by the WASM `grid` call. */
interface WasmGridCell {
	day: number
	period: number
	courses: number[]
}

/** The shape of `WasmEngine.grid`'s return value. */
interface WasmGridResult {
	cells: WasmGridCell[]
	countUnique: number
}

/**
 * Assemble the WASM grid (numeric day/period cells of course *indices*) into the
 * UI's `Map<GridKey, Course[]>`, resolving indices against the view cache.
 *
 * Pure and WASM-free for direct unit testing.
 */
export function assembleGrid(
	cells: WasmGridCell[],
	views: readonly Course[],
	days: readonly string[],
): Map<GridKey, Course[]> {
	const grid = new Map<GridKey, Course[]>()
	for (const day of days) {
		for (const period of PERIODS) {
			grid.set(`${day}-${period}`, [])
		}
	}
	for (const cell of cells) {
		const day = days[cell.day]
		if (day === undefined) continue
		grid.set(
			`${day}-${cell.period}`,
			Array.from(cell.courses, (i) => views[i]),
		)
	}
	return grid
}

/**
 * The browser-side facade over the WASM core.
 *
 * Owns the engine handle plus a read-only cache of every course view-model and
 * the dictionaries (both fetched from WASM once at load). `filter` returns course
 * indices; `grid` resolves them — no per-query data crosses the WASM boundary.
 */
export class SyllabusEngine {
	readonly dicts: Dictionaries
	readonly courses: readonly Course[]
	readonly generatedAt: string
	readonly year: string
	readonly hasSaturday: boolean
	readonly days: readonly string[]

	private readonly wasm: WasmEngine

	private constructor(wasm: WasmEngine, views: Course[], dicts: Dictionaries) {
		this.wasm = wasm
		this.courses = views
		this.dicts = dicts
		this.generatedAt = wasm.generatedAt()
		this.year = wasm.year()
		this.hasSaturday = wasm.hasSaturday()
		this.days = dayLabels(this.hasSaturday)
	}

	/** Initialize WASM, fetch `data.json`, and build the engine. */
	static async create(): Promise<SyllabusEngine> {
		await initWasm()

		// `data.json` is at a stable, unhashed URL, so `no-cache` forces an ETag
		// revalidation each load — a wire-format bump or the monthly refresh reaches
		// users instead of erroring on a hard-cached stale copy. 304s stay cheap.
		const res = await fetch(`${import.meta.env.BASE_URL}data.json`, { cache: 'no-cache' })
		if (!res.ok) {
			throw new Error(`データの取得に失敗しました (HTTP ${res.status})`)
		}
		const text = await res.text()

		let wasm: WasmEngine
		try {
			wasm = WasmEngine.fromJson(text)
		} catch (e) {
			// from_json's thiserror message arrives as a JS Error.
			throw new Error(e instanceof Error ? e.message : String(e))
		}

		return new SyllabusEngine(
			wasm,
			wasm.allCourseViews() as Course[],
			wasm.dicts() as Dictionaries,
		)
	}

	/** Course indices matching the filters (`'all'` = no filter), ascending. */
	filter(semester: string, department: string, campus: string, query: string): Uint32Array {
		return this.wasm.filter(semester, department, campus, query)
	}

	/** Lay filtered indices onto the timetable, with the distinct-course count. */
	grid(
		indices: Uint32Array,
		semester: string,
	): { grid: Map<GridKey, Course[]>; count: number } {
		const result = this.wasm.grid(indices, semester) as WasmGridResult
		return {
			grid: assembleGrid(result.cells, this.courses, this.days),
			count: result.countUnique,
		}
	}
}
