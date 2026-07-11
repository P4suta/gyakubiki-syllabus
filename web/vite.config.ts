import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { svelteTesting } from '@testing-library/svelte/vite'
import { minify } from 'html-minifier-terser'
import Icons from 'unplugin-icons/vite'
import type { PluginOption } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'
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

// The whole app ships one small CSS file, and as a <link> it render-blocks a
// round-trip. Inline it into index.html and drop the asset.
const inlineCss = (): PluginOption => ({
	name: 'inline-css',
	apply: 'build',
	enforce: 'post',
	generateBundle(_options, bundle) {
		const html = bundle['index.html']
		if (!html || html.type !== 'asset') return
		let source = html.source.toString()
		for (const [name, chunk] of Object.entries(bundle)) {
			if (chunk.type !== 'asset' || !name.endsWith('.css')) continue
			const file = name.split('/').pop() ?? name
			const escaped = file.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
			const link = new RegExp(`<link[^>]+href="[^"]*${escaped}"[^>]*>`)
			if (!link.test(source)) continue
			source = source.replace(link, `<style>${chunk.source}</style>`)
			delete bundle[name]
		}
		html.source = source
	},
})

export default defineConfig({
	// On GitHub Pages the app is served from a sub-path; the WASM asset URL
	// (resolved via import.meta.url) follows this base automatically.
	base: process.env.GITHUB_PAGES === 'true' ? '/gyakubiki-syllabus/' : '/',
	// svelteTesting adds the jsdom `resolve.conditions` (browser build) and an
	// afterEach unmount so component tests don't leak between cases.
	// Icons are inlined from the Iconify `ic` set at build time (offline, tree-
	// shaken, zero runtime fetch): `import Foo from '~icons/ic/round-foo'`.
	plugins: [
		svelte(),
		tailwindcss(),
		svelteTesting(),
		Icons({ compiler: 'svelte' }),
		minifyHtml(),
		inlineCss(),
		// GitHub Pages serves everything with max-age=600 and the headers can't
		// be changed, so repeat visits re-fetch the hashed bundles. The SW gives
		// them real immutable caching (precache) and offline navigation, while
		// the daily-updated data stays NetworkFirst so it is never pinned stale.
		VitePWA({
			registerType: 'autoUpdate',
			injectRegister: 'inline', // a registerSW.js request would render-block
			manifest: false, // hand-written public/manifest.webmanifest
			workbox: {
				// The shell precaches (index.html revisions on every deploy; the SW
				// autoUpdates within Pages' 600s window), the data stays NetworkFirst.
				globPatterns: ['index.html', 'assets/*.{js,css,wasm}'],
				runtimeCaching: [
					{
						urlPattern: /\/(data\.json|search\.idx)$/,
						handler: 'NetworkFirst',
						options: { cacheName: 'data' },
					},
					{
						urlPattern: /\/details\/.*\.json$/,
						handler: 'NetworkFirst',
						options: { cacheName: 'details' },
					},
				],
			},
		}),
	],
	// Unit tests live in src/; the Playwright E2E specs in e2e/ run separately.
	test: {
		// Two projects: pure logic in `node` (fast, DOM-free) and component /
		// state-machine specs (`*.svelte.test.ts`) in `jsdom`. Keeping them apart
		// stops jsdom's globals from leaking into the pure tests.
		projects: [
			{
				extends: true,
				test: {
					name: 'node',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.ts'],
				},
			},
			{
				extends: true,
				test: {
					name: 'dom',
					environment: 'jsdom',
					include: ['src/**/*.svelte.{test,spec}.ts'],
					setupFiles: ['./vitest-setup.ts'],
				},
			},
		],
		coverage: {
			provider: 'v8',
			reporter: ['text-summary', 'text'],
			// Gate the pure, node-testable `lib/` modules plus the now-unit-covered
			// DOM helpers (gestures' swipeNavigate, the breakpoint store). Still
			// excluded (need a real worker/network → covered by E2E): the worker proxy
			// in engine.ts, engine.worker.ts, the fetch in details.ts, generated code,
			// and the constant table schedule.ts.
			include: ['src/lib/**/*.ts'],
			exclude: [
				'src/lib/**/*.{test,spec}.ts',
				'src/lib/*.generated.ts',
				'src/lib/engine.ts',
				'src/lib/engine.worker.ts',
				'src/lib/details.ts',
				'src/lib/schedule.ts',
			],
			thresholds: { lines: 90, functions: 90, branches: 85, statements: 90 },
		},
	},
})
