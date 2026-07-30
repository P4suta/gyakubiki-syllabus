// Strict v2 share token: `2.<percent-encoded-code>~...`.
// Limits keep URL parsing bounded and make corruption/version mismatches visible.

export const PLAN_SCHEMA_VERSION = 2
const MAX_TOKEN_LENGTH = 8_192
const MAX_COURSES = 200
const MAX_CODE_LENGTH = 128

export class PlanDecodeError extends Error {
	constructor(message: string) {
		super(message)
		this.name = 'PlanDecodeError'
	}
}

export function encodePlan(cds: readonly string[]): string {
	if (cds.length === 0) return ''
	if (cds.length > MAX_COURSES) throw new PlanDecodeError(`履修プランは最大${MAX_COURSES}科目です`)
	const seen = new Set<string>()
	const values: string[] = []
	for (const cd of cds) {
		validateCode(cd)
		if (seen.has(cd)) continue
		seen.add(cd)
		values.push(encodeURIComponent(cd).replace(/~/g, '%7E'))
	}
	const token = `${PLAN_SCHEMA_VERSION}.${values.join('~')}`
	if (token.length > MAX_TOKEN_LENGTH) throw new PlanDecodeError('履修プランの共有URLが長すぎます')
	return token
}

export function decodePlan(token: string): string[] {
	if (!token) return []
	if (token.length > MAX_TOKEN_LENGTH) throw new PlanDecodeError('履修プランtokenが長すぎます')
	const dot = token.indexOf('.')
	if (dot < 1) throw new PlanDecodeError('履修プランtokenの形式が不正です')
	if (token.slice(0, dot) !== String(PLAN_SCHEMA_VERSION)) {
		throw new PlanDecodeError('この履修プランは対応していないバージョンです')
	}
	const payload = token.slice(dot + 1)
	if (!payload) throw new PlanDecodeError('履修プランに科目がありません')
	const parts = payload.split('~')
	if (parts.length > MAX_COURSES)
		throw new PlanDecodeError('履修プランの科目数が上限を超えています')

	const seen = new Set<string>()
	const values: string[] = []
	for (const part of parts) {
		if (!part) throw new PlanDecodeError('履修プランに空の科目コードがあります')
		let cd: string
		try {
			cd = decodeURIComponent(part)
		} catch {
			throw new PlanDecodeError('履修プランの文字encodingが壊れています')
		}
		validateCode(cd)
		if (seen.has(cd)) throw new PlanDecodeError('履修プランに重複した科目があります')
		seen.add(cd)
		values.push(cd)
	}
	return values
}

function validateCode(cd: string): void {
	const unsafe = Array.from(cd).some((character) => {
		const code = character.charCodeAt(0)
		return character === '/' || character === '\\' || code < 0x20 || code === 0x7f
	})
	if (!cd || cd.length > MAX_CODE_LENGTH || unsafe) {
		throw new PlanDecodeError('履修プランに不正な科目コードがあります')
	}
}
