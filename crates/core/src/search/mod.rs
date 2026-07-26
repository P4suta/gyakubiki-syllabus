//! Position-aware Japanese full-text search.
//!
//! SYX4 is a hybrid n-gram inverted index. It stores every normalized field in
//! display order (the compact source of exact UTF-16 positions) plus a sparse
//! unigram → document posting table. A query chooses its rarest character,
//! visits only those documents, and verifies the complete substring in each
//! field. Sparse normalization runs map NFKC matches back to original UTF-16
//! clusters. Keeping the positional corpus in text order makes Brotli effective,
//! while the inverted table avoids scanning every syllabus.

use std::cell::RefCell;

use crate::index::CourseIndex;
#[cfg(feature = "unicode-search")]
use crate::text::normalize_with_mapping;
use crate::text::{NormalizationRun, normalize};

/// Searchable fields in stable wire order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Field {
    Name = 0,
    Subtitle = 1,
    Instructor = 2,
    Code = 3,
    Department = 4,
    Summary = 5,
    Aims = 6,
    Goals = 7,
    Plan = 8,
    Textbooks = 9,
    Prerequisite = 10,
    Preparation = 11,
    OfficeHour = 12,
    Keywords = 13,
    Teachers = 14,
    Numbering = 15,
    Sdgs = 16,
    Evaluation = 17,
    Delivery = 18,
    Extra = 19,
}

impl Field {
    pub const ALL: [Self; 20] = [
        Self::Name,
        Self::Subtitle,
        Self::Instructor,
        Self::Code,
        Self::Department,
        Self::Summary,
        Self::Aims,
        Self::Goals,
        Self::Plan,
        Self::Textbooks,
        Self::Prerequisite,
        Self::Preparation,
        Self::OfficeHour,
        Self::Keywords,
        Self::Teachers,
        Self::Numbering,
        Self::Sdgs,
        Self::Evaluation,
        Self::Delivery,
        Self::Extra,
    ];

    const fn weight(self) -> f32 {
        match self {
            Self::Name => 5.0,
            Self::Code => 4.5,
            Self::Instructor => 4.0,
            Self::Teachers => 3.5,
            Self::Subtitle => 3.0,
            Self::Keywords => 2.8,
            Self::Department => 2.5,
            Self::Aims | Self::Goals => 2.2,
            Self::Summary => 2.0,
            Self::Prerequisite | Self::Numbering => 1.5,
            Self::Plan => 1.3,
            Self::Extra => 1.1,
            Self::Textbooks
            | Self::Preparation
            | Self::OfficeHour
            | Self::Sdgs
            | Self::Evaluation => 1.0,
            Self::Delivery => 0.8,
        }
    }
}

const FIELD_COUNT: usize = Field::ALL.len();
const MAX_SPANS_PER_HIT: usize = 64;

/// A matched range in UTF-16 code units within one field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub field: Field,
    pub start: u32,
    pub len: u32,
}

/// One ranked course match.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub course: CourseIndex,
    pub score: f32,
    pub spans: Vec<Span>,
}

impl SearchHit {
    #[must_use]
    pub fn unranked(course: CourseIndex) -> Self {
        Self {
            course,
            score: 0.0,
            spans: Vec::new(),
        }
    }
}

/// Borrowed fields supplied by the dataset producer.
#[cfg(any(feature = "producer", test))]
#[derive(Debug, Default, Clone, Copy)]
pub struct DocFields<'a> {
    pub name: &'a str,
    pub subtitle: Option<&'a str>,
    pub instructor: &'a str,
    pub code: &'a str,
    pub department: &'a str,
    pub summary: &'a str,
    pub aims: &'a str,
    pub goals: &'a str,
    pub plan: &'a str,
    pub textbooks: &'a str,
    pub prerequisite: &'a str,
    pub preparation: &'a str,
    pub office_hour: &'a str,
    pub keywords: &'a str,
    pub teachers: &'a str,
    pub numbering: &'a str,
    pub sdgs: &'a str,
    pub evaluation: &'a str,
    pub delivery: &'a str,
    pub extra: &'a str,
}

#[cfg(any(feature = "producer", test))]
impl<'a> DocFields<'a> {
    fn values(self) -> [&'a str; FIELD_COUNT] {
        [
            self.name,
            self.subtitle.unwrap_or(""),
            self.instructor,
            self.code,
            self.department,
            self.summary,
            self.aims,
            self.goals,
            self.plan,
            self.textbooks,
            self.prerequisite,
            self.preparation,
            self.office_hour,
            self.keywords,
            self.teachers,
            self.numbering,
            self.sdgs,
            self.evaluation,
            self.delivery,
            self.extra,
        ]
    }
}

#[derive(Debug)]
struct FieldText {
    folded: String,
    mappings: Vec<NormalizationRun>,
}

impl FieldText {
    #[cfg(any(feature = "producer", test))]
    fn build(original: &str) -> Self {
        let (folded, mappings) = normalize_with_mapping(original);
        Self { folded, mappings }
    }

    fn find(&self, field: Field, query: &str, spans: &mut Vec<Span>) -> u32 {
        let Some(byte_offset) = self.folded.find(query) else {
            return 0;
        };
        if spans.len() < MAX_SPANS_PER_HIT {
            let normalized_start = self.folded[..byte_offset].encode_utf16().count() as u32;
            let normalized_end = normalized_start + query.encode_utf16().count() as u32;
            let (start, end) = self.original_range(normalized_start, normalized_end);
            spans.push(Span {
                field,
                start,
                len: end.saturating_sub(start),
            });
        }
        1
    }

    fn original_range(&self, start: u32, end: u32) -> (u32, u32) {
        (
            map_start_boundary(start, &self.mappings),
            map_end_boundary(end, &self.mappings),
        )
    }
}

fn map_start_boundary(offset: u32, mappings: &[NormalizationRun]) -> u32 {
    let mut delta = 0i64;
    for mapping in mappings {
        if offset < mapping.normalized_start {
            break;
        }
        if offset < mapping.normalized_end {
            return mapping.original_start;
        }
        delta = i64::from(mapping.original_end) - i64::from(mapping.normalized_end);
    }
    (i64::from(offset) + delta).max(0) as u32
}

fn map_end_boundary(offset: u32, mappings: &[NormalizationRun]) -> u32 {
    let mut delta = 0i64;
    for mapping in mappings {
        if offset <= mapping.normalized_start {
            break;
        }
        if offset <= mapping.normalized_end {
            return mapping.original_end;
        }
        delta = i64::from(mapping.original_end) - i64::from(mapping.normalized_end);
    }
    (i64::from(offset) + delta).max(0) as u32
}

/// Validated, dataset-bound positional n-gram index.
#[derive(Debug)]
pub struct SearchIndex {
    dataset_id: String,
    docs: Vec<[FieldText; FIELD_COUNT]>,
    postings: Vec<(char, Vec<CourseIndex>)>,
    single_character_cache: RefCell<Option<(char, Vec<SearchHit>)>>,
}

impl SearchIndex {
    #[must_use]
    #[cfg(any(feature = "producer", test))]
    pub fn build<'a>(docs: impl IntoIterator<Item = DocFields<'a>>) -> Self {
        Self::build_for_dataset("0".repeat(64), docs)
    }

    #[must_use]
    #[cfg(any(feature = "producer", test))]
    pub fn build_for_dataset<'a>(
        dataset_id: impl Into<String>,
        docs: impl IntoIterator<Item = DocFields<'a>>,
    ) -> Self {
        let docs: Vec<[FieldText; FIELD_COUNT]> = docs
            .into_iter()
            .map(|document| document.values().map(FieldText::build))
            .collect();
        let postings = build_postings(&docs);
        Self {
            dataset_id: dataset_id.into(),
            docs,
            postings,
            single_character_cache: RefCell::new(None),
        }
    }

    #[must_use]
    pub fn document_count(&self) -> usize {
        self.docs.len()
    }

    #[must_use]
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    /// Exact normalized substring search over dimension-filtered candidates.
    #[must_use]
    pub fn search(
        &self,
        query: &str,
        candidates: impl IntoIterator<Item = CourseIndex>,
    ) -> Vec<SearchHit> {
        let folded_query = normalize(query);
        let character_count = folded_query.chars().count();
        if character_count == 0 || character_count > 256 {
            return Vec::new();
        }
        let candidates: Vec<CourseIndex> = candidates.into_iter().collect();
        if character_count == 1 {
            let character = folded_query.chars().next().unwrap_or_default();
            let cached = self
                .single_character_cache
                .borrow()
                .as_ref()
                .filter(|(cached, _)| *cached == character)
                .map(|(_, hits)| hits.clone());
            let mut hits = if let Some(hits) = cached {
                hits
            } else {
                let hits = self.search_folded(
                    &folded_query,
                    (0..self.docs.len()).map(CourseIndex::new).collect(),
                );
                *self.single_character_cache.borrow_mut() = Some((character, hits.clone()));
                hits
            };
            if !is_all_candidates(&candidates, self.docs.len()) {
                let mut allowed = vec![false; self.docs.len()];
                for candidate in candidates {
                    if let Some(value) = allowed.get_mut(candidate.get()) {
                        *value = true;
                    }
                }
                hits.retain(|hit| allowed.get(hit.course.get()).copied().unwrap_or(false));
            }
            return hits;
        }
        self.search_folded(&folded_query, candidates)
    }

    fn search_folded(&self, folded_query: &str, candidates: Vec<CourseIndex>) -> Vec<SearchHit> {
        let Some(seed) = folded_query
            .chars()
            .filter_map(|character| {
                self.postings
                    .binary_search_by_key(&character, |(key, _)| *key)
                    .ok()
                    .map(|index| &self.postings[index].1)
            })
            .min_by_key(|posting| posting.len())
        else {
            return Vec::new();
        };

        let all_candidates = is_all_candidates(&candidates, self.docs.len());
        let allowed = if all_candidates {
            Vec::new()
        } else {
            let mut allowed = vec![false; self.docs.len()];
            for course in candidates {
                if let Some(value) = allowed.get_mut(course.get()) {
                    *value = true;
                }
            }
            allowed
        };

        let mut hits = Vec::new();
        for &course in seed {
            if !all_candidates && !allowed.get(course.get()).copied().unwrap_or(false) {
                continue;
            }
            let Some(document) = self.docs.get(course.get()) else {
                continue;
            };
            let mut spans = Vec::new();
            let mut score = 0.0f32;
            for field in Field::ALL {
                let count = document[field as usize].find(field, folded_query, &mut spans);
                score += count as f32 * field.weight();
            }
            if !spans.is_empty() {
                hits.push(SearchHit {
                    course,
                    score,
                    spans,
                });
            }
        }
        hits.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.course.get().cmp(&right.course.get()))
        });
        hits
    }

    /// Serialize the strictly validated SYX4 transport.
    #[must_use]
    #[cfg(any(feature = "producer", test))]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&(self.dataset_id.len() as u16).to_le_bytes());
        out.extend_from_slice(self.dataset_id.as_bytes());
        out.extend_from_slice(&(self.docs.len() as u32).to_le_bytes());
        out.extend_from_slice(&(FIELD_COUNT as u16).to_le_bytes());
        for document in &self.docs {
            for field in document {
                out.extend_from_slice(&(field.folded.len() as u32).to_le_bytes());
                out.extend_from_slice(field.folded.as_bytes());
                out.extend_from_slice(&(field.mappings.len() as u32).to_le_bytes());
                for mapping in &field.mappings {
                    out.extend_from_slice(&mapping.normalized_start.to_le_bytes());
                    out.extend_from_slice(&mapping.normalized_end.to_le_bytes());
                    out.extend_from_slice(&mapping.original_start.to_le_bytes());
                    out.extend_from_slice(&mapping.original_end.to_le_bytes());
                }
            }
        }
        out.extend_from_slice(&(self.postings.len() as u32).to_le_bytes());
        let total: u64 = self
            .postings
            .iter()
            .map(|(_, documents)| documents.len() as u64)
            .sum();
        out.extend_from_slice(&total.to_le_bytes());
        for (character, documents) in &self.postings {
            let mut payload = Vec::new();
            let mut previous = 0usize;
            for (position, document) in documents.iter().enumerate() {
                let value = document.get();
                let delta = if position == 0 {
                    value
                } else {
                    value - previous
                };
                put_varint(delta as u64, &mut payload);
                previous = value;
            }
            out.extend_from_slice(&(*character as u32).to_le_bytes());
            out.extend_from_slice(&(documents.len() as u32).to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&payload);
        }
        out
    }

    /// Decode and fully validate untrusted index bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, IndexError> {
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(IndexError::IndexTooLarge(bytes.len()));
        }
        let mut reader = Reader::new(bytes);
        if reader.take(4)? != MAGIC {
            return Err(IndexError::BadMagic);
        }
        let version = reader.u16()?;
        if version != FORMAT_VERSION {
            return Err(IndexError::UnsupportedVersion(version));
        }
        let dataset_len = reader.u16()? as usize;
        if dataset_len != DATASET_ID_BYTES {
            return Err(IndexError::BadDatasetId);
        }
        let dataset_id = std::str::from_utf8(reader.take(dataset_len)?)
            .map_err(|_| IndexError::BadDatasetId)?
            .to_owned();
        if !dataset_id
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
        {
            return Err(IndexError::BadDatasetId);
        }
        let document_count = reader.u32()? as usize;
        if document_count > MAX_DOCS {
            return Err(IndexError::TooManyDocuments(document_count));
        }
        let field_count = reader.u16()? as usize;
        if field_count != FIELD_COUNT {
            return Err(IndexError::BadFieldCount(field_count));
        }
        let mut docs = Vec::with_capacity(document_count);
        let mut total_text = 0usize;
        let mut total_mappings = 0usize;
        for _ in 0..document_count {
            let mut fields = Vec::with_capacity(FIELD_COUNT);
            for _ in 0..FIELD_COUNT {
                let len = reader.u32()? as usize;
                if len > MAX_FIELD_BYTES {
                    return Err(IndexError::FieldTooLarge(len));
                }
                total_text = total_text
                    .checked_add(len)
                    .ok_or(IndexError::IndexTooLarge(usize::MAX))?;
                if total_text > MAX_TEXT_BYTES {
                    return Err(IndexError::IndexTooLarge(total_text));
                }
                let folded = std::str::from_utf8(reader.take(len)?)
                    .map_err(|_| IndexError::BadUtf8)?
                    .to_owned();
                let mapping_count = reader.u32()? as usize;
                total_mappings = total_mappings
                    .checked_add(mapping_count)
                    .ok_or(IndexError::TooManyMappings(usize::MAX))?;
                if mapping_count > MAX_MAPPINGS_PER_FIELD || total_mappings > MAX_TOTAL_MAPPINGS {
                    return Err(IndexError::TooManyMappings(total_mappings));
                }
                let normalized_utf16 = folded.encode_utf16().count() as u32;
                let mut mappings = Vec::with_capacity(mapping_count);
                let mut previous_normalized_end = 0;
                let mut previous_original_end = 0;
                for _ in 0..mapping_count {
                    let mapping = NormalizationRun {
                        normalized_start: reader.u32()?,
                        normalized_end: reader.u32()?,
                        original_start: reader.u32()?,
                        original_end: reader.u32()?,
                    };
                    if mapping.normalized_start < previous_normalized_end
                        || mapping.normalized_end < mapping.normalized_start
                        || mapping.normalized_end > normalized_utf16
                        || mapping.original_start < previous_original_end
                        || mapping.original_end <= mapping.original_start
                        || mapping.original_end > MAX_ORIGINAL_FIELD_UTF16
                    {
                        return Err(IndexError::InvalidMapping);
                    }
                    previous_normalized_end = mapping.normalized_end;
                    previous_original_end = mapping.original_end;
                    mappings.push(mapping);
                }
                fields.push(FieldText { folded, mappings });
            }
            docs.push(
                fields
                    .try_into()
                    .map_err(|_| IndexError::BadFieldCount(FIELD_COUNT))?,
            );
        }

        let gram_count = reader.u32()? as usize;
        if gram_count > MAX_GRAMS {
            return Err(IndexError::TooManyGrams(gram_count));
        }
        let declared_total = reader.u64()?;
        if declared_total > MAX_TOTAL_POSTINGS {
            return Err(IndexError::TooManyPostings(declared_total));
        }
        let mut postings = Vec::with_capacity(gram_count);
        let mut previous_character = None;
        let mut actual_total = 0u64;
        for _ in 0..gram_count {
            let character = char::from_u32(reader.u32()?).ok_or(IndexError::InvalidScalar)?;
            if previous_character.is_some_and(|previous| character <= previous) {
                return Err(IndexError::GramOrder);
            }
            previous_character = Some(character);
            let count = reader.u32()? as usize;
            if count > document_count {
                return Err(IndexError::InvalidPosting);
            }
            let payload_len = reader.u32()? as usize;
            let payload = reader.take(payload_len)?;
            let documents = decode_postings(payload, count, document_count)?;
            actual_total = actual_total
                .checked_add(count as u64)
                .ok_or(IndexError::TooManyPostings(u64::MAX))?;
            postings.push((character, documents));
        }
        if actual_total != declared_total {
            return Err(IndexError::InvalidPosting);
        }
        if !reader.is_eof() {
            return Err(IndexError::TrailingBytes);
        }
        let rebuilt = build_postings(&docs);
        if rebuilt != postings {
            return Err(IndexError::PostingCorpusMismatch);
        }
        Ok(Self {
            dataset_id,
            docs,
            postings,
            single_character_cache: RefCell::new(None),
        })
    }

    /// Bind the decoded index to the manifest-selected dataset.
    #[cfg(any(feature = "producer", test))]
    pub fn validate_identity(
        &self,
        expected_dataset_id: &str,
        expected_documents: usize,
    ) -> Result<(), IndexError> {
        if self.dataset_id != expected_dataset_id {
            return Err(IndexError::DatasetMismatch);
        }
        if self.docs.len() != expected_documents {
            return Err(IndexError::DocumentCountMismatch {
                expected: expected_documents,
                actual: self.docs.len(),
            });
        }
        Ok(())
    }
}

fn is_all_candidates(candidates: &[CourseIndex], document_count: usize) -> bool {
    candidates.len() == document_count
        && candidates
            .iter()
            .enumerate()
            .all(|(expected, course)| course.get() == expected)
}

fn build_postings(docs: &[[FieldText; FIELD_COUNT]]) -> Vec<(char, Vec<CourseIndex>)> {
    let mut postings: Vec<(char, Vec<CourseIndex>)> = Vec::new();
    let mut seen = Vec::new();
    for (document, fields) in docs.iter().enumerate() {
        seen.clear();
        for field in fields {
            seen.extend(field.folded.chars());
        }
        seen.sort_unstable();
        seen.dedup();
        let course = CourseIndex::new(document);
        for &character in &seen {
            match postings.binary_search_by_key(&character, |(key, _)| *key) {
                Ok(index) => postings[index].1.push(course),
                Err(index) => postings.insert(index, (character, vec![course])),
            }
        }
    }
    postings
}

fn decode_postings(
    payload: &[u8],
    count: usize,
    document_count: usize,
) -> Result<Vec<CourseIndex>, IndexError> {
    let mut reader = VarReader::new(payload);
    let mut documents = Vec::with_capacity(count);
    let mut previous = 0usize;
    for position in 0..count {
        let delta = usize::try_from(reader.varint()?).map_err(|_| IndexError::InvalidPosting)?;
        if position > 0 && delta == 0 {
            return Err(IndexError::InvalidPosting);
        }
        let document = if position == 0 {
            delta
        } else {
            previous
                .checked_add(delta)
                .ok_or(IndexError::InvalidPosting)?
        };
        if document >= document_count {
            return Err(IndexError::InvalidPosting);
        }
        documents.push(CourseIndex::new(document));
        previous = document;
    }
    if !reader.is_eof() {
        return Err(IndexError::InvalidPosting);
    }
    Ok(documents)
}

#[cfg(any(feature = "producer", test))]
fn put_varint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

// === Strict binary reader ===

const MAGIC: &[u8; 4] = b"SYX4";
const FORMAT_VERSION: u16 = 4;
const MAX_DOCS: usize = 100_000;
const DATASET_ID_BYTES: usize = 64;
const MAX_FIELD_BYTES: usize = 16 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 96 * 1024 * 1024;
const MAX_MAPPINGS_PER_FIELD: usize = 1_000_000;
const MAX_TOTAL_MAPPINGS: usize = 4_000_000;
const MAX_ORIGINAL_FIELD_UTF16: u32 = 16 * 1024 * 1024;
const MAX_GRAMS: usize = 65_536;
const MAX_TOTAL_POSTINGS: u64 = 100_000_000;
const MAX_INDEX_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    #[error("not a search index (bad magic)")]
    BadMagic,
    #[error("unsupported search-index version {0}")]
    UnsupportedVersion(u16),
    #[error("search index is truncated")]
    Truncated,
    #[error("search index holds invalid UTF-8")]
    BadUtf8,
    #[error("search index declares too many documents ({0})")]
    TooManyDocuments(usize),
    #[error("search index field is too large ({0} bytes)")]
    FieldTooLarge(usize),
    #[error("search index dataset ID is invalid")]
    BadDatasetId,
    #[error("search index contains trailing bytes")]
    TrailingBytes,
    #[error("search index belongs to a different dataset")]
    DatasetMismatch,
    #[error("search index document count mismatch (expected {expected}, got {actual})")]
    DocumentCountMismatch { expected: usize, actual: usize },
    #[error("search index field count is invalid ({0})")]
    BadFieldCount(usize),
    #[error("search index declares too many Unicode position mappings ({0})")]
    TooManyMappings(usize),
    #[error("search index contains an invalid Unicode position mapping")]
    InvalidMapping,
    #[error("search index declares too many n-grams ({0})")]
    TooManyGrams(usize),
    #[error("search index declares too many postings ({0})")]
    TooManyPostings(u64),
    #[error("search index contains an invalid Unicode scalar")]
    InvalidScalar,
    #[error("search index n-grams are not strictly ordered")]
    GramOrder,
    #[error("search index contains an invalid posting")]
    InvalidPosting,
    #[error("search index contains an overflowing varint")]
    VarintOverflow,
    #[error("search index posting table does not match its positional corpus")]
    PostingCorpusMismatch,
    #[error("search index is too large ({0} bytes)")]
    IndexTooLarge(usize),
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], IndexError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(IndexError::Truncated)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(IndexError::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, IndexError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| IndexError::Truncated)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, IndexError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| IndexError::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, IndexError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| IndexError::Truncated)?,
        ))
    }

    fn is_eof(&self) -> bool {
        self.position == self.bytes.len()
    }
}

struct VarReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> VarReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn varint(&mut self) -> Result<u64, IndexError> {
        let mut value = 0u64;
        for shift in (0..=63).step_by(7) {
            let byte = *self.bytes.get(self.position).ok_or(IndexError::Truncated)?;
            self.position += 1;
            let bits = u64::from(byte & 0x7f);
            if shift == 63 && bits > 1 {
                return Err(IndexError::VarintOverflow);
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(IndexError::VarintOverflow)
    }

    fn is_eof(&self) -> bool {
        self.position == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &str, instructor: &str, code: &str) -> DocFields<'static> {
        DocFields {
            name: Box::leak(name.to_owned().into_boxed_str()),
            instructor: Box::leak(instructor.to_owned().into_boxed_str()),
            code: Box::leak(code.to_owned().into_boxed_str()),
            ..DocFields::default()
        }
    }

    fn all(count: usize) -> Vec<CourseIndex> {
        (0..count).map(CourseIndex::new).collect()
    }

    #[test]
    fn exact_japanese_substring_and_utf16_span() {
        let index = SearchIndex::build([doc("AI😀微分積分学", "", "001")]);
        let hits = index.search("微分", all(1));
        assert_eq!(
            hits[0].spans,
            [Span {
                field: Field::Name,
                start: 4,
                len: 2,
            }]
        );
        assert!(index.search("微積", all(1)).is_empty());
    }

    #[test]
    fn normalizes_width_and_ascii_case() {
        let index = SearchIndex::build([doc("English", "", "A123")]);
        assert_eq!(index.search("ＥＮＧ", all(1)).len(), 1);
        assert_eq!(
            index.search("ａ１２３", all(1))[0].spans[0].field,
            Field::Code
        );
    }

    #[test]
    fn nfkc_search_maps_matches_back_to_original_utf16_clusters_after_roundtrip() {
        let index = SearchIndex::build([doc("先ﾃﾞ㍑後", "", "001")]);
        let decoded = SearchIndex::decode(&index.encode()).unwrap();

        assert_eq!(
            decoded.search("デ", all(1))[0].spans,
            [Span {
                field: Field::Name,
                start: 1,
                len: 2,
            }]
        );
        assert_eq!(
            decoded.search("ット", all(1))[0].spans,
            [Span {
                field: Field::Name,
                start: 3,
                len: 1,
            }]
        );
    }

    #[test]
    fn decoder_rejects_invalid_unicode_position_mapping() {
        let mut encoded = SearchIndex::build([doc("ﾃﾞ", "", "001")]).encode();
        let field_length_offset = 4 + 2 + 2 + DATASET_ID_BYTES + 4 + 2;
        let field_length = u32::from_le_bytes(
            encoded[field_length_offset..field_length_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let mapping_record = field_length_offset + 4 + field_length + 4;
        encoded[mapping_record + 12..mapping_record + 16].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            SearchIndex::decode(&encoded).unwrap_err(),
            IndexError::InvalidMapping
        );
    }

    #[test]
    fn every_syllabus_field_is_independently_searchable() {
        let values = [
            "name",
            "subtitle",
            "instructor",
            "code",
            "department",
            "summary",
            "aims",
            "goals",
            "plan",
            "textbooks",
            "prerequisite",
            "preparation",
            "officehour",
            "keywords",
            "teachers",
            "numbering",
            "sdgs",
            "evaluation",
            "delivery",
            "extra",
        ];
        let fields = DocFields {
            name: values[0],
            subtitle: Some(values[1]),
            instructor: values[2],
            code: values[3],
            department: values[4],
            summary: values[5],
            aims: values[6],
            goals: values[7],
            plan: values[8],
            textbooks: values[9],
            prerequisite: values[10],
            preparation: values[11],
            office_hour: values[12],
            keywords: values[13],
            teachers: values[14],
            numbering: values[15],
            sdgs: values[16],
            evaluation: values[17],
            delivery: values[18],
            extra: values[19],
        };
        let index = SearchIndex::build([fields]);
        for (expected, query) in Field::ALL.into_iter().zip(values) {
            let hits = index.search(query, all(1));
            assert_eq!(hits.len(), 1, "{query}");
            assert_eq!(hits[0].spans[0].field, expected, "{query}");
        }
    }

    #[test]
    fn ranking_prefers_course_name_over_summary() {
        let index = SearchIndex::build([
            DocFields {
                name: "量子力学",
                ..DocFields::default()
            },
            DocFields {
                name: "物理",
                summary: "量子力学を学ぶ",
                ..DocFields::default()
            },
        ]);
        let hits = index.search("量子力学", all(2));
        assert_eq!(hits[0].course, CourseIndex::new(0));
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn candidate_filter_is_respected() {
        let index = SearchIndex::build([doc("数学", "", "001"), doc("数学", "", "002")]);
        let hits = index.search("数学", [CourseIndex::new(1)]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].course, CourseIndex::new(1));
    }

    #[test]
    fn roundtrip_preserves_identity_and_results() {
        let dataset = "a".repeat(64);
        let source = SearchIndex::build_for_dataset(
            &dataset,
            [
                doc("微分積分", "山田", "001"),
                doc("線形代数", "田中", "002"),
            ],
        );
        let decoded = SearchIndex::decode(&source.encode()).unwrap();
        assert_eq!(decoded.dataset_id(), dataset);
        assert_eq!(decoded.search("分", all(2)), source.search("分", all(2)));
    }

    #[test]
    fn rejects_wrong_generation_or_document_count() {
        let dataset_a = "a".repeat(64);
        let dataset_b = "b".repeat(64);
        let index = SearchIndex::build_for_dataset(&dataset_a, [doc("a", "", "1")]);
        assert_eq!(
            index.validate_identity(&dataset_b, 1).unwrap_err(),
            IndexError::DatasetMismatch
        );
        assert_eq!(
            index.validate_identity(&dataset_a, 2).unwrap_err(),
            IndexError::DocumentCountMismatch {
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn adversarial_headers_truncation_and_trailing_bytes_are_rejected() {
        assert_eq!(
            SearchIndex::decode(b"bad").unwrap_err(),
            IndexError::Truncated
        );
        let mut encoded = SearchIndex::build([doc("a", "", "1")]).encode();
        encoded.push(0);
        assert_eq!(
            SearchIndex::decode(&encoded).unwrap_err(),
            IndexError::TrailingBytes
        );
        let mut version = SearchIndex::build([] as [DocFields<'static>; 0]).encode();
        version[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            SearchIndex::decode(&version).unwrap_err(),
            IndexError::UnsupportedVersion(99)
        );
        version[4..6].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        let field_offset = 4 + 2 + 2 + DATASET_ID_BYTES + 4;
        version[field_offset..field_offset + 2].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(
            SearchIndex::decode(&version).unwrap_err(),
            IndexError::BadFieldCount(1)
        );

        let invalid_identity =
            SearchIndex::build_for_dataset("not-a-sha256", [doc("a", "", "1")]).encode();
        assert_eq!(
            SearchIndex::decode(&invalid_identity).unwrap_err(),
            IndexError::BadDatasetId
        );
    }

    #[test]
    fn posting_tampering_is_rejected_against_the_corpus() {
        let mut encoded = SearchIndex::build([doc("a", "", "1")]).encode();
        *encoded.last_mut().unwrap() = 1;
        assert!(matches!(
            SearchIndex::decode(&encoded),
            Err(IndexError::InvalidPosting | IndexError::PostingCorpusMismatch)
        ));
    }

    #[test]
    fn empty_and_oversized_queries_are_bounded() {
        let index = SearchIndex::build([doc("a", "", "1")]);
        assert!(index.search("", all(1)).is_empty());
        assert!(index.search(&"a".repeat(257), all(1)).is_empty());
    }
}
