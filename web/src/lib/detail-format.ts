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

// Conservative numbering decode: only the leading 2 digits, only for verified
// faculty prefixes, else null. The raw code is always shown alongside — this is
// a supplemental hint, never a replacement (fidelity).
const NUMBERING_FIELDS: Record<string, string> = {
	'01': '共通教育',
	'02': '共通教育',
	'03': '共通教育',
	'04': '共通教育',
	'05': '共通教育',
	'32': '理工学部',
	'33': '理工学部',
	'34': '理工学部',
	'35': '理工学部',
	'36': '理工学部',
	'41': '医学部 医学科',
	'42': '医学部 看護学科',
	'61': '地域協働学部',
}

/** A faculty label for a `##-####-##` numbering code, or null when unverified. */
export function decodeNumbering(code: string): { field: string } | null {
	if (!/^\d{2}-\d{2}[0-9A-Z]{2}-\d{2}$/.test(code)) return null
	const field = NUMBERING_FIELDS[code.slice(0, 2)]
	return field ? { field } : null
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
}

// One pass over the text linkifies both 『…』/「…」 book titles (→ Google search,
// wrapped in 『』 so a generic title like「経済学基礎」reads as a book) and email
// addresses (→ mailto). Composed into a single regex so matches never nest.
const LINK_RE =
	/『[^』]*』|「[^」]*」|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g

/** Split text into plain / linked parts (book titles + emails). */
export function linkifyText(s: string): TextPart[] {
	const parts: TextPart[] = []
	let last = 0
	for (const m of s.matchAll(LINK_RE)) {
		const i = m.index ?? 0
		if (i > last) parts.push({ text: s.slice(last, i) })
		const tok = m[0]
		if (tok.startsWith('『') || tok.startsWith('「')) {
			const inner = tok.slice(1, -1).trim()
			parts.push({
				text: tok,
				href: inner
					? `https://www.google.com/search?q=${encodeURIComponent(`『${inner}』`)}`
					: undefined,
			})
		} else {
			parts.push({ text: tok, href: `mailto:${tok}` })
		}
		last = i + tok.length
	}
	if (last < s.length) parts.push({ text: s.slice(last) })
	return parts
}
