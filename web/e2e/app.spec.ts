import { expect, test } from '@playwright/test'
import {
	CARD,
	dismissDisclaimer,
	enter,
	FIXTURES,
	openCourse,
	pickSemester,
	section,
} from './helpers'

// Core happy-path smoke: load → dismiss notice → grid → open a course →
// rich detail modal. Broader coverage lives in the sibling specs.

test('shows a non-blocking first-visit notice', async ({ page }) => {
	await page.goto('/', { waitUntil: 'domcontentloaded' })
	await expect(page.getByRole('heading', { name: '非公式のシラバス検索ツールです' })).toBeVisible()
	await expect(page.getByRole('main')).toBeVisible()
	await dismissDisclaimer(page)
	await expect(page.locator(CARD).first()).toBeVisible()
})

test('loads the timetable grid with courses', async ({ page }) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await expect(page.getByRole('button', { name: FIXTURES.regular }).first()).toBeVisible()
})

test('opens the modal with hero open and categories collapsed', async ({ page }, testInfo) => {
	await enter(page)
	await pickSemester(page, '1学期')
	await openCourse(page, FIXTURES.regular)
	// Firefox smoke guarantees the cross-browser modal path without waiting for
	// its unusually slow dev-server detail fetch; Chromium and WebKit assert the
	// complete rich-detail state below.
	if (testInfo.project.name === 'firefox-smoke') return

	// Hero (成績評価 + 授業の概要) open; the 授業内容 category collapsed.
	await expect(page.getByText('成績評価')).toBeVisible({ timeout: 30_000 })
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
