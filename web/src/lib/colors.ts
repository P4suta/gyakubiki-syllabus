export interface CourseColor {
	bg: string
	border: string
	text: string
}

const COLORS: CourseColor[] = [
	{ bg: '#dbeafe', border: '#3b82f6', text: '#1e3a5f' },
	{ bg: '#dcfce7', border: '#22c55e', text: '#14532d' },
	{ bg: '#fef9c3', border: '#eab308', text: '#713f12' },
	{ bg: '#fce7f3', border: '#ec4899', text: '#831843' },
	{ bg: '#e0e7ff', border: '#6366f1', text: '#312e81' },
	{ bg: '#ffedd5', border: '#f97316', text: '#7c2d12' },
	{ bg: '#f3e8ff', border: '#a855f7', text: '#581c87' },
	{ bg: '#ccfbf1', border: '#14b8a6', text: '#134e4a' },
	{ bg: '#fee2e2', border: '#ef4444', text: '#7f1d1d' },
	{ bg: '#e2e8f0', border: '#64748b', text: '#1e293b' },
]

export function getColor(kogiCd: string): CourseColor {
	let hash = 0
	for (let i = 0; i < kogiCd.length; i++) {
		hash = ((hash << 5) - hash + kogiCd.charCodeAt(i)) | 0
	}
	return COLORS[Math.abs(hash) % COLORS.length]
}
