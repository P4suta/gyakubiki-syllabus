import { type Page, expect } from '@playwright/test'

// Shared fixtures/selectors for the E2E suite. The dataset is deterministic
// (gen-sample, fixed seed): the first courses below are stable regardless of
// `--count`, so tests can anchor on them.

/** Course cards render as left-accent-bordered buttons in the grid/day view. */
export const CARD = 'button[class*="border-l-3"]'

/** Stable anchor courses (index-derived in gen-sample, seed-independent). */
export const FIXTURES = {
	/** cd 00001 — 微分積分学Ⅰ, 1学期 月1限, has a full detail. */
	regular: /微分積分学/,
	/** cd 00010 — 心理学概論, 通年 (shows under every semester), NO detail file. */
	noDetail: /心理学概論/,
	/** cd 00004 — name with HTML metacharacters, must render escaped. */
	htmlName: '理論 & 実践 <入門>',
} as const

export const MOBILE = { width: 390, height: 844 }
export const DESKTOP = { width: 1360, height: 900 }

/** Dismiss the「ご利用にあたって」disclaimer and wait for it to fully leave. */
export async function dismissDisclaimer(page: Page): Promise<void> {
	await page.getByRole('button', { name: /利用する/ }).click()
	await expect(page.getByRole('heading', { name: 'ご利用にあたって' })).toBeHidden()
}

/** Load the app, dismiss the disclaimer, and wait until the grid has rendered. */
export async function enter(page: Page): Promise<void> {
	await page.goto('/')
	await dismissDisclaimer(page)
	// The grid is worker-backed and fills asynchronously; wait for real cards.
	await expect(page.locator(CARD).first()).toBeVisible()
}

/** Pick a semester from the desktop segmented control (exact label). */
export async function pickSemester(page: Page, label: string): Promise<void> {
	await page.getByRole('button', { name: label, exact: true }).first().click()
}

/**
 * Read the desktop header counter「{shown}科目表示中 / 全{total}件」.
 * Retries until it is present so it survives the async grid update.
 */
export async function counts(page: Page): Promise<{ shown: number; total: number }> {
	const label = page.getByText(/科目表示中 \/ 全\d+件/)
	await expect(label).toBeVisible()
	const text = (await label.textContent()) ?? ''
	const m = text.match(/([\d,]+)科目表示中 \/ 全([\d,]+)件/)
	if (!m) throw new Error(`unexpected counter text: ${text}`)
	const n = (s: string) => Number(s.replace(/,/g, ''))
	return { shown: n(m[1]), total: n(m[2]) }
}

/** Open the modal for the first card matching `name` and wait for its heading. */
export async function openCourse(page: Page, name: string | RegExp): Promise<void> {
	await page.getByRole('button', { name }).first().click()
	await expect(page.getByRole('heading', { level: 2 })).toBeVisible()
}

/**
 * A modal disclosure section's `<summary>` by label. Scoped to `summary` so it
 * never collides with a same-named `<option>` (e.g.「その他」is also a filter value).
 */
export function section(page: Page, label: string) {
	return page.locator('summary').filter({ hasText: label })
}

export interface Point {
	x: number
	y: number
}

/**
 * Dispatch a real touch drag via CDP. Spacing the moves in wall-clock time lets
 * the page's `event.timeStamp` advance, so gesture velocity is meaningful (a slow
 * short drag stays under the flick threshold; a long drag commits on distance).
 * Requires a touch-enabled context (`test.use({ ...devices['Pixel 5'] })`).
 */
export async function swipe(
	page: Page,
	from: Point,
	to: Point,
	{ steps = 8, delay = 25 }: { steps?: number; delay?: number } = {},
): Promise<void> {
	const client = await page.context().newCDPSession(page)
	await client.send('Input.dispatchTouchEvent', {
		type: 'touchStart',
		touchPoints: [{ x: from.x, y: from.y }],
	})
	for (let i = 1; i <= steps; i++) {
		await page.waitForTimeout(delay)
		await client.send('Input.dispatchTouchEvent', {
			type: 'touchMove',
			touchPoints: [
				{ x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps },
			],
		})
	}
	await client.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] })
	await client.detach()
}
