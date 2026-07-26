import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { chromium } from '@playwright/test'
import { preview } from 'vite'

// Use the exact Chromium revision installed for Playwright in every environment.
// System Edge can be controlled by desktop policies/background processes on
// Windows, which makes headless Lighthouse report a false NO_FCP.
const chromePath = chromium.executablePath()

const resultsDirectory = path.resolve('.lighthouseci')
mkdirSync(resultsDirectory, { recursive: true })
const url = 'http://127.0.0.1:4173/gyakubiki-syllabus/'
const categories = ['performance', 'accessibility', 'best-practices', 'seo'] as const
const thresholds: Record<(typeof categories)[number], number> = {
	performance: 0.9,
	accessibility: 1,
	'best-practices': 1,
	seo: 1,
}
const scores = new Map(categories.map((category) => [category, [] as number[]]))

const server = await preview({
	clearScreen: false,
	logLevel: 'silent',
	preview: { host: '127.0.0.1', port: 4173, strictPort: true },
})

try {
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
		if (exitCode !== 0) throw new Error(`Lighthouse run ${run} failed with exit code ${exitCode}`)

		const report = JSON.parse(readFileSync(reportPath, 'utf8')) as {
			categories: Record<string, { score: number | null }>
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
} finally {
	await server.close()
}
