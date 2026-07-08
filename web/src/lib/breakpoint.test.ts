import { afterEach, describe, expect, it, vi } from 'vitest'
import { DESKTOP_QUERY, matchesDesktop } from './breakpoint.svelte'

// The reactive `useDesktop` store is exercised by the component render tests
// (Timetable / BottomSheet); here we pin the pure predicate, including its SSR
// guard, which those DOM tests can't hit (jsdom always defines `window`).
describe('matchesDesktop', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
	})

	it('is false with no window (SSR)', () => {
		// node env: `window` is undefined, so the guard short-circuits.
		expect(matchesDesktop()).toBe(false)
	})

	it('reflects matchMedia on the default query', () => {
		const matchMedia = vi.fn((q: string) => ({ matches: true, media: q }))
		vi.stubGlobal('window', { matchMedia })
		expect(matchesDesktop()).toBe(true)
		expect(matchMedia).toHaveBeenCalledWith(DESKTOP_QUERY)
	})

	it('honours a custom query and a non-match', () => {
		const matchMedia = vi.fn((q: string) => ({ matches: false, media: q }))
		vi.stubGlobal('window', { matchMedia })
		expect(matchesDesktop('(min-width: 1200px)')).toBe(false)
		expect(matchMedia).toHaveBeenCalledWith('(min-width: 1200px)')
	})
})
