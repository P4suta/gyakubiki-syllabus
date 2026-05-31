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
	st: string // searchText
}

export interface Dictionaries {
	semesters: string[]
	departments: string[]
	campuses: string[]
	kubun: string[]
	kaikojiki: string[]
}
