# Fuzz targets

Coverage-guided fuzzing for the two parsers that face messy real-world input:
`parse_jikanwari` (timetable strings) and `parse_sansho_html` (syllabus HTML).
Both are also covered by `proptest` never-panic properties in the main test
suite; this crate is for deeper, continuous exploration.

Requires a nightly toolchain and [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz):

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_parse_jikanwari -- -max_total_time=60
cargo +nightly fuzz run fuzz_parse_sansho_html -- -max_total_time=60
```

Seed the HTML corpus for faster convergence:

```sh
mkdir -p corpus/fuzz_parse_sansho_html
cp ../tests/fixtures/sansho_sample.html corpus/fuzz_parse_sansho_html/
```

This crate is intentionally outside the main Cargo workspace (it has its own
`[workspace]`), so `cargo build`/`cargo test` at the repo root ignore it. CI runs
it non-blocking on a schedule (`.github/workflows/fuzz.yml`).
