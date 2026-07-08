// jsdom project setup: register jest-dom matchers (toBeInTheDocument, …) and
// stub the browser APIs jsdom omits but the components touch. Per-test tweaks
// (matchMedia matches, IntersectionObserver callbacks) override these locally.
import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// jsdom has no matchMedia; default to "mobile" (no query matches) so components
// render their mobile branch unless a test opts into desktop.
if (typeof window.matchMedia !== 'function') {
	window.matchMedia = vi.fn().mockImplementation((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		addListener: vi.fn(),
		removeListener: vi.fn(),
		dispatchEvent: vi.fn(),
	}))
}
