import AxeBuilder from '@axe-core/playwright'
import { expect, type Page, test } from '@playwright/test'
import { enter, FIXTURES, MOBILE, openCourse, pickSemester } from './helpers'

// Automated WCAG 2 A/AA audit (axe-core) over the key states. This is the
// end-to-end guard for landmark/state semantics plus a representative rendered
// CourseCard. The palette's complete tint matrix remains covered by unit tests.

// The repeated grid is reduced to one real search result for CourseCard checks;
// other states exclude background cards and focus on their unique semantics.
test.describe.configure({ timeout: 300_000, mode: 'serial' })

async function auditEmpty(page: Page): Promise<void> {
	// best-practice guards the landmark/heading structure (region,
	// landmark-one-main, page-has-heading-one, heading-order, …) on top of WCAG.
	const { violations } = await new AxeBuilder({ page })
		.withTags(['wcag2a', 'wcag2aa', 'best-practice'])
		.analyze()
	const summary = violations.map((v) => ({
		id: v.id,
		impact: v.impact,
		help: v.help,
		nodes: v.nodes.length,
		sample: v.nodes[0]?.html?.slice(0, 140),
	}))
	expect(JSON.stringify(summary, null, 2)).toBe('[]')
}

async function showRepresentativeCard(page: Page, mobile = false): Promise<void> {
	if (mobile) {
		await page.getByRole('button', { name: 'フィルターを開く' }).click()
		await page.locator('#mobile-search').fill('00001')
		await page.keyboard.press('Escape')
		await expect(page.getByRole('dialog', { name: 'フィルター' })).toBeHidden()
	} else {
		await page.getByPlaceholder('科目名・教員・キーワードで検索').fill('00001')
	}
	await expect(page.locator('[data-course-card]')).toHaveCount(1, { timeout: 30_000 })
}

test('first-visit notice has no WCAG A/AA violations', async ({ page }) => {
	await page.goto('/', { waitUntil: 'domcontentloaded' })
	await expect(page.getByRole('heading', { name: '非公式のシラバス検索ツールです' })).toBeVisible()
	await showRepresentativeCard(page)
	await auditEmpty(page)
})

test('timetable grid has no WCAG A/AA violations', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await showRepresentativeCard(page)
	await auditEmpty(page)
})

test('search result count is announced through the live region', async ({ page }) => {
	await enter(page)
	await page.getByPlaceholder('科目名・教員・キーワードで検索').fill('00001')
	await expect(page.getByRole('status').filter({ hasText: '1件の科目を表示中' })).toBeVisible()
})

test('course modal has no WCAG A/AA violations', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await showRepresentativeCard(page)
	await openCourse(page, FIXTURES.regular)
	await auditEmpty(page)
})

test.describe('mobile', () => {
	test.use({ viewport: MOBILE })

	test('mobile day view has no WCAG A/AA violations', async ({ page }) => {
		await enter(page)
		await showRepresentativeCard(page, true)
		await auditEmpty(page)
	})
})

// The dark theme is a full second palette (app.css `prefers-color-scheme` block);
// re-run the colour-heavy states so a dark-only contrast regression fails here.
test.describe('dark theme', () => {
	test.use({ colorScheme: 'dark' })

	test('dark grid has no WCAG A/AA violations', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
		await showRepresentativeCard(page)
		await auditEmpty(page)
	})

	test('dark course modal has no WCAG A/AA violations', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
		await showRepresentativeCard(page)
		await openCourse(page, FIXTURES.regular)
		await auditEmpty(page)
	})
})

test.describe('reduced motion', () => {
	test('collapses dialog animation and transition durations', async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' })
		await enter(page)
		await pickSemester(page, '1学期')
		await showRepresentativeCard(page)
		await openCourse(page, FIXTURES.regular)
		const motion = await page.locator('[data-sheet]').evaluate((element) => {
			const style = getComputedStyle(element)
			const milliseconds = (value: string) =>
				Math.max(
					...value.split(',').map((item) => {
						const duration = Number.parseFloat(item)
						return item.trim().endsWith('ms') ? duration : duration * 1_000
					}),
				)
			return {
				reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
				className: element.className,
				animationName: style.animationName,
				animation: milliseconds(style.animationDuration),
				transition: milliseconds(style.transitionDuration),
			}
		})
		expect(motion.reduced).toBe(true)
		expect(motion.className).not.toContain('animate-dialog-in')
		expect(motion.animationName).toBe('none')
		expect(motion.animation).toBeLessThanOrEqual(0.1)
		expect(motion.transition).toBeLessThanOrEqual(0.1)
	})
})
