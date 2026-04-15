import { describe, expect, it } from 'vitest'
import { getColor, initKubunColors } from './colors'

const SAMPLE_KUBUN = ['実技', '実習', '実験', '演習', '講義']

describe('notion-style kubun colors', () => {
	initKubunColors(SAMPLE_KUBUN)

	it('returns a valid CourseColor', () => {
		const c = getColor(4) // 講義
		expect(c).toHaveProperty('bg')
		expect(c).toHaveProperty('border')
		expect(c).toHaveProperty('text')
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
		expect(c.text).toBe('#1d1d1f')
	})

	it('is deterministic', () => {
		expect(getColor(0)).toEqual(getColor(0))
	})

	it('different kubun types get different colors', () => {
		const colors = SAMPLE_KUBUN.map((_, i) => getColor(i).bg)
		const unique = new Set(colors)
		expect(unique.size).toBe(5)
	})

	it('講義 is blue', () => {
		const idx = SAMPLE_KUBUN.indexOf('講義')
		expect(getColor(idx).border).toBe('#4285f4')
	})

	it('演習 is green', () => {
		const idx = SAMPLE_KUBUN.indexOf('演習')
		expect(getColor(idx).border).toBe('#34a853')
	})

	it('returns fallback for unknown index', () => {
		const c = getColor(999)
		expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
	})

	it('returns fallback before init', () => {
		const c = getColor(100)
		expect(c.bg).toBe('#eceef1')
	})
})
