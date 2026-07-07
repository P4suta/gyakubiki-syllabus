import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

export default defineConfig({
	// On GitHub Pages the app is served from a sub-path; the WASM asset URL
	// (resolved via import.meta.url) follows this base automatically.
	base: process.env.GITHUB_PAGES === 'true' ? '/gyakubiki-syllabus/' : '/',
	plugins: [svelte(), tailwindcss()],
	// Unit tests live in src/; the Playwright E2E specs in e2e/ run separately.
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}'],
	},
})
