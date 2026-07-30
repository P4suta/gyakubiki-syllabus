import { createHash } from 'node:crypto'
import { brotliDecompressSync } from 'node:zlib'
import { type APIRequestContext, expect, test } from '@playwright/test'

interface Asset {
	path: string
	bytes: number
	sha256: string
	decodedBytes?: number
	decodedSha256?: string
}

interface Manifest {
	schemaVersion: number
	appCompatVersion: number
	datasetId: string
	sourceCommit: string
	year: string
	generatedAt: string
	basePath: string
	counts: {
		courses: number
		details: number
		detailCoverage: number
		scheduledCourses: number
		unscheduledCourses: number
	}
	assets: { data: Asset; index: Asset; details: Asset }
}

interface WireCourse {
	cd: string
	nm: string
}

interface Offering {
	kind: 'scheduled' | 'intensive' | 'tba'
}

interface WireData {
	courses: WireCourse[]
	offerings: Offering[][]
}

const CARD = '[data-course-card]'
const productionUrl = new URL(process.env.PRODUCTION_URL ?? '')
if (!productionUrl.pathname.endsWith('/')) productionUrl.pathname += '/'
let manifest: Manifest
let data: WireData
let detailAssets: Record<string, Asset>

const TRANSIENT_HTTP_STATUSES = new Set([429, 500, 502, 503, 504])

async function fetchVerified(request: APIRequestContext, url: URL, asset: Asset): Promise<Buffer> {
	for (let attempt = 0; attempt < 6; attempt += 1) {
		const response = await request.get(url.toString())
		const status = response.status()
		if (status === 200) {
			const body = await response.body()
			expect(body.byteLength, `${url} bytes`).toBe(asset.bytes)
			expect(createHash('sha256').update(body).digest('hex'), `${url} sha256`).toBe(asset.sha256)
			return body
		}
		await response.dispose()
		if (!TRANSIENT_HTTP_STATUSES.has(status) || attempt === 5) {
			expect(status, `${url} status`).toBe(200)
		}
		await new Promise((resolve) => setTimeout(resolve, 250 * 2 ** attempt))
	}
	throw new Error(`${url} exhausted verification retries`)
}

function assetUrl(asset: Asset): URL {
	return new URL(`${manifest.basePath}${asset.path}`, productionUrl)
}

async function verifyEveryDetail(request: APIRequestContext): Promise<void> {
	const entries = Object.entries(detailAssets)
	let cursor = 0
	const workers = Array.from({ length: 4 }, async () => {
		while (cursor < entries.length) {
			const current = entries[cursor]
			cursor += 1
			if (!current) return
			const [, asset] = current
			await fetchVerified(
				request,
				new URL(`${manifest.basePath}${asset.path}`, productionUrl),
				asset,
			)
		}
	})
	await Promise.all(workers)
}

test.beforeAll(async ({ request }) => {
	const response = await request.get(new URL('manifest.json', productionUrl).toString())
	expect(response.status()).toBe(200)
	manifest = (await response.json()) as Manifest
	const dataBytes = await fetchVerified(
		request,
		assetUrl(manifest.assets.data),
		manifest.assets.data,
	)
	data = JSON.parse(dataBytes.toString('utf8')) as WireData
	const encodedIndex = await fetchVerified(
		request,
		assetUrl(manifest.assets.index),
		manifest.assets.index,
	)
	const decodedIndex = brotliDecompressSync(encodedIndex)
	expect(decodedIndex.byteLength, 'decoded index bytes').toBe(manifest.assets.index.decodedBytes)
	expect(createHash('sha256').update(decodedIndex).digest('hex'), 'decoded index sha256').toBe(
		manifest.assets.index.decodedSha256,
	)
	const detailIndexBytes = await fetchVerified(
		request,
		assetUrl(manifest.assets.details),
		manifest.assets.details,
	)
	detailAssets = JSON.parse(detailIndexBytes.toString('utf8')) as Record<string, Asset>
})

test('manifest and published dataset are internally consistent', async ({ request }) => {
	test.setTimeout(process.env.VERIFY_ALL_DETAILS === 'true' ? 15 * 60_000 : 120_000)
	expect(manifest.schemaVersion).toBe(4)
	expect(manifest.appCompatVersion).toBe(4)
	expect(manifest.datasetId).toMatch(/^[a-f0-9]{64}$/u)
	expect(manifest.sourceCommit).toMatch(/^[a-f0-9]{40}$/u)
	expect(data.courses).toHaveLength(manifest.counts.courses)
	expect(data.offerings).toHaveLength(data.courses.length)
	expect(Object.keys(detailAssets)).toHaveLength(manifest.counts.details)
	expect(manifest.counts.details).toBe(manifest.counts.courses)
	expect(manifest.counts.detailCoverage).toBe(1)

	const scheduled = new Set<number>()
	const unscheduled = new Set<number>()
	for (const [index, offerings] of data.offerings.entries()) {
		if (offerings.some((offering) => offering.kind === 'scheduled')) scheduled.add(index)
		if (offerings.some((offering) => offering.kind !== 'scheduled')) unscheduled.add(index)
	}
	expect(scheduled.size).toBe(manifest.counts.scheduledCourses)
	expect(unscheduled.size).toBe(manifest.counts.unscheduledCourses)
	expect(new Set([...scheduled, ...unscheduled]).size).toBe(data.courses.length)

	if (process.env.VERIFY_ALL_DETAILS === 'true') await verifyEveryDetail(request)
})

test('public journey, data status, plan, and cached offline shell work', async ({
	page,
	context,
	request,
}) => {
	const intensiveIndex = data.offerings.findIndex((items) =>
		items.some((item) => item.kind === 'intensive'),
	)
	const tbaIndex = data.offerings.findIndex((items) => items.some((item) => item.kind === 'tba'))
	const scheduledDetailIndex = data.offerings.findIndex(
		(items, index) =>
			items.some((item) => item.kind === 'scheduled') &&
			(data.courses[index]?.cd ?? '') in detailAssets,
	)
	expect(intensiveIndex).toBeGreaterThanOrEqual(0)
	expect(scheduledDetailIndex).toBeGreaterThanOrEqual(0)
	const intensive = data.courses[intensiveIndex]
	const tba = tbaIndex >= 0 ? data.courses[tbaIndex] : undefined
	const detailCourse = data.courses[scheduledDetailIndex]
	if (!intensive || !detailCourse) throw new Error('Production smoke courses were not found')

	await page.goto(productionUrl.toString(), { waitUntil: 'domcontentloaded' })
	await expect(page.locator('[data-app-title]:visible')).toHaveText('逆引きシラバス')
	await expect(page.locator('[data-count-summary]').first()).toContainText(
		manifest.counts.courses.toLocaleString('ja-JP'),
	)
	const notice = page.getByRole('button', { name: '確認しました' })
	if (await notice.isVisible()) await notice.click()
	await expect(page.locator(CARD).first()).toBeVisible()

	const search = page.getByRole('textbox', { name: '科目名・教員・キーワードで検索' })
	for (const course of tba ? [intensive, tba] : [intensive]) {
		await search.fill(course.cd)
		await expect(page.getByRole('heading', { name: '集中講義・時間未定' }).first()).toBeVisible()
		await expect(page.locator(`${CARD}[data-course-code="${course.cd}"]`)).toBeVisible()
	}

	await search.fill(detailCourse.cd)
	const detailCard = page.locator(`${CARD}[data-course-code="${detailCourse.cd}"]`)
	await expect(detailCard).toBeVisible()
	await detailCard.click()
	const courseDialog = page.getByRole('dialog')
	await expect(courseDialog.getByRole('heading', { name: detailCourse.nm })).toBeVisible()
	await expect(courseDialog.getByRole('link', { name: /公式シラバスで見る/u })).toBeVisible()
	const detailAsset = detailAssets[detailCourse.cd]
	if (!detailAsset) throw new Error(`No detail asset for ${detailCourse.cd}`)
	await fetchVerified(
		request,
		new URL(`${manifest.basePath}${detailAsset.path}`, productionUrl),
		detailAsset,
	)
	await courseDialog.getByRole('button', { name: '登録', exact: true }).click()
	await expect(courseDialog.getByRole('button', { name: '登録済み', exact: true })).toBeVisible()
	await courseDialog.getByRole('button', { name: '閉じる' }).click()
	await expect(page.getByRole('complementary', { name: '履修プラン' })).toBeVisible()

	await page.getByRole('button', { name: /データ状態/u }).click()
	const statusDialog = page.getByRole('dialog')
	await expect(
		statusDialog.getByRole('heading', { name: 'このアプリとデータの状態' }),
	).toBeVisible()
	await expect(statusDialog).toContainText(manifest.datasetId)
	await expect(statusDialog.getByRole('link', { name: '不具合を報告' })).toHaveAttribute(
		'href',
		/bug-report\.yml/u,
	)
	await statusDialog.getByRole('button', { name: '閉じる', exact: true }).click()

	await page.evaluate(async () => navigator.serviceWorker.ready)
	await page.reload({ waitUntil: 'domcontentloaded' })
	await expect
		.poll(() => page.evaluate(() => navigator.serviceWorker.controller !== null))
		.toBe(true)
	await expect(page.locator(CARD).first()).toBeVisible()
	await context.setOffline(true)
	try {
		await expect(page.getByRole('status').filter({ hasText: 'オフラインです' })).toBeVisible()
		await page.reload({ waitUntil: 'domcontentloaded' })
		await expect(page.locator('[data-app-title]:visible')).toHaveText('逆引きシラバス')
		await expect(page.locator(CARD).first()).toBeVisible()
	} finally {
		await context.setOffline(false)
	}
})
