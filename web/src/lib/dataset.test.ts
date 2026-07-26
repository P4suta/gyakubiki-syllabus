import { afterEach, describe, expect, it, vi } from 'vitest'
import { loadManifest, readBoundedResponse } from './dataset'

const hash = 'a'.repeat(64)
const manifest = {
	schemaVersion: 4,
	appCompatVersion: 4,
	datasetId: hash,
	sourceCommit: '0123456789abcdef0123456789abcdef01234567',
	year: '2026',
	generatedAt: '2026-07-26T00:00:00+09:00',
	basePath: `datasets/${hash}/`,
	counts: {
		courses: 1,
		details: 1,
		detailCoverage: 1,
		scheduledCourses: 1,
		unscheduledCourses: 0,
	},
	range: { minDay: 0, maxDay: 0, minPeriod: 1, maxPeriod: 1 },
	assets: {
		data: { path: 'data.aaaaaaaaaaaaaaaa.json', bytes: 1, sha256: hash },
		index: {
			path: 'search.aaaaaaaaaaaaaaaa.idx.br',
			bytes: 1,
			sha256: hash,
			decodedBytes: 1,
			decodedSha256: hash,
		},
		details: { path: 'details.aaaaaaaaaaaaaaaa.json', bytes: 1, sha256: hash },
	},
}

function reply(value: unknown) {
	const fetch = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => {
		return new Response(JSON.stringify(value), { status: 200 })
	})
	vi.stubGlobal('fetch', fetch)
	return fetch
}

afterEach(() => {
	vi.unstubAllGlobals()
})

describe('dataset manifest boundary', () => {
	it('accepts one strict v4 generation and fetches only the stable manifest URL', async () => {
		const fetch = reply(manifest)
		await expect(loadManifest()).resolves.toMatchObject({
			datasetId: hash,
			sourceCommit: manifest.sourceCommit,
		})
		expect(fetch).toHaveBeenCalledOnce()
		expect(fetch.mock.calls[0]?.[0]).toMatch(/manifest\.json$/u)
		expect(fetch.mock.calls[0]?.[1]).toMatchObject({ cache: 'no-store' })
	})

	it.each([
		['short source commit', { ...manifest, sourceCommit: 'short' }],
		['uppercase dataset ID', { ...manifest, datasetId: hash.toUpperCase() }],
		['non-RFC3339 generated time', { ...manifest, generatedAt: '2026-07-26' }],
		['unknown manifest field', { ...manifest, sessionToken: 'private' }],
		['mismatched generation path', { ...manifest, basePath: 'datasets/other/' }],
		[
			'asset filename that does not match its hash',
			{
				...manifest,
				assets: {
					...manifest.assets,
					data: { ...manifest.assets.data, path: 'data.bbbbbbbbbbbbbbbb.json' },
				},
			},
		],
		[
			'search index without decoded identity',
			{
				...manifest,
				assets: {
					...manifest.assets,
					index: {
						path: manifest.assets.index.path,
						bytes: 1,
						sha256: hash,
					},
				},
			},
		],
		[
			'oversized search transport',
			{
				...manifest,
				assets: {
					...manifest.assets,
					index: { ...manifest.assets.index, bytes: 5 * 1024 * 1024 },
				},
			},
		],
	])('rejects %s', async (_name, value) => {
		reply(value)
		await expect(loadManifest()).rejects.toThrow()
	})

	it('bounds response bodies even when Content-Length is absent or dishonest', async () => {
		await expect(readBoundedResponse(new Response('abc'), 3, 'fixture')).resolves.toHaveProperty(
			'byteLength',
			3,
		)
		await expect(readBoundedResponse(new Response('abcd'), 3, 'fixture')).rejects.toThrow(
			'サイズ上限',
		)
		await expect(
			readBoundedResponse(
				new Response('a', { headers: { 'content-length': '999' } }),
				3,
				'fixture',
			),
		).rejects.toThrow('Content-Length')
	})
})
