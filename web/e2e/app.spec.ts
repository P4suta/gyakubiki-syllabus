import { expect, test } from '@playwright/test'
import {
	CARD,
	counts,
	dismissDisclaimer,
	enter,
	expectNoOverlap,
	FIXTURES,
	MOBILE,
	openCourse,
	pickSemester,
	section,
} from './helpers'

// Core happy-path smoke: load → dismiss notice → grid → open a course →
// rich detail modal. Broader coverage lives in the sibling specs.

test('shows an in-flow first-visit notice and compact public status', async ({ page }) => {
	await page.goto('/', { waitUntil: 'domcontentloaded' })
	const notice = page.locator('[data-unofficial-banner]')
	await expect(page.getByRole('heading', { name: '非公式のシラバス検索ツールです' })).toBeVisible()
	await expect(page.getByRole('main')).toBeVisible()
	await expect(page.locator('[data-app-title]:visible')).toHaveText('逆引きシラバス')
	await expect(page.locator(CARD).first()).toBeVisible()
	const { shown, total } = await counts(page)
	await expect(page.locator('[data-count-summary]:visible')).toContainText(
		`${shown.toLocaleString('ja-JP')} / ${total.toLocaleString('ja-JP')}件`,
	)
	await expectNoOverlap(notice, page.locator(CARD).first())
	await dismissDisclaimer(page)
})

test('shows the compact public status without overlap on mobile', async ({ page }) => {
	await page.setViewportSize(MOBILE)
	await page.goto('/', { waitUntil: 'domcontentloaded' })
	const notice = page.locator('[data-unofficial-banner]')
	await expect(page.locator('[data-app-title]:visible')).toHaveText('逆引きシラバス')
	await expect(page.locator('[data-count-summary]:visible')).toContainText(/件.*更新/u)
	await expect(page.locator(CARD).first()).toBeVisible()
	await expectNoOverlap(notice, page.locator(CARD).first())
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
	// cd 00010 is 心理学概論 (通年) and has no detail file. Search by
	// the unique code before clicking so this does not depend on grid size/order.
	await page.getByRole('textbox', { name: '科目名・教員・キーワードで検索' }).fill('00010')
	await expect(page.locator('[data-shown-count]')).toHaveAttribute('data-shown-count', '1')
	await openCourse(page, FIXTURES.noDetail)
	await expect(page.getByRole('link', { name: /公式シラバスで見る/ })).toBeVisible()
	await expect(page.getByText('成績評価')).toHaveCount(0)
})

test('shows Japanese app and dataset status with feedback link', async ({ page }) => {
	await enter(page)
	await page.getByRole('button', { name: /データ状態/u }).click()
	const dialog = page.getByRole('dialog')
	await expect(dialog.getByRole('heading', { name: 'このアプリとデータの状態' })).toBeVisible()
	await expect(dialog).toContainText('対象年度')
	await expect(dialog).toContainText('データセットID')
	await expect(dialog).toContainText('生成元コミット')
	await expect(dialog).toContainText('詳細データ')
	await expect(dialog).toContainText('内訳には重複があります')
	await expect(dialog.getByRole('link', { name: '不具合を報告' })).toHaveAttribute(
		'href',
		/bug-report\.yml/u,
	)
})
