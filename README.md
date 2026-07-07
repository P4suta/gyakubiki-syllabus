# 逆引きシラバス

高知大学の講義を「曜日 x 時限」の時間割グリッドで一覧できるツール。

**[https://p4suta.github.io/gyakubiki-syllabus/](https://p4suta.github.io/gyakubiki-syllabus/)**

---

## Features

| Feature | How |
|------|------|
| **Term switching** | Switch terms via the top tabs |
| **Department filter** | Narrow by offering department with the select box |
| **Search** | Search course name, instructor, and course code, plus syllabus body (summary, keywords, objectives) |
| **Details** | Click a course card for grade breakdown (pie chart), lesson plan, objectives, textbook, office hours, and more |
| **Official link** | Open the official KULAS syllabus page directly from the details view |

---

## Disclaimer

本ツールは個人が作成した非公式のものであり、高知大学とは一切の関係がなく、同大学による承認又は推奨を受けたものではありません。表示されるデータは、[同大学が一般に公開しているシラバス情報](https://www.kochi-u.ac.jp/education-support/courses/syllabus/)のみに基づいており（2026年4月13日時点）、非公開情報は一切使用していません。

本ツールは現状有姿（AS IS）で提供され、明示又は黙示を問わず、正確性、完全性、最新性、特定目的への適合性その他一切の保証をいたしません。本ツールの利用又は利用不能により生じた直接的又は間接的な損害について、作成者は一切の責任を負いません。履修登録その他の判断は、必ず大学公式の情報に基づいて行ってください。

---

## Development

### Requirements

- **Docker** (Dev Container bundling Rust 1.95 / wasm-pack / Bun; start with `docker compose up -d`)
- **just** ([just-rs](https://github.com/casey/just) task runner)

Layout: `crates/core` (data conversion, bitset, search) / `crates/wasm` (browser WASM) / `crates/cli` (`raw/` fetch and convert CLI) / `web` (Svelte UI).

### Commands

Run `just --list` for the full recipe list. Common ones:

```bash
just dev            # Start the web dev server (builds WASM first)
just convert        # raw/ + raw-details/ -> data.json + details/
just test           # Run Rust and web tests
just check          # CI-equivalent checks (lint + test)
```

Git hooks (`lefthook`) run the same scope as CI; enable them with `just install-hooks`.

### Data fetching

KULAS is accessed **only from GitHub Actions**, never locally, and only for openly
published syllabus data. Access is deliberately gentle and identifiable — see
[Politeness / responsible access](docs/kulas-api-spec.md#politeness--responsible-access).

- **`fetch-syllabus.yml`** — updates `raw/*.json` (basic info) via the findPage API. The
  fetch is light, so it runs on a seasonal schedule: daily in the pre-term months
  (Mar/Apr/Sep/Oct), weekly otherwise, keeping data fresh when it matters most.
- **`fetch-details.yml`** — crawls syllabus reference pages into `raw-details/{kogiCd}.json`
  (lesson plan, grading, etc.). Daily but incremental and per-run capped, so any backlog
  is spread over many short off-peak runs; partial runs commit and later runs resume.

`convert` builds `data.json` (course cards with searchable body) and `details/{cd}.json` (lazy-loaded by the modal) from `raw/` and `raw-details/`. See [`docs/kulas-api-spec.md`](docs/kulas-api-spec.md) for the fetch spec and [`docs/syllabus-fields.md`](docs/syllabus-fields.md) (generated from `FIELD_SPEC` in `crates/cli/src/fields.rs`) for field priorities.

### Deploy

Pushing to `main` triggers `.github/workflows/deploy.yml`. `fetch-syllabus.yml` explicitly dispatches the deploy after its commit, since a `GITHUB_TOKEN` push does not trigger `on: push`.

---

## License

[GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0). Summary:

- Viewing, modifying, and private use of the source are free.
- Distributing a modified version **or providing it over a network** requires publishing the source under the same license (copyleft).
- The author provides no warranty or liability for use of this tool (see the license's "NO WARRANTY" clause).
