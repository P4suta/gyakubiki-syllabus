export interface Slot {
	semester: string
	day: string
	period: number
}

export interface Course {
	kogiCd: string
	kogiNm: string
	fukudai?: string
	tantoKyoin: string
	jikanwariRaw: string
	slots: Slot[]
	kogiKaikojikiNm: string
	kogiKubunNm: string
	sekininBushoNm: string
	kochiNm: string
	gakusokuKamokuNm: string
	taishoGakka?: string
	taishoNenji?: string
	kamokuBunrui?: string
	kamokuBunya?: string
	searchText?: string
}

export interface ProcessedData {
	version: number
	generatedAt: string
	totalRaw: number
	courses: Course[]
	semesters: string[]
	departments: string[]
	campuses: string[]
}

// --- v2 types ---

export interface SlotV2 {
	s: number // semester index into dicts.semesters
	d: number // day index: 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日
	p: number // period: 1-8
}

export interface CourseV2 {
	cd: string
	nm: string
	sub?: string
	prof: string
	raw: string
	slots: SlotV2[]
	ki: number      // kaikojiki index
	kbn: number     // kubun index
	dept: number    // department index
	campus: number  // campus index
	gaku?: string   // gakusokuKamokuNm (only when != nm)
	gakka?: string
	nen?: string
	bunrui?: string
	bunya?: string
	st: string      // searchText
}

export interface Dictionaries {
	semesters: string[]
	departments: string[]
	campuses: string[]
	kubun: string[]
	kaikojiki: string[]
}

// The full v3 wire envelope (top-level `ProcessedDataV2`, positional bitset
// `IndicesMap`) lives solely in the Rust core: it is parsed inside WASM and
// never materializes in TS. Only the view-models above (CourseV2 / SlotV2 /
// Dictionaries) cross the boundary, so only they need a TS type.
