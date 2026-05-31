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

#[cfg(test)]
mod tests {
    use super::normalize;

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
}
