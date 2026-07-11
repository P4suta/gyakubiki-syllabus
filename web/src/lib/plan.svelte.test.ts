import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { encodePlan } from './plan-codec'
import { initPlanSync, plan, shareUrl } from './plan.svelte'

describe('plan store', () => {
	beforeEach(() => plan.clear())

	it('adds, reports membership, and counts without duplicates', () => {
		plan.add('a')
		plan.add('a')
		plan.add('b')
		expect(plan.has('a')).toBe(true)
		expect(plan.has('z')).toBe(false)
		expect(plan.count).toBe(2)
		expect([...plan.cds]).toEqual(['a', 'b'])
	})

	it('removes and toggles', () => {
		plan.add('a')
		plan.remove('a')
		expect(plan.has('a')).toBe(false)
		plan.toggle('b')
		expect(plan.has('b')).toBe(true)
		plan.toggle('b')
		expect(plan.has('b')).toBe(false)
	})

	it('hydrate replaces the whole plan and clear empties it', () => {
		plan.add('a')
		plan.hydrate(['x', 'y', 'z'])
		expect([...plan.cds]).toEqual(['x', 'y', 'z'])
		plan.clear()
		expect(plan.count).toBe(0)
	})
})

// This jsdom build ships no localStorage (initPlanSync guards it with try/catch);
// give the sync tests a real in-memory one to exercise the persistence paths.
function makeStorage(): Storage {
	const m = new Map<string, string>()
	return {
		get length() {
			return m.size
		},
		clear: () => m.clear(),
		getItem: (k) => (m.has(k) ? (m.get(k) as string) : null),
		key: (i) => [...m.keys()][i] ?? null,
		removeItem: (k) => void m.delete(k),
		setItem: (k, v) => void m.set(k, String(v)),
	}
}

describe('plan sync (hash + localStorage)', () => {
	let teardown: (() => void) | undefined

	beforeEach(() => {
		plan.clear()
		vi.stubGlobal('localStorage', makeStorage())
		location.hash = ''
		teardown = undefined
	})
	afterEach(() => {
		teardown?.()
		vi.unstubAllGlobals()
		location.hash = ''
	})

	it('shareUrl: an empty plan has no hash, a filled plan carries #plan=', () => {
		expect(shareUrl()).not.toContain('#plan=')
		plan.hydrate(['00001', '00002'])
		expect(shareUrl()).toContain(`#plan=${encodePlan(['00001', '00002'])}`)
	})

	it('hydrates from a #plan= link on load, and the link wins over storage', () => {
		localStorage.setItem('myPlan', encodePlan(['99999'])) // must be ignored
		location.hash = `#plan=${encodePlan(['00001', '00002'])}`
		teardown = initPlanSync()
		expect([...plan.cds]).toEqual(['00001', '00002'])
	})

	it('falls back to the stored plan when there is no hash', () => {
		localStorage.setItem('myPlan', encodePlan(['12345']))
		teardown = initPlanSync()
		expect([...plan.cds]).toEqual(['12345'])
	})

	it('starts empty when storage is unavailable', () => {
		vi.stubGlobal('localStorage', {
			...makeStorage(),
			getItem: () => {
				throw new Error('blocked')
			},
		})
		teardown = initPlanSync()
		expect(plan.count).toBe(0)
	})

	it('persists to localStorage when the plan changes', () => {
		teardown = initPlanSync()
		plan.add('00001')
		flushSync()
		expect(localStorage.getItem('myPlan')).toBe(encodePlan(['00001']))
	})

	it('a #plan= link opened while running updates the plan; an empty hash is ignored', () => {
		teardown = initPlanSync()
		plan.hydrate(['00001'])
		location.hash = `#plan=${encodePlan(['22222'])}`
		window.dispatchEvent(new Event('hashchange'))
		expect([...plan.cds]).toEqual(['22222'])
		// An empty hash (e.g. a sheet closing via Back) must never wipe the plan.
		location.hash = ''
		window.dispatchEvent(new Event('hashchange'))
		expect([...plan.cds]).toEqual(['22222'])
	})

	it('teardown detaches the hashchange listener', () => {
		teardown = initPlanSync()
		teardown()
		teardown = undefined
		location.hash = `#plan=${encodePlan(['55555'])}`
		window.dispatchEvent(new Event('hashchange'))
		expect(plan.has('55555')).toBe(false)
	})
})
