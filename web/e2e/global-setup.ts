import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

// Ensure a production-scale, KULAS-free dataset exists before the suite runs.
// Deterministic (fixed seed in `gen-sample`), regenerated only when missing or
// small so local iterations stay fast. Runs the CLI directly — no network.

const ROOT = path.resolve(process.cwd(), '..')
const DATA = path.join(ROOT, 'web/public/data.json')

function courseCount(): number {
	try {
		const d = JSON.parse(fs.readFileSync(DATA, 'utf-8'))
		return Array.isArray(d.courses) ? d.courses.length : 0
	} catch {
		return 0
	}
}

export default function globalSetup(): void {
	if (courseCount() >= 1000) return

	const cli = (args: string[]) =>
		execFileSync('cargo', ['run', '--release', '-q', '-p', 'syllabus-cli', '--', ...args], {
			cwd: ROOT,
			stdio: 'inherit',
		})

	cli(['gen-sample'])
	cli([
		'convert',
		'dev-data/sample-raw.json',
		'--compact',
		'--details-dir',
		'dev-data/sample-details',
		'--details-out',
		'web/public/details',
		'-o',
		'web/public/data.json',
	])
}
