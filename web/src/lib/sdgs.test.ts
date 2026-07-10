import { describe, expect, it } from 'vitest'
import { sdgGoal } from './sdgs'

describe('sdgGoal', () => {
	it('resolves a bare goal number (real KULAS format)', () => {
		const g = sdgGoal('4')
		expect(g?.n).toBe(4)
		expect(g?.title).toBe('質の高い教育をみんなに')
		expect(g?.url).toBe('https://www.unicef.or.jp/kodomo/sdgs/17goals/4-education/')
	})

	it('reads the leading integer from a「number title」variant', () => {
		expect(sdgGoal('4 質の高い教育をみんなに')?.n).toBe(4)
		expect(sdgGoal(' 10 人や国の不平等をなくそう')?.n).toBe(10)
	})

	it('returns null outside 1–17 or with no leading number', () => {
		expect(sdgGoal('0')).toBeNull()
		expect(sdgGoal('18')).toBeNull()
		expect(sdgGoal('SDGs')).toBeNull()
		expect(sdgGoal('')).toBeNull()
	})
})
