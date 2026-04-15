export interface CourseColor {
	bg: string
	border: string
	text: string
}

const GROUP_HUES: [number, number][] = [
	[25, 15],   // 0: 人文社会科学部 — orange
	[220, 12],  // 1: 全学開設科目 — slate blue
	[145, 18],  // 2: 共通教育 — green
	[350, 15],  // 3: 医学部 — rose
	[45, 15],   // 4: 地域協働学部 — amber
	[180, 15],  // 5: 教育学部 — teal
	[215, 18],  // 6: 理工学部 — blue
	[270, 18],  // 7: 総合人間自然科学研究科 — purple
	[115, 18],  // 8: 農林海洋科学部 — emerald
]

const GROUP_PREFIXES: string[] = [
	'人文社会科学部',
	'全学開設科目',
	'共通教育',
	'医学部',
	'地域協働学部',
	'教育学部',
	'理工学部',
	'総合人間自然科学研究科',
	'農林海洋科学部',
]

let deptGroupMap: number[] = []

export function initDeptColors(departments: string[]): void {
	deptGroupMap = departments.map((name) => {
		const idx = GROUP_PREFIXES.findIndex((prefix) => name.startsWith(prefix))
		return idx >= 0 ? idx : 1
	})
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

export function getColor(deptIndex: number, courseCode: string): CourseColor {
	const group = deptGroupMap[deptIndex] ?? 1
	const [hueCenter, hueSpread] = GROUP_HUES[group]
	const hash = hashCode(courseCode)

	const hueOffset = (hash % (hueSpread * 2 + 1)) - hueSpread
	const hue = ((hueCenter + hueOffset) % 360 + 360) % 360

	const satBg = 35 + (hash % 15)
	const litBg = 90 + ((hash >> 4) % 5)
	const satBorder = 55 + ((hash >> 8) % 15)
	const litBorder = 50 + ((hash >> 12) % 10)

	return {
		bg: hslToHex(hue, satBg, litBg),
		border: hslToHex(hue, satBorder, litBorder),
		text: '#1d1d1f',
	}
}
