import AxeBuilder from '@axe-core/playwright'
import { type Page, expect, test } from '@playwright/test'
import { FIXTURES, MOBILE, dismissDisclaimer, enter, openCourse, pickSemester } from './helpers'

// Automated WCAG 2 A/AA audit (axe-core) over the key states. This is the
// end-to-end guard for the contrast work: axe flags every text node below
// 4.5:1, so a regression in the palette fails here, not just in the unit tests.

async function auditEmpty(page: Page): Promise<void> {
	const { violations } = await new AxeBuilder({ page })
		.withTags(['wcag2a', 'wcag2aa'])
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

test('disclaimer dialog has no WCAG A/AA violations', async ({ page }) => {
	await page.goto('/')
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeVisible()
	await auditEmpty(page)
})

test('timetable grid has no WCAG A/AA violations', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await auditEmpty(page)
})

test('course modal has no WCAG A/AA violations', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await openCourse(page, FIXTURES.regular)
	await auditEmpty(page)
})

test.describe('mobile', () => {
	test.use({ viewport: MOBILE })

	test('mobile day view has no WCAG A/AA violations', async ({ page }) => {
		await page.goto('/')
		await dismissDisclaimer(page)
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
		await auditEmpty(page)
	})

	test('dark course modal has no WCAG A/AA violations', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
		await openCourse(page, FIXTURES.regular)
		await auditEmpty(page)
	})
})
