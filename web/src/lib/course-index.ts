import type { Course } from '../types/course'

export function normalize(s: string): string {
	return s.replace(/\u3000/g, ' ').toLowerCase()
}

function buildHaystack(c: Course): string {
	if (c.searchText) return c.searchText
	return normalize(
		[c.kogiNm, c.fukudai, c.tantoKyoin, c.kogiCd, c.sekininBushoNm]
			.filter(Boolean)
			.join(' '),
	)
}

export class CourseIndex {
	readonly courses: Course[]
	private readonly haystacks: string[]
	private readonly byDepartment: Map<string, number[]>
	private readonly bySemester: Map<string, Set<number>>

	constructor(courses: Course[]) {
		this.courses = courses
		this.haystacks = courses.map(buildHaystack)

		this.byDepartment = new Map()
		for (let i = 0; i < courses.length; i++) {
			const dept = courses[i].sekininBushoNm
			let arr = this.byDepartment.get(dept)
			if (!arr) {
				arr = []
				this.byDepartment.set(dept, arr)
			}
			arr.push(i)
		}

		this.bySemester = new Map()
		const tsuunenIndices: number[] = []

		for (let i = 0; i < courses.length; i++) {
			for (const slot of courses[i].slots ?? []) {
				if (slot.semester === '通年') {
					tsuunenIndices.push(i)
				} else {
					let set = this.bySemester.get(slot.semester)
					if (!set) {
						set = new Set()
						this.bySemester.set(slot.semester, set)
					}
					set.add(i)
				}
			}
		}

		for (const set of this.bySemester.values()) {
			for (const idx of tsuunenIndices) {
				set.add(idx)
			}
		}
	}

	filter(semester: string, department: string, query: string): Course[] {
		const normalizedQuery = query ? normalize(query) : ''

		let indices: Iterable<number>

		if (department !== 'all') {
			indices = this.byDepartment.get(department) ?? []
		} else if (semester !== 'all') {
			indices = this.bySemester.get(semester) ?? new Set()
		} else {
			indices = this.courses.map((_, i) => i)
		}

		const needSemesterCheck = semester !== 'all' && department !== 'all'
		const semesterSet = needSemesterCheck
			? this.bySemester.get(semester)
			: undefined

		const results: Course[] = []

		for (const i of indices) {
			if (needSemesterCheck && !(semesterSet?.has(i) ?? false)) {
				continue
			}
			if (normalizedQuery && !this.haystacks[i].includes(normalizedQuery)) {
				continue
			}
			results.push(this.courses[i])
		}

		return results
	}
}
