import { beforeEach, describe, expect, it } from 'vitest'
import { plan } from './plan.svelte'

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
