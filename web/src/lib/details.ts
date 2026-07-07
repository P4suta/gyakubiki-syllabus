import type { CourseDetail } from '../types/course'

// Lazy-load a course's full syllabus detail (`details/{cd}.json`), produced by
// `syllabus-cli convert`. Cached per course; a missing file (course not yet
// crawled) resolves to `null` so the modal degrades gracefully.
const cache = new Map<string, CourseDetail | null>()

export async function loadDetail(cd: string): Promise<CourseDetail | null> {
	const cached = cache.get(cd)
	if (cached !== undefined) return cached

	try {
		const res = await fetch(`${import.meta.env.BASE_URL}details/${encodeURIComponent(cd)}.json`)
		if (!res.ok) {
			cache.set(cd, null)
			return null
		}
		const detail = (await res.json()) as CourseDetail
		cache.set(cd, detail)
		return detail
	} catch {
		cache.set(cd, null)
		return null
	}
}
