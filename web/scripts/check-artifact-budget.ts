import fs from 'node:fs'
import path from 'node:path'
import { gzipSync } from 'node:zlib'

const root = path.resolve(process.argv[2] ?? 'dist')

function files(directory: string): string[] {
	return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
		const target = path.join(directory, entry.name)
		return entry.isDirectory() ? files(target) : [target]
	})
}

function gzipBytes(file: string): number {
	return gzipSync(fs.readFileSync(file), { level: 9 }).byteLength
}

const all = files(root)
const js = all.filter((file) => file.endsWith('.js'))
const wasm = all.filter((file) => file.endsWith('.wasm'))
const html = all.filter((file) => file.endsWith('.html'))
const stableManifest = path.join(root, 'manifest.json')
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'manifest.json'), 'utf8'))
const activeIndex = path.join(root, manifest.basePath, manifest.assets.index.path)
const appJs = js.filter((file) => /^index-[a-zA-Z0-9_-]+\.js$/.test(path.basename(file)))
const appWasm = wasm.filter((file) => /syllabus_bg-[a-zA-Z0-9_-]+\.wasm$/.test(file))

const metrics = {
	appJsGzip: appJs.reduce((sum, file) => sum + gzipBytes(file), 0),
	wasmGzip: appWasm.reduce((sum, file) => sum + gzipBytes(file), 0),
	searchIndexGzip: gzipBytes(activeIndex),
	initialInteractiveGzip: [stableManifest, ...html, ...appJs, ...appWasm].reduce(
		(sum, file) => sum + gzipBytes(file),
		0,
	),
}

const budgets = {
	appJsGzip: 55 * 1024,
	wasmGzip: 115 * 1024,
	searchIndexGzip: 3 * 1024 * 1024,
	initialInteractiveGzip: 300 * 1024,
}

console.log(JSON.stringify({ metrics, budgets }, null, 2))
const failures = Object.entries(budgets).filter(
	([name, budget]) => metrics[name as keyof typeof metrics] > budget,
)
if (failures.length > 0) {
	for (const [name, budget] of failures) {
		console.error(`${name}: ${metrics[name as keyof typeof metrics]} > ${budget}`)
	}
	process.exit(1)
}
