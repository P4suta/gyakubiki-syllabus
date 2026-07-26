import fs from 'node:fs'
import path from 'node:path'
import brotliPromise from 'brotli-dec-wasm'
import initWasm, { SyllabusEngine } from '../src/wasm/syllabus.js'

const root = path.resolve(process.argv[2] ?? 'public')
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'manifest.json'), 'utf8'))
const assetPath = (asset: { path: string }) => path.join(root, manifest.basePath, asset.path)

const wasm = await initWasm({
	module_or_path: fs.readFileSync(path.resolve('src/wasm/syllabus_bg.wasm')),
})
const data = fs.readFileSync(assetPath(manifest.assets.data), 'utf8')
const engine = SyllabusEngine.fromJson(data)

const brotli = await brotliPromise
const compressed = fs.readFileSync(assetPath(manifest.assets.index))
const index = brotli.decompress(compressed)
if (index.byteLength !== manifest.assets.index.decodedBytes) {
	throw new Error('decoded index size mismatch')
}
engine.loadSearchIndex(index)

const queries = ['学', '教育', '情報', '英語', '地域', '実習', '演習', '心理', '健康', '社会']
for (const query of queries) engine.query('all', 'all', 'all', query)

const durations: number[] = []
const durationsByQuery = new Map(queries.map((query) => [query, [] as number[]]))
for (let round = 0; round < 10; round += 1) {
	for (const query of queries) {
		const start = performance.now()
		engine.query('all', 'all', 'all', query)
		const duration = performance.now() - start
		durations.push(duration)
		durationsByQuery.get(query)?.push(duration)
	}
}

// Validate the one-time init payload too, but keep its 3,928-object JS graph
// outside the synchronous WASM query benchmark. In the app it is posted to the
// main thread and released before the user can submit a query.
let snapshot = engine.initSnapshot()
if (snapshot.datasetId !== manifest.datasetId) throw new Error('dataset identity mismatch')
if (snapshot.courses.length !== manifest.counts.courses) {
	throw new Error('course count mismatch')
}
snapshot = null
Bun.gc(true)

durations.sort((a, b) => a - b)
const percentile = (p: number) =>
	durations[Math.min(durations.length - 1, Math.ceil(durations.length * p) - 1)]
const queryP95Ms = Object.fromEntries(
	[...durationsByQuery].map(([query, values]) => {
		values.sort((a, b) => a - b)
		return [query, values[Math.min(values.length - 1, Math.ceil(values.length * 0.95) - 1)]]
	}),
)
const metrics = {
	queryP50Ms: percentile(0.5),
	queryP95Ms: percentile(0.95),
	queryMaxMs: durations.at(-1) ?? 0,
	queryP95MsByQuery: queryP95Ms,
	workerHeapBytes: wasm.memory.buffer.byteLength,
	decodedIndexBytes: index.byteLength,
}
const budgets = {
	queryP95Ms: 50,
	workerHeapBytes: 64 * 1024 * 1024,
}
console.log(JSON.stringify({ metrics, budgets }, null, 2))
engine.free()

if (metrics.queryP95Ms > budgets.queryP95Ms) {
	throw new Error(`query p95 ${metrics.queryP95Ms.toFixed(2)}ms exceeds 50ms`)
}
if (metrics.workerHeapBytes > budgets.workerHeapBytes) {
	throw new Error(`Worker heap ${metrics.workerHeapBytes} exceeds 64MiB`)
}
