// Light, faithful presentation helpers for the course-detail sheet. Heavy data
// extraction (textbook sections, study-time) is precomputed in Rust; these only
// reshape existing text for display — never rewording, summarising, or dropping
// content. Pure + tested so the transforms are verifiable.

export interface GoalItem {
	/** A leading 【区分】tag lifted off the front, if any. */
	tag?: string
	/** The goal text, verbatim (minus tag / leading enumeration). */
	text: string
	/** Ends in 〜できる/出来る — a "can-do" competency. */
	canDo: boolean
}

const GOAL_TAG = /^【([^】]*)】\s*/
// A leading enumeration marker only — a bare digit that is real content (「2進数」
// 「36の母音」) must NOT be stripped, so a delimiter is required after digits.
const LEAD_ENUM = /^(?:\(\d+\)|\d+[.．)、]|[①-⑳]|[・･])\s*/
const CAN_DO = /(?:できる|出来る)[。.．）)」]*$/

/** Flatten goals into can-do checklist items: split internal newlines, lift a
 *  leading 【tag】, strip leading enumeration (the UI renumbers), flag can-do. */
export function classifyGoals(goals: string[]): GoalItem[] {
	const out: GoalItem[] = []
	for (const g of goals) {
		for (const raw of g.split('\n')) {
			let text = raw.trim()
			if (!text) continue
			let tag: string | undefined
			const m = GOAL_TAG.exec(text)
			if (m) {
				tag = m[1].trim()
				text = text.slice(m[0].length)
			}
			text = text.replace(LEAD_ENUM, '').trim()
			if (!text) continue
			out.push({ tag, text, canDo: CAN_DO.test(text) })
		}
	}
	return out
}

export interface TeacherInfo {
	/** The 代表教員 (KULAS marks it with ◎), name only. */
	rep?: string
	/** Co-instructors, in order. */
	others: string[]
}

const REP_MARK = /^[◎○]\s*/

/** Split 担当教員 into representative + co-instructors, stripping the ◎ marker.
 *  No interpretive labels are added — ◎ is KULAS's own representative mark. */
export function parseTeachers(list: string[]): TeacherInfo {
	let rep: string | undefined
	const others: string[] = []
	for (const t of list) {
		const trimmed = t.trim()
		if (!trimmed) continue
		if (rep === undefined && trimmed.startsWith('◎')) {
			rep = trimmed.replace(REP_MARK, '').trim()
		} else {
			others.push(trimmed.replace(REP_MARK, '').trim())
		}
	}
	return { rep, others }
}

// 科目ナンバリング(##-####-##) decode tables. Structure per the official spec
// (download.pdf p1): 区分A[学部,学科] / 区分B[レベル,大,中,小] / 区分C[授業形態,言語].
// We decode only the digits we could verify against the real data (学部・レベル・
// 授業形態・言語); 学科 and 大/中/小分類 are per-department (huge, alphanumeric) and
// left undecoded, and unknown values (e.g. a newer 授業形態 5) are skipped. The raw
// code is always shown, so this is a supplemental, cohort-dependent best-effort.
const NB_FACULTY: Record<string, string> = {
	'0': '共通教育',
	'1': '人文社会科学部',
	'2': '教育学部',
	'3': '理工学部',
	'4': '医学部',
	'5': '農林海洋科学部',
	'6': '地域協働学部',
}
const NB_LEVEL: Record<string, string> = {
	'0': '初年次・教養',
	'1': '基礎',
	'2': 'プラットフォーム科目',
	'3': '専門科目',
	'4': '卒業論文',
}
const NB_FORMAT: Record<string, string> = { '1': '講義', '2': '演習', '3': '実験', '4': '実習' }
const NB_LANG: Record<string, string> = {
	'1': '日本語',
	'2': '英語',
	'3': '日本語・英語',
	'4': '外国語',
	'5': 'その他',
}

export interface NumberingInfo {
	faculty?: string
	level?: string
	format?: string
	/** 授業言語 — the one facet not shown anywhere else, worth surfacing. */
	lang?: string
}

/** Decode a numbering code's resolvable facets, or `null` for a non-standard
 *  shape. Undecodable digits are simply absent. */
export function decodeNumbering(code: string): NumberingInfo | null {
	// AB-CDEF-GH → A=学部 B=学科 C=レベル DEF=大中小 G=授業形態 H=言語.
	const m = /^(\d)\d-(\d)[0-9A-Z]{3}-(\d)(\d)$/.exec(code)
	if (!m) return null
	const [, faculty, level, format, lang] = m
	return {
		faculty: NB_FACULTY[faculty],
		level: NB_LEVEL[level],
		format: NB_FORMAT[format],
		lang: NB_LANG[lang],
	}
}

/** 授業言語 decoded from the first parseable numbering code, if non-Japanese —
 *  the one facet worth surfacing (日本語 is the unstated default, so it's null). */
export function courseLanguage(numbering: readonly string[] | undefined): string | null {
	for (const code of numbering ?? []) {
		const lang = decodeNumbering(code)?.lang
		if (lang && lang !== '日本語') return lang
	}
	return null
}

export interface ProseBlock {
	kind: 'paragraph' | 'list'
	items: string[]
}

const BULLET = /^(?:[・･]|[①-⑳]|\(?\d+\)?[.．)])\s*/

/** Split prose into paragraph / bullet-list blocks on existing structure. A run
 *  of bullet lines becomes a list (marker stripped); everything else stays as
 *  verbatim paragraphs. No structure → a single paragraph. */
export function formatProse(s: string): ProseBlock[] {
	const lines = s.split('\n').map((l) => l.trim())
	const blocks: ProseBlock[] = []
	let para: string[] = []
	const flush = () => {
		if (para.length) {
			blocks.push({ kind: 'paragraph', items: [para.join('\n')] })
			para = []
		}
	}
	let i = 0
	while (i < lines.length) {
		const line = lines[i]
		if (!line) {
			flush()
			i++
		} else if (BULLET.test(line)) {
			flush()
			const items: string[] = []
			while (i < lines.length && lines[i] && BULLET.test(lines[i])) {
				items.push(lines[i].replace(BULLET, '').trim())
				i++
			}
			blocks.push({ kind: 'list', items })
		} else {
			para.push(line)
			i++
		}
	}
	flush()
	return blocks
}

export interface TextPart {
	text: string
	href?: string
	kind?: 'title' | 'email' | 'url'
}

// One pass over the text linkifies, in priority order: http(s) URLs (→ open in a
// new tab, shown with an icon), 『…』 book/work titles (→ Google search, kept
// wrapped in 『』 so a generic title like「経済学基礎」reads as a book), and email
// addresses (→ mailto). ONLY 『』 counts as a title: 「…」 is the general Japanese
// quote (dialogue, emphasis, terms) and must never become a book link. The URL
// class stops at whitespace and Japanese closing punctuation so it never eats a
// trailing 」。）.
const LINK_RE =
	/https?:\/\/[^\s<>"'）」』】、。，]+|『[^』]*』|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g

export interface RelatedPart {
	text: string
	/** When set, `text` is a 5-digit course code that opens that course. */
	code?: string
}

const CODE_RE = /(?<!\d)\d{5}(?!\d)/g

/** Split a「関連科目」blob so 5-digit codes that resolve to a real course become
 *  in-app links; everything else (names, unresolvable codes) stays verbatim. */
export function splitRelated(text: string, known: ReadonlySet<string>): RelatedPart[] {
	const parts: RelatedPart[] = []
	let last = 0
	for (const m of text.matchAll(CODE_RE)) {
		const code = m[0]
		if (!known.has(code)) continue // leave unresolvable codes as plain text
		const i = m.index ?? 0
		if (i > last) parts.push({ text: text.slice(last, i) })
		parts.push({ text: code, code })
		last = i + code.length
	}
	if (last < text.length) parts.push({ text: text.slice(last) })
	return parts
}

/** Is this an extra-field label for the related-courses (COMPUTER LINK) block? */
export function isRelatedLabel(label: string): boolean {
	return (
		label.includes('関連科目') ||
		label.includes('RELATED COURSES') ||
		label.includes('COMPUTER LINK')
	)
}

/** Split text into plain / linked parts (book titles + emails). */
export function linkifyText(s: string): TextPart[] {
	const parts: TextPart[] = []
	let last = 0
	for (const m of s.matchAll(LINK_RE)) {
		const i = m.index ?? 0
		if (i > last) parts.push({ text: s.slice(last, i) })
		const tok = m[0]
		if (tok.startsWith('http')) {
			parts.push({ text: tok, href: tok, kind: 'url' })
		} else if (tok.startsWith('『')) {
			const inner = tok.slice(1, -1).trim()
			parts.push({
				text: tok,
				kind: 'title',
				href: inner
					? `https://www.google.com/search?q=${encodeURIComponent(`『${inner}』`)}`
					: undefined,
			})
		} else {
			parts.push({ text: tok, href: `mailto:${tok}`, kind: 'email' })
		}
		last = i + tok.length
	}
	if (last < s.length) parts.push({ text: s.slice(last) })
	return parts
}
