// Search-match highlighting: the pure `segment` splitter the cards use to render
// <mark> runs, plus a small reactive store of the active query's name-field
// spans, keyed by course `cd`. Keying by `cd` lets any card read its own
// highlight without the grid layout having to thread span data through — and a
// course highlights consistently wherever it appears in the timetable.

export interface Seg {
	text: string
	mark: boolean
}

/** A UTF-16 `[start, start + len)` match range within a display string. */
export interface Span {
	start: number
	len: number
}

/**
 * Split `text` into marked / unmarked runs at the given spans. Spans are clamped
 * to the string and coalesced where they touch, so the output tiles the string
 * exactly once. JS string offsets are UTF-16 code units — the same unit the core
 * reports — so `slice` lands on the intended characters.
 */
export function segment(text: string, spans: readonly Span[] | undefined): Seg[] {
	if (!spans || spans.length === 0) return [{ text, mark: false }]

	const sorted = [...spans].filter((s) => s.len > 0).sort((a, b) => a.start - b.start)
	const segs: Seg[] = []
	let cursor = 0
	for (const s of sorted) {
		const start = Math.min(Math.max(s.start, cursor), text.length)
		const end = Math.min(s.start + s.len, text.length)
		if (end <= start) continue // fully before the cursor (overlap) or out of range
		if (start > cursor) segs.push({ text: text.slice(cursor, start), mark: false })
		segs.push({ text: text.slice(start, end), mark: true })
		cursor = end
	}
	if (cursor < text.length) segs.push({ text: text.slice(cursor), mark: false })
	return segs.length > 0 ? segs : [{ text, mark: false }]
}

/**
 * The active query's course-name match spans, keyed by course `cd`. A reactive
 * singleton (the codebase's store pattern, cf. breakpoint/theme): the query
 * effect replaces the whole map, and every mounted card re-derives its own runs.
 */
class Highlights {
	#byCd = $state(new Map<string, Span[]>())

	get(cd: string): Span[] | undefined {
		return this.#byCd.get(cd)
	}

	set(byCd: Map<string, Span[]>): void {
		this.#byCd = byCd
	}
}

export const highlights = new Highlights()
