export interface Tint {
	bg: string
	border: string
	text: string
	/**
	 * The accent rendered *as text* on `bg` (the card's meta line). Darker (light
	 * theme) / lighter (dark theme) than `border`, which is a decoration colour and
	 * fails WCAG AA as text on its own tint. Each `accentText`/`text` clears 4.5:1
	 * on its `bg` — locked by colors.test.ts.
	 */
	accentText: string
}

/** A course tint in both themes. The consumer picks by `prefers-color-scheme`. */
export interface CourseColor {
	light: Tint
	dark: Tint
}

/**
 * The course-tile palette — 10 hues placed on a shared OKLCH ring (equal
 * lightness & chroma per theme) so the tints read as one calm family rather than
 * ad-hoc web primaries. Exported so colors.test.ts can lock every contrast pair.
 * Light tiles: L≈0.955 pastel; dark tiles: L≈0.265 deep. `text` is the theme ink.
 */
export const COLORS: CourseColor[] = [
	{
		light: { bg: '#def2ff', border: '#4885df', text: '#1c1e27', accentText: '#3463a6' },
		dark: { bg: '#132540', border: '#427fd8', text: '#e9eaf0', accentText: '#74a6ef' },
	},
	{
		light: { bg: '#d9fadf', border: '#429c5a', text: '#1c1e27', accentText: '#21763c' },
		dark: { bg: '#0a2d15', border: '#3b9555', text: '#e9eaf0', accentText: '#69ba7c' },
	},
	{
		light: { bg: '#fcf0cb', border: '#a28200', text: '#1c1e27', accentText: '#7c5f00' },
		dark: { bg: '#302400', border: '#9c7c00', text: '#e9eaf0', accentText: '#c0a241' },
	},
	{
		light: { bg: '#ffe4f2', border: '#c55e8a', text: '#1c1e27', accentText: '#964267' },
		dark: { bg: '#391927', border: '#bf5884', text: '#e9eaf0', accentText: '#df84a8' },
	},
	{
		light: { bg: '#eeecff', border: '#8374da', text: '#1c1e27', accentText: '#6156a3' },
		dark: { bg: '#24203e', border: '#7e6ed3', text: '#e9eaf0', accentText: '#a198eb' },
	},
	{
		light: { bg: '#ffe8d1', border: '#c56c21', text: '#1c1e27', accentText: '#964d09' },
		dark: { bg: '#3b1c04', border: '#be6517', text: '#e9eaf0', accentText: '#de8f57' },
	},
	{
		light: { bg: '#fee7ff', border: '#ad65be', text: '#1c1e27', accentText: '#814a8d' },
		dark: { bg: '#311c36', border: '#a75fb7', text: '#e9eaf0', accentText: '#c68bd3' },
	},
	{
		light: { bg: '#cafbfa', border: '#009c9c', text: '#1c1e27', accentText: '#007778' },
		dark: { bg: '#002e2e', border: '#009696', text: '#e9eaf0', accentText: '#16bbbc' },
	},
	{
		light: { bg: '#ffe5e1', border: '#d15c56', text: '#1c1e27', accentText: '#9c433f' },
		dark: { bg: '#3c1917', border: '#ca5551', text: '#e9eaf0', accentText: '#e6857e' },
	},
	{
		light: { bg: '#ebf0f8', border: '#7f8793', text: '#1c1e27', accentText: '#5d646f' },
		dark: { bg: '#1f2630', border: '#79818d', text: '#e9eaf0', accentText: '#9da5b1' },
	},
]

export function getColor(kogiCd: string): CourseColor {
	let hash = 0
	for (let i = 0; i < kogiCd.length; i++) {
		hash = ((hash << 5) - hash + kogiCd.charCodeAt(i)) | 0
	}
	return COLORS[Math.abs(hash) % COLORS.length]
}
