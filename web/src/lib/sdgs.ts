// SDG goal metadata for the detail sheet. The course data carries only the goal
// number (e.g. "4"); the title/colour/slug are static (goals 1–17 never change).
// The colour is the official UN SDG hue, used only as a decorative badge — the
// goal title is what carries the meaning as AA-locked ink text.

export interface Sdg {
	title: string
	/** Official UN SDG colour, for the number badge (non-text sign). */
	color: string
	/** UNICEF SDGs CLUB per-goal page slug. */
	slug: string
}

// Slugs verified against unicef.or.jp/kodomo/sdgs/17goals/. Colours are the
// official UN SDG palette.
const GOALS: Record<number, Sdg> = {
	1: { title: '貧困をなくそう', color: '#e5243b', slug: '1-poverty' },
	2: { title: '飢餓をゼロに', color: '#dda63a', slug: '2-hunger' },
	3: { title: 'すべての人に健康と福祉を', color: '#4c9f38', slug: '3-health' },
	4: { title: '質の高い教育をみんなに', color: '#c5192d', slug: '4-education' },
	5: { title: 'ジェンダー平等を実現しよう', color: '#ff3a21', slug: '5-gender' },
	6: { title: '安全な水とトイレを世界中に', color: '#26bde2', slug: '6-water' },
	7: { title: 'エネルギーをみんなに そしてクリーンに', color: '#fcc30b', slug: '7-energy' },
	8: { title: '働きがいも経済成長も', color: '#a21942', slug: '8-economic_growth' },
	9: { title: '産業と技術革新の基盤をつくろう', color: '#fd6925', slug: '9-industry' },
	10: { title: '人や国の不平等をなくそう', color: '#dd1367', slug: '10-inequalities' },
	11: { title: '住み続けられるまちづくりを', color: '#fd9d24', slug: '11-cities' },
	12: { title: 'つくる責任 つかう責任', color: '#bf8b2e', slug: '12-responsible' },
	13: { title: '気候変動に具体的な対策を', color: '#3f7e44', slug: '13-climate_action' },
	14: { title: '海の豊かさを守ろう', color: '#0a97d9', slug: '14-sea' },
	15: { title: '陸の豊かさも守ろう', color: '#56c02b', slug: '15-land' },
	16: { title: '平和と公正をすべての人に', color: '#00689d', slug: '16-peace' },
	17: { title: 'パートナーシップで目標を達成しよう', color: '#19486a', slug: '17-partnerships' },
}

const UNICEF_BASE = 'https://www.unicef.or.jp/kodomo/sdgs/17goals/'

/**
 * Resolve an SDG goal from a raw value, or `null` if outside 1–17. Reads the
 * leading integer so both the real「4」and a「4 質の高い教育をみんなに」variant work.
 */
export function sdgGoal(raw: string): (Sdg & { n: number; url: string }) | null {
	const m = /^\s*(\d+)/.exec(raw)
	if (!m) return null
	const n = Number(m[1])
	const g = GOALS[n]
	if (!g) return null
	return { ...g, n, url: `${UNICEF_BASE}${g.slug}/` }
}
