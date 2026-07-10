// Pure geometry/normalization behind the assessment ratio donut (EvalChart).
// Extracted from the component so it can be property-tested without a DOM.

export interface Weighted {
	weight?: number
}

/** The circumference of a circle of the given radius. */
export function circumference(radius: number): number {
	return 2 * Math.PI * radius
}

/**
 * Resolve each row's share as a rounded percentage. With weights, shares are
 * proportional to weight; with none, they split equally (so a weightless
 * breakdown still communicates the mix). `hasWeight` reflects which path ran.
 */
export function evalSegments<T extends Weighted>(
	rows: readonly T[],
): (T & { pct: number; hasWeight: boolean })[] {
	const sum = rows.reduce((acc, r) => acc + (r.weight ?? 0), 0)
	const hasWeight = sum > 0
	const total = hasWeight ? sum : rows.length
	return rows.map((r) => {
		const value = hasWeight ? (r.weight ?? 0) : 1
		const pct = total > 0 ? Math.round((value / total) * 100) : 0
		return { ...r, pct, hasWeight }
	})
}

export interface Typed {
	type: string
	pct: number
}

/**
 * Sum shares by assessment `type`, largest first — so the "main" axis reflects
 * the category (中間レポート30 + 期末レポート30 = レポート60 beats 出席40), not the
 * single biggest row.
 */
export function sumByType<T extends Typed>(segs: readonly T[]): Typed[] {
	const map = new Map<string, number>()
	for (const s of segs) map.set(s.type, (map.get(s.type) ?? 0) + s.pct)
	return [...map.entries()]
		.map(([type, pct]) => ({ type, pct }))
		.sort((a, b) => b.pct - a.pct)
}

export interface Arc {
	/** `stroke-dasharray`: drawn length then the gap. */
	dash: string
	/** `stroke-dashoffset`: negative cumulative start of this arc. */
	offset: number
}

/**
 * Stacked donut arcs for the given percentages: each arc's drawn length is its
 * share of the circumference, offset so they chain end-to-end around the ring.
 */
export function evalArcs(pcts: readonly number[], radius: number): Arc[] {
	const c = circumference(radius)
	let offset = 0
	return pcts.map((pct) => {
		const len = (pct / 100) * c
		const arc: Arc = { dash: `${len} ${c - len}`, offset: -offset }
		offset += len
		return arc
	})
}
