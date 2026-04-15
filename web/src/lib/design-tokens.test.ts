import fs from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

/**
 * Svelte コンポーネント内にハードコードされたデザイン値が残っていないことを検証する。
 * 新しいコンポーネント追加時は SVELTE_FILES に追加すること。
 */

const WEB_SRC = path.resolve(__dirname, '..')
const SVELTE_FILES = [
	'App.svelte',
	'components/CourseCard.svelte',
	'components/CourseModal.svelte',
	'components/Disclaimer.svelte',
	'components/FileLoader.svelte',
	'components/FilterBar.svelte',
	'components/Timetable.svelte',
	'components/TimetableCell.svelte',
]

/** style 属性内の inline CSS は検査対象外 (動的カラー等で必要なため) */
function stripInlineStyles(content: string): string {
	return content.replace(/style="[^"]*"/g, '')
}

/**
 * 各ルールは [正規表現, 説明, 許可パターン(省略可)] の三つ組。
 * 許可パターンにマッチした行はスキップされる。
 */
const RULES: [RegExp, string, RegExp?][] = [
	// --- 色 ---
	[
		/\[#[0-9a-fA-F]{3,8}\]/,
		'Tailwind arbitrary hex color (use design tokens like text-apple-text, bg-surface-page)',
	],
	[
		/(?:text|bg|border|ring)-gray-/,
		'Tailwind default gray palette (use apple-text, surface-*, overlay-* tokens)',
	],
	[
		/(?:text|bg|border|ring)-blue-/,
		'Tailwind default blue palette (use apple-blue token)',
	],
	[
		/(?:text|bg|border|ring)-amber-/,
		'Tailwind default amber palette (use design tokens)',
	],

	// --- フォントサイズ ---
	[
		/text-\[\d+px\]/,
		'Arbitrary font size (use text-micro, text-caption, text-sub, text-body, text-cta)',
	],

	// --- シャドウ ---
	[
		/shadow-\[/,
		'Arbitrary shadow value (use shadow-card-hover, shadow-card, shadow-modal)',
	],

	// --- イージング ---
	[
		/ease-\[cubic-bezier/,
		'Arbitrary easing (use ease-spring)',
	],
]

function findViolations(filePath: string): string[] {
	const raw = fs.readFileSync(filePath, 'utf-8')
	const content = stripInlineStyles(raw)
	const lines = content.split('\n')
	const violations: string[] = []

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i]
		for (const [pattern, message, allow] of RULES) {
			if (pattern.test(line)) {
				if (allow && allow.test(line)) continue
				const lineNum = i + 1
				violations.push(
					`  ${path.basename(filePath)}:${lineNum} — ${message}\n    ${line.trim()}`,
				)
			}
		}
	}
	return violations
}

describe('design token enforcement', () => {
	it('all Svelte files exist', () => {
		for (const file of SVELTE_FILES) {
			const full = path.join(WEB_SRC, file)
			expect(fs.existsSync(full), `Missing: ${file}`).toBe(true)
		}
	})

	for (const file of SVELTE_FILES) {
		it(`${file} has no hardcoded design values`, () => {
			const full = path.join(WEB_SRC, file)
			const violations = findViolations(full)
			if (violations.length > 0) {
				expect.fail(
					`Found ${violations.length} hardcoded value(s):\n${violations.join('\n')}`,
				)
			}
		})
	}

	it('app.css defines all required token categories', () => {
		const css = fs.readFileSync(path.join(WEB_SRC, 'app.css'), 'utf-8')

		// 色トークン
		expect(css).toContain('--color-apple-text:')
		expect(css).toContain('--color-apple-blue:')
		expect(css).toContain('--color-apple-blue-hover:')
		expect(css).toContain('--color-surface-primary:')
		expect(css).toContain('--color-surface-page:')
		expect(css).toContain('--color-overlay-subtle:')
		expect(css).toContain('--color-overlay-backdrop:')

		// フォントサイズトークン
		expect(css).toContain('--font-size-micro:')
		expect(css).toContain('--font-size-caption:')
		expect(css).toContain('--font-size-sub:')
		expect(css).toContain('--font-size-body:')
		expect(css).toContain('--font-size-cta:')

		// シャドウトークン
		expect(css).toContain('--shadow-card-hover:')
		expect(css).toContain('--shadow-card:')
		expect(css).toContain('--shadow-modal:')

		// イージング
		expect(css).toContain('--ease-spring:')

		// アニメーション
		expect(css).toContain('--animate-fade-in:')
		expect(css).toContain('--animate-spinner:')
	})

	it('index.html does not load external fonts', () => {
		const html = fs.readFileSync(
			path.resolve(WEB_SRC, '..', 'index.html'),
			'utf-8',
		)
		expect(html).not.toContain('fonts.googleapis.com')
		expect(html).not.toContain('fonts.gstatic.com')
	})
})
