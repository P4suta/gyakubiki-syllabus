import { expect, test } from '@playwright/test'
import { FIXTURES, counts, enter, pickSemester } from './helpers'

// Filters run on the desktop layout (segmented semester control + native
// selects); counts come from the root's data-*-count attributes (helpers).

const deptSelect = (page: import('@playwright/test').Page) =>
	page.locator('select').filter({ has: page.locator('option', { hasText: '全部署' }) })
const campusSelect = (page: import('@playwright/test').Page) =>
	page.locator('select').filter({ has: page.locator('option', { hasText: '全キャンパス' }) })

test.describe('filters', () => {
	test('semester narrows the set while the total stays fixed', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		const all = await counts(page)
		await pickSemester(page, '1学期')
		await expect(async () => {
			const term = await counts(page)
			expect(term.shown).toBeLessThan(all.shown)
			expect(term.total).toBe(all.total)
		}).toPass()
	})

	test('department filter narrows results', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		const before = await counts(page)
		await deptSelect(page).selectOption({ label: '理工学部' })
		await expect(async () => {
			const after = await counts(page)
			expect(after.shown).toBeGreaterThan(0)
			expect(after.shown).toBeLessThan(before.shown)
		}).toPass()
	})

	test('campus and department filters compose', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '全て')
		await deptSelect(page).selectOption({ label: '理工学部' })
		const afterDept = await counts(page)
		await campusSelect(page).selectOption({ label: '朝倉キャンパス' })
		await expect(async () => {
			const both = await counts(page)
			expect(both.shown).toBeLessThanOrEqual(afterDept.shown)
		}).toPass()
	})

	test('通年 courses appear under every semester', async ({ page }) => {
		await enter(page)
		for (const term of ['1学期', '2学期']) {
			await pickSemester(page, term)
			await expect(page.getByRole('button', { name: FIXTURES.noDetail }).first()).toBeVisible()
		}
	})
})
