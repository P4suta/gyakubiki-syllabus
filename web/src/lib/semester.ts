// Pick the timetable's default semester from the current date. Kochi U runs two
// terms; the exact per-year boundaries aren't published as text (image only), so
// this approximates by month: show a term only while it's clearly in session,
// and fall back to「全て」during the exam/break shoulder months (Feb–Mar, Aug–Sep)
// where switching to the next term would be jarring.

/** The term clearly in session for `month` (1-12), or null in a shoulder month. */
export function seasonSemester(month: number): '1学期' | '2学期' | null {
	if (month >= 4 && month <= 7) return '1学期'
	if (month >= 10 || month === 1) return '2学期'
	return null // 2, 3, 8, 9 — exams / long break
}

/**
 * The default semester filter: the in-session term when it exists in the data,
 * otherwise the「全て」sentinel (`'all'`).
 */
export function defaultSemester(available: readonly string[], now = new Date()): string {
	const term = seasonSemester(now.getMonth() + 1)
	return term && available.includes(term) ? term : 'all'
}
