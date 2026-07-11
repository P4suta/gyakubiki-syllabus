import fc from 'fast-check'
import { describe, expect, it } from 'vitest'
import { decodePlan, encodePlan, PLAN_SCHEMA_VERSION } from './plan-codec'

describe('plan-codec', () => {
	it('round-trips a list of codes', () => {
		const cds = ['00001', '12345', '99999']
		expect(decodePlan(encodePlan(cds))).toEqual(cds)
	})

	it('encodes the empty plan as the empty string', () => {
		expect(encodePlan([])).toBe('')
		expect(decodePlan('')).toEqual([])
	})

	it('carries the schema version', () => {
		expect(encodePlan(['1'])).toBe(`${PLAN_SCHEMA_VERSION}.1`)
	})

	it('de-duplicates while preserving first-seen order', () => {
		expect(decodePlan('1.a-b-a-c')).toEqual(['a', 'b', 'c'])
	})

	it('tolerates junk, missing version, and empty entries', () => {
		expect(decodePlan('garbage')).toEqual([])
		expect(decodePlan('.abc')).toEqual([])
		expect(decodePlan('1.')).toEqual([])
		expect(decodePlan('1.a--b')).toEqual(['a', 'b'])
	})

	it('reads the known codes from a newer, additive token', () => {
		// A future v2 appends `;color=...`; a v1 decoder still recovers the codes.
		expect(decodePlan('2.a-b;color=red')).toEqual(['a', 'b'])
	})

	it('survives codes containing the separator or reserved chars', () => {
		const cds = ['a-b', 'c/d', 'e f', '10%']
		expect(decodePlan(encodePlan(cds))).toEqual(cds)
	})

	it('round-trips any list of non-empty codes (property)', () => {
		fc.assert(
			fc.property(
				fc.array(fc.string({ minLength: 1, maxLength: 8 }).filter((s) => s.trim().length > 0)),
				(raw) => {
					// The store never holds duplicates; mirror that for the round-trip.
					const cds = [...new Set(raw)]
					expect(decodePlan(encodePlan(cds))).toEqual(cds)
				},
			),
		)
	})
})
