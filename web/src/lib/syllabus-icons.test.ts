import { describe, expect, it } from 'vitest'
import { deliveryMode, EVAL_KIND, evalKind } from './syllabus-icons'

describe('evalKind', () => {
	it('maps every known assessment type to its own style', () => {
		for (const [type, style] of Object.entries(EVAL_KIND)) {
			expect(evalKind(type)).toBe(style)
		}
	})

	it('falls back to「その他」for unknown types', () => {
		expect(evalKind('nonexistent')).toBe(EVAL_KIND.other)
		expect(evalKind('')).toBe(EVAL_KIND.other)
	})
})

describe('deliveryMode', () => {
	it('maps the four known modes', () => {
		expect(deliveryMode('onsite')?.label).toBe('対面')
		expect(deliveryMode('online')?.label).toBe('オンライン')
		expect(deliveryMode('ondemand')?.label).toBe('オンデマンド')
		expect(deliveryMode('hybrid')?.label).toBe('ハイブリッド')
	})

	it('returns null for missing or unknown modes (e.g. the "unknown" sentinel)', () => {
		expect(deliveryMode(undefined)).toBeNull()
		expect(deliveryMode('')).toBeNull()
		expect(deliveryMode('unknown')).toBeNull()
	})
})
