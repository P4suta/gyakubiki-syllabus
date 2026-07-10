//! Full-text course search: an index over each course's searchable fields that
//! answers a query with **ranked hits and per-field match spans**.
//!
//! The index is built from the *original* display strings, and folding is
//! position-preserving (see [`crate::text::fold_char`]) — so a match found in
//! the folded text carries back to the exact character range in the text the UI
//! renders, and highlighting needs no re-derivation. Spans are reported in
//! UTF-16 code units (what JS string slicing uses).
//!
//! This module is intentionally decoupled from [`crate::Engine`]: it takes the
//! searchable text explicitly ([`DocFields`]), so the data-generation step can
//! feed fields — such as the syllabus keywords — that never live on the wire
//! `Course` on their own.

use crate::index::CourseIndex;
use crate::text::fold_char;

/// A searchable field of a course. The discriminant is stable (it will be
/// persisted in the on-disk index) and orders fields by descending display
/// priority, so spans come out name-first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Field {
    /// Course name — highest signal, and the only field highlighted on the card.
    Name = 0,
    Subtitle = 1,
    Instructor = 2,
    Code = 3,
    /// Department + taxonomy + syllabus keywords: searchable for recall, never
    /// highlighted.
    Keywords = 4,
}

impl Field {
    /// Every field, in stable (discriminant) order.
    pub const ALL: [Field; 5] = [
        Field::Name,
        Field::Subtitle,
        Field::Instructor,
        Field::Code,
        Field::Keywords,
    ];

    /// Relevance weight: a name hit outranks an instructor hit, which outranks a
    /// code or keyword hit.
    #[must_use]
    fn weight(self) -> f32 {
        match self {
            Field::Name => 3.0,
            Field::Instructor => 2.0,
            Field::Subtitle => 1.5,
            Field::Code => 1.2,
            Field::Keywords => 1.0,
        }
    }
}

/// A matched character range within one field, as **UTF-16 code-unit** offsets
/// into the *original* display text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub field: Field,
    pub start: u32,
    pub len: u32,
}

/// One course that matched a query: its index, relevance score, and match spans
/// (name-first field order, then left to right within a field).
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub course: CourseIndex,
    pub score: f32,
    pub spans: Vec<Span>,
}

impl SearchHit {
    /// A candidate carried through with no ranking — score 0, no spans. Used for
    /// the browse view (empty query) and the pre-index fallback.
    #[must_use]
    pub fn unranked(course: CourseIndex) -> Self {
        Self {
            course,
            score: 0.0,
            spans: Vec::new(),
        }
    }
}

/// The searchable text of one course, borrowed at build time. `keywords` is the
/// pre-joined department + taxonomy + syllabus-keyword text.
#[derive(Debug, Default, Clone, Copy)]
pub struct DocFields<'a> {
    pub name: &'a str,
    pub subtitle: Option<&'a str>,
    pub instructor: &'a str,
    pub code: &'a str,
    pub keywords: &'a str,
}

/// One field's folded text plus, for every character, the UTF-16 offset of that
/// character in the *original* string. `char_utf16` has `char_count + 1` entries
/// — the final one is the field's total UTF-16 length — so a match spanning
/// chars `a..b` maps to `char_utf16[a]..char_utf16[b]`.
#[derive(Debug)]
struct FieldText {
    folded: String,
    char_utf16: Vec<u32>,
}

impl FieldText {
    fn build(original: &str) -> Self {
        Self::from_folded(original.chars().map(fold_char).collect())
    }

    /// Build from an already-folded string. Every fold rule preserves a
    /// character's UTF-16 length (all are BMP↔BMP, one code unit), so the offset
    /// of char `i` in the folded text equals its offset in the original — which
    /// is why the on-disk index need only store the folded text, and a span
    /// computed here indexes straight into the original display string.
    fn from_folded(folded: String) -> Self {
        let mut char_utf16 = Vec::with_capacity(folded.len() + 1);
        let mut utf16 = 0u32;
        for c in folded.chars() {
            char_utf16.push(utf16);
            utf16 += c.len_utf16() as u32;
        }
        char_utf16.push(utf16);
        Self { folded, char_utf16 }
    }

    /// Push a [`Span`] for every non-overlapping occurrence of the folded query,
    /// returning the number of matches.
    fn find(
        &self,
        field: Field,
        folded_query: &str,
        query_chars: usize,
        out: &mut Vec<Span>,
    ) -> u32 {
        if folded_query.is_empty() {
            return 0;
        }
        let mut count = 0;
        for (byte, matched) in self.folded.match_indices(folded_query) {
            // Folding is 1:1, so the folded char index equals the original char
            // index — the key into `char_utf16`.
            let char_start = self.folded[..byte].chars().count();
            let start = self.char_utf16[char_start];
            let end = self.char_utf16[char_start + query_chars];
            debug_assert_eq!(matched.chars().count(), query_chars);
            out.push(Span {
                field,
                start,
                len: end - start,
            });
            count += 1;
        }
        count
    }
}

/// A searchable index over every course, in ascending course-index order.
#[derive(Debug)]
pub struct SearchIndex {
    docs: Vec<[FieldText; 5]>,
}

impl SearchIndex {
    /// Build the index from each course's fields, in ascending course-index
    /// order (element `i` is course `i`).
    #[must_use]
    pub fn build<'a>(docs: impl IntoIterator<Item = DocFields<'a>>) -> Self {
        let docs = docs
            .into_iter()
            .map(|d| {
                [
                    FieldText::build(d.name),
                    FieldText::build(d.subtitle.unwrap_or("")),
                    FieldText::build(d.instructor),
                    FieldText::build(d.code),
                    FieldText::build(d.keywords),
                ]
            })
            .collect();
        Self { docs }
    }

    /// Number of indexed courses.
    #[must_use]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Whether the index holds no courses.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Search `query` over `candidates` (the dimension-filtered course set),
    /// returning the matches ranked by score descending, ties broken by
    /// ascending course index. An empty query yields no hits — the caller treats
    /// "no query" as "every candidate", where ranking does not apply.
    #[must_use]
    pub fn search(
        &self,
        query: &str,
        candidates: impl IntoIterator<Item = CourseIndex>,
    ) -> Vec<SearchHit> {
        let folded_query: String = query.chars().map(fold_char).collect();
        let query_chars = folded_query.chars().count();
        let mut hits = Vec::new();
        if folded_query.is_empty() {
            return hits;
        }
        for course in candidates {
            let Some(doc) = self.docs.get(course.get()) else {
                continue;
            };
            let mut spans = Vec::new();
            let mut score = 0.0f32;
            for field in Field::ALL {
                let count = doc[field as usize].find(field, &folded_query, query_chars, &mut spans);
                if count > 0 {
                    score += field.weight() * count as f32;
                }
            }
            if !spans.is_empty() {
                hits.push(SearchHit {
                    course,
                    score,
                    spans,
                });
            }
        }
        // total_cmp gives a stable order on the (non-NaN) scores; the index
        // tie-break keeps results deterministic across builds.
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.course.get().cmp(&b.course.get()))
        });
        hits
    }
}

// === Binary format ===
//
// The index is serialized once at data-generation time and shipped as its own
// `search.idx`, so it stays off the payload that gates first paint and is loaded
// lazily in the worker. Layout (all integers little-endian):
//
//   magic "SYX1" | version:u16 | n_docs:u32
//   per doc, per field (5, in `Field` order): folded_len:u32, folded:UTF-8 bytes
//
// Only the folded text is stored — the UTF-16 offset table is recomputed on load
// (see `FieldText::from_folded`), since folding preserves each char's UTF-16
// length. Decoding parses once into the owned structure the query uses.

const MAGIC: &[u8; 4] = b"SYX1";
const FORMAT_VERSION: u16 = 1;

/// Errors from decoding a `search.idx` blob.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    /// The blob does not start with the `search.idx` magic.
    #[error("not a search index (bad magic)")]
    BadMagic,
    /// The format version is newer/older than this build understands.
    #[error("unsupported search-index version {0}")]
    UnsupportedVersion(u16),
    /// The blob ended mid-record.
    #[error("search index is truncated")]
    Truncated,
    /// A field's bytes were not valid UTF-8.
    #[error("search index holds invalid UTF-8")]
    BadUtf8,
}

/// A bounds-checked little-endian cursor over the index blob.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], IndexError> {
        let end = self.pos.checked_add(n).ok_or(IndexError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(IndexError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u16(&mut self) -> Result<u16, IndexError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, IndexError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

impl SearchIndex {
    /// Serialize the index to its compact binary form (`search.idx`).
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&(self.docs.len() as u32).to_le_bytes());
        for doc in &self.docs {
            for field in doc {
                let bytes = field.folded.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
        }
        out
    }

    /// Parse an index from its binary form.
    ///
    /// # Errors
    /// Returns an [`IndexError`] if the blob is not a `search.idx`, is a version
    /// this build does not understand, is truncated, or holds invalid UTF-8.
    pub fn decode(bytes: &[u8]) -> Result<Self, IndexError> {
        let mut reader = Reader::new(bytes);
        if reader.take(4)? != MAGIC {
            return Err(IndexError::BadMagic);
        }
        let version = reader.u16()?;
        if version != FORMAT_VERSION {
            return Err(IndexError::UnsupportedVersion(version));
        }
        let n_docs = reader.u32()? as usize;
        let mut docs = Vec::with_capacity(n_docs);
        for _ in 0..n_docs {
            let mut fields: Vec<FieldText> = Vec::with_capacity(5);
            for _ in 0..5 {
                let len = reader.u32()? as usize;
                let folded = std::str::from_utf8(reader.take(len)?)
                    .map_err(|_| IndexError::BadUtf8)?
                    .to_owned();
                fields.push(FieldText::from_folded(folded));
            }
            // Exactly five fields were pushed, so the conversion cannot fail.
            let doc: [FieldText; 5] = fields.try_into().map_err(|_| IndexError::Truncated)?;
            docs.push(doc);
        }
        Ok(Self { docs })
    }
}

#[cfg(test)]
mod tests {
    use super::{DocFields, Field, IndexError, SearchIndex, Span};
    use crate::index::CourseIndex;

    fn doc(name: &str, instructor: &str, code: &str) -> DocFields<'static> {
        // Leak is fine in tests; keeps the fixtures terse.
        DocFields {
            name: Box::leak(name.to_owned().into_boxed_str()),
            subtitle: None,
            instructor: Box::leak(instructor.to_owned().into_boxed_str()),
            code: Box::leak(code.to_owned().into_boxed_str()),
            keywords: "",
        }
    }

    fn all(n: usize) -> Vec<CourseIndex> {
        (0..n).map(CourseIndex::new).collect()
    }

    #[test]
    fn finds_a_name_match_and_reports_its_span() {
        let idx = SearchIndex::build([doc("微分積分学", "山田 太郎", "001")]);
        let hits = idx.search("積分", all(1));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].course, CourseIndex::new(0));
        // 微(0) 分(1) 積(2) 分(3) 学(4) — all BMP, 1 UTF-16 unit each.
        assert_eq!(
            hits[0].spans,
            [Span {
                field: Field::Name,
                start: 2,
                len: 2
            }]
        );
    }

    #[test]
    fn maps_utf16_offsets_across_ascii_and_cjk() {
        // "AI と 数学" — A,I,space,と,space,数,学. Query 数学 starts at char 5.
        let idx = SearchIndex::build([doc("AI と 数学", "", "")]);
        let hits = idx.search("数学", all(1));
        assert_eq!(
            hits[0].spans,
            [Span {
                field: Field::Name,
                start: 5,
                len: 2
            }]
        );
    }

    #[test]
    fn ranks_name_hits_above_instructor_hits() {
        // Both courses contain 田; course 0 in the name, course 1 in the
        // instructor — the name hit must rank first.
        let idx =
            SearchIndex::build([doc("田中の講義", "佐藤", "001"), doc("英語", "山田", "002")]);
        let hits = idx.search("田", all(2));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].course, CourseIndex::new(0));
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn folds_full_width_query_to_match_half_width_code() {
        let idx = SearchIndex::build([doc("経済学", "", "AB12")]);
        let hits = idx.search("ＡＢ１２", all(1)); // full-width query
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].spans[0].field, Field::Code);
    }

    #[test]
    fn is_case_insensitive() {
        let idx = SearchIndex::build([doc("English Communication", "Smith", "004")]);
        assert_eq!(idx.search("english", all(1)).len(), 1);
        assert_eq!(idx.search("SMITH", all(1)).len(), 1);
    }

    #[test]
    fn counts_repeated_occurrences_in_the_score() {
        let one = SearchIndex::build([doc("学", "", "")]);
        let two = SearchIndex::build([doc("学学", "", "")]);
        let s1 = one.search("学", all(1))[0].score;
        let s2 = two.search("学", all(1))[0].score;
        assert!(s2 > s1, "two occurrences should outscore one: {s2} vs {s1}");
        assert_eq!(two.search("学", all(1))[0].spans.len(), 2);
    }

    #[test]
    fn empty_query_yields_no_hits() {
        let idx = SearchIndex::build([doc("微分積分学", "", "")]);
        assert!(idx.search("", all(1)).is_empty());
    }

    #[test]
    fn only_searches_the_given_candidates() {
        let idx = SearchIndex::build([doc("微分積分学", "", "001"), doc("微分方程式", "", "002")]);
        // Restrict to course 1 only.
        let hits = idx.search("微分", [CourseIndex::new(1)]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].course, CourseIndex::new(1));
    }

    #[test]
    fn no_match_yields_no_hit() {
        let idx = SearchIndex::build([doc("微分積分学", "山田", "001")]);
        assert!(idx.search("物理", all(1)).is_empty());
    }

    #[test]
    fn binary_round_trips_and_preserves_search() {
        let idx = SearchIndex::build([
            doc("微分積分学", "山田 太郎", "001"),
            doc("English Communication", "Smith", "E12"),
        ]);
        let bytes = idx.encode();
        let back = SearchIndex::decode(&bytes).expect("decodes");
        assert_eq!(back.len(), idx.len());
        // Same query, same ranked hits (course + score + spans).
        for q in ["積分", "english", "E12", "山田"] {
            assert_eq!(idx.search(q, all(2)), back.search(q, all(2)), "query {q:?}");
        }
    }

    #[test]
    fn decode_rejects_bad_magic_and_version() {
        assert_eq!(
            SearchIndex::decode(b"nope").unwrap_err(),
            IndexError::BadMagic
        );
        let mut bytes = SearchIndex::build([doc("学", "", "")]).encode();
        bytes[4] = 9; // bump the version's low byte
        assert!(matches!(
            SearchIndex::decode(&bytes).unwrap_err(),
            IndexError::UnsupportedVersion(_)
        ));
    }

    #[test]
    fn decode_rejects_truncation() {
        let bytes = SearchIndex::build([doc("微分積分学", "山田", "001")]).encode();
        assert_eq!(
            SearchIndex::decode(&bytes[..bytes.len() - 3]).unwrap_err(),
            IndexError::Truncated
        );
    }

    #[test]
    fn empty_index_round_trips() {
        let idx = SearchIndex::build([] as [DocFields; 0]);
        let back = SearchIndex::decode(&idx.encode()).expect("decodes");
        assert!(back.is_empty());
    }

    use proptest::prelude::*;

    proptest! {
        /// Encoding then decoding yields an index that answers identically.
        #[test]
        fn binary_round_trip_is_query_stable(
            names in proptest::collection::vec("[\\p{Han}a-zA-Z0-9 ]{0,12}", 0..6),
            query in "[\\p{Han}a-z0-9]{1,5}",
        ) {
            let docs: Vec<DocFields> = names.iter().map(|n| DocFields { name: n, ..Default::default() }).collect();
            let idx = SearchIndex::build(docs);
            let back = SearchIndex::decode(&idx.encode()).expect("decodes");
            let candidates = all(names.len());
            prop_assert_eq!(idx.search(&query, candidates.clone()), back.search(&query, candidates));
        }

        /// Every reported span lies within its field's UTF-16 length, and slicing
        /// the original name at the span recovers exactly the (folded) query — the
        /// contract the highlighter relies on.
        #[test]
        fn spans_are_in_bounds_and_recover_the_query(
            prefix in "[\\p{Han}a-zA-Z0-9]{0,6}",
            needle in "[\\p{Han}a-z0-9]{1,6}",
            suffix in "[\\p{Han}a-zA-Z0-9]{0,6}",
        ) {
            let name = format!("{prefix}{needle}{suffix}");
            let idx = SearchIndex::build([DocFields { name: &name, ..Default::default() }]);
            let hits = idx.search(&needle, [CourseIndex::new(0)]);
            prop_assert!(!hits.is_empty(), "needle {needle:?} must be found in {name:?}");

            let name_utf16: Vec<u16> = name.encode_utf16().collect();
            let folded_needle: String = crate::normalize(&needle);
            for span in &hits[0].spans {
                let end = span.start + span.len;
                prop_assert!(end as usize <= name_utf16.len());
                let slice = &name_utf16[span.start as usize..end as usize];
                let recovered = String::from_utf16(slice).unwrap();
                prop_assert_eq!(crate::normalize(&recovered), folded_needle.clone());
            }
        }
    }
}
