import type { CourseV2, Dictionaries } from '../types/course'

const WEEKDAYS = ['月', '火', '水', '木', '金'] as const
export const DAYS: readonly string[] = __HAS_SATURDAY__
	? [...WEEKDAYS, '土']
	: [...WEEKDAYS]
export const PERIODS = [1, 2, 3, 4, 5, 6] as const

export type GridKey = `${string}-${number}`

export function buildGrid(
	courses: CourseV2[],
	semester: string,
	dicts: Dictionaries,
): Map<GridKey, CourseV2[]> {
	const grid = new Map<GridKey, CourseV2[]>()

	for (const d of DAYS) {
		for (const p of PERIODS) {
			grid.set(`${d}-${p}`, [])
		}
	}

	const semIdx = semester !== 'all' ? dicts.semesters.indexOf(semester) : -1
	const tsuunenIdx = dicts.semesters.indexOf('通年')

	for (const course of courses) {
		for (const slot of course.slots) {
			if (semIdx >= 0 && slot.s !== semIdx && slot.s !== tsuunenIdx) {
				continue
			}
			if (slot.d < 0 || slot.d >= DAYS.length) continue
			const day = DAYS[slot.d]
			const key: GridKey = `${day}-${slot.p}`
			const cell = grid.get(key)
			if (cell && !cell.some((c) => c.cd === course.cd)) {
				cell.push(course)
			}
		}
	}

	return grid
}

export function countUnique(grid: Map<GridKey, CourseV2[]>): number {
	const seen = new Set<string>()
	for (const courses of grid.values()) {
		for (const c of courses) {
			seen.add(c.cd)
		}
	}
	return seen.size
}
