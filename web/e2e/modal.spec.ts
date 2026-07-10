import { expect, test } from '@playwright/test'
import { FIXTURES, enter, openCourse, pickSemester, section } from './helpers'

test.describe('course modal', () => {
	test.beforeEach(async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
	})

	test('shows the title, metadata chips, and the eval chart', async ({ page }) => {
		await openCourse(page, FIXTURES.regular)
		await expect(page.getByRole('heading', { level: 2, name: FIXTURES.regular })).toBeVisible()
		await expect(page.getByText('単位', { exact: false }).first()).toBeVisible()
		await expect(page.getByText('成績評価')).toBeVisible()
		// The ratio chart labels its dominant share with a percentage. Scope to the
		// dialog: the cards behind it now carry a mini-donut `<title>…%</title>`
		// (hidden) that a page-wide match would resolve to first.
		await expect(page.getByRole('dialog').getByText(/\d+%/).first()).toBeVisible()
	})

	test('expands 授業内容 and その他 to reveal 授業計画 and 科目情報', async ({ page }) => {
		await openCourse(page, FIXTURES.regular)
		await section(page, '授業内容').click()
		await expect(page.getByText('授業計画')).toBeVisible()
		await section(page, 'その他').click()
		await expect(page.getByText('科目情報')).toBeVisible()
		await expect(page.getByText('授業コード')).toBeVisible()
	})

	test('links to the official KULAS syllabus with the course code', async ({ page }) => {
		await openCourse(page, FIXTURES.regular)
		const link = page.getByRole('link', { name: /公式シラバスで見る/ })
		await expect(link).toHaveAttribute('href', /kulas\.kochi-u\.ac\.jp.*kogiCd=00001/)
		await expect(link).toHaveAttribute('target', '_blank')
		await expect(link).toHaveAttribute('rel', /noopener/)
	})

	test('closes via the × button and via Escape', async ({ page }) => {
		await openCourse(page, FIXTURES.regular)
		await page.getByRole('button', { name: '閉じる' }).click()
		await expect(page.getByRole('heading', { level: 2 })).toBeHidden()

		await openCourse(page, FIXTURES.regular)
		await page.keyboard.press('Escape')
		await expect(page.getByRole('heading', { level: 2 })).toBeHidden()
	})

	test('renders an HTML-metacharacter course name as text, not markup', async ({ page }) => {
		// cd 00004 (「理論 & 実践 <入門>」) is a 2学期前半 course.
		await pickSemester(page, '2学期前半')
		await openCourse(page, /理論 & 実践/)
		await expect(page.getByRole('heading', { level: 2 })).toHaveText(FIXTURES.htmlName)
	})
})
