export interface CourseColor {
	bg: string
	border: string
	text: string
}

const COLORS: CourseColor[] = [
	{ bg: '#e8f0fe', border: '#4285f4', text: '#1d1d1f' },
	{ bg: '#e6f4ea', border: '#34a853', text: '#1d1d1f' },
	{ bg: '#fef7e0', border: '#f9ab00', text: '#1d1d1f' },
	{ bg: '#fce8ef', border: '#e8607a', text: '#1d1d1f' },
	{ bg: '#eae6ff', border: '#7c6cdb', text: '#1d1d1f' },
	{ bg: '#fde8d8', border: '#e8844a', text: '#1d1d1f' },
	{ bg: '#f3e8ff', border: '#9b72cf', text: '#1d1d1f' },
	{ bg: '#e0f5f0', border: '#40b5a0', text: '#1d1d1f' },
	{ bg: '#fde7e7', border: '#dc6060', text: '#1d1d1f' },
	{ bg: '#eceef1', border: '#7c8590', text: '#1d1d1f' },
]

export function getColor(kogiCd: string): CourseColor {
	let hash = 0
	for (let i = 0; i < kogiCd.length; i++) {
		hash = ((hash << 5) - hash + kogiCd.charCodeAt(i)) | 0
	}
	return COLORS[Math.abs(hash) % COLORS.length]
}
