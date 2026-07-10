//! Search-string normalization, shared by the haystack and the query.

/// Fold one character to its canonical search form — a strict 1:1 (char → char)
/// map, so folding preserves character positions. This is what lets a match span
/// found in the folded haystack point straight back into the original display
/// text (see the search index): a variable-length `to_lowercase` would desync
/// those offsets.
///
/// Folds the ideographic space (U+3000) to ASCII space, full-width ASCII
/// (U+FF01..=U+FF5E) to its half-width form, and ASCII upper- to lower-case, so a
/// query typed either width matches the haystack. Other characters (Japanese,
/// accented Latin) pass through unchanged.
#[must_use]
pub fn fold_char(c: char) -> char {
    match c {
        '\u{3000}' => ' ',
        '\u{FF01}'..='\u{FF5E}' => {
            // Shift the full-width block down to ASCII, then lower-case it.
            char::from_u32(c as u32 - 0xFEE0)
                .unwrap_or(c)
                .to_ascii_lowercase()
        }
        _ => c.to_ascii_lowercase(),
    }
}

/// Normalize a string so a query matches the precomputed search haystack (`st`),
/// by [`fold_char`]-ing every character. The haystack (`st`) is normalized the
/// same way at data-generation time; only the query is normalized here, at search
/// time — so both share one definition.
#[must_use]
pub fn normalize(input: &str) -> String {
    input.chars().map(fold_char).collect()
}

/// Build a course's search haystack and [`normalize`] it. The parts (name,
/// optional subtitle, instructor, code, department) are joined with ASCII spaces,
/// followed by any non-empty `extra` taxonomy parts (e.g. 分野・分類). Inputs are
/// expected already trimmed.
#[must_use]
pub fn search_text(
    name: &str,
    subtitle: Option<&str>,
    instructor: &str,
    code: &str,
    department: &str,
    extra: &[&str],
) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(5 + extra.len());
    parts.push(name);
    if let Some(sub) = subtitle {
        parts.push(sub);
    }
    parts.extend([instructor, code, department]);
    parts.extend(extra.iter().copied().filter(|s| !s.is_empty()));
    normalize(&parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::{fold_char, normalize, search_text};

    #[test]
    fn converts_full_width_spaces_to_half_width() {
        assert_eq!(normalize("山田\u{3000}太郎"), "山田 太郎");
    }

    #[test]
    fn lowercases_ascii_characters() {
        assert_eq!(normalize("English"), "english");
    }

    #[test]
    fn folds_full_width_ascii_to_half_width_lowercase() {
        // Full-width letters, digits, and punctuation all collapse to the ASCII
        // form a query would be typed in.
        assert_eq!(normalize("ＡＢＣ１２３"), "abc123");
        assert_eq!(normalize("ｅｎｇｌｉｓｈ"), "english");
    }

    #[test]
    fn handles_empty_string() {
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn leaves_japanese_untouched() {
        assert_eq!(normalize("微分積分学"), "微分積分学");
    }

    #[test]
    fn search_text_joins_and_normalizes_full_width_space() {
        assert_eq!(
            search_text(
                "微分積分学",
                None,
                "山田\u{3000}太郎",
                "001",
                "理工学部",
                &[]
            ),
            "微分積分学 山田 太郎 001 理工学部"
        );
    }

    #[test]
    fn search_text_includes_subtitle_and_lowercases() {
        assert_eq!(
            search_text(
                "English",
                Some("Communication"),
                "Smith",
                "004",
                "理工学部",
                &[]
            ),
            "english communication smith 004 理工学部"
        );
    }

    #[test]
    fn search_text_appends_nonempty_extra_taxonomy() {
        // 分野・分類 become part of the haystack; empty parts are dropped so no
        // stray double spaces creep in.
        assert_eq!(
            search_text(
                "線形代数",
                None,
                "田中",
                "002",
                "理工学部",
                &["数学", "", "専門"]
            ),
            "線形代数 田中 002 理工学部 数学 専門"
        );
    }

    use proptest::prelude::*;

    proptest! {
        /// Folding preserves character count (it is strictly 1 char → 1 char) —
        /// the invariant that lets a match span in the folded haystack map back to
        /// the same character range in the original display text.
        #[test]
        fn fold_preserves_char_count(s in ".*") {
            prop_assert_eq!(normalize(&s).chars().count(), s.chars().count());
        }

        /// `fold_char` is itself idempotent, character by character.
        #[test]
        fn fold_char_is_idempotent(c in proptest::char::any()) {
            prop_assert_eq!(fold_char(fold_char(c)), fold_char(c));
        }

        /// Normalizing an already-normalized string is a no-op (the query and the
        /// haystack must land on the same canonical form).
        #[test]
        fn normalize_is_idempotent(s in ".*") {
            let once = normalize(&s);
            prop_assert_eq!(normalize(&once), once);
        }

        /// The canonical form contains no full-width space and no ASCII uppercase —
        /// the two things that would otherwise make a query miss the haystack.
        #[test]
        fn normalized_output_has_no_fullwidth_space_or_ascii_upper(s in ".*") {
            let full_width_space = '\u{3000}';
            let out = normalize(&s);
            prop_assert!(!out.contains(full_width_space));
            prop_assert!(!out.bytes().any(|b| b.is_ascii_uppercase()));
        }

        /// `search_text` is exactly `normalize` of the space-joined parts (core
        /// five + non-empty extra taxonomy) — the contract the search relies on
        /// (haystack and query share `normalize`).
        #[test]
        fn search_text_is_normalize_of_the_join(
            name in "[\\p{Han}a-zA-Z0-9]{0,10}",
            instr in "[\\p{Han}a-zA-Z0-9]{0,10}",
            code in "[a-zA-Z0-9]{0,8}",
            dept in "[\\p{Han}a-zA-Z0-9]{0,10}",
            extra in proptest::collection::vec("[\\p{Han}a-zA-Z0-9]{0,8}", 0..4),
        ) {
            let extra_refs: Vec<&str> = extra.iter().map(String::as_str).collect();
            let st = search_text(&name, None, &instr, &code, &dept, &extra_refs);

            let mut parts = vec![name.as_str(), instr.as_str(), code.as_str(), dept.as_str()];
            parts.extend(extra_refs.iter().copied().filter(|s| !s.is_empty()));
            prop_assert_eq!(&st, &normalize(&parts.join(" ")));

            // A normalized query drawn from any part is found in the haystack.
            prop_assert!(st.contains(&normalize(&instr)));
            for e in extra_refs.iter().filter(|s| !s.is_empty()) {
                prop_assert!(st.contains(&normalize(e)), "non-empty extra must appear in st");
            }
        }
    }
}
