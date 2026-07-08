import { fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Harness from './BottomSheetHarness.svelte'

// The sheet routes *every* dismissal (×, backdrop, Esc, swipe, device Back)
// through a single history entry, so the browser Back button and in-app closes
// share one path and leave no dangling state. These specs pin that invariant;
// the drag geometry itself stays in the E2E gesture suite.

function setViewport(desktop: boolean) {
	window.matchMedia = vi.fn().mockImplementation((query: string) => ({
		matches: desktop,
		media: query,
		onchange: null,
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		addListener: vi.fn(),
		removeListener: vi.fn(),
		dispatchEvent: vi.fn(),
	}))
}

let pushState: ReturnType<typeof vi.spyOn>
let back: ReturnType<typeof vi.spyOn>

beforeEach(() => {
	setViewport(false) // default to the mobile sheet
	pushState = vi.spyOn(window.history, 'pushState')
	// Stub back() so jsdom doesn't actually navigate; we assert it was routed to.
	back = vi.spyOn(window.history, 'back').mockImplementation(() => {})
})

afterEach(() => {
	vi.useRealTimers()
	vi.restoreAllMocks()
})

describe('BottomSheet close routing', () => {
	it('pushes a single history entry when opened', () => {
		render(Harness, { props: { onclose: vi.fn() } })
		flushSync()
		expect(pushState).toHaveBeenCalledWith({ __sheet: true }, '')
	})

	it('closes exactly once on popstate, even if fired twice (guard)', () => {
		const onclose = vi.fn()
		render(Harness, { props: { onclose } })
		flushSync()
		window.dispatchEvent(new PopStateEvent('popstate'))
		window.dispatchEvent(new PopStateEvent('popstate'))
		flushSync()
		expect(onclose).toHaveBeenCalledTimes(1)
	})

	it('routes an Escape (native dialog cancel) through the history entry (mobile slide-out)', () => {
		vi.useFakeTimers()
		const { container } = render(Harness, { props: { onclose: vi.fn() } })
		flushSync()
		back.mockClear()
		// The dialog is in the top layer; Esc fires the native `cancel`, which we
		// preventDefault and route through our dismiss instead of the native close.
		const dialog = container.querySelector('dialog')
		if (!dialog) throw new Error('dialog not found')
		dialog.dispatchEvent(new Event('cancel', { cancelable: true }))
		flushSync()
		vi.advanceTimersByTime(240) // mobile dismiss waits for the slide-out
		expect(back).toHaveBeenCalledTimes(1) // → popstate → actualClose, one path
	})

	it('closes immediately on desktop (no slide-out) via the backdrop', () => {
		setViewport(true)
		const { container } = render(Harness, { props: { onclose: vi.fn() } })
		flushSync()
		back.mockClear()
		const backdrop = container.querySelector('[aria-hidden="true"]')
		if (!backdrop) throw new Error('backdrop not found')
		fireEvent.click(backdrop)
		flushSync()
		expect(back).toHaveBeenCalledTimes(1)
	})

	it('pops its lingering entry when unmounted without being consumed', () => {
		const { unmount } = render(Harness, { props: { onclose: vi.fn() } })
		flushSync()
		back.mockClear()
		unmount()
		expect(back).toHaveBeenCalledTimes(1)
	})

	it('does not pop again on unmount once already closed via popstate', () => {
		const { unmount } = render(Harness, { props: { onclose: vi.fn() } })
		flushSync()
		window.dispatchEvent(new PopStateEvent('popstate')) // consumed
		flushSync()
		back.mockClear()
		unmount()
		expect(back).not.toHaveBeenCalled()
	})
})
