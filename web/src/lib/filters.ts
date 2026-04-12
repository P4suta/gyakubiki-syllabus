import type { Course } from '../types/course'

// Normalize full-width spaces to half-width for consistent matching.
function normalize(s: string): string {
	return s.replace(/\u3000/g, ' ').toLowerCase()
}

export function filterCourses(
	courses: Course[],
	semester: string,
	department: string,
	search: string,
): Course[] {
	const query = normalize(search)

	return courses.filter((c) => {
		if (department !== 'all' && c.sekininBushoNm !== department) {
			return false
		}

		if (query) {
			const haystack = normalize(
				[c.kogiNm, c.fukudai, c.tantoKyoin, c.kogiCd, c.sekininBushoNm]
					.filter(Boolean)
					.join(' '),
			)
			if (!haystack.includes(query)) {
				return false
			}
		}

		if (semester !== 'all') {
			const slots = c.slots ?? []
			if (!slots.some((s) => s.semester === semester || s.semester === '通年')) {
				return false
			}
		}

		return true
	})
}
