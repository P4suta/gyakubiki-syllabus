import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { expect, test } from '@playwright/test'
import { enter, FIXTURES, openCourse, pickSemester } from './helpers'

const searchBox = (page: import('@playwright/test').Page) =>
	page.getByPlaceholder('科目名・教員・キーワードで検索')

test('search index failure is explicit and retry completes the same query', async ({ page }) => {
	let indexRequests = 0
	await page.route(/\/search\.[^/]+\.idx\.br$/, async (route) => {
		indexRequests += 1
		if (indexRequests === 1) {
			await route.fulfill({ status: 503, body: 'temporary index failure' })
		} else {
			await route.continue()
		}
	})

	await enter(page)
	await searchBox(page).fill('微分積分学')
	const alert = page.getByRole('alert').filter({ hasText: '検索indexの取得に失敗しました' })
	await expect(alert).toContainText('HTTP 503')
	await alert.getByRole('button', { name: '再試行' }).click()
	await expect(page.getByRole('button', { name: FIXTURES.regular }).first()).toBeVisible()
	await expect(alert).toBeHidden()
	expect(indexRequests).toBeGreaterThanOrEqual(2)
})

for (const status of [404, 500]) {
	test(`mapped detail HTTP ${status} is an error, not permanent unavailability, and retries`, async ({
		page,
	}) => {
		let detailRequests = 0
		await page.route(/\/details\/[^/]+\.json$/, async (route) => {
			detailRequests += 1
			if (detailRequests === 1) {
				await route.fulfill({ status, body: 'temporary detail failure' })
			} else {
				await route.continue()
			}
		})

		await enter(page)
		await pickSemester(page, '1学期')
		await openCourse(page, FIXTURES.regular)
		const dialog = page.getByRole('dialog')
		await expect(dialog.getByRole('alert')).toContainText(`HTTP ${status}`)
		await dialog.getByRole('button', { name: '再試行' }).click()
		await expect(dialog.getByText('成績評価')).toBeVisible()
		expect(detailRequests).toBeGreaterThanOrEqual(2)
	})
}

test('hash-valid but invalid detail JSON is shown as a retryable error', async ({ page }) => {
	const publicDir = process.env.E2E_PUBLIC_DIR
	if (!publicDir) throw new Error('E2E_PUBLIC_DIR is required')
	const manifest = JSON.parse(fs.readFileSync(path.join(publicDir, 'manifest.json'), 'utf8'))
	const datasetRoot = path.join(publicDir, manifest.basePath)
	const detailIndex = JSON.parse(
		fs.readFileSync(path.join(datasetRoot, manifest.assets.details.path), 'utf8'),
	)
	const invalidDetail = Buffer.from('{"cd":')
	const originalAsset = detailIndex['00001']
	const invalidDetailSha = createHash('sha256').update(invalidDetail).digest('hex')
	const invalidAsset = {
		...originalAsset,
		path: originalAsset.path.replace(
			/\.[a-f0-9]{16}\.json$/u,
			`.${invalidDetailSha.slice(0, 16)}.json`,
		),
		bytes: invalidDetail.byteLength,
		sha256: invalidDetailSha,
	}
	const modifiedIndex = Buffer.from(
		JSON.stringify({
			...detailIndex,
			'00001': invalidAsset,
		}),
	)
	const modifiedIndexSha = createHash('sha256').update(modifiedIndex).digest('hex')
	const modifiedIndexPath = `details.${modifiedIndexSha.slice(0, 16)}.json`
	const modifiedManifest = {
		...manifest,
		assets: {
			...manifest.assets,
			details: {
				...manifest.assets.details,
				path: modifiedIndexPath,
				bytes: modifiedIndex.byteLength,
				sha256: modifiedIndexSha,
			},
		},
	}
	await page.route('**/manifest.json', (route) =>
		route.fulfill({ contentType: 'application/json', body: JSON.stringify(modifiedManifest) }),
	)
	await page.route(`**/${modifiedIndexPath}`, (route) =>
		route.fulfill({ contentType: 'application/json', body: modifiedIndex }),
	)
	await page.route(`**/${invalidAsset.path}`, (route) =>
		route.fulfill({ contentType: 'application/json', body: invalidDetail }),
	)

	await enter(page)
	await pickSemester(page, '1学期')
	await openCourse(page, FIXTURES.regular)
	await expect(page.getByRole('dialog').getByRole('alert')).toContainText(
		'詳細データのJSONが壊れています',
	)
	await expect(page.getByRole('dialog').getByRole('button', { name: '再試行' })).toBeVisible()
})

test('worker crash rejects the pending query and a retry uses the regenerated worker', async ({
	page,
}) => {
	await enter(page)
	await page.evaluate(() => window.__GYAKUBIKI_E2E__?.crashNextWorkerRequest())
	await searchBox(page).fill('微分積分学')
	const alert = page.getByRole('alert').filter({ hasText: 'E2E worker crash' })
	await expect(alert).toBeVisible()
	await alert.getByRole('button', { name: '再試行' }).click()
	await expect(page.getByRole('button', { name: FIXTURES.regular }).first()).toBeVisible()
	await expect(alert).toBeHidden()
})

test('worker timeout rejects the pending query and retry does not reuse stale state', async ({
	page,
}) => {
	await enter(page)
	await page.evaluate(() => window.__GYAKUBIKI_E2E__?.stallNextWorkerRequest())
	await searchBox(page).fill('微分積分学')
	const alert = page.getByRole('alert').filter({ hasText: 'query がタイムアウトしました' })
	await expect(alert).toBeVisible({ timeout: 15_000 })
	await alert.getByRole('button', { name: '再試行' }).click()
	await expect(page.getByRole('button', { name: FIXTURES.regular }).first()).toBeVisible()
	await expect(alert).toBeHidden()
})

test('offline state is announced and clears when connectivity returns', async ({
	page,
	context,
}) => {
	await enter(page)
	await context.setOffline(true)
	await expect(page.getByRole('status').filter({ hasText: 'オフラインです' })).toBeVisible()
	await context.setOffline(false)
	await expect(page.getByRole('status').filter({ hasText: 'オフラインです' })).toBeHidden()
})
