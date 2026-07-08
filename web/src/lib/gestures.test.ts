import { afterEach, describe, expect, it, vi } from 'vitest'
import { haptic, rubberBand, shouldCommit } from './gestures'

describe('shouldCommit', () => {
	const size = 400

	it('commits when the drag passes the distance ratio', () => {
		expect(shouldCommit(101, 0, size, { distanceRatio: 0.25 })).toBe(true) // >100
		expect(shouldCommit(99, 0, size, { distanceRatio: 0.25 })).toBe(false)
	})

	it('commits on a fast flick regardless of distance', () => {
		expect(shouldCommit(10, 0.6, size, { velocityThreshold: 0.5 })).toBe(true)
		expect(shouldCommit(10, 0.4, size, { velocityThreshold: 0.5 })).toBe(false)
	})

	it('uses magnitudes only (direction is the caller’s concern)', () => {
		// Same magnitude → same decision.
		expect(shouldCommit(150, 0, size)).toBe(shouldCommit(150, 0, size))
	})
})

describe('rubberBand', () => {
	it('passes through non-positive overshoot unchanged', () => {
		expect(rubberBand(0)).toBe(0)
		expect(rubberBand(-5)).toBe(-5)
	})

	it('is monotonically increasing and bounded by the dimension', () => {
		const dim = 400
		let prev = 0
		for (let x = 1; x <= 2000; x += 50) {
			const y = rubberBand(x, dim)
			expect(y).toBeGreaterThan(prev) // strictly increasing
			expect(y).toBeLessThan(dim) // asymptotically bounded
			prev = y
		}
	})

	it('resists: the damped offset is always less than the raw overshoot', () => {
		for (const x of [10, 100, 400, 1000]) {
			expect(rubberBand(x, 400)).toBeLessThan(x)
		}
	})
})

describe('haptic', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('does not throw when vibrate is unavailable', () => {
		vi.stubGlobal('navigator', {})
		expect(() => haptic()).not.toThrow()
	})

	it('calls navigator.vibrate with a short duration when available', () => {
		const vibrate = vi.fn()
		vi.stubGlobal('navigator', { vibrate })
		haptic('select')
		expect(vibrate).toHaveBeenCalledWith(4)
	})

	it('swallows errors thrown by vibrate', () => {
		vi.stubGlobal('navigator', {
			vibrate: () => {
				throw new Error('blocked')
			},
		})
		expect(() => haptic()).not.toThrow()
	})
})
