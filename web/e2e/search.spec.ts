import { type Page, expect, test } from '@playwright/test'
import { CARD, counts, enter, pickSemester } from './helpers'

const searchBox = (page: Page) => page.getByPlaceholder('科目名・教員・キーワードで検索')

test.describe('search', () => {
	test('by course name narrows the grid', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		const before = await counts(page)
		await searchBox(page).fill('線形代数')
		await expect(async () => {
			const after = await counts(page)
			expect(after.shown).toBeGreaterThan(0)
			expect(after.shown).toBeLessThan(before.shown)
		}).toPass()
		await expect(page.getByRole('button', { name: /線形代数/ }).first()).toBeVisible()
	})

	test('by instructor name is case-insensitive (normalized haystack)', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		// Instructor「Smith John」(cd 00004) — an upper-case query must still match.
		await searchBox(page).fill('SMITH')
		await expect(page.getByRole('button', { name: /理論 & 実践/ }).first()).toBeVisible()
	})

	test('clearing search restores the full set', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		const before = await counts(page)
		await searchBox(page).fill('線形代数')
		await expect(async () => expect((await counts(page)).shown).toBeLessThan(before.shown)).toPass()
		await page.getByRole('button', { name: '検索をクリア' }).click()
		await expect(async () => expect((await counts(page)).shown).toBe(before.shown)).toPass()
	})

	test('a query with no matches empties the grid', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		await searchBox(page).fill('該当なしzzz')
		await expect(async () => expect((await counts(page)).shown).toBe(0)).toPass()
		await expect(page.locator(CARD)).toHaveCount(0)
	})
})
