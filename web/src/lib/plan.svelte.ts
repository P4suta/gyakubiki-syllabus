// The user's plan: a reactive set of registered course codes, plus the sync that
// keeps it mirrored in the URL hash (for sharing) and localStorage (for return
// visits). A module singleton, the codebase's store pattern (cf. breakpoint /
// theme / highlight) — any component reads or toggles registration without
// prop-drilling through the timetable.

import { decodePlan, encodePlan } from './plan-codec'

class PlanStore {
	#cds = $state<string[]>([])

	get cds(): readonly string[] {
		return this.#cds
	}
	get count(): number {
		return this.#cds.length
	}
	has(cd: string): boolean {
		return this.#cds.includes(cd)
	}
	add(cd: string): void {
		if (!this.has(cd)) this.#cds = [...this.#cds, cd]
	}
	remove(cd: string): void {
		this.#cds = this.#cds.filter((x) => x !== cd)
	}
	toggle(cd: string): void {
		if (this.has(cd)) this.remove(cd)
		else this.add(cd)
	}
	clear(): void {
		this.#cds = []
	}
	/** Replace the whole plan (used by the sync layer on load / hashchange). */
	hydrate(cds: string[]): void {
		this.#cds = cds
	}
}

export const plan = new PlanStore()

const STORAGE_KEY = 'myPlan'
const HASH_PREFIX = '#plan='

/** The plan token in the current URL hash, or null. */
function readHash(): string | null {
	const h = typeof location === 'undefined' ? '' : location.hash
	return h.startsWith(HASH_PREFIX) ? h.slice(HASH_PREFIX.length) : null
}

/**
 * The shareable URL for the current plan (empty plan → no hash). Built on demand
 * by the share button; the hash is deliberately NOT kept live in the address bar
 * — that would fight the modal sheet's own history integration (a sheet closing
 * via Back reverts the URL, which must never clear the plan).
 */
export function shareUrl(): string {
	const token = encodePlan(plan.cds)
	const base = `${location.origin}${location.pathname}${location.search}`
	return token ? `${base}${HASH_PREFIX}${token}` : base
}

/**
 * Wire the plan to localStorage (persistence) and, one-way, to a shared `#plan=`
 * link. Call once on mount; returns a teardown.
 *
 * On load: a shared `#plan=` link wins, else the stored plan. On change: only
 * localStorage is written — the URL is set solely by the share button (see
 * [`shareUrl`]). A `#plan=` link opened while the app runs updates the plan; an
 * empty hash (e.g. a sheet closing via Back) is ignored, so navigation never
 * wipes a registration.
 */
export function initPlanSync(): () => void {
	const fromHash = readHash()
	if (fromHash) {
		plan.hydrate(decodePlan(fromHash))
	} else {
		try {
			const stored = localStorage.getItem(STORAGE_KEY)
			if (stored) plan.hydrate(decodePlan(stored))
		} catch {
			// storage unavailable (private mode) — start empty, still works
		}
	}

	const stop = $effect.root(() => {
		$effect(() => {
			const token = encodePlan(plan.cds)
			try {
				localStorage.setItem(STORAGE_KEY, token)
			} catch {
				// ignore storage failures
			}
		})
	})

	const onHashChange = () => {
		const token = readHash()
		if (token) plan.hydrate(decodePlan(token)) // only a real share link, never empty
	}
	window.addEventListener('hashchange', onHashChange)

	return () => {
		window.removeEventListener('hashchange', onHashChange)
		stop()
	}
}
