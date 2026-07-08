import { expect, test } from '@playwright/test'
import { CARD, FIXTURES, dismissDisclaimer, enter, openCourse, pickSemester, section } from './helpers'

// Core happy-path smoke: load → dismiss disclaimer → grid → open a course →
// rich detail modal. Broader coverage lives in the sibling specs.

test('shows the disclaimer first and reveals the app after accepting', async ({ page }) => {
	await page.goto('/')
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeVisible()
	await dismissDisclaimer(page)
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeHidden()
	await expect(page.locator(CARD).first()).toBeVisible()
})

test('loads the timetable grid with courses', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await expect(page.getByRole('button', { name: FIXTURES.regular }).first()).toBeVisible()
})

test('opens the modal with hero open and categories collapsed', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await openCourse(page, FIXTURES.regular)

	// Hero (成績評価 + 授業の概要) open; the 授業内容 category collapsed.
	await expect(page.getByText('成績評価')).toBeVisible()
	await expect(page.getByText('授業内容', { exact: true })).toBeVisible()

	// Expanding 授業内容 reveals 授業計画 with two-digit markers (第10回…) on one line.
	await section(page, '授業内容').click()
	await expect(page.getByText('授業計画')).toBeVisible()
	await expect(page.getByText('第10回', { exact: false }).first()).toBeVisible()
})

test('a course without detail degrades gracefully', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	// 心理学概論 (通年) has no detail file; 通年 shows under every semester.
	await openCourse(page, FIXTURES.noDetail)
	await expect(page.getByRole('link', { name: /公式シラバスで見る/ })).toBeVisible()
	await expect(page.getByText('成績評価')).toHaveCount(0)
})
