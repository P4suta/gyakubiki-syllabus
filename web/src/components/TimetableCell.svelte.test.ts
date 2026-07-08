import { render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Course } from '../types/course'
import TimetableCell from './TimetableCell.svelte'

// The cell defers mounting its cards until it nears the viewport (an
// IntersectionObserver). jsdom has no IO, so we stand one in and drive its
// callback by hand to assert the lazy-mount + one-shot-disconnect behaviour.

function course(cd: string, nm: string): Course {
	return { cd, nm, prof: '', raw: '', slots: [], ki: 0, kbn: 0, dept: 0, campus: 0, st: nm }
}

let ioCallback: IntersectionObserverCallback
const observe = vi.fn()
const disconnect = vi.fn()

beforeEach(() => {
	observe.mockClear()
	disconnect.mockClear()
	vi.stubGlobal(
		'IntersectionObserver',
		class {
			root = null
			rootMargin = ''
			thresholds: number[] = []
			constructor(cb: IntersectionObserverCallback) {
				ioCallback = cb
			}
			observe = observe
			disconnect = disconnect
			unobserve = vi.fn()
			takeRecords = vi.fn(() => [])
		},
	)
})

afterEach(() => {
	vi.unstubAllGlobals()
})

function intersect(isIntersecting: boolean) {
	ioCallback([{ isIntersecting } as IntersectionObserverEntry], {} as IntersectionObserver)
	flushSync()
}

describe('TimetableCell lazy mount', () => {
	it('renders no cards until the cell intersects, then mounts them', () => {
		const { getByRole, queryByRole } = render(TimetableCell, {
			props: { courses: [course('00001', '微分積分学Ⅰ')], onselect: vi.fn() },
		})
		flushSync()
		expect(observe).toHaveBeenCalledTimes(1)
		expect(queryByRole('button')).toBeNull()

		intersect(true)
		expect(getByRole('button', { name: /微分積分学/ })).toBeInTheDocument()
		// Disconnected once the cards are up — the observer's job is done and the
		// `visible` gate keeps a re-run from ever observing again.
		expect(disconnect).toHaveBeenCalled()
	})

	it('stays empty while the cell is only approaching (not intersecting)', () => {
		const { queryByRole } = render(TimetableCell, {
			props: { courses: [course('00001', '微分積分学Ⅰ')], onselect: vi.fn() },
		})
		flushSync()
		intersect(false)
		expect(queryByRole('button')).toBeNull()
		expect(disconnect).not.toHaveBeenCalled()
	})
})
