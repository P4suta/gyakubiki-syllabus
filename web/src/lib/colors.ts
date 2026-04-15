export interface CourseColor {
	bg: string
	border: string
	text: string
}

function hashCode(s: string): number {
	let h = 0
	for (let i = 0; i < s.length; i++) {
		h = ((h << 5) - h + s.charCodeAt(i)) | 0
	}
	return Math.abs(h)
}

function hslToHex(h: number, s: number, l: number): string {
	s /= 100
	l /= 100
	const a = s * Math.min(l, 1 - l)
	const f = (n: number) => {
		const k = (n + h / 30) % 12
		const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1)
		return Math.round(255 * color)
			.toString(16)
			.padStart(2, '0')
	}
	return `#${f(0)}${f(8)}${f(4)}`
}

const GOLDEN_ANGLE = 137.508

export function getColor(kogiCd: string): CourseColor {
	const hash = hashCode(kogiCd)
	const hue = (hash * GOLDEN_ANGLE) % 360
	return {
		bg: hslToHex(hue, 40, 93),
		border: hslToHex(hue, 60, 55),
		text: '#1d1d1f',
	}
}
