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

function readHash(): string | null {
	const h = typeof location === 'undefined' ? '' : location.hash
	return h.startsWith(HASH_PREFIX) ? h.slice(HASH_PREFIX.length) : null
}

function writeHash(token: string): void {
	// replaceState (not a new hash navigation) so sharing never spams history.
	const url = token ? `${HASH_PREFIX}${token}` : location.pathname + location.search
	history.replaceState(null, '', url)
}

/**
 * Wire the plan to the URL hash and localStorage. Call once on mount; returns a
 * teardown. Initial state prefers a shared `#plan=` link over the stored plan;
 * later edits write both, and an external hash change (share link, Back) is
 * pulled back in. A re-entrancy flag stops our own writes from echoing.
 */
export function initPlanSync(): () => void {
	const fromHash = readHash()
	if (fromHash !== null) {
		plan.hydrate(decodePlan(fromHash))
	} else {
		try {
			const stored = localStorage.getItem(STORAGE_KEY)
			if (stored) plan.hydrate(decodePlan(stored))
		} catch {
			// storage unavailable (private mode) — start empty, still works
		}
	}

	let echoing = false
	const stop = $effect.root(() => {
		$effect(() => {
			const token = encodePlan(plan.cds)
			echoing = true
			try {
				localStorage.setItem(STORAGE_KEY, token)
			} catch {
				// ignore storage failures
			}
			writeHash(token)
			queueMicrotask(() => {
				echoing = false
			})
		})
	})

	const onHashChange = () => {
		if (echoing) return
		const h = readHash()
		plan.hydrate(h !== null ? decodePlan(h) : [])
	}
	window.addEventListener('hashchange', onHashChange)
	window.addEventListener('popstate', onHashChange)

	return () => {
		window.removeEventListener('hashchange', onHashChange)
		window.removeEventListener('popstate', onHashChange)
		stop()
	}
}
