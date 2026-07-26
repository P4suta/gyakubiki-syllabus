import { describe, expect, it } from 'vitest'
import { isWorkerReply, isWorkerRequest } from './worker-protocol'

const asset = {
	path: 'search.aaaaaaaaaaaaaaaa.idx.br',
	bytes: 1,
	sha256: 'a'.repeat(64),
	decodedBytes: 1,
	decodedSha256: 'b'.repeat(64),
}

describe('worker protocol validation', () => {
	it('accepts only exact discriminated request shapes', () => {
		expect(
			isWorkerRequest({
				id: 1,
				type: 'init',
				buffer: new ArrayBuffer(0),
				datasetId: 'c'.repeat(64),
				indexUrl: '/dataset/search',
				indexAsset: asset,
			}),
		).toBe(true)
		expect(
			isWorkerRequest({
				id: 1,
				type: 'query',
				semester: 'all',
				department: 'all',
				campus: 'all',
				query: 'a'.repeat(257),
			}),
		).toBe(false)
		expect(
			isWorkerRequest({
				id: 1,
				type: 'init',
				buffer: new ArrayBuffer(0),
				datasetId: 'C'.repeat(64),
				indexUrl: '/dataset/search',
				indexAsset: asset,
			}),
		).toBe(false)
		expect(
			isWorkerRequest({
				id: 1,
				type: 'init',
				buffer: new ArrayBuffer(0),
				datasetId: 'c'.repeat(64),
				indexUrl: '/dataset/search',
				indexAsset: { ...asset, path: 'search.bbbbbbbbbbbbbbbb.idx.br' },
			}),
		).toBe(false)
		expect(isWorkerRequest({ id: 1, type: 'dispose', injected: true })).toBe(false)
	})

	it('rejects extra reply fields and oversized errors', () => {
		expect(isWorkerReply({ id: 1, ok: true, result: {}, extra: true })).toBe(false)
		expect(
			isWorkerReply({
				id: 1,
				ok: false,
				error: 'x'.repeat(16_385),
				retryable: true,
			}),
		).toBe(false)
		expect(isWorkerReply({ id: 1, ok: false, error: 'failed', retryable: true })).toBe(true)
	})
})
