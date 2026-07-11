// View-model types that cross the WASM boundary into the UI.
//
// The full v3 wire envelope (top-level `ProcessedData`, positional bitset
// `IndicesMap`) lives solely in the Rust core: it is parsed inside WASM and
// never materializes in TS. Only the view-models here — resolved from the
// engine's `allCourseViews()` and `dicts()` — cross the boundary.

export interface Slot {
	s: number // semester index into dicts.semesters
	d: number // day index: 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日
	p: number // period: 1-8
}

export interface Course {
	cd: string
	nm: string
	sub?: string
	prof: string
	raw: string
	slots: Slot[]
	ki: number // kaikojiki index
	kbn: number // kubun index
	dept: number // department index
	campus: number // campus index
	gaku?: string // gakusokuKamokuNm (only when != nm)
	gakka?: string
	nen?: string
	bunrui?: string
	bunya?: string
	pat?: string // syllabusKomokuPatternId, for the official syllabus deep link
	unit?: string // credits (from syllabus detail)
	dm?: string // delivery mode: onsite | online | ondemand | hybrid
	ev?: string[] // assessment summary for the card, e.g. ["attendance:40","exam:60"]
}

// --- Full syllabus detail (lazy-loaded from details/{cd}.json) ---
// Mirrors the Rust `SanshoDetail` emitted by `syllabus-cli`.

export interface Delivery {
	mode: string
	raw?: string
	isMedia?: boolean
}

export interface EvalRow {
	item: string
	weight?: number
	type: string // exam | report | minireport | attendance | quiz | other
}

export interface Eval {
	rows: EvalRow[]
	note?: string
}

export interface PlanItem {
	n: number
	text: string
	/** Highlight hint from `enrich`: exam | milestone | start (else absent). */
	kind?: string
}

// --- Derived at convert time (see crates/cli/src/detail/enrich.rs) ---

export interface TextbookSection {
	label?: string
	lines: string[]
}

export interface TextbookInfo {
	isNone: boolean
	sections: TextbookSection[]
}

export interface PrepInfo {
	hours?: number
	yoshu?: string
	fukushu?: string
}

export interface OfficeHour {
	name?: string
	day?: string
	time?: string
	place?: string
}

export interface Labelled {
	label: string
	text: string
}

export interface CourseDetail {
	cd: string
	unit?: string
	delivery?: Delivery
	eval?: Eval
	summary?: string
	aims?: string
	goals?: string[]
	plan?: PlanItem[]
	textbooks?: string
	prereq?: string
	prep?: string
	officeHour?: OfficeHour[]
	keywords?: string[]
	teachers?: string[]
	numbering?: string[]
	sdgs?: string[]
	extra?: Labelled[]
	textbookInfo?: TextbookInfo
	prepInfo?: PrepInfo
}

export interface Dictionaries {
	semesters: string[]
	departments: string[]
	campuses: string[]
	kubun: string[]
	kaikojiki: string[]
}
