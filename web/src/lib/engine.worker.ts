/// <reference lib="webworker" />
//
// Off-main-thread home for the WASM core. Parsing `data.json` (1MB+) and
// marshaling every course view across the WASM boundary used to run on the main
// thread and dominated Total Blocking Time; here it no longer blocks input or
// paint. The main thread talks to this worker through the tiny request/response
// protocol below (see engine.ts).

import initWasm, { SyllabusEngine as WasmEngine } from '../wasm/syllabus.js'
import { type DatasetAsset, readBoundedResponse } from './dataset'
import { isWorkerRequest, type WorkerRequest } from './worker-protocol'

/** Course view-models + dictionaries + dataset metadata, sent once after init. */
interface InitResult {
	courses: unknown
	dicts: unknown
	generatedAt: string
	year: string
	datasetId: string
	dayCount: number
	maxPeriod: number
	searchStatus: SearchStatus
}

let engine: WasmEngine | null = null
let indexUrl = ''
let indexAsset: DatasetAsset | null = null
let searchStatus: SearchStatus = 'loading'
let searchLoad: Promise<void> | null = null
let searchError = ''
let latestQueryId = 0
let indexAbort: AbortController | null = null

type SearchStatus = 'loading' | 'ready' | 'error'

async function handleInit(
	buffer: ArrayBuffer,
	datasetId: string,
	searchIndexUrl: string,
	searchIndexAsset: DatasetAsset,
): Promise<InitResult> {
	await initWasm()

	// Decode the transferred bytes and parse — the expensive part (serde parse +
	// marshaling every course view) runs here, off the main thread.
	const text = new TextDecoder().decode(buffer)

	// from_json's thiserror message surfaces as a JS Error; let it propagate to
	// the main thread verbatim.
	engine = WasmEngine.fromJson(text)
	const snapshot = engine.initSnapshot() as Omit<InitResult, 'searchStatus'>
	if (snapshot.datasetId !== datasetId) {
		engine = null
		throw new Error('manifestとdata assetのdataset IDが一致しません')
	}
	indexUrl = searchIndexUrl
	indexAsset = searchIndexAsset
	searchStatus = 'loading'
	searchError = ''
	indexAbort?.abort()

	// Pull in the companion search index lazily, off the init path: it enables
	// ranked search with match highlights but must never gate first paint.
	// Text queries await this promise without blocking initial filter/grid paint.
	searchLoad = loadSearchIndex()
	void searchLoad.catch(() => undefined)

	return { ...snapshot, searchStatus }
}

/** Fetch and hand the verified full-text index to the core. Initial grid paint
 *  remains independent, while text queries await this task explicitly. */
async function loadSearchIndex(): Promise<void> {
	const controller = new AbortController()
	indexAbort?.abort()
	indexAbort = controller
	try {
		if (!engine || !indexAsset || !indexUrl) throw new Error('検索index情報がありません')
		searchStatus = 'loading'
		searchError = ''
		const res = await fetch(indexUrl, { signal: controller.signal })
		if (!res.ok) throw new Error(`検索indexの取得に失敗しました (HTTP ${res.status})`)
		if (!indexAsset.decodedBytes || !indexAsset.decodedSha256) {
			throw new Error('検索indexの展開後identityがありません')
		}
		const buffer = await readBoundedResponse(
			res,
			Math.max(indexAsset.bytes, indexAsset.decodedBytes),
			'検索index',
		)
		let bytes: Uint8Array
		if (buffer.byteLength === indexAsset.bytes) {
			const actual = await sha256Hex(buffer)
			if (actual !== indexAsset.sha256.toLowerCase()) {
				throw new Error('検索indexの圧縮hashが一致しません')
			}
			const { default: brotliPromise } = await import('brotli-dec-wasm')
			const brotli = await brotliPromise
			bytes = decompressBrotliBounded(brotli, new Uint8Array(buffer), indexAsset.decodedBytes)
		} else if (buffer.byteLength === indexAsset.decodedBytes) {
			// Some HTTP servers transparently decode a `.br` response. Accept it
			// only after verifying the separately committed decoded identity.
			bytes = new Uint8Array(buffer)
		} else {
			throw new Error(
				`検索indexのサイズが一致しません (expected ${indexAsset.bytes} or ${indexAsset.decodedBytes}, got ${buffer.byteLength})`,
			)
		}
		if (bytes.byteLength !== indexAsset.decodedBytes) {
			throw new Error('検索indexの展開後サイズが一致しません')
		}
		if (
			(await sha256Hex(Uint8Array.from(bytes).buffer)) !== indexAsset.decodedSha256.toLowerCase()
		) {
			throw new Error('検索indexの展開後hashが一致しません')
		}
		engine?.loadSearchIndex(bytes)
		searchStatus = 'ready'
	} catch (error) {
		searchStatus = 'error'
		searchError = error instanceof Error ? error.message : String(error)
		throw error
	} finally {
		if (indexAbort === controller) indexAbort = null
	}
}

function decompressBrotliBounded(
	brotli: Awaited<typeof import('brotli-dec-wasm')['default']>,
	input: Uint8Array,
	expectedBytes: number,
): Uint8Array {
	const output = new Uint8Array(expectedBytes)
	const stream = new brotli.BrotliDecStream()
	let inputOffset = 0
	let outputOffset = 0
	try {
		while (true) {
			const remaining = expectedBytes - outputOffset
			const result = stream.decompress(
				input.subarray(inputOffset),
				Math.min(64 * 1024, Math.max(1, remaining)),
			)
			try {
				const chunk = result.buf
				if (chunk.byteLength > remaining) {
					throw new Error('検索indexの展開後サイズが宣言値を超えています')
				}
				output.set(chunk, outputOffset)
				outputOffset += chunk.byteLength
				inputOffset += result.input_offset
				if (result.code === brotli.BrotliStreamResultCode.ResultSuccess) {
					if (inputOffset !== input.byteLength || outputOffset !== expectedBytes) {
						throw new Error('検索indexのBrotli stream長が一致しません')
					}
					return output
				}
				if (
					result.code === brotli.BrotliStreamResultCode.NeedsMoreInput &&
					inputOffset >= input.byteLength
				) {
					throw new Error('検索indexのBrotli streamが途中で終了しています')
				}
				if (result.input_offset === 0 && chunk.byteLength === 0) {
					throw new Error('検索indexのBrotli展開が進行しません')
				}
			} finally {
				result.free()
			}
		}
	} finally {
		stream.free()
	}
}

async function sha256Hex(bytes: ArrayBuffer): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', bytes)
	return Array.from(new Uint8Array(digest), (value) => value.toString(16).padStart(2, '0')).join('')
}

/** Dispatch a non-init request against the ready engine. */
async function handle(msg: Exclude<WorkerRequest, { type: 'init' }>): Promise<unknown> {
	if (!engine) throw new Error('エンジンが初期化されていません')
	switch (msg.type) {
		case 'query':
			if (msg.query && searchStatus === 'loading' && searchLoad) await searchLoad
			if (msg.query && searchStatus !== 'ready') {
				throw new Error(searchError || '全文検索indexを読み込めませんでした。再試行してください')
			}
			// Filter, rank, and lay out in one hop — cells come back best-first.
			return engine.query(
				msg.semester,
				msg.department,
				msg.campus,
				msg.query.normalize('NFKC').toLowerCase(),
			)
		case 'retrySearchIndex':
			searchLoad = loadSearchIndex()
			void searchLoad.catch(() => undefined)
			return { searchStatus: 'loading' }
		case 'plan':
			return engine.plan(msg.cds, msg.semester)
		case 'dispose':
			indexAbort?.abort()
			indexAbort = null
			engine.free()
			engine = null
			self.close()
			return null
	}
}

self.onmessage = async (e: MessageEvent<unknown>) => {
	const msg = e.data
	if (
		import.meta.env.DEV &&
		import.meta.env.VITE_E2E === 'true' &&
		typeof msg === 'object' &&
		msg !== null &&
		(msg as Record<string, unknown>).type === '__e2eCrash'
	) {
		setTimeout(() => {
			throw new Error('E2E worker crash')
		}, 0)
		return
	}
	if (!isWorkerRequest(msg)) {
		if (
			typeof msg === 'object' &&
			msg !== null &&
			Number.isSafeInteger((msg as Record<string, unknown>).id)
		) {
			self.postMessage({
				id: (msg as Record<string, unknown>).id,
				ok: false,
				error: 'ワーカー要求の形式が不正です',
				retryable: false,
			})
		}
		return
	}
	if (msg.type === 'query') latestQueryId = msg.id
	try {
		const result =
			msg.type === 'init'
				? await handleInit(msg.buffer, msg.datasetId, msg.indexUrl, msg.indexAsset)
				: await handle(msg)
		if (msg.type === 'query' && msg.id !== latestQueryId) return
		self.postMessage({ id: msg.id, ok: true, result })
	} catch (err) {
		if (msg.type === 'query' && msg.id !== latestQueryId) return
		self.postMessage({
			id: msg.id,
			ok: false,
			error: err instanceof Error ? err.message : String(err),
			retryable: msg.type !== 'dispose',
		})
	}
}
