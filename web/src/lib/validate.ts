import type { ProcessedData } from '../types/course'

export type ValidationResult =
	| { ok: true; data: ProcessedData }
	| { ok: false; error: string }

export function validateProcessedData(parsed: unknown): ValidationResult {
	if (parsed === null || parsed === undefined) {
		return { ok: false, error: 'データが空です。ファイルの内容を確認してください。' }
	}

	if (typeof parsed !== 'object') {
		return { ok: false, error: 'JSONオブジェクトが期待されましたが、別の形式でした。' }
	}

	const obj = parsed as Record<string, unknown>

	// Detect raw API response (not yet processed by syllabus-cli)
	if ('selectKogiDtoList' in obj) {
		return {
			ok: false,
			error:
				'これはKULASのAPIレスポンス(未処理)です。\n' +
				'先に syllabus-cli convert で変換してください。\n' +
				'  例: syllabus-cli convert raw.json -o data.json',
		}
	}

	// Check for version field
	if (!('version' in obj) || typeof obj.version !== 'number') {
		return {
			ok: false,
			error:
				'syllabus-cliの出力形式ではありません。\n' +
				'"version" フィールドが見つかりません。\n' +
				'syllabus-cli convert で変換したdata.jsonを読み込んでください。',
		}
	}

	// Check for courses field
	if (!('courses' in obj) || !Array.isArray(obj.courses)) {
		return {
			ok: false,
			error:
				'syllabus-cliの出力形式ですが、"courses" フィールドが配列ではありません。\n' +
				'ファイルが破損している可能性があります。再度変換を試してください。',
		}
	}

	// Validate at least some courses have required fields
	const courses = obj.courses as Record<string, unknown>[]
	if (courses.length > 0) {
		const first = courses[0]
		if (!first.kogiCd || !first.kogiNm) {
			return {
				ok: false,
				error:
					'courses配列の要素にkogiCdまたはkogiNmがありません。\n' +
					'ファイルが破損している可能性があります。',
			}
		}
	}

	// Validate semesters and departments exist
	const data = parsed as ProcessedData
	if (!Array.isArray(data.semesters)) {
		return {
			ok: false,
			error: '"semesters" フィールドが配列ではありません。ファイルが破損している可能性があります。',
		}
	}
	if (!Array.isArray(data.departments)) {
		return {
			ok: false,
			error:
				'"departments" フィールドが配列ではありません。ファイルが破損している可能性があります。',
		}
	}

	return { ok: true, data }
}
