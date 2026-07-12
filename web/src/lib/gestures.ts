// Touch-gesture primitives shared by the bottom sheet and the day-view pager.
// The pure helpers (shouldCommit / rubberBand) are unit-tested; the action and
// haptic wrapper are the thin DOM/hardware layers on top.

export interface CommitOptions {
	/** Fraction of `size` the drag must pass to commit on distance alone. */
	distanceRatio?: number
	/** Speed (px/ms) that commits regardless of distance (a flick). */
	velocityThreshold?: number
}

/**
 * Decide whether a drag commits: far enough (past `size * distanceRatio`) OR
 * fast enough (a flick past `velocityThreshold`). Both inputs are magnitudes;
 * the caller owns direction.
 */
export function shouldCommit(
	distance: number,
	speed: number,
	size: number,
	{ distanceRatio = 0.25, velocityThreshold = 0.5 }: CommitOptions = {},
): boolean {
	return distance > size * distanceRatio || speed > velocityThreshold
}

/**
 * iOS-style rubber-band resistance for dragging past a boundary. Monotonically
 * increasing in `overshoot` and asymptotically bounded by `dimension`, so an
 * over-pull moves less and less — the "you've hit the edge" feel.
 */
export function rubberBand(overshoot: number, dimension = 400, constant = 0.55): number {
	if (overshoot <= 0) return overshoot
	return (1 - 1 / ((overshoot * constant) / dimension + 1)) * dimension
}

/** Constrain `value` to the `[min, max]` range. Returns `min` if `min > max`. */
export function clamp(value: number, min: number, max: number): number {
	return Math.max(Math.min(value, max), min)
}

/** Fire a short haptic tick where supported (Android/Chrome); a no-op elsewhere. */
export function haptic(kind: 'light' | 'medium' | 'select' = 'light'): void {
	if (typeof navigator === 'undefined' || typeof navigator.vibrate !== 'function') return
	const ms = kind === 'medium' ? 12 : kind === 'select' ? 4 : 6
	try {
		navigator.vibrate(ms)
	} catch {
		// Some browsers throw if called outside a user gesture — harmless.
	}
}

/** Which way a horizontal swipe settled: -1 prev, +1 next, 0 cancelled. */
export type SwipeDir = -1 | 0 | 1

export interface SwipeNavigateOptions {
	/** Live drag offset (px, signed: + right / - left), for finger-follow. */
	onDrag: (dx: number) => void
	/** Release outcome: which neighbour to commit to (0 = snap back). */
	onSettle: (dir: SwipeDir) => void
	canPrev: () => boolean
	canNext: () => boolean
}

// Stryker disable all: this touch/DOM action is exercised by the jsdom vitest
// project (gestures.svelte.test.ts) + the E2E gesture specs, but Stryker's
// vitest runner executes only the `node` project — so every mutant here reports
// as NoCoverage, a tool artifact, not a test gap. The pure helpers above stay
// mutated. Re-enable if the runner learns to drive both vitest projects.
/**
 * Horizontal swipe navigation with finger-follow and edge rubber-banding. Only
 * hijacks once the gesture is clearly horizontal, so vertical scrolling is never
 * stolen. `touchmove` is non-passive so it can `preventDefault` while dragging.
 */
export function swipeNavigate(node: HTMLElement, options: SwipeNavigateOptions) {
	let opts = options
	let startX = 0
	let startY = 0
	let startT = 0
	let width = 0
	let tracking = false
	let decided = false
	let horizontal = false

	function start(e: TouchEvent) {
		const t = e.touches[0]
		startX = t.clientX
		startY = t.clientY
		startT = e.timeStamp
		width = node.clientWidth || 1
		tracking = true
		decided = false
		horizontal = false
	}

	function move(e: TouchEvent) {
		if (!tracking) return
		const t = e.touches[0]
		const dx = t.clientX - startX
		const dy = t.clientY - startY
		if (!decided) {
			if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return
			horizontal = Math.abs(dx) > Math.abs(dy) * 1.2
			decided = true
			if (!horizontal) {
				tracking = false
				return
			}
		}
		e.preventDefault()
		let d = dx
		if ((d > 0 && !opts.canPrev()) || (d < 0 && !opts.canNext())) {
			d = Math.sign(d) * rubberBand(Math.abs(d), width)
		}
		opts.onDrag(d)
	}

	function end(e: TouchEvent) {
		if (!tracking) return
		tracking = false
		if (!horizontal) return
		const touch = e.changedTouches[0]
		const dx = (touch ? touch.clientX : startX) - startX
		const speed = Math.abs(dx) / Math.max(1, e.timeStamp - startT)
		const commit = shouldCommit(Math.abs(dx), speed, width, {
			distanceRatio: 0.25,
			velocityThreshold: 0.4,
		})
		let dir: SwipeDir = 0
		if (commit) {
			if (dx > 0 && opts.canPrev()) dir = -1
			else if (dx < 0 && opts.canNext()) dir = 1
		}
		opts.onSettle(dir)
	}

	node.addEventListener('touchstart', start, { passive: true })
	node.addEventListener('touchmove', move, { passive: false })
	node.addEventListener('touchend', end, { passive: true })
	node.addEventListener('touchcancel', end, { passive: true })

	return {
		update(next: SwipeNavigateOptions) {
			opts = next
		},
		destroy() {
			node.removeEventListener('touchstart', start)
			node.removeEventListener('touchmove', move)
			node.removeEventListener('touchend', end)
			node.removeEventListener('touchcancel', end)
		},
	}
}
// Stryker restore all
