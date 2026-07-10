import { describe, expect, it } from 'vitest'
import {
	classifyGoals,
	decodeNumbering,
	formatProse,
	linkifyTitles,
	parseTeachers,
} from './detail-format'

describe('classifyGoals', () => {
	it('flags can-do and splits internal newlines', () => {
		const g = classifyGoals(['基礎を理解することができる。\n応用を説明できる'])
		expect(g).toHaveLength(2)
		expect(g[0]).toEqual({ tag: undefined, text: '基礎を理解することができる。', canDo: true })
		expect(g[1].canDo).toBe(true)
	})
	it('lifts a leading 【tag】 and strips enumeration', () => {
		const g = classifyGoals(['【学部卒】1. 課題を分析できる'])
		expect(g[0].tag).toBe('学部卒')
		expect(g[0].text).toBe('課題を分析できる')
	})
	it('non-can-do goals are kept verbatim, not can-do', () => {
		const g = classifyGoals(['歴史的背景に関心を持つ'])
		expect(g[0].canDo).toBe(false)
		expect(g[0].text).toBe('歴史的背景に関心を持つ')
	})
})

describe('parseTeachers', () => {
	it('picks the ◎ representative and strips the marker', () => {
		const t = parseTeachers(['◎ 山田太郎', '鈴木花子'])
		expect(t.rep).toBe('山田太郎')
		expect(t.others).toEqual(['鈴木花子'])
		expect(t.omnibus).toBe(false)
	})
	it('no ◎ → no assumed rep', () => {
		const t = parseTeachers(['田中一郎'])
		expect(t.rep).toBeUndefined()
		expect(t.others).toEqual(['田中一郎'])
	})
	it('detects omnibus by size', () => {
		expect(parseTeachers(['a', 'b', 'c', 'd', 'e', 'f']).omnibus).toBe(true)
	})
})

describe('decodeNumbering', () => {
	it('decodes a verified faculty prefix', () => {
		expect(decodeNumbering('01-0200-11')).toEqual({ field: '共通教育' })
		expect(decodeNumbering('41-0100-21')).toEqual({ field: '医学部 医学科' })
	})
	it('returns null for unknown prefix or bad format', () => {
		expect(decodeNumbering('99-0000-11')).toBeNull()
		expect(decodeNumbering('GEN-100')).toBeNull()
		expect(decodeNumbering('')).toBeNull()
	})
})

describe('formatProse', () => {
	it('promotes a bullet run to a list and keeps prose as paragraphs', () => {
		const b = formatProse('概要です。\n・点A\n・点B')
		expect(b[0]).toEqual({ kind: 'paragraph', items: ['概要です。'] })
		expect(b[1]).toEqual({ kind: 'list', items: ['点A', '点B'] })
	})
	it('unstructured text is a single paragraph', () => {
		expect(formatProse('ただの一文です。')).toEqual([
			{ kind: 'paragraph', items: ['ただの一文です。'] },
		])
	})
})

describe('linkifyTitles', () => {
	it('linkifies 『』 and 「」 titles, leaving other text plain', () => {
		const parts = linkifyTitles('教科書『入門テキスト』を使う')
		expect(parts[0]).toEqual({ text: '教科書' })
		expect(parts[1].text).toBe('『入門テキスト』')
		expect(parts[1].href).toContain('google.com/search')
		expect(parts[2]).toEqual({ text: 'を使う' })
	})
})
