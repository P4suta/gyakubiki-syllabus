import { describe, expect, it } from 'vitest'
import {
	classifyGoals,
	courseLanguage,
	decodeNumbering,
	formatProse,
	isRelatedLabel,
	linkifyText,
	parseTeachers,
	splitRelated,
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
	it('a leading digit that is real content is never stripped', () => {
		// Regression: 「2進数」「36の母音」must survive (delimiter required for enum).
		expect(classifyGoals(['2進数で小数を表せる'])[0].text).toBe('2進数で小数を表せる')
		expect(classifyGoals(['36の母音を発音できる'])[0].text).toBe('36の母音を発音できる')
		// But a real enumeration marker is still stripped.
		expect(classifyGoals(['1. 課題を分析できる'])[0].text).toBe('課題を分析できる')
	})
	it('strips varied enumeration markers (paren / circled / bullet)', () => {
		expect(classifyGoals(['(1) 説明できる'])[0].text).toBe('説明できる')
		expect(classifyGoals(['① 説明できる'])[0].text).toBe('説明できる')
		expect(classifyGoals(['・説明できる'])[0].text).toBe('説明できる')
	})
	it('trims whitespace inside the 【tag】', () => {
		expect(classifyGoals(['【 学部卒 】課題を解決できる'])[0].tag).toBe('学部卒')
	})
	it('only a 【tag】 at the very start is lifted', () => {
		const g = classifyGoals(['前置き【区分】本文できる'])
		expect(g[0].tag).toBeUndefined()
		expect(g[0].text).toBe('前置き【区分】本文できる')
	})
	it('can-do requires 〜できる at the END, not mid-sentence', () => {
		expect(classifyGoals(['できるだけ努力する'])[0].canDo).toBe(false)
	})
})

describe('parseTeachers', () => {
	it('picks the ◎ representative and strips the marker', () => {
		const t = parseTeachers(['◎ 山田太郎', '鈴木花子'])
		expect(t.rep).toBe('山田太郎')
		expect(t.others).toEqual(['鈴木花子'])
	})
	it('no ◎ → no assumed rep', () => {
		const t = parseTeachers(['田中一郎'])
		expect(t.rep).toBeUndefined()
		expect(t.others).toEqual(['田中一郎'])
	})
	it('picks a non-first ◎ as representative', () => {
		const t = parseTeachers(['鈴木花子', '◎ 山田太郎'])
		expect(t.rep).toBe('山田太郎')
		expect(t.others).toEqual(['鈴木花子'])
	})
})

describe('decodeNumbering', () => {
	it('decodes 学部・レベル・授業形態・言語 from the code digits', () => {
		expect(decodeNumbering('01-0200-11')).toEqual({
			faculty: '共通教育',
			level: '初年次・教養',
			format: '講義',
			lang: '日本語',
		})
		expect(decodeNumbering('31-2261-31')?.faculty).toBe('理工学部')
	})
	it('tolerates a letter in the 大/中/小分類 region and leaves unknown digits absent', () => {
		expect(decodeNumbering('11-13A2-11')).toEqual({
			faculty: '人文社会科学部',
			level: '基礎',
			format: '講義',
			lang: '日本語',
		})
		// 授業形態5 (a newer code) is undecodable → absent, not fabricated.
		expect(decodeNumbering('02-0420-51')?.format).toBeUndefined()
	})
	it('returns null for a non-standard shape', () => {
		expect(decodeNumbering('GEN-100')).toBeNull()
		expect(decodeNumbering('')).toBeNull()
	})
	it('is fully anchored — extra leading/trailing characters do not match', () => {
		expect(decodeNumbering('X01-0200-11')).toBeNull()
		expect(decodeNumbering('01-0200-11X')).toBeNull()
	})
})

describe('courseLanguage', () => {
	it('surfaces a non-Japanese teaching language, else null', () => {
		expect(courseLanguage(['01-0200-22'])).toBe('英語') // 言語2
		expect(courseLanguage(['01-0200-11'])).toBeNull() // 日本語 = default, not surfaced
		expect(courseLanguage([])).toBeNull()
		expect(courseLanguage(undefined)).toBeNull()
	})
	it('skips unparsable codes without throwing, then reads the next', () => {
		expect(courseLanguage(['GEN-100', '01-0200-22'])).toBe('英語')
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
	it('a blank line flushes and separates paragraphs', () => {
		expect(formatProse('前段です。\n\n後段です。')).toEqual([
			{ kind: 'paragraph', items: ['前段です。'] },
			{ kind: 'paragraph', items: ['後段です。'] },
		])
	})
	it('recognises varied bullet markers (circled / parenthesised)', () => {
		expect(formatProse('①一つ目\n②二つ目')).toEqual([{ kind: 'list', items: ['一つ目', '二つ目'] }])
		expect(formatProse('(1)甲\n(2)乙')).toEqual([{ kind: 'list', items: ['甲', '乙'] }])
	})
})

describe('splitRelated', () => {
	const known = new Set(['07011', '28008'])
	it('links only resolvable 5-digit codes; names/others stay plain', () => {
		const parts = splitRelated('07011はじめての金融経済，28008金融論，99999未開講', known)
		const links = parts.filter((p) => p.code)
		expect(links.map((p) => p.code)).toEqual(['07011', '28008'])
		// The unresolvable 99999 is not linked (stays in a plain part).
		expect(parts.some((p) => !p.code && p.text.includes('99999'))).toBe(true)
	})
	it('a name-only entry produces no links', () => {
		expect(splitRelated('「学問基礎論」', known).every((p) => !p.code)).toBe(true)
	})
	it('preserves the exact text around each code, in order', () => {
		expect(splitRelated('前07011中28008後', known)).toEqual([
			{ text: '前' },
			{ text: '07011', code: '07011' },
			{ text: '中' },
			{ text: '28008', code: '28008' },
			{ text: '後' },
		])
	})
	it('a code at the very start adds no leading empty part', () => {
		expect(splitRelated('07011のみ', known)).toEqual([
			{ text: '07011', code: '07011' },
			{ text: 'のみ' },
		])
	})
	it('a code at the very end adds no trailing empty part', () => {
		expect(splitRelated('科目は07011', known)).toEqual([
			{ text: '科目は' },
			{ text: '07011', code: '07011' },
		])
	})
})

describe('linkifyText', () => {
	it('linkifies 『』 titles (query wrapped in 『』), leaving other text plain', () => {
		const parts = linkifyText('教科書『入門テキスト』を使う')
		expect(parts[0]).toEqual({ text: '教科書' })
		expect(parts[1].text).toBe('『入門テキスト』')
		expect(parts[1].href).toContain('google.com/search')
		// Query keeps the 『』 so a generic title reads as a book to Google.
		expect(decodeURIComponent(parts[1].href ?? '')).toContain('『入門テキスト』')
		expect(parts[2]).toEqual({ text: 'を使う' })
	})
	it('linkifies an email address to mailto', () => {
		const parts = linkifyText('連絡は rshima@kochi-u.ac.jp まで')
		const link = parts.find((p) => p.href?.startsWith('mailto:'))
		expect(link?.href).toBe('mailto:rshima@kochi-u.ac.jp')
		expect(link?.text).toBe('rshima@kochi-u.ac.jp')
	})
	it('NEVER links 「…」 — it is a quote/emphasis, not a book title', () => {
		// Regression: 「食」の哲学 quotes/dialogue were all turned into book links.
		const samples = [
			'彼は「いただきます」と言った。',
			'キーワードは「持続可能性」である。',
			'「なぜ食べるのか」を問う。',
		]
		for (const s of samples) {
			expect(linkifyText(s).every((p) => !p.href), s).toBe(true)
		}
	})
	it('linkifies a URL and stops before trailing Japanese punctuation', () => {
		const parts = linkifyText('詳細は https://www.ipa.go.jp/security/vuln/websecurity.html を参照。')
		const url = parts.find((p) => p.kind === 'url')
		expect(url?.href).toBe('https://www.ipa.go.jp/security/vuln/websecurity.html')
		// The trailing「。」is not swallowed into the link.
		expect(parts.some((p) => !p.href && p.text.includes('。'))).toBe(true)
	})
	it('only 『…』 (not 「…」) is treated as a title link', () => {
		const parts = linkifyText('『経済学基礎』と「経済学」は別物')
		const links = parts.filter((p) => p.href)
		expect(links).toHaveLength(1)
		expect(links[0].text).toBe('『経済学基礎』')
	})
	it('a link at the very start adds no leading empty text part', () => {
		const parts = linkifyText('『本』の話')
		expect(parts[0].kind).toBe('title')
		expect(parts.some((p) => p.text === '')).toBe(false)
	})
})

describe('isRelatedLabel', () => {
	it('matches the related-courses / COMPUTER LINK labels only', () => {
		expect(isRelatedLabel('関連科目')).toBe(true)
		expect(isRelatedLabel('RELATED COURSES')).toBe(true)
		expect(isRelatedLabel('COMPUTER LINK')).toBe(true)
		expect(isRelatedLabel('到達目標')).toBe(false)
		expect(isRelatedLabel('')).toBe(false)
	})
})
