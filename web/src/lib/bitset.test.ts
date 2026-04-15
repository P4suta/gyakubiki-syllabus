import { describe, expect, it } from 'vitest'
import { BitSet } from './bitset'

describe('BitSet', () => {
	it('creates from base64', () => {
		// 8 bytes = 1 uint64, little-endian: 0x05 = bits 0 and 2 set
		const b64 = btoa(String.fromCharCode(5, 0, 0, 0, 0, 0, 0, 0))
		const bs = BitSet.fromBase64(b64)
		expect(bs.has(0)).toBe(true)
		expect(bs.has(1)).toBe(false)
		expect(bs.has(2)).toBe(true)
		expect(bs.has(3)).toBe(false)
	})

	it('creates allOnes bitset', () => {
		const bs = BitSet.allOnes(10)
		for (let i = 0; i < 10; i++) {
			expect(bs.has(i)).toBe(true)
		}
	})

	it('AND operation', () => {
		// a = bits 0,1,2 set
		const a = BitSet.fromBase64(btoa(String.fromCharCode(7, 0, 0, 0, 0, 0, 0, 0)))
		// b = bits 1,2,3 set
		const b = BitSet.fromBase64(btoa(String.fromCharCode(14, 0, 0, 0, 0, 0, 0, 0)))
		const result = a.and(b)
		expect(result.has(0)).toBe(false)
		expect(result.has(1)).toBe(true)
		expect(result.has(2)).toBe(true)
		expect(result.has(3)).toBe(false)
	})

	it('popIndices returns set bit positions', () => {
		// bits 0, 2, 5 set = 0b100101 = 37
		const bs = BitSet.fromBase64(btoa(String.fromCharCode(37, 0, 0, 0, 0, 0, 0, 0)))
		expect(bs.popIndices()).toEqual([0, 2, 5])
	})

	it('popcount returns number of set bits', () => {
		// bits 0, 2, 5 set = 3 bits
		const bs = BitSet.fromBase64(btoa(String.fromCharCode(37, 0, 0, 0, 0, 0, 0, 0)))
		expect(bs.popcount()).toBe(3)
	})

	it('handles empty bitset', () => {
		const bs = BitSet.fromBase64(btoa(String.fromCharCode(0, 0, 0, 0, 0, 0, 0, 0)))
		expect(bs.popIndices()).toEqual([])
		expect(bs.popcount()).toBe(0)
	})

	it('handles multi-word bitset (>64 items)', () => {
		// Two words: first all zeros, second has bit 0 set (= item 32 overall using Uint32Array)
		const bytes = new Uint8Array(16)
		bytes[8] = 1 // bit 64 in uint64 terms, but we use Uint32Array so bit 64 = word[2] bit 0
		const b64 = btoa(String.fromCharCode(...bytes))
		const bs = BitSet.fromBase64(b64)
		expect(bs.has(64)).toBe(true)
		expect(bs.has(63)).toBe(false)
		expect(bs.has(65)).toBe(false)
	})

	it('AND with allOnes returns the original', () => {
		const bs = BitSet.fromBase64(btoa(String.fromCharCode(42, 0, 0, 0, 0, 0, 0, 0)))
		const all = BitSet.allOnes(8)
		const result = bs.and(all)
		expect(result.popIndices()).toEqual(bs.popIndices())
	})
})
