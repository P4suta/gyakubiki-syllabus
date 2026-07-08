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

const appCss = () => fs.readFileSync(path.join(WEB_SRC, 'app.css'), 'utf-8')
/** Light tokens live in `@theme`; dark overrides in the `prefers-color-scheme` block. */
const splitThemes = (css: string) => {
	const [light, dark = ''] = css.split('@media (prefers-color-scheme: dark)')
	return { light, dark }
}
const tokenIn = (block: string, name: string) => {
	const m = block.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})`))
	if (!m) throw new Error(`token ${name} not found in the expected block`)
	return m[1]
}

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
	[
		/text-apple-text\/\d/,
		'Opacity-dimmed ink text (use text-apple-text-secondary / -tertiary — an opacity-of-ink vanishes on the dark field)',
	],
	[
		/(?:bg|border|ring)-apple-text\/\d/,
		'Black-tinted UI from apple-text opacity (use an overlay-* token, or on-accent for accent fills)',
	],

	// --- Dividers: one canonical hairline. overlay-light/medium/strong are fills
	// (bg only), never borders — a divider is always border-overlay-subtle. ---
	[
		/\bborder(?:-[btlrxy0-9]+)*-overlay-(?:light|muted|medium|strong|backdrop)\b/,
		'Non-canonical divider colour (dividers use border-overlay-subtle)',
	],

	// --- Radius: use the scale (rounded-lg / -xl / -2xl / -full, incl. -t- variants). ---
	[/rounded-\[/, 'Arbitrary radius (use the scale: rounded-lg / -xl / -2xl / -full)'],
	[/\brounded\b(?!-)/, 'Bare rounded (pick a scale step: rounded-lg / -xl / -2xl / -full)'],
	[/\brounded-(?:sm|md|3xl)\b/, 'Off-scale radius (use rounded-lg / -xl / -2xl / -full)'],

	// --- Arbitrary size / spacing / position: promote to a token or named
	// @utility (min-h-tap, max-h-overlay, z-sheet, content-auto) instead of a
	// magic `[…]`. (scale-[…] micro-transforms are the one sanctioned exception.) ---
	[
		/\b(?:min-h|max-h|min-w|max-w|w|h|z|gap|p[xytblr]?|m[xytblr]?|top|left|right|bottom|inset|leading)-\[/,
		'Arbitrary size/spacing (use a token — min-h-tap — or a named @utility like max-h-overlay / z-sheet)',
	],
	[
		/\bz-\d/,
		'Raw z-index (use a named rung: z-sticky / z-sticky-head / z-sticky-corner / z-nav / z-overlay / z-sheet)',
	],

	// --- Font sizes: use the token scale, never raw Tailwind sizes. ---
	[
		/text-\[\d+px\]/,
		'Arbitrary font size (use text-micro / -caption / -sub / -body / -cta / -headline / -title)',
	],
	[
		/\btext-(?:xs|sm|base|lg|xl|[2-9]xl)\b/,
		'Raw Tailwind font size (use the token scale: text-micro … text-title)',
	],

	// --- Font weight: three roles only (normal body, medium labels, semibold emphasis). ---
	[
		/\bfont-(?:bold|extrabold|black|thin|light)\b/,
		'Off-scale font weight (use font-normal / font-medium / font-semibold)',
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

	it('the lint rules catch known violations and pass canonical usage', () => {
		// Guards the linter itself: a typo that makes a rule a no-op would let real
		// drift through. Every bad string must trip some rule; every canonical one
		// must trip none.
		const caught = (s: string) => RULES.some(([re]) => re.test(s))
		const bad = [
			'rounded-md',
			'rounded-sm',
			'rounded',
			'rounded-[7px]',
			'text-lg',
			'text-xl',
			'text-sm',
			'text-[13px]',
			'font-bold',
			'font-light',
			'border-b border-overlay-light',
			'border-t-overlay-strong',
			'bg-[#ffffff]',
			'text-gray-500',
			'text-blue-600',
			'text-apple-text/70',
			'bg-apple-text/20',
			'shadow-[0_1px_2px]',
			'min-h-[44px]',
			'max-h-[90dvh]',
			'z-[200]',
			'z-50',
			'gap-[2px]',
		]
		for (const s of bad) expect(caught(s), `rule missed: ${s}`).toBe(true)
		const good = [
			'rounded-full',
			'rounded-lg',
			'rounded-xl',
			'rounded-2xl',
			'rounded-t-2xl',
			'text-body',
			'text-title',
			'text-headline',
			'text-caption',
			'font-normal',
			'font-medium',
			'font-semibold',
			'border-b border-overlay-subtle',
			'bg-overlay-light',
			'text-apple-text',
			'text-apple-text-secondary',
			'ring-apple-blue/40',
			'min-h-tap',
			'max-h-overlay',
			'sm:max-h-overlay-sm',
			'z-sheet',
			'z-nav',
			'z-sticky-corner',
			'content-auto',
			'scale-[1.02]',
			'max-w-36',
		]
		for (const s of good) expect(caught(s), `false positive: ${s}`).toBe(false)
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
		expect(css).toContain('--color-surface-elevated:')
		expect(css).toContain('--color-overlay-subtle:')
		expect(css).toContain('--color-overlay-backdrop:')

		// Font-size tokens (the full type scale)
		expect(css).toContain('--font-size-micro:')
		expect(css).toContain('--font-size-caption:')
		expect(css).toContain('--font-size-sub:')
		expect(css).toContain('--font-size-body:')
		expect(css).toContain('--font-size-cta:')
		expect(css).toContain('--font-size-headline:')
		expect(css).toContain('--font-size-title:')

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

	it('muted text tokens clear WCAG AA on every surface, in both themes', () => {
		const { light: lightCss, dark: darkCss } = splitThemes(appCss())
		const MUTED = ['--color-apple-text-secondary', '--color-apple-text-tertiary']
		const checkAA = (block: string, surfaces: string[]) => {
			for (const name of MUTED) {
				const fg = tokenIn(block, name)
				for (const bg of surfaces) {
					expect(contrastRatio(fg, bg), `${name} (${fg}) on ${bg}`).toBeGreaterThanOrEqual(4.5)
				}
			}
		}
		// Muted text can land on: the base surfaces, the subtle-overlay badge
		// (composited hex, computed once — see palette derivation), and any tile.
		checkAA(lightCss, [
			'#ffffff',
			'#fbfbfd', // --color-surface-page
			'#f3f3f6', // --color-overlay-subtle composited over the page
			...COLORS.map((c) => c.light.bg),
		])
		checkAA(darkCss, [
			'#0f1116', // dark --color-surface-page
			'#191c22', // dark --color-surface-primary
			'#1f232b', // dark --color-surface-elevated
			'#23262d', // dark --color-overlay-subtle composited over the card
			...COLORS.map((c) => c.dark.bg),
		])
	})

	it('accent clears WCAG AA as link text and under on-accent, both themes', () => {
		const { light, dark } = splitThemes(appCss())
		const check = (block: string, textSurfaces: string[]) => {
			const accent = tokenIn(block, '--color-apple-blue')
			const onAccent = tokenIn(block, '--color-on-accent')
			// Link/label text uses the accent on the base surfaces.
			for (const bg of textSurfaces) {
				expect(contrastRatio(accent, bg), `apple-blue (${accent}) on ${bg}`).toBeGreaterThanOrEqual(
					4.5,
				)
			}
			// Buttons render on-accent text on the accent fill.
			expect(
				contrastRatio(onAccent, accent),
				`on-accent (${onAccent}) on apple-blue (${accent})`,
			).toBeGreaterThanOrEqual(4.5)
		}
		check(light, ['#ffffff', '#fbfbfd'])
		check(dark, ['#0f1116', '#191c22', '#1f232b'])
	})

	it('every colour token has a dark override (light/dark parity)', () => {
		const { light, dark } = splitThemes(appCss())
		const names = (block: string) => new Set(block.match(/--color-[a-z-]+(?=:)/g) ?? [])
		const lightNames = names(light)
		const darkNames = names(dark)
		expect(lightNames.size, 'light defines colour tokens').toBeGreaterThan(0)
		// Every semantic colour flips — no token may ship without a dark value, or it
		// would render its light value on the dark field (the class of bug this
		// redesign had to hunt by hand).
		const missing = [...lightNames].filter((n) => !darkNames.has(n))
		expect(missing, `tokens missing a dark override: ${missing.join(', ')}`).toEqual([])
	})

	it('index.html does not load external fonts', () => {
		const html = fs.readFileSync(path.resolve(WEB_SRC, '..', 'index.html'), 'utf-8')
		expect(html).not.toContain('fonts.googleapis.com')
		expect(html).not.toContain('fonts.gstatic.com')
	})
})
