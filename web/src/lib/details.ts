import type { CourseDetail } from '../types/course'
import { currentManifest, detailAsset, fetchVerifiedAsset } from './dataset'

const cache = new Map<string, CourseDetail>()

export class DetailUnavailableError extends Error {
	constructor() {
		super('この科目の詳細情報は取得元にありません')
		this.name = 'DetailUnavailableError'
	}
}

export async function loadDetail(cd: string, signal?: AbortSignal): Promise<CourseDetail> {
	const manifest = currentManifest()
	if (!manifest) throw new Error('dataset manifestがまだ読み込まれていません')
	const cacheKey = `${manifest.datasetId}:${cd}`
	const cached = cache.get(cacheKey)
	if (cached) return cached

	const asset = await detailAsset(manifest, cd, signal)
	if (!asset) throw new DetailUnavailableError()
	const buffer = await fetchVerifiedAsset(manifest, asset, signal)
	let value: unknown
	try {
		value = JSON.parse(new TextDecoder().decode(buffer))
	} catch {
		throw new Error('詳細データのJSONが壊れています')
	}
	const detail = validateCourseDetail(value)
	if (detail.cd !== cd) throw new Error('詳細データの科目コードが一致しません')
	cache.set(cacheKey, detail)
	return detail
}

/** Strict runtime boundary for a manifest-addressed public detail asset. */
export function validateCourseDetail(value: unknown): CourseDetail {
	const detail = recordWithKeys(value, [
		'cd',
		'unit',
		'delivery',
		'eval',
		'summary',
		'aims',
		'goals',
		'plan',
		'textbooks',
		'prereq',
		'prep',
		'officeHour',
		'keywords',
		'teachers',
		'numbering',
		'sdgs',
		'extra',
		'textbookInfo',
		'prepInfo',
	])
	if (!courseCode(detail.cd)) throw new Error('詳細データの科目コードが不正です')
	for (const key of ['unit', 'summary', 'aims', 'textbooks', 'prereq', 'prep'] as const) {
		optionalString(detail[key], key)
	}
	if (detail.delivery !== undefined) validateDelivery(detail.delivery)
	if (detail.eval !== undefined) validateEval(detail.eval)
	stringArray(detail.goals, 'goals', 256)
	arrayOf(detail.plan, 'plan', 512, validatePlanItem)
	arrayOf(detail.officeHour, 'officeHour', 256, validateOfficeHour)
	stringArray(detail.keywords, 'keywords', 512)
	stringArray(detail.teachers, 'teachers', 512)
	stringArray(detail.numbering, 'numbering', 512)
	stringArray(detail.sdgs, 'sdgs', 512)
	arrayOf(detail.extra, 'extra', 512, validateLabelled)
	if (detail.textbookInfo !== undefined) validateTextbookInfo(detail.textbookInfo)
	if (detail.prepInfo !== undefined) validatePrepInfo(detail.prepInfo)
	return detail as unknown as CourseDetail
}

function validateDelivery(value: unknown): void {
	const delivery = recordWithKeys(value, ['mode', 'raw', 'isMedia'])
	if (!['onsite', 'online', 'ondemand', 'hybrid', 'unknown'].includes(String(delivery.mode))) {
		throw new Error('詳細データのdelivery.modeが不正です')
	}
	optionalString(delivery.raw, 'delivery.raw')
	if (delivery.isMedia !== undefined && typeof delivery.isMedia !== 'boolean') {
		throw new Error('詳細データのdelivery.isMediaが不正です')
	}
}

function validateEval(value: unknown): void {
	const evaluation = recordWithKeys(value, ['rows', 'note'])
	arrayOf(evaluation.rows, 'eval.rows', 256, (row) => {
		const item = recordWithKeys(row, ['item', 'weight', 'type'])
		requiredString(item.item, 'eval.rows.item')
		requiredString(item.type, 'eval.rows.type')
		if (
			item.weight !== undefined &&
			(typeof item.weight !== 'number' ||
				!Number.isSafeInteger(item.weight) ||
				Math.abs(item.weight) > 1_000_000)
		) {
			throw new Error('詳細データのeval.rows.weightが不正です')
		}
	})
	optionalString(evaluation.note, 'eval.note')
}

function validatePlanItem(value: unknown): void {
	const item = recordWithKeys(value, ['n', 'text', 'kind'])
	if (!Number.isSafeInteger(item.n) || (item.n as number) < 0 || (item.n as number) > 10_000) {
		throw new Error('詳細データのplan.nが不正です')
	}
	requiredString(item.text, 'plan.text')
	if (item.kind !== undefined && !['exam', 'milestone', 'start'].includes(String(item.kind))) {
		throw new Error('詳細データのplan.kindが不正です')
	}
}

function validateOfficeHour(value: unknown): void {
	const item = recordWithKeys(value, ['name', 'day', 'time', 'place'])
	for (const key of ['name', 'day', 'time', 'place'] as const) {
		optionalString(item[key], `officeHour.${key}`)
	}
}

function validateLabelled(value: unknown): void {
	const item = recordWithKeys(value, ['label', 'text'])
	requiredString(item.label, 'extra.label')
	requiredString(item.text, 'extra.text')
}

function validateTextbookInfo(value: unknown): void {
	const info = recordWithKeys(value, ['isNone', 'sections'])
	if (typeof info.isNone !== 'boolean') throw new Error('詳細データのtextbookInfo.isNoneが不正です')
	arrayOf(info.sections, 'textbookInfo.sections', 256, (section) => {
		const item = recordWithKeys(section, ['label', 'lines'])
		optionalString(item.label, 'textbookInfo.sections.label')
		stringArray(item.lines, 'textbookInfo.sections.lines', 512, true)
	})
}

function validatePrepInfo(value: unknown): void {
	const info = recordWithKeys(value, ['hours', 'yoshu', 'fukushu'])
	if (
		info.hours !== undefined &&
		(typeof info.hours !== 'number' ||
			!Number.isFinite(info.hours) ||
			Math.abs(info.hours) > 10_000)
	) {
		throw new Error('詳細データのprepInfo.hoursが不正です')
	}
	optionalString(info.yoshu, 'prepInfo.yoshu')
	optionalString(info.fukushu, 'prepInfo.fukushu')
}

function stringArray(value: unknown, name: string, max: number, required = false): void {
	if (value === undefined && !required) return
	arrayOf(value, name, max, (item) => requiredString(item, name))
}

function arrayOf(
	value: unknown,
	name: string,
	max: number,
	validate: (item: unknown) => void,
): void {
	if (value === undefined) return
	if (!Array.isArray(value) || value.length > max) throw new Error(`詳細データの${name}が不正です`)
	for (const item of value) validate(item)
}

function requiredString(value: unknown, name: string): asserts value is string {
	if (typeof value !== 'string' || value.length === 0 || value.length > 1_000_000) {
		throw new Error(`詳細データの${name}が不正です`)
	}
}

function optionalString(value: unknown, name: string): void {
	if (value !== undefined) requiredString(value, name)
}

function courseCode(value: unknown): value is string {
	if (typeof value !== 'string' || value.length === 0 || value.length > 128) return false
	for (const character of value) {
		const code = character.charCodeAt(0)
		if (character === '/' || character === '\\' || code < 0x20 || code === 0x7f) return false
	}
	return true
}

function recordWithKeys(value: unknown, keys: readonly string[]): Record<string, unknown> {
	if (typeof value !== 'object' || value === null || Array.isArray(value)) {
		throw new Error('詳細データにobject以外の値があります')
	}
	const record = value as Record<string, unknown>
	const allowed = new Set(keys)
	for (const key of Object.keys(record)) {
		if (!allowed.has(key)) throw new Error(`詳細データに未知のfield ${key} があります`)
	}
	return record
}
