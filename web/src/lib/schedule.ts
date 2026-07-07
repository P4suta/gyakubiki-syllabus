// Kochi University's fixed class-period times: a 5-period day plus a 6th slot
// used for makeup (補講) classes. Saturdays/Sundays are normally holidays;
// concentrated courses and term exams run in these same slots.
// Source: https://www.kochi-u.ac.jp/education-support/courses/class-schedule/

export interface PeriodTime {
	start: string
	end: string
	/** Non-empty for slots that aren't regular (e.g. the 6th makeup period). */
	note?: string
}

export const PERIOD_TIMES: Record<number, PeriodTime> = {
	1: { start: '8:50', end: '10:20' },
	2: { start: '10:30', end: '12:00' },
	3: { start: '13:10', end: '14:40' },
	4: { start: '14:50', end: '16:20' },
	5: { start: '16:30', end: '18:00' },
	6: { start: '18:10', end: '19:40', note: '補講' },
}
