/**
 * BitSet backed by Uint32Array for efficient bitwise operations.
 * Uses 32-bit words since JavaScript bitwise ops work on 32-bit integers.
 */
export class BitSet {
	readonly words: Uint32Array

	private constructor(words: Uint32Array) {
		this.words = words
	}

	/** Decode a base64-encoded little-endian bitset (from Go's []uint64 encoding). */
	static fromBase64(encoded: string): BitSet {
		const binary = atob(encoded)
		const bytes = new Uint8Array(binary.length)
		for (let i = 0; i < binary.length; i++) {
			bytes[i] = binary.charCodeAt(i)
		}
		// Convert bytes to Uint32Array (little-endian, which is native on most platforms)
		const words = new Uint32Array(bytes.buffer)
		return new BitSet(words)
	}

	/** Create a bitset with bits 0..n-1 all set. */
	static allOnes(n: number): BitSet {
		const numWords = Math.ceil(n / 32)
		const words = new Uint32Array(numWords)
		const fullWords = Math.floor(n / 32)
		for (let i = 0; i < fullWords; i++) {
			words[i] = 0xffffffff
		}
		const remainder = n % 32
		if (remainder > 0) {
			words[fullWords] = (1 << remainder) - 1
		}
		return new BitSet(words)
	}

	/** Test if bit at position i is set. */
	has(i: number): boolean {
		const wordIdx = i >>> 5 // i / 32
		if (wordIdx >= this.words.length) return false
		return (this.words[wordIdx] & (1 << (i & 31))) !== 0
	}

	/** Return a new BitSet that is the AND of this and other. */
	and(other: BitSet): BitSet {
		const len = Math.min(this.words.length, other.words.length)
		const result = new Uint32Array(len)
		for (let i = 0; i < len; i++) {
			result[i] = this.words[i] & other.words[i]
		}
		return new BitSet(result)
	}

	/** Return sorted array of indices where bits are set. */
	popIndices(): number[] {
		const indices: number[] = []
		for (let w = 0; w < this.words.length; w++) {
			let word = this.words[w]
			const base = w << 5 // w * 32
			while (word !== 0) {
				const bit = word & (-word) // isolate lowest set bit
				const pos = 31 - Math.clz32(bit)
				indices.push(base + pos)
				word ^= bit // clear lowest set bit
			}
		}
		return indices
	}

	/** Count the number of set bits. */
	popcount(): number {
		let count = 0
		for (let i = 0; i < this.words.length; i++) {
			let x = this.words[i]
			// Brian Kernighan's algorithm
			while (x !== 0) {
				x &= x - 1
				count++
			}
		}
		return count
	}
}
