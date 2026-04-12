import type { Course } from '../types/course'

export const DAYS = ['月', '火', '水', '木', '金', '土'] as const
export const PERIODS = [1, 2, 3, 4, 5, 6] as const

export type GridKey = `${string}-${number}`

export function buildGrid(courses: Course[], semester: string): Map<GridKey, Course[]> {
	const grid = new Map<GridKey, Course[]>()

	for (const d of DAYS) {
		for (const p of PERIODS) {
			grid.set(`${d}-${p}`, [])
		}
	}

	for (const course of courses) {
		for (const slot of course.slots ?? []) {
			if (semester !== 'all' && slot.semester !== semester && slot.semester !== '通年') {
				continue
			}
			const key: GridKey = `${slot.day}-${slot.period}`
			const cell = grid.get(key)
			if (cell && !cell.some((c) => c.kogiCd === course.kogiCd)) {
				cell.push(course)
			}
		}
	}

	return grid
}

export function countUnique(grid: Map<GridKey, Course[]>): number {
	const seen = new Set<string>()
	for (const courses of grid.values()) {
		for (const c of courses) {
			seen.add(c.kogiCd)
		}
	}
	return seen.size
}
