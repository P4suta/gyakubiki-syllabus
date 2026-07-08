//! Search-string normalization, shared by the haystack and the query.

/// Normalize a string so a query matches the precomputed search haystack (`st`).
///
/// Converts the full-width space (U+3000) to an ASCII space, then lowercases.
/// The haystack (`st`) is normalized the same way at data-generation time; only
/// the query is normalized here, at search time — so both share one definition.
#[must_use]
pub fn normalize(input: &str) -> String {
    input.replace('\u{3000}', " ").to_lowercase()
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
    use super::{normalize, search_text};

    #[test]
    fn converts_full_width_spaces_to_half_width() {
        assert_eq!(normalize("山田\u{3000}太郎"), "山田 太郎");
    }

    #[test]
    fn lowercases_ascii_characters() {
        assert_eq!(normalize("English"), "english");
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

        /// `search_text` is exactly `normalize` of the space-joined parts — the
        /// contract the search relies on (haystack and query share `normalize`).
        #[test]
        fn search_text_is_normalize_of_the_join(
            name in "[\\p{Han}a-zA-Z0-9]{0,10}",
            instr in "[\\p{Han}a-zA-Z0-9]{0,10}",
            code in "[a-zA-Z0-9]{0,8}",
            dept in "[\\p{Han}a-zA-Z0-9]{0,10}",
        ) {
            let st = search_text(&name, None, &instr, &code, &dept, &[]);
            let joined = normalize(&[name.as_str(), instr.as_str(), code.as_str(), dept.as_str()].join(" "));
            prop_assert_eq!(&st, &joined);
            // A normalized query drawn from one part is found in the haystack.
            prop_assert!(st.contains(&normalize(&instr)));
        }
    }
}
