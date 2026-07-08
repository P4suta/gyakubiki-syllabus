import { defineConfig, devices } from '@playwright/test'

// E2E tests for the syllabus viewer, run against the KULAS-free dummy dataset.
//
// `global-setup.ts` synthesizes a production-scale dataset (a few thousand
// courses, fixed seed → deterministic) if one is not already present, so the
// suite exercises the real load and no test ever contacts KULAS.
export default defineConfig({
	testDir: './e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 1 : 0,
	reporter: 'list',
	globalSetup: './e2e/global-setup.ts',
	// The dataset is large; give the async (worker-backed) grid room to settle.
	timeout: 45_000,
	expect: { timeout: 10_000 },
	use: {
		baseURL: 'http://localhost:5173',
		trace: 'on-first-retry',
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: {
		command: 'bun run dev',
		url: 'http://localhost:5173',
		reuseExistingServer: !process.env.CI,
		timeout: 120_000,
	},
})
