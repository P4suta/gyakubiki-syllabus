import type { Course, Dictionaries } from '../types/course'
import { assetUrl, type DatasetManifest, fetchVerifiedAsset, loadManifest } from './dataset'
import { isWorkerReply } from './worker-protocol'

/** A timetable cell key, `"<day label>-<period>"` (e.g. `"月-1"`). */
export type GridKey = `${string}-${number}`

/** Periods shown in the grid, derived from the v4 dataset (1…8). */
export function periodLabels(maxPeriod: number): number[] {
	return Array.from({ length: Math.max(1, Math.min(8, maxPeriod)) }, (_, index) => index + 1)
}

const ALL_DAYS = ['月', '火', '水', '木', '金', '土', '日'] as const

/** Day column labels, widened through Sunday only when the data needs it. */
export function dayLabels(dayCount: number): readonly string[] {
	return ALL_DAYS.slice(0, Math.max(5, Math.min(7, dayCount)))
}

/** One populated cell as returned by the WASM `grid` call. */
interface WasmGridCell {
	day: number
	period: number
	courses: number[]
}

/** Per-course match spans as returned by the WASM `query` call: `i` is the
 *  course index, each span carries its field discriminant `f` and UTF-16
 *  offset/length `o`/`l`. */
interface WasmHighlight {
	i: number
	spans: { f: number; o: number; l: number }[]
}

/** The shape of `WasmEngine.query`'s return value (a superset of `grid`'s). */
interface WasmGridResult {
	cells: WasmGridCell[]
	total: number
	scheduledCount: number
	unscheduledCount: number
	unscheduled: number[]
	matches: WasmHighlight[]
}

/** The `Field::Name` discriminant — the only field highlighted on a card. */
const FIELD_NAME = 0
const FIELD_LABELS = [
	'科目名',
	'副題',
	'担当教員',
	'科目コード',
	'開講部署・分類',
	'授業概要',
	'授業の目的',
	'到達目標',
	'授業計画',
	'教科書・参考書',
	'履修条件',
	'予習・復習',
	'オフィスアワー',
	'キーワード',
	'授業担当教員',
	'ナンバリング',
	'SDGs',
	'成績評価',
	'授業実施方法',
	'追加シラバス項目',
] as const

/** Match spans keyed by course `cd` (course-name field only). */
export type Highlights = Map<string, { start: number; len: number }[]>

/** Resolve the WASM highlight list into name-field spans keyed by course cd. */
function resolveHighlights(list: WasmHighlight[], views: readonly Course[]): Highlights {
	const byCd: Highlights = new Map()
	for (const h of list ?? []) {
		const cd = views[h.i]?.cd
		if (cd === undefined) continue
		const spans = h.spans.filter((s) => s.f === FIELD_NAME).map((s) => ({ start: s.o, len: s.l }))
		if (spans.length > 0) byCd.set(cd, spans)
	}
	return byCd
}

function resolveMatchFields(
	list: WasmHighlight[],
	views: readonly Course[],
): Map<string, string[]> {
	const byCd = new Map<string, string[]>()
	for (const hit of list ?? []) {
		const cd = views[hit.i]?.cd
		if (!cd) continue
		const labels = [
			...new Set(
				hit.spans.flatMap((span) => {
					const label = FIELD_LABELS[span.f]
					return label ? [label] : []
				}),
			),
		]
		if (labels.length > 0) byCd.set(cd, labels)
	}
	return byCd
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
	periods: readonly number[] = periodLabels(8),
): Map<GridKey, Course[]> {
	const grid = new Map<GridKey, Course[]>()
	for (const day of days) {
		for (const period of periods) {
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

/** A timetable collision, as returned by the WASM `planSummary` call. */
export interface Conflict {
	day: number
	period: number
	courses: number[]
}

/** One category's rolled-up credits and course count. */
export interface Tally {
	key: string
	credits: number
	count: number
}

/** A plan's summary: every conflict and the credit tallies across three axes. */
export interface PlanSummaryResult {
	conflicts: Conflict[]
	credits: {
		totalCredits: number
		totalCourses: number
		uncredited: number
		byKubun: Tally[]
		byBunrui: Tally[]
		byNen: Tally[]
	}
}

interface WasmPlanResult extends PlanSummaryResult {
	validCodes: string[]
	unknownCodes: string[]
	cells: WasmGridCell[]
	unscheduled: number[]
}

export interface PlanResult extends PlanSummaryResult {
	validCodes: string[]
	unknownCodes: string[]
	grid: Map<GridKey, Course[]>
	unscheduled: Course[]
}

/** What the worker returns from `init` — see engine.worker.ts. */
interface InitResult {
	courses: Course[]
	dicts: Dictionaries
	generatedAt: string
	year: string
	datasetId: string
	dayCount: number
	maxPeriod: number
	searchStatus: 'loading' | 'ready' | 'error'
}

function boundedString(value: unknown, maxLength = 16_384): value is string {
	return typeof value === 'string' && value.length <= maxLength
}

function validDictionaryValue(value: unknown): value is string {
	return boundedString(value, 1_024) && value.trim().length > 0 && !/\p{Cc}/u.test(value)
}

function validateDictionary(
	value: unknown,
	name: string,
	maxEntries = 20_000,
): asserts value is string[] {
	if (
		!Array.isArray(value) ||
		value.length > maxEntries ||
		!value.every(validDictionaryValue) ||
		new Set(value).size !== value.length
	) {
		throw new Error(`dataの${name}辞書形式が不正です`)
	}
}

function validateCourse(
	value: unknown,
	dicts: Dictionaries,
	seenCodes: Set<string>,
): asserts value is Course {
	if (!isRecord(value)) throw new Error('dataのcourse形式が不正です')
	exactKeys(
		value,
		[
			'cd',
			'nm',
			'sub',
			'prof',
			'raw',
			'ki',
			'kbn',
			'dept',
			'campus',
			'gaku',
			'gakka',
			'nen',
			'bunrui',
			'bunya',
			'pat',
			'unit',
			'dm',
			'ev',
		],
		'dataのcourse',
	)
	if (
		!validCourseCode(value.cd) ||
		seenCodes.has(value.cd) ||
		!boundedString(value.nm, 4_096) ||
		value.nm.trim().length === 0 ||
		!boundedString(value.prof, 4_096) ||
		!boundedString(value.raw, 4_096) ||
		!boundedString(value.pat, 128) ||
		value.pat.trim().length === 0
	) {
		throw new Error('dataのcourse必須fieldが不正です')
	}
	seenCodes.add(value.cd)
	for (const [key, limit] of [
		['ki', dicts.kaikojiki.length],
		['kbn', dicts.kubun.length],
		['dept', dicts.departments.length],
		['campus', dicts.campuses.length],
	] as const) {
		const index = value[key]
		if (!Number.isSafeInteger(index) || (index as number) < 0 || (index as number) >= limit) {
			throw new Error(`dataのcourse.${key}辞書参照が不正です`)
		}
	}
	for (const key of ['sub', 'gaku', 'gakka', 'nen', 'bunrui', 'bunya', 'unit'] as const) {
		if (value[key] !== undefined && !boundedString(value[key], 4_096)) {
			throw new Error(`dataのcourse.${key}が不正です`)
		}
	}
	if (
		value.dm !== undefined &&
		!['onsite', 'online', 'ondemand', 'hybrid'].includes(String(value.dm))
	) {
		throw new Error('dataのcourse.dmが不正です')
	}
	if (
		value.ev !== undefined &&
		(!Array.isArray(value.ev) ||
			value.ev.length > 32 ||
			!value.ev.every((item) => boundedString(item, 256)))
	) {
		throw new Error('dataのcourse.evが不正です')
	}
}

export function validateInitResult(value: unknown, manifest: DatasetManifest): InitResult {
	if (!isRecord(value)) throw new Error('検索ワーカーの初期化応答が不正です')
	exactKeys(
		value,
		[
			'courses',
			'dicts',
			'generatedAt',
			'year',
			'datasetId',
			'dayCount',
			'maxPeriod',
			'searchStatus',
		],
		'検索ワーカーの初期化応答',
	)
	if (!isRecord(value.dicts)) {
		throw new Error('dataの辞書形式が不正です')
	}
	const dicts = value.dicts
	exactKeys(dicts, ['semesters', 'departments', 'campuses', 'kubun', 'kaikojiki'], 'dataの辞書')
	for (const key of ['semesters', 'departments', 'campuses', 'kubun', 'kaikojiki'] as const) {
		validateDictionary(dicts[key], key)
	}
	const typedDicts = dicts as unknown as Dictionaries
	if (!Array.isArray(value.courses) || value.courses.length !== manifest.counts.courses) {
		throw new Error('manifestとdataの科目件数が一致しません')
	}
	const seenCodes = new Set<string>()
	for (const course of value.courses) {
		validateCourse(course, typedDicts, seenCodes)
	}
	if (
		value.datasetId !== manifest.datasetId ||
		value.generatedAt !== manifest.generatedAt ||
		value.year !== manifest.year
	) {
		throw new Error('manifestとdataのidentityが一致しません')
	}
	if (
		!boundedString(value.datasetId, 64) ||
		!/^[0-9a-f]{64}$/u.test(value.datasetId) ||
		!boundedString(value.generatedAt, 128) ||
		!Number.isFinite(Date.parse(value.generatedAt)) ||
		!boundedString(value.year, 4) ||
		!/^\d{4}$/u.test(value.year)
	) {
		throw new Error('dataのidentity fieldが不正です')
	}
	if (
		!Number.isInteger(value.dayCount) ||
		(value.dayCount as number) < 5 ||
		(value.dayCount as number) > 7 ||
		!Number.isInteger(value.maxPeriod) ||
		(value.maxPeriod as number) < 1 ||
		(value.maxPeriod as number) > 8
	) {
		throw new Error('dataの曜日・時限範囲が不正です')
	}
	const expectedDayCount =
		manifest.range.maxDay == null ? 5 : Math.max(5, manifest.range.maxDay + 1)
	const expectedMaxPeriod = manifest.range.maxPeriod ?? 1
	if (value.dayCount !== expectedDayCount || value.maxPeriod !== expectedMaxPeriod) {
		throw new Error('manifestとdataの曜日・時限範囲が一致しません')
	}
	if (!['loading', 'ready', 'error'].includes(String(value.searchStatus))) {
		throw new Error('検索index状態が不正です')
	}
	return value as unknown as InitResult
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function exactKeys(
	value: Record<string, unknown>,
	keys: readonly string[],
	boundary: string,
): void {
	const allowed = new Set(keys)
	for (const key of Object.keys(value)) {
		if (!allowed.has(key)) throw new Error(`${boundary}に未知のfield ${key}があります`)
	}
}

function safeIndex(value: unknown, courseCount: number): value is number {
	return Number.isSafeInteger(value) && (value as number) >= 0 && (value as number) < courseCount
}

function validateCells(
	value: unknown,
	courseCount: number,
	dayCount: number,
	maxPeriod: number,
): WasmGridCell[] {
	if (!Array.isArray(value) || value.length > dayCount * maxPeriod) {
		throw new Error('検索ワーカーのcell配列が不正です')
	}
	const coordinates = new Set<string>()
	for (const cellValue of value) {
		if (!isRecord(cellValue)) throw new Error('検索ワーカーのcellが不正です')
		exactKeys(cellValue, ['day', 'period', 'courses'], '検索ワーカーのcell')
		if (
			!Number.isSafeInteger(cellValue.day) ||
			(cellValue.day as number) < 0 ||
			(cellValue.day as number) >= dayCount ||
			!Number.isSafeInteger(cellValue.period) ||
			(cellValue.period as number) < 1 ||
			(cellValue.period as number) > maxPeriod ||
			!Array.isArray(cellValue.courses) ||
			cellValue.courses.length > courseCount ||
			!cellValue.courses.every((index) => safeIndex(index, courseCount)) ||
			new Set(cellValue.courses).size !== cellValue.courses.length
		) {
			throw new Error('検索ワーカーのcell内容が不正です')
		}
		const coordinate = `${cellValue.day}:${cellValue.period}`
		if (coordinates.has(coordinate)) throw new Error('検索ワーカーのcellが重複しています')
		coordinates.add(coordinate)
	}
	return value as WasmGridCell[]
}

/** Validate the complete WASM query boundary before resolving course indices. */
export function validateWasmGridResult(
	value: unknown,
	courseCount: number,
	dayCount: number,
	maxPeriod: number,
	hasTextQuery: boolean,
): WasmGridResult {
	if (!isRecord(value)) throw new Error('検索ワーカーのquery応答が不正です')
	exactKeys(
		value,
		['cells', 'total', 'scheduledCount', 'unscheduledCount', 'unscheduled', 'matches'],
		'検索ワーカーのquery応答',
	)
	const cells = validateCells(value.cells, courseCount, dayCount, maxPeriod)
	for (const key of ['total', 'scheduledCount', 'unscheduledCount'] as const) {
		if (
			!Number.isSafeInteger(value[key]) ||
			(value[key] as number) < 0 ||
			(value[key] as number) > courseCount
		) {
			throw new Error(`検索ワーカーの${key}が不正です`)
		}
	}
	if (
		!Array.isArray(value.unscheduled) ||
		!value.unscheduled.every((index) => safeIndex(index, courseCount)) ||
		new Set(value.unscheduled).size !== value.unscheduled.length ||
		value.unscheduled.length !== value.unscheduledCount
	) {
		throw new Error('検索ワーカーのunscheduled応答が不正です')
	}
	const scheduled = new Set(cells.flatMap((cell) => cell.courses))
	const union = new Set([...scheduled, ...value.unscheduled])
	if (
		scheduled.size !== value.scheduledCount ||
		union.size !== value.total ||
		(value.scheduledCount as number) + (value.unscheduledCount as number) < (value.total as number)
	) {
		throw new Error('検索ワーカーのquery件数が整合しません')
	}
	if (!Array.isArray(value.matches) || value.matches.length > courseCount) {
		throw new Error('検索ワーカーのmatch応答が不正です')
	}
	const matched = new Set<number>()
	for (const matchValue of value.matches) {
		if (!isRecord(matchValue)) throw new Error('検索ワーカーのmatchが不正です')
		exactKeys(matchValue, ['i', 'spans'], '検索ワーカーのmatch')
		if (
			!safeIndex(matchValue.i, courseCount) ||
			matched.has(matchValue.i) ||
			!union.has(matchValue.i) ||
			!Array.isArray(matchValue.spans) ||
			matchValue.spans.length === 0 ||
			matchValue.spans.length > 64
		) {
			throw new Error('検索ワーカーのmatch内容が不正です')
		}
		matched.add(matchValue.i)
		for (const spanValue of matchValue.spans) {
			if (!isRecord(spanValue)) throw new Error('検索ワーカーのspanが不正です')
			exactKeys(spanValue, ['f', 'o', 'l'], '検索ワーカーのspan')
			if (
				!Number.isSafeInteger(spanValue.f) ||
				(spanValue.f as number) < 0 ||
				(spanValue.f as number) >= FIELD_LABELS.length ||
				!Number.isSafeInteger(spanValue.o) ||
				(spanValue.o as number) < 0 ||
				(spanValue.o as number) > 32 * 1024 * 1024 ||
				!Number.isSafeInteger(spanValue.l) ||
				(spanValue.l as number) <= 0 ||
				(spanValue.l as number) > 4096
			) {
				throw new Error('検索ワーカーのspan内容が不正です')
			}
		}
	}
	if ((hasTextQuery && matched.size !== value.total) || (!hasTextQuery && matched.size !== 0)) {
		throw new Error('検索ワーカーのmatch件数がqueryと一致しません')
	}
	return value as unknown as WasmGridResult
}

function validCourseCode(value: unknown): value is string {
	if (
		typeof value !== 'string' ||
		value !== value.trim() ||
		value.length === 0 ||
		[...value].length > 128
	) {
		return false
	}
	for (const character of value) {
		const code = character.charCodeAt(0)
		if (character === '/' || character === '\\' || code < 0x20 || code === 0x7f) return false
	}
	return true
}

function validateTallies(value: unknown, name: string, maxCourses: number): Tally[] {
	if (!Array.isArray(value) || value.length > maxCourses) throw new Error(`${name}が不正です`)
	const keys = new Set<string>()
	for (const tallyValue of value) {
		if (!isRecord(tallyValue)) throw new Error(`${name}の項目が不正です`)
		exactKeys(tallyValue, ['key', 'credits', 'count'], name)
		if (
			typeof tallyValue.key !== 'string' ||
			tallyValue.key.length === 0 ||
			tallyValue.key.length > 4096 ||
			keys.has(tallyValue.key) ||
			typeof tallyValue.credits !== 'number' ||
			!Number.isFinite(tallyValue.credits) ||
			Math.abs(tallyValue.credits) > 1_000_000 ||
			!Number.isSafeInteger(tallyValue.count) ||
			(tallyValue.count as number) < 0 ||
			(tallyValue.count as number) > maxCourses
		) {
			throw new Error(`${name}の内容が不正です`)
		}
		keys.add(tallyValue.key)
	}
	return value as Tally[]
}

/** Validate the complete atomic course-code plan response. */
export function validateWasmPlanResult(
	value: unknown,
	courses: readonly Course[],
	dayCount: number,
	maxPeriod: number,
): WasmPlanResult {
	if (!isRecord(value)) throw new Error('検索ワーカーのplan応答が不正です')
	exactKeys(
		value,
		['validCodes', 'unknownCodes', 'cells', 'unscheduled', 'conflicts', 'credits'],
		'検索ワーカーのplan応答',
	)
	for (const key of ['validCodes', 'unknownCodes'] as const) {
		if (
			!Array.isArray(value[key]) ||
			value[key].length > 200 ||
			!value[key].every(validCourseCode) ||
			new Set(value[key]).size !== value[key].length
		) {
			throw new Error(`検索ワーカーの${key}が不正です`)
		}
	}
	const knownCodes = new Set(courses.map((course) => course.cd))
	if (
		!(value.validCodes as string[]).every((code) => knownCodes.has(code)) ||
		!(value.unknownCodes as string[]).every((code) => !knownCodes.has(code))
	) {
		throw new Error('検索ワーカーのplan code分類が不正です')
	}
	const validCodes = new Set(value.validCodes as string[])
	const cells = validateCells(value.cells, courses.length, dayCount, maxPeriod)
	if (
		!Array.isArray(value.unscheduled) ||
		value.unscheduled.length > courses.length ||
		!value.unscheduled.every((index) => safeIndex(index, courses.length)) ||
		new Set(value.unscheduled).size !== value.unscheduled.length
	) {
		throw new Error('検索ワーカーのplan未定科目が不正です')
	}
	for (const index of [...cells.flatMap((cell) => cell.courses), ...value.unscheduled]) {
		if (!validCodes.has(courses[index as number]?.cd ?? '')) {
			throw new Error('検索ワーカーのplan indexがvalid codeと一致しません')
		}
	}
	if (!Array.isArray(value.conflicts) || value.conflicts.length > dayCount * maxPeriod) {
		throw new Error('検索ワーカーのplan衝突が不正です')
	}
	for (const conflictValue of value.conflicts) {
		if (!isRecord(conflictValue)) throw new Error('検索ワーカーのplan衝突が不正です')
		exactKeys(conflictValue, ['day', 'period', 'courses'], '検索ワーカーのplan衝突')
		if (
			!Number.isSafeInteger(conflictValue.day) ||
			(conflictValue.day as number) < 0 ||
			(conflictValue.day as number) >= dayCount ||
			!Number.isSafeInteger(conflictValue.period) ||
			(conflictValue.period as number) < 1 ||
			(conflictValue.period as number) > maxPeriod ||
			!Array.isArray(conflictValue.courses) ||
			conflictValue.courses.length < 2 ||
			!conflictValue.courses.every((index) => safeIndex(index, courses.length)) ||
			new Set(conflictValue.courses).size !== conflictValue.courses.length
		) {
			throw new Error('検索ワーカーのplan衝突内容が不正です')
		}
	}
	if (!isRecord(value.credits)) throw new Error('検索ワーカーの単位集計が不正です')
	exactKeys(
		value.credits,
		['totalCredits', 'totalCourses', 'uncredited', 'byKubun', 'byBunrui', 'byNen'],
		'検索ワーカーの単位集計',
	)
	if (
		typeof value.credits.totalCredits !== 'number' ||
		!Number.isFinite(value.credits.totalCredits) ||
		Math.abs(value.credits.totalCredits) > 1_000_000 ||
		value.credits.totalCourses !== validCodes.size ||
		!Number.isSafeInteger(value.credits.uncredited) ||
		(value.credits.uncredited as number) < 0 ||
		(value.credits.uncredited as number) > validCodes.size
	) {
		throw new Error('検索ワーカーの単位集計内容が不正です')
	}
	validateTallies(value.credits.byKubun, '授業形態集計', validCodes.size)
	validateTallies(value.credits.byBunrui, '分類集計', validCodes.size)
	validateTallies(value.credits.byNen, '年次集計', validCodes.size)
	return value as unknown as WasmPlanResult
}

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
	readonly datasetId: string
	readonly manifest: DatasetManifest
	readonly days: readonly string[]
	readonly periods: readonly number[]

	private worker: Worker | null
	private restarting: Promise<void> | null = null
	private disposed = false
	private crashNextRequestForE2e = false
	private stallNextRequestForE2e = false
	private seq = 0
	private readonly pending = new Map<
		number,
		{
			resolve: (v: unknown) => void
			reject: (e: Error) => void
			timer: ReturnType<typeof setTimeout>
			type: string
		}
	>()

	private constructor(worker: Worker, init: InitResult, manifest: DatasetManifest) {
		this.worker = worker
		this.manifest = manifest
		this.courses = init.courses
		this.dicts = init.dicts
		this.generatedAt = init.generatedAt
		this.year = init.year
		this.datasetId = init.datasetId
		this.days = dayLabels(init.dayCount)
		this.periods = periodLabels(init.maxPeriod)
		this.attachWorker(worker)
	}

	private attachWorker(worker: Worker): void {
		worker.onmessage = (e: MessageEvent<unknown>) => {
			const reply = e.data
			if (!isWorkerReply(reply)) {
				this.failWorker(new Error('ワーカーから不正な応答を受信しました'), worker)
				return
			}
			const p = this.pending.get(reply.id)
			if (!p) return
			this.pending.delete(reply.id)
			clearTimeout(p.timer)
			if (reply.ok) p.resolve(reply.result)
			else p.reject(new Error(reply.error))
		}
		worker.onerror = (event) => {
			this.failWorker(new Error(event.message || '検索ワーカーが停止しました'), worker)
		}
		worker.onmessageerror = () => {
			this.failWorker(new Error('検索ワーカーの応答を読み取れませんでした'), worker)
		}
	}

	/**
	 * Resolve the stable manifest, verify its content-addressed data asset, and
	 * initialize a fresh worker.
	 *
	 * The selected data asset is fetched and SHA-256 verified on the main thread,
	 * then transferred to the worker without copying; parsing remains off-main-thread.
	 */
	static async create(): Promise<SyllabusEngine> {
		const worker = SyllabusEngine.spawnWorker()
		try {
			const manifest = await loadManifest('no-store')
			const init = await SyllabusEngine.load(worker, manifest)
			return new SyllabusEngine(worker, init, manifest)
		} catch (e) {
			worker.terminate()
			throw e
		}
	}

	private static spawnWorker(): Worker {
		return new Worker(new URL('./engine.worker.ts', import.meta.url), {
			type: 'module',
		})
	}

	/** Fetch and verify the manifest-selected data asset, then init the worker. */
	private static async load(worker: Worker, manifest: DatasetManifest): Promise<InitResult> {
		const buffer = await fetchVerifiedAsset(manifest, manifest.assets.data)
		return new Promise<InitResult>((resolve, reject) => {
			const timer = setTimeout(
				() => reject(new Error('データ初期化がタイムアウトしました')),
				15_000,
			)
			worker.onmessage = (e: MessageEvent<unknown>) => {
				const reply = e.data
				if (!isWorkerReply(reply)) return
				if (reply.id !== 0) return
				clearTimeout(timer)
				if (reply.ok) {
					try {
						resolve(validateInitResult(reply.result, manifest))
					} catch (error) {
						reject(error)
					}
				} else reject(new Error(reply.error))
			}
			worker.onerror = (e) => {
				clearTimeout(timer)
				reject(new Error(e.message || 'ワーカーエラー'))
			}
			// Transfer the buffer (second arg) — ownership moves to the worker, no copy.
			worker.postMessage(
				{
					id: 0,
					type: 'init',
					buffer,
					datasetId: manifest.datasetId,
					indexUrl: assetUrl(manifest, manifest.assets.index),
					indexAsset: manifest.assets.index,
				},
				[buffer],
			)
		})
	}

	private async ensureWorker(): Promise<void> {
		if (this.disposed) return Promise.reject(new Error('検索エンジンは終了済みです'))
		if (this.worker) return
		if (this.restarting) return this.restarting

		const worker = SyllabusEngine.spawnWorker()
		this.restarting = (async () => {
			try {
				const init = await SyllabusEngine.load(worker, this.manifest)
				if (
					init.courses.length !== this.courses.length ||
					init.courses.some((course, index) => course.cd !== this.courses[index]?.cd) ||
					JSON.stringify(init.dicts) !== JSON.stringify(this.dicts)
				) {
					throw new Error('再生成した検索ワーカーのdataset内容が一致しません')
				}
				if (this.disposed) {
					worker.terminate()
					throw new Error('検索エンジンは終了済みです')
				}
				this.worker = worker
				this.attachWorker(worker)
			} catch (error) {
				worker.terminate()
				throw error
			} finally {
				this.restarting = null
			}
		})()
		return this.restarting
	}

	/** Post a query keyed by a fresh id; resolves when the worker echoes it. */
	private async send(payload: Record<string, unknown>): Promise<unknown> {
		await this.ensureWorker()
		const worker = this.worker
		if (!worker) throw new Error('検索ワーカーを再生成できませんでした')
		const id = ++this.seq
		const type = typeof payload.type === 'string' ? payload.type : 'unknown'
		if (type === 'query') {
			for (const [pendingId, pending] of this.pending) {
				if (pending.type !== 'query') continue
				clearTimeout(pending.timer)
				pending.reject(new DOMException('新しい検索で取り消されました', 'AbortError'))
				this.pending.delete(pendingId)
			}
		}
		const stallForE2e =
			import.meta.env.DEV && import.meta.env.VITE_E2E === 'true' && this.stallNextRequestForE2e
		if (stallForE2e) this.stallNextRequestForE2e = false
		return new Promise((resolve, reject) => {
			const timer = setTimeout(
				() => {
					this.failWorker(new Error(`${type} がタイムアウトしました`), worker)
				},
				stallForE2e ? 1_000 : type === 'query' ? 10_000 : 15_000,
			)
			this.pending.set(id, { resolve, reject, timer, type })
			if (import.meta.env.DEV && this.crashNextRequestForE2e) {
				this.crashNextRequestForE2e = false
				worker.postMessage({ id, type: '__e2eCrash' })
			} else if (stallForE2e) {
			} else {
				worker.postMessage({ id, ...payload })
			}
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
	): Promise<{
		grid: Map<GridKey, Course[]>
		unscheduled: Course[]
		count: number
		scheduledCount: number
		unscheduledCount: number
		highlights: Highlights
		matchFields: Map<string, string[]>
	}> {
		const res = validateWasmGridResult(
			await this.send({
				type: 'query',
				semester,
				department,
				campus,
				query,
			}),
			this.courses.length,
			this.days.length,
			this.periods.at(-1) ?? 1,
			query.length > 0,
		)
		return {
			grid: assembleGrid(res.cells, this.courses, this.days, this.periods),
			unscheduled: res.unscheduled.flatMap((index) => {
				const course = this.courses[index]
				return course ? [course] : []
			}),
			count: res.total,
			scheduledCount: res.scheduledCount,
			unscheduledCount: res.unscheduledCount,
			highlights: resolveHighlights(res.matches, this.courses),
			matchFields: resolveMatchFields(res.matches, this.courses),
		}
	}

	async retrySearchIndex(): Promise<void> {
		await this.send({ type: 'retrySearchIndex' })
	}

	/** @internal Test-only hook eliminated from production builds. */
	debugCrashNextRequest(): void {
		if (!import.meta.env.DEV || import.meta.env.VITE_E2E !== 'true') {
			throw new Error('Worker crash hook is available only in E2E development mode')
		}
		this.crashNextRequestForE2e = true
	}

	/** @internal Test-only hook eliminated from production builds. */
	debugStallNextRequest(): void {
		if (!import.meta.env.DEV || import.meta.env.VITE_E2E !== 'true') {
			throw new Error('Worker timeout hook is available only in E2E development mode')
		}
		this.stallNextRequestForE2e = true
	}

	/** Validate stable course codes and return the complete plan atomically. */
	async plan(cds: readonly string[], semester: string): Promise<PlanResult> {
		const res = validateWasmPlanResult(
			await this.send({
				type: 'plan',
				cds: [...cds],
				semester,
			}),
			this.courses,
			this.days.length,
			this.periods.at(-1) ?? 1,
		)
		return {
			validCodes: res.validCodes,
			unknownCodes: res.unknownCodes,
			grid: assembleGrid(res.cells, this.courses, this.days, this.periods),
			unscheduled: res.unscheduled.flatMap((index) => {
				const course = this.courses[index]
				return course ? [course] : []
			}),
			conflicts: res.conflicts,
			credits: res.credits,
		}
	}

	dispose(): void {
		if (this.disposed) return
		this.disposed = true
		this.rejectAll(new Error('検索エンジンを終了しました'))
		const worker = this.worker
		this.worker = null
		if (worker) {
			worker.postMessage({ id: ++this.seq, type: 'dispose' })
			worker.terminate()
		}
	}

	private failWorker(error: Error, source: Worker): void {
		if (this.worker !== source) return
		source.terminate()
		this.worker = null
		this.rejectAll(error)
		if (!this.disposed) void this.ensureWorker().catch(() => undefined)
	}

	private rejectAll(error: Error): void {
		for (const pending of this.pending.values()) {
			clearTimeout(pending.timer)
			pending.reject(error)
		}
		this.pending.clear()
	}
}
