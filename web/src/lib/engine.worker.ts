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

	return {
		courses: engine.allCourseViews(),
		dicts: engine.dicts(),
		generatedAt: engine.generatedAt(),
		year: engine.year(),
		hasSaturday: engine.hasSaturday(),
	}
}

/** Filter then lay out in one hop — the index array never crosses the boundary. */
function handleQuery(msg: Extract<Request, { type: 'filterAndGrid' }>): unknown {
	if (!engine) throw new Error('エンジンが初期化されていません')
	const indices = engine.filter(msg.semester, msg.department, msg.campus, msg.query)
	return engine.grid(indices, msg.semester)
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
