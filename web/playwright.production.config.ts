import { defineConfig, devices } from '@playwright/test'

const configuredUrl = process.env.PRODUCTION_URL
if (!configuredUrl) throw new Error('PRODUCTION_URL is required for production smoke tests')
const productionUrl = new URL(configuredUrl)
if (productionUrl.protocol !== 'https:' && productionUrl.hostname !== 'localhost') {
	throw new Error(`PRODUCTION_URL must use HTTPS: ${productionUrl}`)
}
if (!productionUrl.pathname.endsWith('/')) productionUrl.pathname += '/'

export default defineConfig({
	testDir: './e2e/production',
	fullyParallel: false,
	workers: 1,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	reporter: [['list'], ['junit', { outputFile: 'test-results/production-smoke.xml' }]],
	timeout: 120_000,
	expect: { timeout: 30_000 },
	use: {
		baseURL: productionUrl.toString(),
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
		video: 'retain-on-failure',
		serviceWorkers: 'allow',
	},
	projects: [
		{
			name: 'production-chromium',
			use: { ...devices['Desktop Chrome'] },
		},
	],
	outputDir: 'test-results/production-smoke',
})
