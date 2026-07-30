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
		expect(decodePlan(encodePlan(['a', 'b', 'a', 'c']))).toEqual(['a', 'b', 'c'])
	})

	it('rejects junk, old versions, and empty entries', () => {
		expect(() => decodePlan('garbage')).toThrow()
		expect(() => decodePlan('.abc')).toThrow()
		expect(() => decodePlan('1.a')).toThrow(/バージョン/)
		expect(() => decodePlan('2.')).toThrow()
		expect(() => decodePlan('2.a~~b')).toThrow()
	})

	it('rejects unsupported future tokens instead of silently truncating', () => {
		expect(() => decodePlan('3.a~b')).toThrow(/バージョン/)
	})

	it('survives reserved URL characters while rejecting path separators', () => {
		const cds = ['a-b', 'e f', '10%']
		expect(decodePlan(encodePlan(cds))).toEqual(cds)
		expect(() => encodePlan(['c/d'])).toThrow(/科目コード/)
	})

	it('round-trips any list of non-empty codes (property)', () => {
		fc.assert(
			fc.property(
				fc.array(
					fc
						.string({ minLength: 1, maxLength: 8 })
						.filter(
							(s) =>
								s.trim().length > 0 &&
								![...s].some((c) => c === '/' || c === '\\' || c.charCodeAt(0) < 0x20),
						),
				),
				(raw) => {
					// The store never holds duplicates; mirror that for the round-trip.
					const cds = [...new Set(raw)]
					expect(decodePlan(encodePlan(cds))).toEqual(cds)
				},
			),
		)
	})
})
