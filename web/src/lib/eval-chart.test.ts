import fc from 'fast-check'
import { describe, expect, it } from 'vitest'
import { circumference, evalArcs, evalSegments } from './eval-chart'

describe('evalSegments', () => {
	it('every share is a percentage in [0,100] with consistent hasWeight', () => {
		fc.assert(
			fc.property(
				fc.array(
					fc.record({
						weight: fc.option(fc.integer({ min: 0, max: 1000 }), { nil: undefined }),
					}),
					{ minLength: 1, maxLength: 8 },
				),
				(rows) => {
					const segs = evalSegments(rows)
					const anyWeight = rows.some((r) => (r.weight ?? 0) > 0)
					for (const s of segs) {
						expect(s.pct).toBeGreaterThanOrEqual(0)
						expect(s.pct).toBeLessThanOrEqual(100)
						expect(s.hasWeight).toBe(anyWeight)
					}
				},
			),
		)
	})

	it('with weights, rounded shares sum to ~100 (drift ≤ row count)', () => {
		fc.assert(
			fc.property(
				fc.array(fc.integer({ min: 1, max: 1000 }), { minLength: 1, maxLength: 8 }),
				(ws) => {
					const sum = evalSegments(ws.map((w) => ({ weight: w }))).reduce((a, s) => a + s.pct, 0)
					expect(Math.abs(sum - 100)).toBeLessThanOrEqual(ws.length)
				},
			),
		)
	})

	it('weightless rows split equally', () => {
		const segs = evalSegments([{}, {}, {}, {}])
		expect(segs.map((s) => s.pct)).toEqual([25, 25, 25, 25])
		expect(segs.every((s) => !s.hasWeight)).toBe(true)
	})

	it('empty input yields no segments', () => {
		expect(evalSegments([])).toEqual([])
	})
})

describe('evalArcs', () => {
	it('drawn length ∈ [0, circumference]; offsets chain monotonically', () => {
		fc.assert(
			fc.property(
				fc.array(fc.integer({ min: 0, max: 100 }), { maxLength: 8 }),
				fc.integer({ min: 1, max: 200 }),
				(pcts, radius) => {
					const c = circumference(radius)
					const arcs = evalArcs(pcts, radius)
					expect(arcs).toHaveLength(pcts.length)
					let prev = 0
					for (const arc of arcs) {
						const [len, gap] = arc.dash.split(' ').map(Number)
						expect(len).toBeGreaterThanOrEqual(0)
						expect(len).toBeLessThanOrEqual(c + 1e-6)
						// dashed length + gap span exactly one circumference
						expect(len + gap).toBeCloseTo(c)
						expect(arc.offset).toBeLessThanOrEqual(prev + 1e-9) // non-increasing
						prev = arc.offset
					}
				},
			),
		)
	})
})
