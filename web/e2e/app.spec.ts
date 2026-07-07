import { expect, test } from '@playwright/test'

// Smoke test the full happy path against the dummy dataset: load → dismiss the
// disclaimer → see the timetable → open a course → see the rich detail modal.

async function enter(page: import('@playwright/test').Page) {
	await page.goto('/')
	await page.getByRole('button', { name: /利用する/ }).click()
	// Wait for the disclaimer to finish fading out before interacting/shooting.
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeHidden()
}

test('loads the timetable grid with courses', async ({ page }) => {
	await enter(page)
	// A course from the default (1学期) view is placed in the grid.
	await expect(page.getByRole('button', { name: /微分積分学/ }).first()).toBeVisible()
	await page.screenshot({ path: 'test-results/grid.png' })
})

test('opens the modal with hero open and categories collapsed', async ({ page }) => {
	await enter(page)
	await page.getByRole('button', { name: /微分積分学/ }).first().click()

	// Hero (成績評価 + 授業の概要) open; the 授業内容 / その他 categories collapsed,
	// so the default view stays short.
	await expect(page.getByRole('heading', { name: /微分積分学/ })).toBeVisible()
	await expect(page.getByText('成績評価')).toBeVisible()
	await expect(page.getByText('授業内容', { exact: true })).toBeVisible()
	await page.screenshot({ path: 'test-results/modal.png' })
	// Expanding 授業内容 reveals 授業計画 with two-digit markers (第10回…) on one line.
	await page.getByText('授業内容', { exact: true }).click()
	await expect(page.getByText('授業計画')).toBeVisible()
	await expect(page.getByText('第10回', { exact: false }).first()).toBeVisible()
	await page.screenshot({ path: 'test-results/modal-plan.png' })
})

test('a course without detail degrades gracefully', async ({ page }) => {
	await enter(page)
	// 心理学概論 (cd 00010, 通年) is generated without a detail file; 通年 courses
	// appear under every semester filter, so it is in the default grid.
	await page.getByRole('button', { name: /心理学概論/ }).first().click()
	// The base info + official-syllabus link remain; no rich eval section renders.
	await expect(page.getByRole('link', { name: /公式シラバスで見る/ })).toBeVisible()
	await expect(page.getByText('成績評価')).toHaveCount(0)
})
