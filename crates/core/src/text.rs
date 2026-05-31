//! Search-string normalization, shared by the haystack and the query.

/// Normalize a string so a query matches the precomputed search haystack (`st`).
///
/// Converts the full-width space (U+3000) to an ASCII space, then lowercases —
/// the same two steps, in the same order, as the Go haystack builder
/// `buildSearchTextV2` in `internal/transform/v2.go` (which bakes `st`).
///
/// The match is exact over the ASCII and Japanese text this dataset contains.
/// It is *not* guaranteed byte-for-byte across the full Unicode range: Go's
/// `strings.ToLower` uses simple case mapping, whereas Rust's `to_lowercase`
/// applies the full mapping (e.g. Greek final sigma `Σ`→`ς`, dotted `İ`), so a
/// query in those exotic scripts could fold differently than the haystack did.
///
/// The haystack is normalized once at data-generation time; only the query is
/// normalized here, at search time.
#[must_use]
pub fn normalize(input: &str) -> String {
    input.replace('\u{3000}', " ").to_lowercase()
}

/// Build a course's search haystack and [`normalize`] it — the port of Go's
/// `buildSearchTextV2`. The parts (name, optional subtitle, instructor, code,
/// department) are joined with ASCII spaces and run through [`normalize`], so the
/// producer's haystack and the consumer's query share one definition of
/// "normalized". Inputs are expected already trimmed, as in the Go pipeline.
#[must_use]
pub fn search_text(
    name: &str,
    subtitle: Option<&str>,
    instructor: &str,
    code: &str,
    department: &str,
) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(5);
    parts.push(name);
    if let Some(sub) = subtitle {
        parts.push(sub);
    }
    parts.extend([instructor, code, department]);
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
            search_text("微分積分学", None, "山田\u{3000}太郎", "001", "理工学部"),
            "微分積分学 山田 太郎 001 理工学部"
        );
    }

    #[test]
    fn search_text_includes_subtitle_and_lowercases() {
        assert_eq!(
            search_text("English", Some("Communication"), "Smith", "004", "理工学部"),
            "english communication smith 004 理工学部"
        );
    }
}
