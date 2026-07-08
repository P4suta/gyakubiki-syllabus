import fs from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'
import { COLORS } from './colors'
import { contrastRatio } from './contrast'

/**
 * Assert no hardcoded design values remain in the Svelte components.
 * Add new components to SVELTE_FILES.
 */

const WEB_SRC = path.resolve(__dirname, '..')
const SVELTE_FILES = [
	'App.svelte',
	'components/CourseCard.svelte',
	'components/CourseModal.svelte',
	'components/Disclaimer.svelte',
	'components/EvalChart.svelte',
	'components/FilterBar.svelte',
	'components/SearchBar.svelte',
	'components/Timetable.svelte',
	'components/TimetableCell.svelte',
]

/** Inline CSS in `style` attributes is exempt (needed for dynamic colors). */
function stripInlineStyles(content: string): string {
	return content.replace(/style="[^"]*"/g, '')
}

/**
 * Each rule is [regex, description, allow-pattern?]. Lines matching the
 * allow-pattern are skipped.
 */
const RULES: [RegExp, string, RegExp?][] = [
	// --- Colors ---
	[
		/\[#[0-9a-fA-F]{3,8}\]/,
		'Tailwind arbitrary hex color (use design tokens like text-apple-text, bg-surface-page)',
	],
	[
		/(?:text|bg|border|ring)-gray-/,
		'Tailwind default gray palette (use apple-text, surface-*, overlay-* tokens)',
	],
	[/(?:text|bg|border|ring)-blue-/, 'Tailwind default blue palette (use apple-blue token)'],
	[/(?:text|bg|border|ring)-amber-/, 'Tailwind default amber palette (use design tokens)'],

	// --- Font sizes ---
	[
		/text-\[\d+px\]/,
		'Arbitrary font size (use text-micro, text-caption, text-sub, text-body, text-cta)',
	],

	// --- Shadows ---
	[/shadow-\[/, 'Arbitrary shadow value (use shadow-card-hover, shadow-card, shadow-modal)'],

	// --- Easing ---
	[/ease-\[cubic-bezier/, 'Arbitrary easing (use ease-spring)'],
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
				if (allow?.test(line)) continue
				const lineNum = i + 1
				violations.push(`  ${path.basename(filePath)}:${lineNum} — ${message}\n    ${line.trim()}`)
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
				expect.fail(`Found ${violations.length} hardcoded value(s):\n${violations.join('\n')}`)
			}
		})
	}

	it('app.css defines all required token categories', () => {
		const css = fs.readFileSync(path.join(WEB_SRC, 'app.css'), 'utf-8')

		// Color tokens
		expect(css).toContain('--color-apple-text:')
		expect(css).toContain('--color-apple-blue:')
		expect(css).toContain('--color-apple-blue-hover:')
		expect(css).toContain('--color-apple-text-secondary:')
		expect(css).toContain('--color-apple-text-tertiary:')
		expect(css).toContain('--color-surface-primary:')
		expect(css).toContain('--color-surface-page:')
		expect(css).toContain('--color-overlay-subtle:')
		expect(css).toContain('--color-overlay-backdrop:')

		// Font-size tokens
		expect(css).toContain('--font-size-micro:')
		expect(css).toContain('--font-size-caption:')
		expect(css).toContain('--font-size-sub:')
		expect(css).toContain('--font-size-body:')
		expect(css).toContain('--font-size-cta:')

		// Shadow tokens
		expect(css).toContain('--shadow-card-hover:')
		expect(css).toContain('--shadow-card:')
		expect(css).toContain('--shadow-modal:')

		// Easing
		expect(css).toContain('--ease-spring:')

		// Animations
		expect(css).toContain('--animate-fade-in:')
		expect(css).toContain('--animate-spinner:')
		expect(css).toContain('--animate-dialog-in:')
	})

	it('muted text tokens clear WCAG AA on every surface they render on', () => {
		const css = fs.readFileSync(path.join(WEB_SRC, 'app.css'), 'utf-8')
		const tokenValue = (name: string) => {
			const m = css.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})`))
			if (!m) throw new Error(`token ${name} not found in app.css`)
			return m[1]
		}
		// Every background muted text can land on: white, the page, the subtle
		// overlay badge, and any of the 10 course tiles.
		const surfaces = [
			'#ffffff',
			'#f5f5f7', // --color-surface-page
			'#efeff1', // --color-overlay-subtle composited over the page
			...COLORS.map((c) => c.bg),
		]
		for (const name of ['--color-apple-text-secondary', '--color-apple-text-tertiary']) {
			const fg = tokenValue(name)
			for (const bg of surfaces) {
				expect(contrastRatio(fg, bg), `${name} (${fg}) on ${bg}`).toBeGreaterThanOrEqual(4.5)
			}
		}
	})

	it('index.html does not load external fonts', () => {
		const html = fs.readFileSync(path.resolve(WEB_SRC, '..', 'index.html'), 'utf-8')
		expect(html).not.toContain('fonts.googleapis.com')
		expect(html).not.toContain('fonts.gstatic.com')
	})
})
