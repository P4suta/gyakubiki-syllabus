import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

// Build a production-scale, fixed-seed dataset in a per-run temporary public
// directory. Tests never read or overwrite a developer's web/public.

export function cleanupE2ePublic(publicDir: string, tempRoot: string): void {
	const resolved = path.resolve(publicDir)
	const relative = path.relative(tempRoot, resolved)
	if (relative.startsWith('..') || path.isAbsolute(relative) || relative === '') {
		throw new Error(`Refusing to remove E2E directory outside ${tempRoot}: ${resolved}`)
	}
	fs.rmSync(resolved, { recursive: true, force: true })
}

export function prepareE2ePublic(publicDir: string, root: string): void {
	// Playwright evaluates its config in each worker. The stable manifest is
	// promoted last, so its presence proves the shared fixture is complete.
	if (fs.existsSync(path.join(publicDir, 'manifest.json'))) return

	const cli = (args: string[]) =>
		execFileSync(
			'cargo',
			['run', '--locked', '--release', '-q', '-p', 'syllabus-cli', '--', ...args],
			{
				cwd: root,
				stdio: ['ignore', 'ignore', 'inherit'],
			},
		)
	const seedDir = `${publicDir}-seed`
	const seedRaw = path.join(seedDir, 'sample-raw.json')
	const seedDetails = path.join(seedDir, 'details')

	try {
		cli(['gen-sample', '--count', '3000', '--out-raw', seedRaw, '--out-details', seedDetails])
		fs.mkdirSync(publicDir, { recursive: true })
		cli([
			'build-dataset',
			seedRaw,
			'--details-dir',
			seedDetails,
			'--output',
			publicDir,
			'--source-commit',
			'0000000000000000000000000000000000000000',
			'--allow-incomplete-details',
			'--generated-at',
			'2026-01-01T00:00:00+09:00',
		])
	} catch (error) {
		cleanupE2ePublic(publicDir, path.dirname(publicDir))
		throw error
	} finally {
		fs.rmSync(seedDir, { recursive: true, force: true })
	}
}

export default function globalSetup(): () => void {
	const publicDir = process.env.E2E_PUBLIC_DIR
	if (!publicDir) throw new Error('E2E_PUBLIC_DIR was not configured by playwright.config.ts')
	const tempRoot = process.env.E2E_TEMP_ROOT
	if (!tempRoot) throw new Error('E2E_TEMP_ROOT was not configured by playwright.config.ts')
	return () => cleanupE2ePublic(publicDir, tempRoot)
}
