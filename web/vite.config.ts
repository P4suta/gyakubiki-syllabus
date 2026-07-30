import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { svelteTesting } from '@testing-library/svelte/vite'
import { minify } from 'html-minifier-terser'
import Icons from 'unplugin-icons/vite'
import type { PluginOption } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'
import { defineConfig } from 'vitest/config'

const appPackage = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')) as {
	version?: unknown
}
const appVersion = typeof appPackage.version === 'string' ? appPackage.version : 'unknown'
const appCommit = process.env.GITHUB_SHA ?? process.env.APP_COMMIT ?? 'local'

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

// GitHub Pages cannot set response headers, so the production artifact carries
// the strictest CSP that a meta element can enforce. Inline structured-data
// scripts are individually hashed after HTML minification; executable code,
// workers, WASM, and data may otherwise load only from this deployment.
const injectContentSecurityPolicy = (): PluginOption => ({
	name: 'inject-content-security-policy',
	apply: 'build',
	transformIndexHtml: {
		order: 'post',
		handler(html: string) {
			const inlineScriptHashes = Array.from(
				html.matchAll(/<script\b(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/giu),
				(match) =>
					`'sha256-${createHash('sha256')
						.update(match[1] ?? '')
						.digest('base64')}'`,
			)
			const policy = [
				"default-src 'self'",
				"base-uri 'self'",
				"object-src 'none'",
				"frame-src 'none'",
				"form-action 'none'",
				`script-src 'self' 'wasm-unsafe-eval' ${inlineScriptHashes.join(' ')}`.trim(),
				"style-src 'self' 'unsafe-inline'",
				"img-src 'self' data:",
				"font-src 'self'",
				"connect-src 'self'",
				"worker-src 'self'",
				"manifest-src 'self'",
			].join('; ')
			return html.replace(
				/<head([^>]*)>/iu,
				(_match, attributes: string) =>
					`<head${attributes}><meta http-equiv="Content-Security-Policy" content="${policy}">`,
			)
		},
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
		if (html?.type !== 'asset') return
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
	publicDir: process.env.E2E_PUBLIC_DIR || 'public',
	define: {
		__APP_VERSION__: JSON.stringify(appVersion),
		__APP_COMMIT__: JSON.stringify(appCommit),
	},
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
		injectContentSecurityPolicy(),
		inlineCss(),
		// GitHub Pages serves everything with max-age=600 and the headers can't
		// be changed, so repeat visits re-fetch the hashed bundles. The SW gives
		// them real immutable caching (precache) and offline navigation. The sole
		// stable dataset manifest is NetworkFirst; its content-addressed assets
		// are immutable CacheFirst entries.
		VitePWA({
			registerType: 'prompt',
			injectRegister: null,
			manifest: false, // hand-written public/manifest.webmanifest
			workbox: {
				// The shell precaches (index.html revisions on every deploy; the SW
				// autoUpdates within Pages' 600s window). Runtime policy below keeps
				// only the manifest mutable.
				globPatterns: ['index.html', 'assets/*.{js,css,wasm}'],
				globIgnores: ['assets/brotli_dec_wasm_bg-*.wasm'],
				runtimeCaching: [
					{
						urlPattern: /\/manifest\.json$/,
						handler: 'NetworkFirst',
						options: {
							cacheName: 'dataset-manifest',
							networkTimeoutSeconds: 3,
							expiration: { maxEntries: 2, maxAgeSeconds: 14 * 24 * 60 * 60 },
						},
					},
					{
						urlPattern:
							/\/datasets\/[^/]+\/(data\.[a-f0-9]+\.json|search\.[a-f0-9]+\.idx\.br|details\.[a-f0-9]+\.json)$/,
						handler: 'CacheFirst',
						options: {
							cacheName: 'dataset-assets',
							expiration: { maxEntries: 6, maxAgeSeconds: 180 * 24 * 60 * 60 },
						},
					},
					{
						urlPattern: /\/datasets\/[^/]+\/details\/[a-f0-9.]+\.json$/,
						handler: 'CacheFirst',
						options: {
							cacheName: 'dataset-details',
							expiration: { maxEntries: 8_000, maxAgeSeconds: 180 * 24 * 60 * 60 },
						},
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
				'src/lib/dataset.ts',
				'src/lib/details.ts',
				'src/lib/schedule.ts',
				'src/lib/worker-protocol.ts',
			],
			thresholds: { lines: 90, functions: 90, branches: 85, statements: 90 },
		},
	},
})
