import { defineConfig, devices } from '@playwright/test'

// E2E smoke tests for the syllabus viewer.
//
// Prerequisite: web/public/data.json + web/public/details/ must exist. Generate
// a KULAS-free dummy dataset first with `just dev-sample` (or `gen-sample` +
// `convert`). No test here ever contacts KULAS.
export default defineConfig({
	testDir: './e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 1 : 0,
	reporter: 'list',
	use: {
		baseURL: 'http://localhost:5173',
		trace: 'on-first-retry',
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: {
		command: 'bun run dev',
		url: 'http://localhost:5173',
		reuseExistingServer: true,
		timeout: 60_000,
	},
})
