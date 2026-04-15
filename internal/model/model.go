package model

import "time"

// RawResponse represents the API response from KULAS.
type RawResponse struct {
	SelectKogiDtoList []RawCourse `json:"selectKogiDtoList"`
}

// RawCourse represents a single course from the raw API response.
type RawCourse struct {
	KogiCd           string  `json:"kogiCd"`
	KogiNm           string  `json:"kogiNm"`
	Fukudai          *string `json:"fukudai"`
	TantoKyoin       string  `json:"tantoKyoin"`
	Jikanwari        string  `json:"jikanwari"`
	KogiKaikojikiNm  string  `json:"kogiKaikojikiNm"`
	KogiKubunNm      string  `json:"kogiKubunNm"`
	SekininBushoNm   string  `json:"sekininBushoNm"`
	KochiNm          string  `json:"kochiNm"`
	GakusokuKamokuNm string  `json:"gakusokuKamokuNm"`
	TaishoGakka      *string `json:"taishoGakka"`
	TaishoNenji      *string `json:"taishoNenji"`
	KamokuBunrui     *string `json:"kamokuBunrui"`
	KamokuBunya      *string `json:"kamokuBunya"`
}

// Slot represents a parsed time slot (semester + day + period).
type Slot struct {
	Semester string `json:"semester"`
	Day      string `json:"day"`
	Period   int    `json:"period"`
}

// Course represents a processed course for the frontend.
type Course struct {
	KogiCd           string  `json:"kogiCd"`
	KogiNm           string  `json:"kogiNm"`
	Fukudai          *string `json:"fukudai,omitempty"`
	TantoKyoin       string  `json:"tantoKyoin"`
	JikanwariRaw     string  `json:"jikanwariRaw"`
	Slots            []Slot  `json:"slots"`
	KogiKaikojikiNm  string  `json:"kogiKaikojikiNm"`
	KogiKubunNm      string  `json:"kogiKubunNm"`
	SekininBushoNm   string  `json:"sekininBushoNm"`
	KochiNm          string  `json:"kochiNm"`
	GakusokuKamokuNm string  `json:"gakusokuKamokuNm"`
	TaishoGakka      *string `json:"taishoGakka,omitempty"`
	TaishoNenji      *string `json:"taishoNenji,omitempty"`
	KamokuBunrui     *string `json:"kamokuBunrui,omitempty"`
	KamokuBunya      *string `json:"kamokuBunya,omitempty"`
	SearchText       string  `json:"searchText"`
}

// ProcessedData is the top-level output schema (v1, kept for backward compatibility).
type ProcessedData struct {
	Version     int       `json:"version"`
	GeneratedAt time.Time `json:"generatedAt"`
	TotalRaw    int       `json:"totalRaw"`
	Courses     []Course  `json:"courses"`
	Semesters   []string  `json:"semesters"`
	Departments []string  `json:"departments"`
}

// SlotV2 represents a time slot with dictionary indices instead of strings.
type SlotV2 struct {
	Semester int `json:"s"` // index into Dicts.Semesters
	Day      int `json:"d"` // 0=月, 1=火, 2=水, 3=木, 4=金, 5=土, 6=日
	Period   int `json:"p"` // 1-8
}

// CourseV2 represents a course optimized for the frontend (v2).
type CourseV2 struct {
	KogiCd       string  `json:"cd"`
	KogiNm       string  `json:"nm"`
	Fukudai      *string `json:"sub,omitempty"`
	TantoKyoin   string  `json:"prof"`
	JikanwariRaw string  `json:"raw"`
	Slots        []SlotV2 `json:"slots"`
	Kaikojiki    int     `json:"ki"`            // index into Dicts.Kaikojiki
	Kubun        int     `json:"kbn"`           // index into Dicts.Kubun
	Department   int     `json:"dept"`          // index into Dicts.Departments
	Campus       int     `json:"campus"`        // index into Dicts.Campuses
	GakusokuNm   *string `json:"gaku,omitempty"` // only when != KogiNm
	TaishoGakka  *string `json:"gakka,omitempty"`
	TaishoNenji  *string `json:"nen,omitempty"`
	KamokuBunrui *string `json:"bunrui,omitempty"`
	KamokuBunya  *string `json:"bunya,omitempty"`
	SearchText   string  `json:"st"`
}

// Dictionaries holds the lookup tables for indexed fields.
type Dictionaries struct {
	Semesters   []string `json:"semesters"`
	Departments []string `json:"departments"`
	Campuses    []string `json:"campuses"`
	Kubun       []string `json:"kubun"`
	Kaikojiki   []string `json:"kaikojiki"`
}

// IndicesMap holds precomputed bitset indices for each filter dimension.
// Keys are dictionary index (as string), values are base64-encoded bitsets.
type IndicesMap struct {
	Semester   map[string]string `json:"semester"`
	Department map[string]string `json:"department"`
	Campus     map[string]string `json:"campus"`
}

// ProcessedDataV2 is the optimized top-level output schema (v2).
type ProcessedDataV2 struct {
	Version     int          `json:"version"`
	GeneratedAt time.Time    `json:"generatedAt"`
	TotalRaw    int          `json:"totalRaw"`
	Dicts       Dictionaries `json:"dicts"`
	Indices     IndicesMap   `json:"indices"`
	Courses     []CourseV2   `json:"courses"`
}
