import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { minify } from 'html-minifier-terser'
import type { PluginOption } from 'vite'
import { defineConfig } from 'vitest/config'

// Vite minifies JS/CSS but leaves index.html untouched; minify it too on build.
const minifyHtml = (): PluginOption => ({
	name: 'minify-html',
	apply: 'build',
	transformIndexHtml: {
		order: 'post',
		handler: (html: string) =>
			minify(html, {
				collapseWhitespace: true,
				removeComments: true,
				minifyCSS: true,
				minifyJS: true,
			}),
	},
})

export default defineConfig({
	// On GitHub Pages the app is served from a sub-path; the WASM asset URL
	// (resolved via import.meta.url) follows this base automatically.
	base: process.env.GITHUB_PAGES === 'true' ? '/gyakubiki-syllabus/' : '/',
	plugins: [svelte(), tailwindcss(), minifyHtml()],
	// Unit tests live in src/; the Playwright E2E specs in e2e/ run separately.
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}'],
	},
})
