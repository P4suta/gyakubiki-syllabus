// Reactive `prefers-color-scheme`, mirroring breakpoint.svelte.ts. The CSS tokens
// flip themselves via the `@media (prefers-color-scheme: dark)` block in app.css;
// this store is only for the JS-resolved course-tile and eval-arc colours, which
// are applied as inline styles and so can't be a CSS token.

const DARK_QUERY = '(prefers-color-scheme: dark)'

/** Whether the OS prefers a dark colour scheme right now. */
export function prefersDark(): boolean {
	return typeof window !== 'undefined' && window.matchMedia(DARK_QUERY).matches
}

/**
 * Reactive `isDark`, seeded synchronously (so the first paint picks the right
 * tint) and kept in sync with the OS setting. Call during component init.
 */
export function useTheme(): { readonly isDark: boolean } {
	let isDark = $state(prefersDark())
	$effect(() => {
		const mq = window.matchMedia(DARK_QUERY)
		const sync = () => {
			isDark = mq.matches
		}
		sync()
		mq.addEventListener('change', sync)
		return () => mq.removeEventListener('change', sync)
	})
	return {
		get isDark() {
			return isDark
		},
	}
}
