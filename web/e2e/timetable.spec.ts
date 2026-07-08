import { expect, test } from '@playwright/test'
import { CARD, MOBILE, enter, pickSemester } from './helpers'

test.describe('timetable — desktop grid', () => {
	test('shows weekday columns and period rows', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
		for (const day of ['月', '火', '水', '木', '金']) {
			await expect(page.getByText(day, { exact: true }).first()).toBeVisible()
		}
		await expect(page.getByText('1限').first()).toBeVisible()
		await expect(page.getByText('6限').first()).toBeVisible()
	})

	test('clicking a card opens its modal', async ({ page }) => {
		await enter(page)
		await pickSemester(page, '1学期')
		await page.locator(CARD).first().click()
		await expect(page.getByRole('heading', { level: 2 })).toBeVisible()
	})
})

test.describe('timetable — mobile day view', () => {
	test.use({ viewport: MOBILE })

	test('switches the visible day via the tab bar', async ({ page }) => {
		await enter(page)
		const tab = page.getByRole('button', { name: '火', exact: true })
		await expect(tab).toBeVisible()
		await tab.click()
		// The active tab takes the accent treatment.
		await expect(tab).toHaveClass(/text-apple-blue/)
		await expect(page.getByText('1限').first()).toBeVisible()
	})
})
