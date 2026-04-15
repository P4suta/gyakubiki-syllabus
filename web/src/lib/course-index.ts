import type { CourseV2, Dictionaries, IndicesMap } from '../types/course'
import { BitSet } from './bitset'

export function normalize(s: string): string {
	return s.replace(/\u3000/g, ' ').toLowerCase()
}

export class CourseIndex {
	readonly courses: CourseV2[]
	private readonly dicts: Dictionaries
	private readonly semesterBitsets: Map<number, BitSet>
	private readonly departmentBitsets: Map<number, BitSet>
	private readonly campusBitsets: Map<number, BitSet>
	private readonly allBits: BitSet

	constructor(courses: CourseV2[], dicts: Dictionaries, indices: IndicesMap) {
		this.courses = courses
		this.dicts = dicts
		this.allBits = BitSet.allOnes(courses.length)

		this.semesterBitsets = decodeBitsetMap(indices.semester)
		this.departmentBitsets = decodeBitsetMap(indices.department)
		this.campusBitsets = decodeBitsetMap(indices.campus)
	}

	filter(semester: string, department: string, campus: string, query: string): CourseV2[] {
		if (this.courses.length === 0) return []

		let bits = this.allBits

		if (semester !== 'all') {
			const semIdx = this.dicts.semesters.indexOf(semester)
			const semBits = semIdx >= 0 ? this.semesterBitsets.get(semIdx) : undefined
			bits = semBits ? bits.and(semBits) : BitSet.allOnes(0)
		}

		if (department !== 'all') {
			const deptIdx = this.dicts.departments.indexOf(department)
			const deptBits = deptIdx >= 0 ? this.departmentBitsets.get(deptIdx) : undefined
			bits = deptBits ? bits.and(deptBits) : BitSet.allOnes(0)
		}

		if (campus !== 'all') {
			const campIdx = this.dicts.campuses.indexOf(campus)
			const campBits = campIdx >= 0 ? this.campusBitsets.get(campIdx) : undefined
			bits = campBits ? bits.and(campBits) : BitSet.allOnes(0)
		}

		const candidateIndices = bits.popIndices()

		if (query) {
			const normalizedQuery = normalize(query)
			return candidateIndices
				.filter((i) => this.courses[i].st.includes(normalizedQuery))
				.map((i) => this.courses[i])
		}

		return candidateIndices.map((i) => this.courses[i])
	}
}

function decodeBitsetMap(encoded: Record<string, string>): Map<number, BitSet> {
	const map = new Map<number, BitSet>()
	for (const [key, value] of Object.entries(encoded)) {
		map.set(Number(key), BitSet.fromBase64(value))
	}
	return map
}
