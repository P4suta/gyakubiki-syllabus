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
        let mut folded = String::with_capacity(original.len());
        let mut char_utf16 = Vec::with_capacity(original.len() + 1);
        let mut utf16 = 0u32;
        for c in original.chars() {
            char_utf16.push(utf16);
            utf16 += c.len_utf16() as u32;
            folded.push(fold_char(c));
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

#[cfg(test)]
mod tests {
    use super::{DocFields, Field, SearchIndex, Span};
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

    use proptest::prelude::*;

    proptest! {
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
