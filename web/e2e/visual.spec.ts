import { type Page, expect, test } from '@playwright/test'
import { DESKTOP, FIXTURES, MOBILE, enter, openCourse } from './helpers'

// Visual regression: screenshot the major layouts and diff against baselines
// rendered on the CI OS (Linux). `toBeVisible()` passes for an element that is
// on-screen-but-mispositioned; a pixel diff catches the layout regressions it
// can't. See docs/testing.md.
//
// Skipped off Linux — baselines are OS-specific and only the Linux set is
// committed (dev here is Windows). Regenerate/refresh them with the
// `visual-baseline` workflow (workflow_dispatch), which lands the PNGs via a
// signed commit and opens a PR.
test.describe('visual regression', () => {
	test.skip(process.platform !== 'linux', 'baselines are rendered on Linux (CI)')

	// The data's「最終更新」date is the only non-deterministic pixel in the chrome.
	const dateMask = (page: Page) => [page.getByText(/最終更新/)]

	test('desktop grid — top and scrolled', async ({ page }) => {
		await page.setViewportSize(DESKTOP)
		await enter(page)
		// 全て = the busiest grid, so the shot exercises tall rows and sticky rails.
		await page.getByRole('button', { name: '全て', exact: true }).first().click()
		await expect(page.locator('.overflow-auto.flex-1')).toBeVisible()
		await expect(page).toHaveScreenshot('grid-desktop-top.png', { mask: dateMask(page) })

		// A screenful down guards the sticky day header and period rail mid-scroll.
		await page.locator('.overflow-auto.flex-1').evaluate((s) => {
			s.scrollTop = (s.scrollHeight - s.clientHeight) * 0.45
		})
		await expect(page).toHaveScreenshot('grid-desktop-scrolled.png', { mask: dateMask(page) })
	})

	test('course modal — desktop', async ({ page }) => {
		await page.setViewportSize(DESKTOP)
		await enter(page)
		await openCourse(page, FIXTURES.regular)
		await expect(page.getByRole('dialog')).toHaveScreenshot('modal-desktop.png')
	})

	test('course modal — mobile', async ({ page }) => {
		await page.setViewportSize(MOBILE)
		await enter(page)
		await openCourse(page, FIXTURES.regular)
		await expect(page.getByRole('dialog')).toHaveScreenshot('modal-mobile.png')
	})

	test('filter sheet — mobile', async ({ page }) => {
		await page.setViewportSize(MOBILE)
		await enter(page)
		await page.getByRole('button', { name: 'フィルターを開く' }).first().click()
		const sheet = page.getByRole('dialog', { name: 'フィルター' })
		await expect(sheet).toBeVisible()
		await expect(sheet).toHaveScreenshot('filter-sheet-mobile.png', { mask: dateMask(page) })
	})

	test('mobile day view', async ({ page }) => {
		await page.setViewportSize(MOBILE)
		await enter(page)
		await expect(page.getByRole('tabpanel')).toHaveScreenshot('day-view-mobile.png')
	})
})
