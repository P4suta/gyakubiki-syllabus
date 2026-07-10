/// <reference lib="webworker" />
//
// Off-main-thread home for the WASM core. Parsing `data.json` (1MB+) and
// marshaling every course view across the WASM boundary used to run on the main
// thread and dominated Total Blocking Time; here it no longer blocks input or
// paint. The main thread talks to this worker through the tiny request/response
// protocol below (see engine.ts).

import initWasm, { SyllabusEngine as WasmEngine } from '../wasm/syllabus.js'

/**
 * `init`: boot WASM and build the engine from the already-fetched `data.json`
 * bytes (transferred from the main thread, which owns the fetch so the document's
 * `<link rel=preload>` is consumed). `filterAndGrid`: one query round-trip.
 */
type Request =
	| { id: number; type: 'init'; buffer: ArrayBuffer }
	| {
			id: number
			type: 'filterAndGrid'
			semester: string
			department: string
			campus: string
			query: string
	  }

/** Course view-models + dictionaries + dataset metadata, sent once after init. */
interface InitResult {
	courses: unknown
	dicts: unknown
	generatedAt: string
	year: string
	hasSaturday: boolean
}

let engine: WasmEngine | null = null

async function handleInit(buffer: ArrayBuffer): Promise<InitResult> {
	await initWasm()

	// Decode the transferred bytes and parse — the expensive part (serde parse +
	// marshaling every course view) runs here, off the main thread.
	const text = new TextDecoder().decode(buffer)

	// from_json's thiserror message surfaces as a JS Error; let it propagate to
	// the main thread verbatim.
	engine = WasmEngine.fromJson(text)

	// Pull in the companion search index lazily, off the init path: it enables
	// ranked search with match highlights but must never gate first paint.
	// Queries that arrive before it loads fall back to an unranked substring scan
	// (handled in the core), so search keeps working meanwhile.
	void loadSearchIndex()

	return {
		courses: engine.allCourseViews(),
		dicts: engine.dicts(),
		generatedAt: engine.generatedAt(),
		year: engine.year(),
		hasSaturday: engine.hasSaturday(),
	}
}

/** Fetch and hand `search.idx` to the core. A missing/late index is non-fatal —
 *  the query falls back to the substring scan until it arrives. */
async function loadSearchIndex(): Promise<void> {
	try {
		const res = await fetch(`${import.meta.env.BASE_URL}search.idx`)
		if (!res.ok) return
		const bytes = new Uint8Array(await res.arrayBuffer())
		engine?.loadSearchIndex(bytes)
	} catch {
		// Leave search on the fallback path; ranking/highlights simply stay off.
	}
}

/** Filter, rank, and lay out in one hop — returns `{ cells, countUnique,
 *  highlights }` with cells already ordered best-first. */
function handleQuery(msg: Extract<Request, { type: 'filterAndGrid' }>): unknown {
	if (!engine) throw new Error('エンジンが初期化されていません')
	return engine.query(msg.semester, msg.department, msg.campus, msg.query)
}

self.onmessage = async (e: MessageEvent<Request>) => {
	const msg = e.data
	try {
		const result = msg.type === 'init' ? await handleInit(msg.buffer) : handleQuery(msg)
		self.postMessage({ id: msg.id, ok: true, result })
	} catch (err) {
		self.postMessage({
			id: msg.id,
			ok: false,
			error: err instanceof Error ? err.message : String(err),
		})
	}
}
