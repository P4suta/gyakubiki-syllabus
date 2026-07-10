export interface Tint {
	bg: string
	border: string
	/** Course name — the theme ink (max contrast on the tile). */
	text: string
	/** Professor / credits line — a calm, tile-tinted text that clears AA on `bg`. */
	mutedText: string
	/** The meta line rendered as text on `bg` — a vivid, tile-tinted accent. */
	accentText: string
}

/** A course tint in both themes. The consumer picks by `prefers-color-scheme`. */
export interface CourseColor {
	light: Tint
	dark: Tint
}

/**
 * "Macaron" course palette — derived, not hand-picked. 10 hues evenly spaced in
 * OKLCH; every tile holds ONE chroma (equal vividness, no neon/washed outliers)
 * at the lightest lightness that can carry it (so yellow stays light, blue a
 * touch deeper — each hue clean, none muddy/olive). `mutedText` / `accentText`
 * lightness is solved to clear WCAG AA (4.5:1) on the tile — all locked by
 * colors.test.ts. See DESIGN.md for the derivation; regenerate, don't hand-edit.
 */
export const COLORS: CourseColor[] = [
	{ light: { bg: '#fec7df', border: '#ff8ec6', text: '#4d2e3d', mutedText: '#94406c', accentText: '#ac1f74' }, dark: { bg: '#2e0d1e', border: '#9b3b6e', text: '#d0a4b8', mutedText: '#ca5a95', accentText: '#ea2e9f' } },
	{ light: { bg: '#ffc3bc', border: '#fe847a', text: '#4b2c29', mutedText: '#943f39', accentText: '#ad1e22' }, dark: { bg: '#310d0b', border: '#a43b36', text: '#cfa5a1', mutedText: '#d45d55', accentText: '#f53739' } },
	{ light: { bg: '#fed2ac', border: '#ffa348', text: '#45372b', mutedText: '#7c5837', accentText: '#895117' }, dark: { bg: '#381c00', border: '#bc6c00', text: '#d4b498', mutedText: '#b38052', accentText: '#c47725' } },
	{ light: { bg: '#fef3bd', border: '#d8be34', text: '#4d4937', mutedText: '#796f40', accentText: '#7d6d1b' }, dark: { bg: '#342c00', border: '#b09803', text: '#cec599', mutedText: '#9f9356', accentText: '#a89428' } },
	{ light: { bg: '#dffdcd', border: '#95d26b', text: '#404e38', mutedText: '#577940', accentText: '#487c1b' }, dark: { bg: '#0e2001', border: '#3f7307', text: '#9cbb89', mutedText: '#668e4c', accentText: '#579322' } },
	{ light: { bg: '#bffee7', border: '#2cdcaf', text: '#384b44', mutedText: '#437665', accentText: '#1f7b62' }, dark: { bg: '#00382a', border: '#01bc93', text: '#a4d6c3', mutedText: '#60a58e', accentText: '#2fab89' } },
	{ light: { bg: '#b0f5fe', border: '#00889c', text: '#35474a', mutedText: '#417076', accentText: '#1d737c' }, dark: { bg: '#00444a', border: '#00bccf', text: '#c1e4e9', mutedText: '#69b2bb', accentText: '#34b8c7' } },
	{ light: { bg: '#acdafe', border: '#3db2fe', text: '#2a3944', mutedText: '#375f7d', accentText: '#18608e' }, dark: { bg: '#00253c', border: '#0285c9', text: '#a2bdd2', mutedText: '#5590bb', accentText: '#2890d1' } },
	{ light: { bg: '#c6cdff', border: '#8d97fe', text: '#2d3159', mutedText: '#4a49bb', accentText: '#4e2def' }, dark: { bg: '#151636', border: '#5357b1', text: '#aaafcd', mutedText: '#727acf', accentText: '#7074f3' } },
	{ light: { bg: '#eed0fe', border: '#de9cfe', text: '#493354', mutedText: '#83459f', accentText: '#9723c3' }, dark: { bg: '#24112e', border: '#80469a', text: '#c0a6cd', mutedText: '#ad61d1', accentText: '#c143f4' } },
]

export function getColor(kogiCd: string): CourseColor {
	let hash = 0
	for (let i = 0; i < kogiCd.length; i++) {
		hash = ((hash << 5) - hash + kogiCd.charCodeAt(i)) | 0
	}
	return COLORS[Math.abs(hash) % COLORS.length]
}
