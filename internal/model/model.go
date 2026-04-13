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

// ProcessedData is the top-level output schema.
type ProcessedData struct {
	Version     int       `json:"version"`
	GeneratedAt time.Time `json:"generatedAt"`
	TotalRaw    int       `json:"totalRaw"`
	Courses     []Course  `json:"courses"`
	Semesters   []string  `json:"semesters"`
	Departments []string  `json:"departments"`
}
