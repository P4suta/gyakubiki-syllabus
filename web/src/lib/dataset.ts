export interface DatasetAsset {
	path: string
	bytes: number
	sha256: string
	decodedBytes?: number
	decodedSha256?: string
}

export interface DatasetManifest {
	schemaVersion: 4
	appCompatVersion: 4
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
	range: {
		minDay?: number
		maxDay?: number
		minPeriod?: number
		maxPeriod?: number
	}
	assets: {
		data: DatasetAsset
		index: DatasetAsset
		details: DatasetAsset
	}
}

let activeManifest: DatasetManifest | null = null
let detailIndexCache: { datasetId: string; assets: Record<string, DatasetAsset> } | null = null

export function currentManifest(): DatasetManifest | null {
	return activeManifest
}

export function assetUrl(manifest: DatasetManifest, asset: DatasetAsset): string {
	const base = import.meta.env.BASE_URL
	return `${base}${manifest.basePath}${asset.path}`
}

export async function loadManifest(cache: RequestCache = 'no-store'): Promise<DatasetManifest> {
	const controller = new AbortController()
	const timeout = setTimeout(() => controller.abort(), 3_000)
	try {
		const response = await fetch(`${import.meta.env.BASE_URL}manifest.json`, {
			cache,
			signal: controller.signal,
		})
		if (!response.ok) throw new Error(`データ状態の取得に失敗しました (HTTP ${response.status})`)
		const bytes = await readBoundedResponse(response, 64 * 1024, 'manifest.json')
		let value: unknown
		try {
			value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
		} catch {
			throw new Error('manifest.json のJSONが壊れています')
		}
		const manifest = validateManifest(value)
		if (detailIndexCache?.datasetId !== manifest.datasetId) detailIndexCache = null
		activeManifest = manifest
		return manifest
	} catch (error) {
		if (error instanceof DOMException && error.name === 'AbortError') {
			throw new Error('データ状態の取得がタイムアウトしました')
		}
		throw error
	} finally {
		clearTimeout(timeout)
	}
}

export async function fetchVerifiedAsset(
	manifest: DatasetManifest,
	asset: DatasetAsset,
	signal?: AbortSignal,
): Promise<ArrayBuffer> {
	const response = await fetch(assetUrl(manifest, asset), { signal })
	if (!response.ok) throw new Error(`データassetの取得に失敗しました (HTTP ${response.status})`)
	const buffer = await readBoundedResponse(response, asset.bytes, 'データasset')
	if (buffer.byteLength !== asset.bytes) throw new Error('データassetのサイズが一致しません')
	const digest = await crypto.subtle.digest('SHA-256', buffer)
	const actual = Array.from(new Uint8Array(digest), (value) =>
		value.toString(16).padStart(2, '0'),
	).join('')
	if (actual !== asset.sha256.toLowerCase()) throw new Error('データassetのハッシュが一致しません')
	return buffer
}

/** Read an untrusted response without allowing it to exceed its declared bound. */
export async function readBoundedResponse(
	response: Response,
	maxBytes: number,
	label: string,
): Promise<ArrayBuffer> {
	if (!Number.isSafeInteger(maxBytes) || maxBytes <= 0 || maxBytes > 128 * 1024 * 1024) {
		throw new Error(`${label}のサイズ上限が不正です`)
	}
	const declared = response.headers.get('content-length')
	if (declared !== null) {
		const length = Number(declared)
		if (!Number.isSafeInteger(length) || length < 0 || length > maxBytes) {
			throw new Error(`${label}のContent-Lengthが上限を超えています`)
		}
	}
	if (!response.body) {
		const buffer = await response.arrayBuffer()
		if (buffer.byteLength > maxBytes) throw new Error(`${label}がサイズ上限を超えています`)
		return buffer
	}

	const reader = response.body.getReader()
	const chunks: Uint8Array[] = []
	let total = 0
	try {
		while (true) {
			const { done, value } = await reader.read()
			if (done) break
			total += value.byteLength
			if (total > maxBytes) {
				await reader.cancel().catch(() => undefined)
				throw new Error(`${label}がサイズ上限を超えています`)
			}
			chunks.push(value)
		}
	} finally {
		reader.releaseLock()
	}
	const bytes = new Uint8Array(total)
	let offset = 0
	for (const chunk of chunks) {
		bytes.set(chunk, offset)
		offset += chunk.byteLength
	}
	return bytes.buffer
}

export async function detailAsset(
	manifest: DatasetManifest,
	courseCode: string,
	signal?: AbortSignal,
): Promise<DatasetAsset | undefined> {
	if (detailIndexCache?.datasetId === manifest.datasetId) {
		return detailIndexCache.assets[courseCode]
	}
	const buffer = await fetchVerifiedAsset(manifest, manifest.assets.details, signal)
	let value: unknown
	try {
		value = JSON.parse(new TextDecoder().decode(buffer))
	} catch {
		throw new Error('詳細asset indexのJSONが壊れています')
	}
	if (!isRecord(value)) throw new Error('詳細asset indexの形式が不正です')
	const entries = Object.entries(value)
	if (entries.length !== manifest.counts.details) {
		throw new Error('manifestと詳細asset indexの件数が一致しません')
	}
	for (const [code, asset] of entries) {
		if (!isCourseCode(code) || !isDetailAsset(asset)) {
			throw new Error('詳細asset indexに不正な項目があります')
		}
	}
	const assets = value as Record<string, DatasetAsset>
	detailIndexCache = { datasetId: manifest.datasetId, assets }
	return assets[courseCode]
}

function validateManifest(value: unknown): DatasetManifest {
	if (!isRecord(value)) throw new Error('manifest.json の形式が不正です')
	exactKeys(value, [
		'schemaVersion',
		'appCompatVersion',
		'datasetId',
		'sourceCommit',
		'year',
		'generatedAt',
		'basePath',
		'counts',
		'range',
		'assets',
	])
	if (value.schemaVersion !== 4 || value.appCompatVersion !== 4) {
		throw new Error('このアプリとデータの互換バージョンが一致しません')
	}
	for (const key of ['datasetId', 'sourceCommit', 'year', 'generatedAt', 'basePath'] as const) {
		if (typeof value[key] !== 'string' || value[key].length === 0) {
			throw new Error(`manifest.json の ${key} が不正です`)
		}
	}
	if (!/^[a-f0-9]{64}$/u.test(value.datasetId as string)) {
		throw new Error('manifest.json のdataset IDが不正です')
	}
	if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/u.test(value.sourceCommit as string)) {
		throw new Error('manifest.json の生成元commitが不正です')
	}
	if (!/^\d{4}$/u.test(value.year as string)) {
		throw new Error('manifest.json の年度が不正です')
	}
	if (
		!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/u.test(
			value.generatedAt as string,
		) ||
		Number.isNaN(Date.parse(value.generatedAt as string))
	) {
		throw new Error('manifest.json の生成日時が不正です')
	}
	if (value.basePath !== `datasets/${value.datasetId}/`) {
		throw new Error('manifest.json のbasePathがdataset IDと一致しません')
	}
	if (
		!isRecord(value.assets) ||
		!exactKeys(value.assets, ['data', 'index', 'details']) ||
		!isRootAsset(value.assets.data, /^data\.([a-f0-9]{16})\.json$/u, false) ||
		!isRootAsset(value.assets.index, /^search\.([a-f0-9]{16})\.idx\.br$/u, true, 4 * 1024 * 1024) ||
		!isRootAsset(value.assets.details, /^details\.([a-f0-9]{16})\.json$/u, false)
	) {
		throw new Error('manifest.json のasset情報が不正です')
	}
	if (!isRecord(value.counts) || !isRecord(value.range)) {
		throw new Error('manifest.json の件数・範囲情報が不正です')
	}
	const counts = value.counts
	exactKeys(counts, [
		'courses',
		'details',
		'detailCoverage',
		'scheduledCourses',
		'unscheduledCourses',
	])
	exactKeys(value.range, ['minDay', 'maxDay', 'minPeriod', 'maxPeriod'])
	for (const key of ['courses', 'details', 'scheduledCourses', 'unscheduledCourses'] as const) {
		if (
			!Number.isSafeInteger(counts[key]) ||
			(counts[key] as number) < 0 ||
			(counts[key] as number) > 100_000
		) {
			throw new Error(`manifest.json の ${key} 件数が不正です`)
		}
	}
	const courseCount = counts.courses as number
	const detailCount = counts.details as number
	if (
		typeof counts.detailCoverage !== 'number' ||
		!Number.isFinite(counts.detailCoverage) ||
		counts.detailCoverage < 0 ||
		counts.detailCoverage > 1 ||
		detailCount > courseCount ||
		Math.abs(counts.detailCoverage - (courseCount === 0 ? 1 : detailCount / courseCount)) >
			Number.EPSILON * Math.max(1, courseCount) ||
		(counts.scheduledCourses as number) > courseCount ||
		(counts.unscheduledCourses as number) > courseCount ||
		(counts.scheduledCourses as number) + (counts.unscheduledCourses as number) < courseCount
	) {
		throw new Error('manifest.json の詳細coverageが不正です')
	}
	for (const [key, min, max] of [
		['minDay', 0, 6],
		['maxDay', 0, 6],
		['minPeriod', 1, 8],
		['maxPeriod', 1, 8],
	] as const) {
		const item = value.range[key]
		if (
			item !== null &&
			item !== undefined &&
			(typeof item !== 'number' || !Number.isInteger(item) || item < min || item > max)
		) {
			throw new Error(`manifest.json の ${key} が不正です`)
		}
	}
	const range = value.range
	const minDayPresent = typeof range.minDay === 'number'
	const maxDayPresent = typeof range.maxDay === 'number'
	const minPeriodPresent = typeof range.minPeriod === 'number'
	const maxPeriodPresent = typeof range.maxPeriod === 'number'
	const hasDay = minDayPresent || maxDayPresent
	const hasPeriod = minPeriodPresent || maxPeriodPresent
	if (
		hasDay !== hasPeriod ||
		minDayPresent !== maxDayPresent ||
		minPeriodPresent !== maxPeriodPresent ||
		(typeof range.minDay === 'number' &&
			typeof range.maxDay === 'number' &&
			range.minDay > range.maxDay) ||
		(typeof range.minPeriod === 'number' &&
			typeof range.maxPeriod === 'number' &&
			range.minPeriod > range.maxPeriod) ||
		((counts.scheduledCourses as number) === 0) !== !hasDay
	) {
		throw new Error('manifest.json の曜日・時限範囲の整合性が不正です')
	}
	return value as unknown as DatasetManifest
}

function isAsset(value: unknown, maxBytes: number): value is DatasetAsset {
	return (
		isRecord(value) &&
		exactKeys(value, ['path', 'bytes', 'sha256', 'decodedBytes', 'decodedSha256']) &&
		typeof value.path === 'string' &&
		value.path.length > 0 &&
		!value.path.startsWith('/') &&
		!value.path.includes('..') &&
		!hasUnsafePathCharacter(value.path) &&
		typeof value.bytes === 'number' &&
		Number.isSafeInteger(value.bytes) &&
		value.bytes > 0 &&
		value.bytes <= maxBytes &&
		typeof value.sha256 === 'string' &&
		/^[a-f0-9]{64}$/u.test(value.sha256) &&
		(value.decodedBytes === undefined ||
			(typeof value.decodedBytes === 'number' &&
				Number.isSafeInteger(value.decodedBytes) &&
				value.decodedBytes > 0 &&
				value.decodedBytes <= 64 * 1024 * 1024)) &&
		(value.decodedSha256 === undefined ||
			(typeof value.decodedSha256 === 'string' && /^[a-f0-9]{64}$/u.test(value.decodedSha256))) &&
		(value.decodedBytes === undefined) === (value.decodedSha256 === undefined)
	)
}

function isRootAsset(
	value: unknown,
	pathPattern: RegExp,
	decoded: boolean,
	maxBytes = 16 * 1024 * 1024,
): value is DatasetAsset {
	if (!isAsset(value, maxBytes)) return false
	const match = pathPattern.exec(value.path)
	return (
		match?.[1] === value.sha256.slice(0, 16) &&
		(value.decodedBytes !== undefined) === decoded &&
		(value.decodedSha256 !== undefined) === decoded
	)
}

function isDetailAsset(value: unknown): value is DatasetAsset {
	if (
		!isAsset(value, 2 * 1024 * 1024) ||
		value.decodedBytes !== undefined ||
		value.decodedSha256 !== undefined
	) {
		return false
	}
	const match = /^details\/[a-f0-9]{16}\.([a-f0-9]{16})\.json$/u.exec(value.path)
	return match?.[1] === value.sha256.slice(0, 16)
}

function isCourseCode(value: string): boolean {
	return value.length > 0 && value.length <= 128 && !hasUnsafePathCharacter(value, true)
}

function hasUnsafePathCharacter(value: string, rejectSlash = false): boolean {
	for (const character of value) {
		const code = character.charCodeAt(0)
		if (character === '\\' || (rejectSlash && character === '/') || code < 0x20 || code === 0x7f) {
			return true
		}
	}
	return false
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function exactKeys(value: Record<string, unknown>, allowedKeys: readonly string[]): true {
	const allowed = new Set(allowedKeys)
	for (const key of Object.keys(value)) {
		if (!allowed.has(key)) throw new Error(`manifest.json に未知のfield ${key} があります`)
	}
	return true
}
