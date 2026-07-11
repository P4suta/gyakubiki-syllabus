import { type Page, devices, expect, test } from '@playwright/test'
import { CARD, enter, swipe } from './helpers'

// Real touch-gesture coverage on a mobile device (hasTouch). Drags are
// dispatched via CDP so the app's actual touch handlers run.
test.use({ ...devices['Pixel 5'] })

async function openFirstCourse(page: Page) {
	await page.locator(CARD).first().tap()
	await expect(page.getByRole('dialog')).toBeVisible()
	await page.waitForTimeout(400) // let the slide-in settle before measuring/dragging
}

test('swiping the detail sheet down dismisses it', async ({ page }) => {
	await enter(page)
	await openFirstCourse(page)
	const box = await page.getByRole('dialog').boundingBox()
	if (!box) throw new Error('no sheet box')
	// Grab the handle and pull well past the commit threshold.
	await swipe(page, { x: box.x + box.width / 2, y: box.y + 12 }, { x: box.x + box.width / 2, y: box.y + box.height * 0.85 })
	await expect(page.getByRole('dialog')).toBeHidden()
})

test('a short slow drag snaps the sheet back (stays open)', async ({ page }) => {
	await enter(page)
	await openFirstCourse(page)
	const box = await page.getByRole('dialog').boundingBox()
	if (!box) throw new Error('no sheet box')
	// ~30px, slow → under both the distance and velocity thresholds.
	await swipe(
		page,
		{ x: box.x + box.width / 2, y: box.y + 12 },
		{ x: box.x + box.width / 2, y: box.y + 42 },
		{ steps: 8, delay: 35 },
	)
	await expect(page.getByRole('dialog')).toBeVisible()
})

test('the device Back button closes the sheet without leaving the app', async ({ page }) => {
	await enter(page)
	await openFirstCourse(page)
	await page.goBack()
	await expect(page.getByRole('dialog')).toBeHidden()
	// Still in the app (no reload → the disclaimer does not reappear).
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeHidden()
	await expect(page.locator(CARD).first()).toBeVisible()
})

test('swiping the filter sheet down dismisses it', async ({ page }) => {
	await enter(page)
	await page.getByRole('button', { name: 'フィルターを開く' }).first().tap()
	await expect(page.getByRole('dialog', { name: 'フィルター' })).toBeVisible()
	await page.waitForTimeout(400) // let the slide-in settle
	const box = await page.getByRole('dialog', { name: 'フィルター' }).boundingBox()
	if (!box) throw new Error('no sheet box')
	await swipe(page, { x: box.x + box.width / 2, y: box.y + 12 }, { x: box.x + box.width / 2, y: box.y + box.height * 0.85 })
	await expect(page.getByRole('dialog', { name: 'フィルター' })).toBeHidden()
})

test('swiping the day view left advances to the next day', async ({ page }) => {
	await enter(page)
	// Default day is 月 (index 0). Swipe left → 火 becomes active.
	await swipe(page, { x: 320, y: 420 }, { x: 60, y: 420 })
	await expect(page.getByRole('tab', { name: '火', exact: true })).toHaveClass(/text-apple-blue/)
})

test('swiping the day view right is a no-op at the first day', async ({ page }) => {
	await enter(page)
	// 月 is already the first day; a right (prev) swipe rubber-bands and snaps back.
	await swipe(page, { x: 60, y: 420 }, { x: 340, y: 420 })
	await expect(page.getByRole('tab', { name: '月', exact: true })).toHaveClass(/text-apple-blue/)
})
