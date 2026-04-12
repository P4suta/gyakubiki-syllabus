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
}

export interface ProcessedData {
	version: number
	generatedAt: string
	totalRaw: number
	courses: Course[]
	semesters: string[]
	departments: string[]
}
