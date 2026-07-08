// WCAG 2.1 relative-luminance contrast, used by the palette tests to lock every
// text color at AA (4.5:1). Kept dependency-free; not imported by the app, so it
// never reaches the bundle.

/** Parse `#rrggbb` (or `#rgb`) into 8-bit `[r, g, b]`. */
export function parseHex(hex: string): [number, number, number] {
	const h = hex.replace('#', '')
	const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h
	return [0, 2, 4].map((i) => Number.parseInt(full.slice(i, i + 2), 16)) as [number, number, number]
}

/** WCAG relative luminance of an sRGB color. */
export function luminance([r, g, b]: [number, number, number]): number {
	const lin = (c: number) => {
		const s = c / 255
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
	}
	return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/** WCAG contrast ratio (1–21) between two opaque hex colors. */
export function contrastRatio(fg: string, bg: string): number {
	const l1 = luminance(parseHex(fg))
	const l2 = luminance(parseHex(bg))
	const [hi, lo] = l1 >= l2 ? [l1, l2] : [l2, l1]
	return (hi + 0.05) / (lo + 0.05)
}
