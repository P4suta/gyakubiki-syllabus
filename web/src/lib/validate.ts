import type { ProcessedDataV2 } from '../types/course'

export type ValidationResult =
	| { ok: true; data: ProcessedDataV2 }
	| { ok: false; error: string }

export function validateProcessedData(parsed: unknown): ValidationResult {
	if (parsed === null || parsed === undefined) {
		return { ok: false, error: 'データが空です。ファイルの内容を確認してください。' }
	}

	if (typeof parsed !== 'object') {
		return { ok: false, error: 'JSONオブジェクトが期待されましたが、別の形式でした。' }
	}

	const obj = parsed as Record<string, unknown>

	if ('selectKogiDtoList' in obj) {
		return {
			ok: false,
			error:
				'これはKULASのAPIレスポンス(未処理)です。\n' +
				'先に syllabus-cli convert --v2 で変換してください。\n' +
				'  例: syllabus-cli convert --v2 raw.json -o data.json',
		}
	}

	if (!('version' in obj) || typeof obj.version !== 'number') {
		return {
			ok: false,
			error:
				'syllabus-cliの出力形式ではありません。\n' +
				'"version" フィールドが見つかりません。\n' +
				'syllabus-cli convert --v2 で変換したdata.jsonを読み込んでください。',
		}
	}

	if (obj.version !== 2) {
		return {
			ok: false,
			error:
				`version ${String(obj.version)} は対応していません。version 2 が必要です。\n` +
				'syllabus-cli convert --v2 で再変換してください。',
		}
	}

	if (!('courses' in obj) || !Array.isArray(obj.courses)) {
		return {
			ok: false,
			error:
				'"courses" フィールドが配列ではありません。\n' +
				'ファイルが破損している可能性があります。再度変換を試してください。',
		}
	}

	const courses = obj.courses as Record<string, unknown>[]
	if (courses.length > 0) {
		const first = courses[0]
		if (!first.cd || !first.nm) {
			return {
				ok: false,
				error:
					'courses配列の要素にcdまたはnmがありません。\n' +
					'v2フォーマットではない可能性があります。--v2 フラグで再変換してください。',
			}
		}
	}

	if (!('dicts' in obj) || typeof obj.dicts !== 'object' || obj.dicts === null) {
		return {
			ok: false,
			error: '"dicts" フィールドがありません。v2フォーマットが必要です。',
		}
	}

	const dicts = obj.dicts as Record<string, unknown>
	for (const key of ['semesters', 'departments', 'campuses', 'kubun', 'kaikojiki']) {
		if (!Array.isArray(dicts[key])) {
			return {
				ok: false,
				error: `"dicts.${key}" フィールドが配列ではありません。ファイルが破損している可能性があります。`,
			}
		}
	}

	if (!('indices' in obj) || typeof obj.indices !== 'object' || obj.indices === null) {
		return {
			ok: false,
			error: '"indices" フィールドがありません。v2フォーマットが必要です。',
		}
	}

	return { ok: true, data: parsed as ProcessedDataV2 }
}
