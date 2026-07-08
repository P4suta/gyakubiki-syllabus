export interface CourseColor {
	bg: string
	border: string
	text: string
	/**
	 * The accent rendered *as text* on `bg` (the card's meta line). Darker than
	 * `border`, which is a border/decoration color and fails WCAG AA as text on
	 * its own tint. Each value clears 4.5:1 on its `bg` — locked by colors.test.ts.
	 */
	accentText: string
}

/** The full course-color palette, exported so colors.test.ts can lock its contrast. */
export const COLORS: CourseColor[] = [
	{ bg: '#e8f0fe', border: '#4285f4', text: '#1d1d1f', accentText: '#3264b7' },
	{ bg: '#e6f4ea', border: '#34a853', text: '#1d1d1f', accentText: '#247439' },
	{ bg: '#fef7e0', border: '#f9ab00', text: '#1d1d1f', accentText: '#8e6100' },
	{ bg: '#fce8ef', border: '#e8607a', text: '#1d1d1f', accentText: '#a54457' },
	{ bg: '#eae6ff', border: '#7c6cdb', text: '#1d1d1f', accentText: '#6255ad' },
	{ bg: '#fde8d8', border: '#e8844a', text: '#1d1d1f', accentText: '#92532f' },
	{ bg: '#f3e8ff', border: '#9b72cf', text: '#1d1d1f', accentText: '#74569b' },
	{ bg: '#e0f5f0', border: '#40b5a0', text: '#1d1d1f', accentText: '#287265' },
	{ bg: '#fde7e7', border: '#dc6060', text: '#1d1d1f', accentText: '#a34747' },
	{ bg: '#eceef1', border: '#7c8590', text: '#1d1d1f', accentText: '#5e656d' },
]

export function getColor(kogiCd: string): CourseColor {
	let hash = 0
	for (let i = 0; i < kogiCd.length; i++) {
		hash = ((hash << 5) - hash + kogiCd.charCodeAt(i)) | 0
	}
	return COLORS[Math.abs(hash) % COLORS.length]
}
