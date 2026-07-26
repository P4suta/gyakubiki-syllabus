import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { chromium } from '@playwright/test'
import { preview } from 'vite'

// Use the exact Chromium revision installed for Playwright in every environment.
// System Edge can be controlled by desktop policies/background processes on
// Windows, which makes headless Lighthouse report a false NO_FCP.
const chromePath = chromium.executablePath()

const resultsDirectory = path.resolve('.lighthouseci')
// Never accept a report left by an earlier run after a launch/navigation
// failure. This directory contains generated audit evidence only.
rmSync(resultsDirectory, { recursive: true, force: true })
mkdirSync(resultsDirectory, { recursive: true })
const pagesBase = '/gyakubiki-syllabus/'
const url = `http://127.0.0.1:4173${pagesBase}`
const categories = ['performance', 'accessibility', 'best-practices', 'seo'] as const
const thresholds: Record<(typeof categories)[number], number> = {
	performance: 0.9,
	accessibility: 1,
	'best-practices': 1,
	seo: 1,
}
const scores = new Map(categories.map((category) => [category, [] as number[]]))

const server = await preview({
	// The artifact was built with GITHUB_PAGES=true. Vite reloads its config for
	// preview, so override the base explicitly instead of depending on that
	// build-only environment variable.
	base: pagesBase,
	clearScreen: false,
	logLevel: 'silent',
	preview: { host: '127.0.0.1', port: 4173, strictPort: true },
})

try {
	const htmlResponse = await fetch(url)
	if (!htmlResponse.ok) throw new Error(`Preview HTML returned HTTP ${htmlResponse.status}`)
	const html = await htmlResponse.text()
	const scriptPath = html.match(/<script[^>]+type="module"[^>]+src="([^"]+)"/u)?.[1]
	if (!scriptPath) throw new Error('Production HTML has no module script')
	const scriptResponse = await fetch(new URL(scriptPath, url))
	const scriptType = scriptResponse.headers.get('content-type') ?? ''
	if (!scriptResponse.ok || !/javascript/u.test(scriptType)) {
		throw new Error(
			`Production module returned HTTP ${scriptResponse.status} with Content-Type ${scriptType || '(missing)'}`,
		)
	}

	for (let run = 1; run <= 3; run += 1) {
		const reportPath = path.join(resultsDirectory, `report-${run}.json`)
		const lighthouse = Bun.spawn(
			[
				process.execPath,
				'run',
				'lighthouse',
				url,
				'--quiet',
				'--output=json',
				`--output-path=${reportPath}`,
				`--only-categories=${categories.join(',')}`,
				'--chrome-flags=--headless=new --no-sandbox --disable-background-timer-throttling --disable-renderer-backgrounding',
			],
			{
				env: { ...Bun.env, CHROME_PATH: chromePath },
				stdin: 'inherit',
				stdout: 'inherit',
				stderr: 'inherit',
			},
		)
		const exitCode = await lighthouse.exited
		const report = existsSync(reportPath)
			? (JSON.parse(readFileSync(reportPath, 'utf8')) as {
					categories: Record<string, { score: number | null }>
					runtimeError?: { code?: string; message?: string }
				})
			: null
		if (!report) {
			throw new Error(`Lighthouse run ${run} failed with exit code ${exitCode} and no report`)
		}
		if (report.runtimeError) {
			throw new Error(
				`Lighthouse run ${run} failed: ${report.runtimeError.code ?? 'runtime error'}: ${report.runtimeError.message ?? 'no message'}`,
			)
		}
		if (exitCode !== 0) {
			// Chrome can finish a valid audit yet fail to remove its temporary
			// profile on Windows (EPERM). The report is authoritative once it
			// exists and has no Lighthouse runtime error.
			console.warn(`Lighthouse run ${run} produced a valid report but exited ${exitCode}`)
		}
		for (const category of categories) {
			const score = report.categories[category]?.score
			if (typeof score !== 'number') throw new Error(`Lighthouse omitted ${category}`)
			scores.get(category)?.push(score)
		}
	}

	const metrics = Object.fromEntries(
		categories.map((category) => {
			const values = scores.get(category)?.sort((a, b) => a - b) ?? []
			return [category, { runs: values, median: values[1] }]
		}),
	)
	writeFileSync(
		path.join(resultsDirectory, 'summary.json'),
		`${JSON.stringify({ metrics, thresholds }, null, 2)}\n`,
	)
	console.log(JSON.stringify({ metrics, thresholds }, null, 2))

	for (const category of categories) {
		const median = metrics[category].median
		if (median < thresholds[category]) {
			throw new Error(
				`Lighthouse ${category} median ${Math.round(median * 100)} is below ${Math.round(thresholds[category] * 100)}`,
			)
		}
	}
} catch (reason) {
	const failure =
		reason instanceof Error
			? { name: reason.name, message: reason.message, stack: reason.stack }
			: { name: 'Error', message: String(reason) }
	writeFileSync(
		path.join(resultsDirectory, 'failure.json'),
		`${JSON.stringify(failure, null, 2)}\n`,
	)
	throw reason
} finally {
	await server.close()
}
