import { expect, test } from '@playwright/test'
import { DESKTOP, MOBILE, enter } from './helpers'

// The timetable mounts only the layout for the current breakpoint (see
// Timetable.svelte's matchMedia gate), so the inactive view is absent from the
// DOM — not merely hidden. The mobile single-day view carries role="tabpanel".

test('mounts only the active layout and swaps it across the breakpoint', async ({ page }) => {
	await page.setViewportSize(DESKTOP)
	await enter(page)
	// Desktop: the bar's campus select shows; the mobile day view is not mounted.
	await expect(page.getByLabel('キャンパスで絞り込み')).toBeVisible()
	await expect(page.getByRole('tabpanel')).toHaveCount(0)

	// Cross to mobile: the compact bar's filter button appears and the single-day
	// view mounts, while the desktop-only select drops out.
	await page.setViewportSize(MOBILE)
	await expect(page.getByRole('button', { name: 'フィルターを開く' })).toBeVisible()
	await expect(page.getByRole('tabpanel')).toHaveCount(1)
	await expect(page.getByLabel('キャンパスで絞り込み')).toBeHidden()
})
