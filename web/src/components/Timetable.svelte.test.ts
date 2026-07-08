import { render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GridKey } from '../lib/engine'
import type { Course } from '../types/course'
import Timetable from './Timetable.svelte'

// The mobile day pager is a timed state machine: a committed swipe slides the
// current day out, swaps `activeDay` while the neighbour is parked off-screen,
// then slides it in. Driven here with synthetic touches + fake timers; the
// geometry itself stays in the E2E suite (jsdom has no layout).

interface Pt {
	clientX: number
	clientY: number
}

function fire(
	node: Element,
	type: string,
	opts: { touches?: Pt[]; changedTouches?: Pt[]; timeStamp?: number },
) {
	const e = new Event(type, { bubbles: true, cancelable: true })
	Object.defineProperty(e, 'touches', { value: opts.touches ?? [] })
	Object.defineProperty(e, 'changedTouches', { value: opts.changedTouches ?? [] })
	if (opts.timeStamp !== undefined) Object.defineProperty(e, 'timeStamp', { value: opts.timeStamp })
	node.dispatchEvent(e)
}

/** A decisive horizontal drag across `scroller`, from `x0` to `x1`. */
function swipeX(scroller: Element, x0: number, x1: number) {
	fire(scroller, 'touchstart', { touches: [{ clientX: x0, clientY: 200 }], timeStamp: 0 })
	fire(scroller, 'touchmove', { touches: [{ clientX: x1, clientY: 202 }] })
	fire(scroller, 'touchend', { changedTouches: [{ clientX: x1, clientY: 202 }], timeStamp: 120 })
	flushSync()
}

const DAYS = ['月', '火', '水'] as const

function mount() {
	const grid = new Map<GridKey, Course[]>()
	const utils = render(Timetable, { props: { grid, days: DAYS, onselect: vi.fn() } })
	flushSync()
	const scroller = utils.container.querySelector('.touch-pan-y')
	if (!scroller) throw new Error('day scroller not found — is the mobile view mounted?')
	return { ...utils, scroller }
}

const activeDay = (
	utils: { getByRole: (r: string, o: { name: string }) => HTMLElement },
	label: string,
) => utils.getByRole('button', { name: label }).className.includes('text-apple-blue')

beforeEach(() => {
	vi.useFakeTimers()
})

afterEach(() => {
	vi.useRealTimers()
	vi.restoreAllMocks()
})

describe('Timetable day pager', () => {
	it('advances to the next day on a committed left swipe', () => {
		const utils = mount()
		expect(activeDay(utils, '月')).toBe(true)

		swipeX(utils.scroller, 320, 60) // drag left → next
		// The outgoing day is sliding and translated off to the left.
		expect(utils.scroller.getAttribute('style')).toMatch(/translate:\s*-\d/)

		vi.runAllTimers()
		flushSync()
		expect(activeDay(utils, '火')).toBe(true)
		expect(activeDay(utils, '月')).toBe(false)
		// Settled back to rest with no residual offset.
		expect(utils.scroller.getAttribute('style')).toMatch(/translate:\s*0px 0/)
	})

	it('goes back to the previous day on a committed right swipe', () => {
		const utils = mount()
		// Start on 火 by advancing once.
		swipeX(utils.scroller, 320, 60)
		vi.runAllTimers()
		flushSync()
		expect(activeDay(utils, '火')).toBe(true)

		swipeX(utils.scroller, 60, 320) // drag right → prev
		vi.runAllTimers()
		flushSync()
		expect(activeDay(utils, '月')).toBe(true)
	})

	it('rubber-bands and stays put when swiping past the first day', () => {
		const utils = mount()
		swipeX(utils.scroller, 60, 320) // right at day 0 → no previous
		vi.runAllTimers()
		flushSync()
		expect(activeDay(utils, '月')).toBe(true)
		expect(utils.scroller.getAttribute('style')).toMatch(/translate:\s*0px 0/)
	})

	it('switches day when a tab is tapped', async () => {
		const utils = mount()
		utils.getByRole('button', { name: '水' }).click()
		flushSync()
		expect(activeDay(utils, '水')).toBe(true)
	})
})
