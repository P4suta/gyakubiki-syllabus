export interface CourseColor {
	bg: string
	border: string
	text: string
}

const KUBUN_COLOR_MAP: Record<string, CourseColor> = {
	'講義': { bg: '#e8f0fe', border: '#4285f4', text: '#1d1d1f' },
	'演習': { bg: '#e6f4ea', border: '#34a853', text: '#1d1d1f' },
	'実験': { bg: '#eae6ff', border: '#7c6cdb', text: '#1d1d1f' },
	'実習': { bg: '#fde8d8', border: '#e8844a', text: '#1d1d1f' },
	'実技': { bg: '#e0f5f0', border: '#40b5a0', text: '#1d1d1f' },
}

const FALLBACK: CourseColor = { bg: '#eceef1', border: '#7c8590', text: '#1d1d1f' }

let kubunNames: string[] = []

export function initKubunColors(kubun: string[]): void {
	kubunNames = kubun
}

export function getColor(kubunIndex: number): CourseColor {
	const name = kubunNames[kubunIndex]
	if (name && name in KUBUN_COLOR_MAP) {
		return KUBUN_COLOR_MAP[name]
	}
	return FALLBACK
}
