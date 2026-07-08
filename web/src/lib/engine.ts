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

/** What the worker returns from `init` — see engine.worker.ts. */
interface InitResult {
	courses: Course[]
	dicts: Dictionaries
	generatedAt: string
	year: string
	hasSaturday: boolean
}

/** A worker reply: `id` echoes the request; `ok` gates `result` vs `error`. */
type WorkerReply =
	| { id: number; ok: true; result: unknown }
	| { id: number; ok: false; error: string }

/**
 * The browser-side facade over the WASM core, which now lives in a Web Worker
 * (engine.worker.ts) so the heavy one-time parse never blocks the main thread.
 *
 * Owns a read-only cache of every course view-model and the dictionaries (sent
 * by the worker once at load), plus the worker handle for `filterAndGrid`
 * queries. The filter index array never crosses back — the worker filters and
 * lays out in one hop, and this side resolves cells against the cache.
 */
export class SyllabusEngine {
	readonly dicts: Dictionaries
	readonly courses: readonly Course[]
	readonly generatedAt: string
	readonly year: string
	readonly hasSaturday: boolean
	readonly days: readonly string[]

	private readonly worker: Worker
	private seq = 0
	private readonly pending = new Map<
		number,
		{ resolve: (v: unknown) => void; reject: (e: Error) => void }
	>()

	private constructor(worker: Worker, init: InitResult) {
		this.worker = worker
		this.courses = init.courses
		this.dicts = init.dicts
		this.generatedAt = init.generatedAt
		this.year = init.year
		this.hasSaturday = init.hasSaturday
		this.days = dayLabels(this.hasSaturday)

		worker.onmessage = (e: MessageEvent<WorkerReply>) => {
			const reply = e.data
			const p = this.pending.get(reply.id)
			if (!p) return
			this.pending.delete(reply.id)
			if (reply.ok) p.resolve(reply.result)
			else p.reject(new Error(reply.error))
		}
	}

	/**
	 * Spin up the worker, fetch `data.json`, and seed the cache.
	 *
	 * The fetch runs here on the main thread, not in the worker, so the document's
	 * `<link rel=preload as=fetch>` is actually consumed — a worker's own fetch
	 * would not adopt it, and the payload would download twice. The body is handed
	 * to the worker as a zero-copy transfer; the expensive parse still runs off-main.
	 *
	 * The default cache mode lets the fetch reuse the preloaded response (a
	 * `no-cache` revalidation would not adopt it). The stable, unhashed URL is
	 * served with a short max-age, so the only staleness risk is a cached copy from
	 * just before a wire-format bump — which fails to parse. That case retries once
	 * with the cache bypassed, so it self-heals instead of erroring.
	 */
	static async create(): Promise<SyllabusEngine> {
		const worker = new Worker(new URL('./engine.worker.ts', import.meta.url), {
			type: 'module',
		})
		try {
			const init = await SyllabusEngine.load(worker, 'default').catch(() =>
				SyllabusEngine.load(worker, 'reload'),
			)
			return new SyllabusEngine(worker, init)
		} catch (e) {
			worker.terminate()
			throw e
		}
	}

	/** Fetch `data.json` (main thread → preload adoption) and init the worker. */
	private static async load(worker: Worker, cache: RequestCache): Promise<InitResult> {
		const res = await fetch(`${import.meta.env.BASE_URL}data.json`, { cache })
		if (!res.ok) {
			throw new Error(`データの取得に失敗しました (HTTP ${res.status})`)
		}
		const buffer = await res.arrayBuffer()
		return new Promise<InitResult>((resolve, reject) => {
			worker.onmessage = (e: MessageEvent<WorkerReply>) => {
				const reply = e.data
				if (reply.id !== 0) return
				if (reply.ok) resolve(reply.result as InitResult)
				else reject(new Error(reply.error))
			}
			worker.onerror = (e) => reject(new Error(e.message || 'ワーカーエラー'))
			// Transfer the buffer (second arg) — ownership moves to the worker, no copy.
			worker.postMessage({ id: 0, type: 'init', buffer }, [buffer])
		})
	}

	/** Post a query keyed by a fresh id; resolves when the worker echoes it. */
	private send(payload: Record<string, unknown>): Promise<unknown> {
		const id = ++this.seq
		return new Promise((resolve, reject) => {
			this.pending.set(id, { resolve, reject })
			this.worker.postMessage({ id, ...payload })
		})
	}

	/**
	 * Filter by the given selectors and lay the matches onto the timetable in one
	 * worker round-trip, returning the resolved grid and the distinct-course count.
	 */
	async filterAndGrid(
		semester: string,
		department: string,
		campus: string,
		query: string,
	): Promise<{ grid: Map<GridKey, Course[]>; count: number }> {
		const res = (await this.send({
			type: 'filterAndGrid',
			semester,
			department,
			campus,
			query,
		})) as WasmGridResult
		return {
			grid: assembleGrid(res.cells, this.courses, this.days),
			count: res.countUnique,
		}
	}
}
