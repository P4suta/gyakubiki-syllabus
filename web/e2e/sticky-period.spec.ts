import { expect, test } from '@playwright/test'
import { type Box, box, boxesOverlap, enter, expectWithinBand } from './helpers'

// Regression guard for the sticky period label. These are geometry assertions,
// not `toBeVisible()`: they catch the three bugs that shipped — an off-screen
// period's badge pulled into the rail, a badge overlapping the day-header row,
// and a badge sitting off the visible band. With the native-`sticky` rewrite the
// behaviour is deterministic (no rAF), so these are stable in headless CI.

test.describe('sticky period label (desktop grid)', () => {
	test.beforeEach(async ({ page }) => {
		await enter(page)
		// 全て = the busiest grid, so several period cells are far taller than the viewport.
		await page.getByRole('button', { name: '全て', exact: true }).first().click()
	})

	test('current period always on screen; off-screen periods never appear', async ({ page }) => {
		const scroller = page.locator('.overflow-auto.flex-1')
		const headerH = (await box(page.locator('.grid > .sticky.top-0.left-0'))).height
		const cells = page.locator('[data-period-label]')
		const badges = page.locator('[data-period-badge]')
		const count = await badges.count()
		expect(count).toBe(6)

		for (const frac of [0, 0.2, 0.45, 0.7, 0.9, 0.99]) {
			await scroller.evaluate((s, f) => {
				s.scrollTop = (s.scrollHeight - s.clientHeight) * f
			}, frac)
			await page.waitForTimeout(120)

			const s = await box(scroller)

			// (1) The period owning the viewport centre must have its badge on screen,
			//     below the day header, within the scroller — the label is always there.
			const currentIdx = await scroller.evaluate((el) => {
				const center = el.scrollTop + el.clientHeight / 2
				return [...el.querySelectorAll<HTMLElement>('[data-period-label]')].findIndex(
					(c) => c.offsetTop <= center && center < c.offsetTop + c.offsetHeight,
				)
			})
			expect(currentIdx, `a period owns the viewport centre at frac ${frac}`).toBeGreaterThanOrEqual(0)
			await expectWithinBand(badges.nth(currentIdx), scroller, headerH)

			// (2) Per-badge invariants across all 6 periods.
			for (let i = 0; i < count; i++) {
				const cellBox = await cells.nth(i).boundingBox()
				const badgeBox = await badges.nth(i).boundingBox()
				if (!cellBox || !badgeBox) continue
				const cellOnScreen = boxesOverlap(cellBox, s)
				const badgeOnScreen = boxesOverlap(badgeBox, s)

				// A period whose cell is entirely off-screen must NOT show a badge (the
				//「6限補講が列下部に見切れる」bug).
				if (!cellOnScreen) {
					expect(badgeOnScreen, `period #${i + 1} badge hidden when its cell is off-screen (frac ${frac})`).toBe(false)
				}

				// Any on-screen badge sits below the header and inside the scroller (the
				// 「曜日行と被る」/ off-band bugs).
				if (badgeOnScreen) {
					assertBadgeInBand(badgeBox, s, headerH, i, frac)
				}
			}
		}
	})
})

function assertBadgeInBand(b: Box, s: Box, headerH: number, i: number, frac: number): void {
	const tol = 2
	expect(b.y, `period #${i + 1} badge below the header (frac ${frac})`).toBeGreaterThanOrEqual(
		s.y + headerH - tol,
	)
	expect(
		b.y + b.height,
		`period #${i + 1} badge within the scroller (frac ${frac})`,
	).toBeLessThanOrEqual(s.y + s.height + tol)
}
