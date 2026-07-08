// Shared desktop/mobile breakpoint, matching the Tailwind `sm` (640px) split.
// The timetable and the bottom sheet each render a different layout on either
// side of it, so both read the same source of truth here instead of duplicating
// the matchMedia dance.

export const DESKTOP_QUERY = '(min-width: 640px)'

/** Whether the viewport is at or above the desktop breakpoint right now. */
export function matchesDesktop(query = DESKTOP_QUERY): boolean {
	return typeof window !== 'undefined' && window.matchMedia(query).matches
}

/**
 * Reactive `isDesktop`, seeded synchronously (so the correct layout renders on
 * the first frame) and kept in sync via the media query's `change` event. Call
 * it during component init; the subscription is torn down with the component.
 */
export function useDesktop(query = DESKTOP_QUERY): { readonly current: boolean } {
	let isDesktop = $state(matchesDesktop(query))
	$effect(() => {
		const mq = window.matchMedia(query)
		const sync = () => {
			isDesktop = mq.matches
		}
		sync()
		mq.addEventListener('change', sync)
		return () => mq.removeEventListener('change', sync)
	})
	return {
		get current() {
			return isDesktop
		},
	}
}
