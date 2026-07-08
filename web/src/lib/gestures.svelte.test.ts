import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { type SwipeNavigateOptions, swipeNavigate } from './gestures'

// swipeNavigate is a DOM action (touch listeners), so it lives in the jsdom
// project rather than the pure `gestures.test.ts`. jsdom has no TouchEvent, but
// the action only reads `touches`/`changedTouches`/`timeStamp`/`preventDefault`,
// so a plain Event with those properties attached drives every branch.

interface Pt {
	clientX: number
	clientY: number
}

function fire(
	node: HTMLElement,
	type: string,
	opts: { touches?: Pt[]; changedTouches?: Pt[]; timeStamp?: number },
): Event {
	const e = new Event(type, { bubbles: true, cancelable: true })
	Object.defineProperty(e, 'touches', { value: opts.touches ?? [] })
	Object.defineProperty(e, 'changedTouches', { value: opts.changedTouches ?? [] })
	if (opts.timeStamp !== undefined) {
		Object.defineProperty(e, 'timeStamp', { value: opts.timeStamp })
	}
	node.dispatchEvent(e)
	return e
}

let node: HTMLElement
let onDrag: ReturnType<typeof vi.fn>
let onSettle: ReturnType<typeof vi.fn>

function attach(over: Partial<SwipeNavigateOptions> = {}) {
	onDrag = vi.fn()
	onSettle = vi.fn()
	return swipeNavigate(node, {
		onDrag,
		onSettle,
		canPrev: () => true,
		canNext: () => true,
		...over,
	})
}

beforeEach(() => {
	node = document.createElement('div')
	// jsdom reports clientWidth 0; give the pager a realistic width so the
	// distance threshold (width * 0.25) is meaningful.
	Object.defineProperty(node, 'clientWidth', { value: 400, configurable: true })
	document.body.appendChild(node)
})

afterEach(() => {
	node.remove()
	vi.restoreAllMocks()
})

describe('swipeNavigate', () => {
	it('commits to the next neighbour on a decisive left swipe', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 320, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 100, clientY: 105 }] })
		expect(onDrag).toHaveBeenCalledWith(-220) // finger-follow, no rubber-band mid-range
		fire(node, 'touchend', { changedTouches: [{ clientX: 100, clientY: 105 }], timeStamp: 100 })
		expect(onSettle).toHaveBeenCalledWith(1)
		action.destroy()
	})

	it('commits to the previous neighbour on a decisive right swipe', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 80, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 300, clientY: 100 }] })
		fire(node, 'touchend', { changedTouches: [{ clientX: 300, clientY: 100 }], timeStamp: 120 })
		expect(onSettle).toHaveBeenCalledWith(-1)
		action.destroy()
	})

	it('rubber-bands and refuses to commit past an edge (canPrev false)', () => {
		const action = attach({ canPrev: () => false })
		fire(node, 'touchstart', { touches: [{ clientX: 80, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 300, clientY: 100 }] })
		// A rightward pull at the first day is damped well below the raw 220px.
		const dragged = onDrag.mock.calls.at(-1)?.[0] as number
		expect(dragged).toBeGreaterThan(0)
		expect(dragged).toBeLessThan(220)
		fire(node, 'touchend', { changedTouches: [{ clientX: 300, clientY: 100 }], timeStamp: 120 })
		expect(onSettle).toHaveBeenCalledWith(0) // no neighbour to commit to
		action.destroy()
	})

	it('snaps back when the drag is too small and too slow', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 200, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 180, clientY: 100 }] }) // 20px, horizontal
		fire(node, 'touchend', { changedTouches: [{ clientX: 180, clientY: 100 }], timeStamp: 400 })
		expect(onSettle).toHaveBeenCalledWith(0)
		action.destroy()
	})

	it('never hijacks a vertical scroll', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 200, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 205, clientY: 200 }] }) // mostly vertical
		expect(onDrag).not.toHaveBeenCalled()
		fire(node, 'touchend', { changedTouches: [{ clientX: 205, clientY: 200 }], timeStamp: 100 })
		expect(onSettle).not.toHaveBeenCalled()
		action.destroy()
	})

	it('ignores a jitter below the decision threshold', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 200, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 204, clientY: 103 }] }) // < 8px either axis
		expect(onDrag).not.toHaveBeenCalled()
		action.destroy()
	})

	it('preventDefault stops the browser stealing the horizontal gesture', () => {
		const action = attach()
		fire(node, 'touchstart', { touches: [{ clientX: 320, clientY: 100 }], timeStamp: 0 })
		const move = fire(node, 'touchmove', { touches: [{ clientX: 100, clientY: 100 }] })
		expect(move.defaultPrevented).toBe(true)
		action.destroy()
	})

	it('update swaps the callbacks the action fires', () => {
		const action = attach()
		const onSettle2 = vi.fn()
		action.update({
			onDrag: vi.fn(),
			onSettle: onSettle2,
			canPrev: () => true,
			canNext: () => true,
		})
		fire(node, 'touchstart', { touches: [{ clientX: 320, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 100, clientY: 100 }] })
		fire(node, 'touchend', { changedTouches: [{ clientX: 100, clientY: 100 }], timeStamp: 100 })
		expect(onSettle2).toHaveBeenCalledWith(1)
		expect(onSettle).not.toHaveBeenCalled()
		action.destroy()
	})

	it('destroy removes the listeners', () => {
		const action = attach()
		action.destroy()
		fire(node, 'touchstart', { touches: [{ clientX: 320, clientY: 100 }], timeStamp: 0 })
		fire(node, 'touchmove', { touches: [{ clientX: 100, clientY: 100 }] })
		expect(onDrag).not.toHaveBeenCalled()
	})
})
