import type { DatasetAsset } from './dataset'

export type WorkerRequest =
	| {
			id: number
			type: 'init'
			buffer: ArrayBuffer
			datasetId: string
			indexUrl: string
			indexAsset: DatasetAsset
	  }
	| {
			id: number
			type: 'query'
			semester: string
			department: string
			campus: string
			query: string
	  }
	| { id: number; type: 'retrySearchIndex' }
	| { id: number; type: 'plan'; cds: string[]; semester: string }
	| { id: number; type: 'dispose' }

export type WorkerReply =
	| { id: number; ok: true; result: unknown }
	| { id: number; ok: false; error: string; retryable: boolean }

export function isWorkerReply(value: unknown): value is WorkerReply {
	if (typeof value !== 'object' || value === null) return false
	const reply = value as Record<string, unknown>
	if (
		!Number.isSafeInteger(reply.id) ||
		(reply.id as number) < 0 ||
		typeof reply.ok !== 'boolean'
	) {
		return false
	}
	return reply.ok === true
		? hasOnlyKeys(reply, ['id', 'ok', 'result']) && 'result' in reply
		: hasOnlyKeys(reply, ['id', 'ok', 'error', 'retryable']) &&
				typeof reply.error === 'string' &&
				reply.error.length <= 16_384 &&
				typeof reply.retryable === 'boolean'
}

export function isWorkerRequest(value: unknown): value is WorkerRequest {
	if (typeof value !== 'object' || value === null) return false
	const request = value as Record<string, unknown>
	if (!Number.isSafeInteger(request.id) || typeof request.type !== 'string') return false
	switch (request.type) {
		case 'init':
			return (
				hasOnlyKeys(request, ['id', 'type', 'buffer', 'datasetId', 'indexUrl', 'indexAsset']) &&
				request.buffer instanceof ArrayBuffer &&
				typeof request.datasetId === 'string' &&
				/^[a-f0-9]{64}$/u.test(request.datasetId) &&
				typeof request.indexUrl === 'string' &&
				request.indexUrl.length > 0 &&
				request.indexUrl.length <= 4096 &&
				isAsset(request.indexAsset)
			)
		case 'query':
			return (
				hasOnlyKeys(request, ['id', 'type', 'semester', 'department', 'campus', 'query']) &&
				typeof request.semester === 'string' &&
				request.semester.length <= 4096 &&
				typeof request.department === 'string' &&
				request.department.length <= 4096 &&
				typeof request.campus === 'string' &&
				request.campus.length <= 4096 &&
				typeof request.query === 'string' &&
				request.query.length <= 256
			)
		case 'plan':
			return (
				hasOnlyKeys(request, ['id', 'type', 'cds', 'semester']) &&
				typeof request.semester === 'string' &&
				request.semester.length <= 4096 &&
				Array.isArray(request.cds) &&
				request.cds.length <= 200 &&
				request.cds.every(
					(code) =>
						typeof code === 'string' &&
						code.length > 0 &&
						code.length <= 128 &&
						!hasUnsafeCodeCharacter(code),
				)
			)
		case 'retrySearchIndex':
		case 'dispose':
			return hasOnlyKeys(request, ['id', 'type'])
		default:
			return false
	}
}

function hasUnsafeCodeCharacter(value: string): boolean {
	for (const character of value) {
		const code = character.charCodeAt(0)
		if (character === '\\' || character === '/' || code < 0x20 || code === 0x7f) return true
	}
	return false
}

function isAsset(value: unknown): value is DatasetAsset {
	if (typeof value !== 'object' || value === null) return false
	const asset = value as Record<string, unknown>
	return (
		hasOnlyKeys(asset, ['path', 'bytes', 'sha256', 'decodedBytes', 'decodedSha256']) &&
		typeof asset.path === 'string' &&
		typeof asset.bytes === 'number' &&
		Number.isSafeInteger(asset.bytes) &&
		asset.bytes > 0 &&
		asset.bytes <= 4 * 1024 * 1024 &&
		typeof asset.sha256 === 'string' &&
		/^[a-f0-9]{64}$/u.test(asset.sha256) &&
		/^search\.([a-f0-9]{16})\.idx\.br$/u.exec(asset.path)?.[1] === asset.sha256.slice(0, 16) &&
		typeof asset.decodedBytes === 'number' &&
		Number.isSafeInteger(asset.decodedBytes) &&
		asset.decodedBytes > 0 &&
		asset.decodedBytes <= 64 * 1024 * 1024 &&
		typeof asset.decodedSha256 === 'string' &&
		/^[a-f0-9]{64}$/u.test(asset.decodedSha256)
	)
}

function hasOnlyKeys(value: Record<string, unknown>, allowedKeys: readonly string[]): boolean {
	const allowed = new Set(allowedKeys)
	return Object.keys(value).every((key) => allowed.has(key))
}
