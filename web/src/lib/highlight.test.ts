import { describe, expect, it } from 'vitest'
import { type Span, segment } from './highlight.svelte'

/** Concatenating the segments must always reproduce the input exactly. */
function joined(text: string, spans: Span[] | undefined): string {
	return segment(text, spans)
		.map((s) => s.text)
		.join('')
}

describe('segment', () => {
	it('returns the whole string unmarked when there are no spans', () => {
		expect(segment('微分積分学', undefined)).toEqual([{ text: '微分積分学', mark: false }])
		expect(segment('微分積分学', [])).toEqual([{ text: '微分積分学', mark: false }])
	})

	it('marks a single interior run', () => {
		expect(segment('微分積分学', [{ start: 2, len: 2 }])).toEqual([
			{ text: '微分', mark: false },
			{ text: '積分', mark: true },
			{ text: '学', mark: false },
		])
	})

	it('marks a run at the very start and end', () => {
		expect(segment('AI入門', [{ start: 0, len: 2 }])).toEqual([
			{ text: 'AI', mark: true },
			{ text: '入門', mark: false },
		])
		expect(segment('入門AI', [{ start: 2, len: 2 }])).toEqual([
			{ text: '入門', mark: false },
			{ text: 'AI', mark: true },
		])
	})

	it('handles multiple, out-of-order spans', () => {
		const segs = segment('abcabc', [
			{ start: 3, len: 1 },
			{ start: 0, len: 1 },
		])
		expect(segs).toEqual([
			{ text: 'a', mark: true },
			{ text: 'bc', mark: false },
			{ text: 'a', mark: true },
			{ text: 'bc', mark: false },
		])
	})

	it('coalesces overlapping and adjacent spans without dropping text', () => {
		expect(
			joined('abcdef', [
				{ start: 1, len: 3 },
				{ start: 2, len: 3 },
			]),
		).toBe('abcdef')
		expect(
			joined('abcdef', [
				{ start: 0, len: 2 },
				{ start: 2, len: 2 },
			]),
		).toBe('abcdef')
	})

	it('clamps spans that run past the string end', () => {
		expect(segment('ab', [{ start: 1, len: 99 }])).toEqual([
			{ text: 'a', mark: false },
			{ text: 'b', mark: true },
		])
		// A span entirely out of range leaves the text plain.
		expect(segment('ab', [{ start: 5, len: 2 }])).toEqual([{ text: 'ab', mark: false }])
	})

	it('ignores zero-length spans', () => {
		expect(segment('abc', [{ start: 1, len: 0 }])).toEqual([{ text: 'abc', mark: false }])
	})

	it('always reproduces the input when concatenated', () => {
		const text = 'データ構造とアルゴリズム'
		for (const spans of [
			[{ start: 0, len: 3 }],
			[{ start: 4, len: 4 }],
			[
				{ start: 0, len: 2 },
				{ start: 7, len: 3 },
			],
			[{ start: 2, len: 100 }],
		]) {
			expect(joined(text, spans)).toBe(text)
		}
	})
})
