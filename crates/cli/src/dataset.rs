//! Transactional v4 dataset publisher.
//!
//! Every immutable asset is staged and verified below `datasets/<dataset-id>/`.
//! The only stable URL is `manifest.json`, promoted last. A failure before that
//! point cannot mix generations in the public dataset.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syllabus_core::{CourseCode, Offering, RawCourse};

use crate::convert::render_data_json;
use crate::detail::{PublicDetail, SanshoDetail, enrich};
use crate::io;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

#[cfg(any(windows, test))]
const fn combine_move_file_flags(replace_existing: u32, write_through: u32) -> u32 {
    replace_existing | write_through
}

#[cfg(any(windows, test))]
const fn move_file_ex_failed(result: i32) -> bool {
    result == 0
}

#[cfg(windows)]
const ATOMIC_REPLACE_FLAGS: u32 =
    combine_move_file_flags(MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH);
const BROTLI_BUFFER_BYTES: usize = 32 * 1024;
const MAX_DECODED_INDEX_BYTES: usize = 128 * 1024 * 1024;
const MAX_COMPRESSED_INDEX_BYTES: usize = 32 * 1024 * 1024;
const MAX_DECODED_INDEX_READ_BYTES: u64 = MAX_DECODED_INDEX_BYTES as u64 + 1;

#[derive(Debug)]
pub struct BuildDatasetOptions {
    pub files: Vec<PathBuf>,
    pub details_dir: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub generated_at: String,
    pub source_commit: String,
    pub compact: bool,
    /// Permit missing detail/unavailable records for synthetic development
    /// fixtures. Production callers must leave this false.
    pub allow_incomplete_details: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Asset {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetCounts {
    pub courses: u32,
    pub details: u32,
    pub detail_coverage: f64,
    pub scheduled_courses: u32,
    pub unscheduled_courses: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetRange {
    pub min_day: Option<u8>,
    pub max_day: Option<u8>,
    pub min_period: Option<u8>,
    pub max_period: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetAssets {
    pub data: Asset,
    pub index: Asset,
    /// Content-addressed JSON mapping course code → detail asset.
    pub details: Asset,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DatasetManifest {
    pub schema_version: u32,
    pub app_compat_version: u32,
    pub dataset_id: String,
    pub source_commit: String,
    pub year: String,
    pub generated_at: String,
    pub base_path: String,
    pub counts: DatasetCounts,
    pub range: DatasetRange,
    pub assets: DatasetAssets,
}

pub fn build(options: &BuildDatasetOptions) -> Result<DatasetManifest> {
    ensure!(
        matches!(options.source_commit.len(), 40 | 64)
            && options
                .source_commit
                .bytes()
                .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value)),
        "source commit must be a full lowercase hexadecimal Git object ID"
    );
    let loaded = io::load(&options.files)?;
    validate_raw(&loaded.courses)?;

    let details = load_details(options.details_dir.as_deref(), &loaded.courses)?;
    let generated_at = chrono::DateTime::parse_from_rfc3339(&options.generated_at)
        .context("generatedAt must be a valid RFC 3339 timestamp")?
        .with_timezone(&chrono::Utc);
    let confirmed_unavailable = if let Some(directory) = options.details_dir.as_deref() {
        crate::fetch_details::confirmed_unavailable_codes(directory, &loaded.courses, generated_at)?
    } else {
        HashSet::new()
    };
    let missing_details: Vec<&str> = loaded
        .courses
        .iter()
        .map(|course| course.kogi_cd.trim())
        .filter(|code| !details.contains_key(*code) && !confirmed_unavailable.contains(*code))
        .collect();
    ensure!(
        options.allow_incomplete_details || missing_details.is_empty(),
        "detail crawl is incomplete: {} active courses have neither a detail nor an unexpired official-unavailable record (first: {:?})",
        missing_details.len(),
        &missing_details[..missing_details.len().min(10)]
    );
    let rendered = render_data_json(
        &loaded.courses,
        options.generated_at.clone(),
        &options.source_commit,
        options.compact,
        &details,
    )?;
    let data: syllabus_core::ProcessedData =
        serde_json::from_slice(&rendered.bytes).context("v4 output did not deserialize")?;
    syllabus_core::Engine::from_json(
        std::str::from_utf8(&rendered.bytes).context("data.json is not UTF-8")?,
    )
    .context("v4 output failed integrity validation")?;

    fs::create_dir_all(&options.output_dir).with_context(|| {
        format!(
            "failed to create dataset output directory {}",
            options.output_dir.display()
        )
    })?;
    let datasets_root = options.output_dir.join("datasets");
    fs::create_dir_all(&datasets_root)?;
    let stage = datasets_root.join(format!(
        ".stage-{}-{}",
        std::process::id(),
        &data.dataset_id[..data.dataset_id.len().min(16)]
    ));
    ensure!(!stage.exists(), "staging directory already exists");
    fs::create_dir_all(stage.join("details"))?;

    let result = (|| -> Result<DatasetManifest> {
        let data_asset = write_asset(&stage, "data", "json", &rendered.bytes)?;
        inject_failure("data")?;
        let compressed_index = compress_index(&rendered.index)?;
        let index_asset = write_asset_with_decoded(
            &stage,
            "search",
            "idx.br",
            &compressed_index,
            Some(&rendered.index),
        )?;
        inject_failure("index")?;

        let active: HashSet<&str> = data
            .courses
            .iter()
            .map(|course| course.cd.as_str())
            .collect();
        let mut detail_assets = BTreeMap::new();
        for code in active {
            let Some(detail) = details.get(code) else {
                continue;
            };
            let public = PublicDetail::try_from(detail)
                .with_context(|| format!("detail {code:?} is not safe for publication"))?;
            let bytes = serde_json::to_vec(&public).context("failed to serialize detail")?;
            let code_hash = sha256(code.as_bytes());
            let content_hash = sha256(&bytes);
            let relative = format!("details/{}.{}.json", &code_hash[..16], &content_hash[..16]);
            fs::write(stage.join(&relative), &bytes)?;
            detail_assets.insert(
                code.to_owned(),
                Asset {
                    path: relative,
                    bytes: bytes.len() as u64,
                    sha256: content_hash,
                    decoded_bytes: None,
                    decoded_sha256: None,
                },
            );
        }
        inject_failure("details")?;
        let detail_index_bytes =
            serde_json::to_vec(&detail_assets).context("failed to serialize detail index")?;
        let detail_index_asset = write_asset(&stage, "details", "json", &detail_index_bytes)?;

        let scheduled_courses = data
            .offerings
            .iter()
            .filter(|values| values.iter().any(Offering::is_scheduled))
            .count() as u32;
        let unscheduled_courses = data
            .offerings
            .iter()
            .filter(|values| values.iter().any(|value| !value.is_scheduled()))
            .count() as u32;
        ensure!(
            data.offerings.iter().all(|values| !values.is_empty()),
            "every course must have at least one offering"
        );

        let scheduled = data.offerings.iter().flatten().filter_map(|value| {
            if let Offering::Scheduled { d, p, .. } = value {
                Some((*d, *p))
            } else {
                None
            }
        });
        let scheduled: Vec<(u8, u8)> = scheduled.collect();
        let counts = DatasetCounts {
            courses: data.courses.len() as u32,
            details: detail_assets.len() as u32,
            detail_coverage: if data.courses.is_empty() {
                1.0
            } else {
                detail_assets.len() as f64 / data.courses.len() as f64
            },
            scheduled_courses,
            unscheduled_courses,
        };
        let range = DatasetRange {
            min_day: scheduled.iter().map(|(day, _)| *day).min(),
            max_day: scheduled.iter().map(|(day, _)| *day).max(),
            min_period: scheduled.iter().map(|(_, period)| *period).min(),
            max_period: scheduled.iter().map(|(_, period)| *period).max(),
        };
        let manifest = DatasetManifest {
            schema_version: 4,
            app_compat_version: 4,
            dataset_id: data.dataset_id.clone(),
            source_commit: options.source_commit.clone(),
            year: data.year.clone(),
            generated_at: data.generated_at.clone(),
            base_path: format!("datasets/{}/", data.dataset_id),
            counts,
            range,
            assets: DatasetAssets {
                data: data_asset,
                index: index_asset,
                details: detail_index_asset,
            },
        };
        let manifest_bytes =
            serde_json::to_vec_pretty(&manifest).context("failed to serialize manifest")?;
        fs::write(stage.join("manifest.json"), &manifest_bytes)?;
        inject_failure("manifest")?;

        verify_staged(&stage, &manifest)?;
        let public_manifest = options.output_dir.join("manifest.json");
        let next_manifest = options.output_dir.join("manifest.next.json");
        let previous_dataset_id = read_dataset_id(&public_manifest);
        {
            let mut file = fs::File::create(&next_manifest)?;
            file.write_all(&manifest_bytes)?;
            file.sync_all()?;
        }
        inject_failure("promote-manifest")?;

        let final_dir = datasets_root.join(&data.dataset_id);
        let final_already_existed = final_dir.exists();
        if final_already_existed {
            verify_staged(&final_dir, &manifest)
                .context("existing dataset directory does not match its content identity")?;
            fs::remove_dir_all(&stage)?;
        } else {
            inject_failure("promote-dataset")?;
            fs::rename(&stage, &final_dir).context("failed to promote staged dataset")?;
        }

        if let Err(error) = replace_file_atomic(&next_manifest, &public_manifest) {
            if !final_already_existed && final_dir.exists() {
                let _ = fs::remove_dir_all(&final_dir);
            }
            let _ = fs::remove_file(&next_manifest);
            return Err(error).context("failed to atomically promote stable manifest");
        }
        remove_legacy_outputs(&options.output_dir)?;
        prune_dataset_generations(
            &datasets_root,
            &data.dataset_id,
            previous_dataset_id.as_deref(),
        )?;
        Ok(manifest)
    })();

    if result.is_err() {
        if stage.exists() {
            let _ = fs::remove_dir_all(&stage);
        }
        let _ = fs::remove_file(options.output_dir.join("manifest.next.json"));
    }
    result
}

fn replace_file_atomic(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // Both paths are sibling files inside the caller-selected output directory.
        // MoveFileExW with REPLACE_EXISTING is Windows' atomic same-volume replace.
        let result =
            unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), ATOMIC_REPLACE_FLAGS) };
        if move_file_ex_failed(result) {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
fn validate_raw(raw: &[RawCourse]) -> Result<()> {
    ensure!(!raw.is_empty(), "no course data found");
    let mut codes = HashSet::new();
    let mut years = HashSet::new();
    for (position, course) in raw.iter().enumerate() {
        let code = CourseCode::parse(&course.kogi_cd)
            .with_context(|| format!("invalid course code at record {}", position + 1))?;
        ensure!(
            codes.insert(code.as_str().to_owned()),
            "duplicate course code {:?}",
            code.as_str()
        );
        let year = course.kaiko_nendo.as_deref().unwrap_or("").trim();
        ensure!(
            !year.is_empty(),
            "course {:?} has no academic year",
            code.as_str()
        );
        ensure!(
            year.chars().all(|value| value.is_ascii_digit()) && year.len() == 4,
            "course {:?} has invalid academic year {:?}",
            code.as_str(),
            year
        );
        years.insert(year.to_owned());
        ensure!(
            course
                .syllabus_komoku_pattern_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
            "course {:?} has no syllabus pattern ID",
            code.as_str()
        );
    }
    ensure!(years.len() == 1, "input mixes academic years: {years:?}");
    Ok(())
}

fn load_details(
    directory: Option<&Path>,
    raw: &[RawCourse],
) -> Result<HashMap<String, SanshoDetail>> {
    let active: HashSet<&str> = raw.iter().map(|course| course.kogi_cd.trim()).collect();
    let Some(directory) = directory else {
        return Ok(HashMap::new());
    };
    let mut details = HashMap::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read details directory {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().is_none_or(|value| value != "json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.starts_with('_'))
        {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !active.contains(stem) {
            continue;
        }
        let mut detail: SanshoDetail = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        CourseCode::parse(&detail.cd)
            .with_context(|| format!("unsafe detail code in {}", path.display()))?;
        ensure!(
            stem == detail.cd,
            "detail file/code mismatch: {} contains {:?}",
            path.display(),
            detail.cd
        );
        enrich(&mut detail);
        ensure!(
            details.insert(detail.cd.clone(), detail).is_none(),
            "duplicate detail code in {}",
            directory.display()
        );
    }
    Ok(details)
}

fn write_asset(stage: &Path, stem: &str, extension: &str, bytes: &[u8]) -> Result<Asset> {
    write_asset_with_decoded(stage, stem, extension, bytes, None)
}

fn write_asset_with_decoded(
    stage: &Path,
    stem: &str,
    extension: &str,
    bytes: &[u8],
    decoded: Option<&[u8]>,
) -> Result<Asset> {
    let hash = sha256(bytes);
    let file_name = format!("{stem}.{}.{}", &hash[..16], extension);
    fs::write(stage.join(&file_name), bytes)?;
    Ok(Asset {
        path: file_name,
        bytes: bytes.len() as u64,
        sha256: hash,
        decoded_bytes: decoded.map(|value| value.len() as u64),
        decoded_sha256: decoded.map(sha256),
    })
}

fn compress_index(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        bytes.len() <= MAX_DECODED_INDEX_BYTES,
        "search index exceeds the decoded size limit"
    );
    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, BROTLI_BUFFER_BYTES, 9, 22);
        writer
            .write_all(bytes)
            .context("failed to Brotli-compress search index")?;
    }
    ensure!(
        compressed.len() <= MAX_COMPRESSED_INDEX_BYTES,
        "compressed search index exceeds the transport size limit"
    );
    Ok(compressed)
}

fn verify_staged(stage: &Path, manifest: &DatasetManifest) -> Result<()> {
    ensure!(
        manifest.schema_version == 4 && manifest.app_compat_version == 4,
        "unsupported manifest compatibility version"
    );
    ensure!(
        manifest.dataset_id.len() == 64
            && manifest
                .dataset_id
                .bytes()
                .all(|value| value.is_ascii_hexdigit()),
        "manifest dataset ID is not a SHA-256 identity"
    );
    ensure!(
        manifest.base_path == format!("datasets/{}/", manifest.dataset_id),
        "manifest base path does not match dataset identity"
    );
    ensure!(
        manifest.year.len() == 4 && manifest.year.bytes().all(|value| value.is_ascii_digit()),
        "manifest academic year is invalid"
    );
    ensure!(
        !manifest.generated_at.is_empty()
            && manifest.generated_at.len() <= 128
            && !manifest.generated_at.chars().any(char::is_control),
        "manifest generation time is invalid"
    );
    ensure!(
        manifest.counts.details <= manifest.counts.courses,
        "detail count exceeds course count"
    );
    let expected_coverage = if manifest.counts.courses == 0 {
        1.0
    } else {
        f64::from(manifest.counts.details) / f64::from(manifest.counts.courses)
    };
    ensure!(
        (manifest.counts.detail_coverage - expected_coverage).abs() <= f64::EPSILON,
        "detail coverage does not match counts"
    );
    ensure!(
        manifest.assets.data.decoded_bytes.is_none()
            && manifest.assets.data.decoded_sha256.is_none()
            && manifest.assets.details.decoded_bytes.is_none()
            && manifest.assets.details.decoded_sha256.is_none()
            && manifest.assets.index.decoded_bytes.is_some()
            && manifest.assets.index.decoded_sha256.is_some(),
        "manifest decoded asset identities are assigned to the wrong asset"
    );

    let assets = std::iter::once(&manifest.assets.data)
        .chain(std::iter::once(&manifest.assets.index))
        .chain(std::iter::once(&manifest.assets.details));
    for asset in assets {
        validate_asset_path(&asset.path)?;
        validate_asset_identity(asset)?;
        let bytes = fs::read(stage.join(&asset.path))
            .with_context(|| format!("missing staged asset {}", asset.path))?;
        ensure!(bytes.len() as u64 == asset.bytes, "asset size mismatch");
        ensure!(sha256(&bytes) == asset.sha256, "asset hash mismatch");
    }

    let data_bytes = fs::read(stage.join(&manifest.assets.data.path))?;
    let data: syllabus_core::ProcessedData =
        serde_json::from_slice(&data_bytes).context("invalid staged data asset")?;
    ensure!(
        data.dataset_id == manifest.dataset_id,
        "staged data/manifest dataset identity mismatch"
    );
    ensure!(
        data.year == manifest.year,
        "staged data/manifest year mismatch"
    );
    ensure!(
        data.generated_at == manifest.generated_at,
        "staged data/manifest generation time mismatch"
    );
    ensure!(
        data.courses.len() == manifest.counts.courses as usize,
        "staged data/manifest course count mismatch"
    );
    ensure!(
        data.offerings.iter().all(|values| !values.is_empty()),
        "staged data contains a course without an offering"
    );
    let scheduled_courses = data
        .offerings
        .iter()
        .filter(|values| values.iter().any(Offering::is_scheduled))
        .count() as u32;
    let unscheduled_courses = data
        .offerings
        .iter()
        .filter(|values| values.iter().any(|value| !value.is_scheduled()))
        .count() as u32;
    ensure!(
        scheduled_courses == manifest.counts.scheduled_courses
            && unscheduled_courses == manifest.counts.unscheduled_courses,
        "staged data/manifest offering counts mismatch"
    );
    let scheduled_slots: Vec<(u8, u8)> = data
        .offerings
        .iter()
        .flatten()
        .filter_map(|offering| match offering {
            Offering::Scheduled { d, p, .. } => Some((*d, *p)),
            Offering::Intensive { .. } | Offering::Tba { .. } => None,
        })
        .collect();
    ensure!(
        manifest.range.min_day == scheduled_slots.iter().map(|(day, _)| *day).min()
            && manifest.range.max_day == scheduled_slots.iter().map(|(day, _)| *day).max()
            && manifest.range.min_period == scheduled_slots.iter().map(|(_, period)| *period).min()
            && manifest.range.max_period == scheduled_slots.iter().map(|(_, period)| *period).max(),
        "staged data/manifest timetable range mismatch"
    );
    let mut engine = syllabus_core::Engine::from_json(
        std::str::from_utf8(&data_bytes).context("staged data is not UTF-8")?,
    )
    .context("staged data failed integrity validation")?;

    let encoded_index = fs::read(stage.join(&manifest.assets.index.path))?;
    let mut decoded_index = Vec::new();
    brotli::Decompressor::new(encoded_index.as_slice(), BROTLI_BUFFER_BYTES)
        .take(MAX_DECODED_INDEX_READ_BYTES)
        .read_to_end(&mut decoded_index)
        .context("failed to decode staged search index")?;
    ensure!(
        decoded_index.len() <= MAX_DECODED_INDEX_BYTES,
        "decoded staged search index exceeds size limit"
    );
    ensure!(
        manifest.assets.index.decoded_bytes == Some(decoded_index.len() as u64),
        "decoded search index size mismatch"
    );
    let decoded_index_hash = sha256(&decoded_index);
    ensure!(
        manifest.assets.index.decoded_sha256.as_deref() == Some(decoded_index_hash.as_str()),
        "decoded search index hash mismatch"
    );
    engine
        .load_search_index(&decoded_index)
        .context("staged data/search index identity mismatch")?;

    let detail_index_bytes = fs::read(stage.join(&manifest.assets.details.path))?;
    let detail_assets: BTreeMap<String, Asset> =
        serde_json::from_slice(&detail_index_bytes).context("invalid staged detail index")?;
    ensure!(
        detail_assets.len() == manifest.counts.details as usize,
        "detail index count mismatch"
    );
    let active_codes: HashSet<&str> = data
        .courses
        .iter()
        .map(|course| course.cd.as_str())
        .collect();
    for (code, asset) in detail_assets {
        CourseCode::parse(&code).context("detail index contains an invalid course code")?;
        ensure!(
            active_codes.contains(code.as_str()),
            "detail index contains an orphan course {code:?}"
        );
        validate_asset_path(&asset.path)?;
        validate_asset_identity(&asset)?;
        ensure!(
            asset.decoded_bytes.is_none() && asset.decoded_sha256.is_none(),
            "detail asset unexpectedly declares a decoded identity"
        );
        let bytes = fs::read(stage.join(&asset.path))
            .with_context(|| format!("missing staged detail asset {}", asset.path))?;
        ensure!(
            bytes.len() as u64 == asset.bytes,
            "detail asset size mismatch"
        );
        ensure!(sha256(&bytes) == asset.sha256, "detail asset hash mismatch");
        let detail: PublicDetail = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid staged public detail for {code:?}"))?;
        ensure!(
            detail.cd == code,
            "staged public detail/index code mismatch: {code:?} != {:?}",
            detail.cd
        );
    }
    Ok(())
}

fn validate_asset_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && path.len() <= 240
            && !path.chars().any(char::is_control)
            && !path.contains('\\'),
        "invalid asset path"
    );
    let path = Path::new(path);
    ensure!(!path.is_absolute(), "asset path must be relative");
    ensure!(
        path.components()
            .all(|part| matches!(part, std::path::Component::Normal(_))),
        "asset path contains a traversal component"
    );
    Ok(())
}

fn validate_asset_identity(asset: &Asset) -> Result<()> {
    ensure!(
        asset.sha256.len() == 64 && asset.sha256.bytes().all(|value| value.is_ascii_hexdigit()),
        "asset SHA-256 identity is invalid"
    );
    ensure!(asset.bytes > 0, "asset must not be empty");
    ensure!(
        asset.decoded_bytes.is_some() == asset.decoded_sha256.is_some(),
        "asset decoded identity is incomplete"
    );
    if let Some(hash) = &asset.decoded_sha256 {
        ensure!(
            hash.len() == 64 && hash.bytes().all(|value| value.is_ascii_hexdigit()),
            "asset decoded SHA-256 identity is invalid"
        );
    }
    Ok(())
}

fn inject_failure(point: &str) -> Result<()> {
    if std::env::var("SYLLABUS_FAIL_AT").ok().as_deref() == Some(point) {
        bail!("injected dataset build failure after {point}");
    }
    Ok(())
}

fn read_dataset_id(path: &Path) -> Option<String> {
    let manifest: DatasetManifest = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    Some(manifest.dataset_id)
}

fn remove_legacy_outputs(output_dir: &Path) -> Result<()> {
    for file_name in ["data.json", "search.idx"] {
        let path = output_dir.join(file_name);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove legacy asset {}", path.display()))?;
        }
    }
    let details = output_dir.join("details");
    if details.exists() {
        fs::remove_dir_all(&details)
            .with_context(|| format!("failed to remove legacy details {}", details.display()))?;
    }
    Ok(())
}

fn prune_dataset_generations(
    datasets_root: &Path,
    current_dataset_id: &str,
    previous_dataset_id: Option<&str>,
) -> Result<()> {
    for entry in fs::read_dir(datasets_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".stage-")
            || name == current_dataset_id
            || previous_dataset_id.is_some_and(|previous| name == previous)
        {
            continue;
        }
        fs::remove_dir_all(entry.path()).with_context(|| {
            format!(
                "failed to prune obsolete dataset generation {}",
                entry.path().display()
            )
        })?;
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn scratch(label: &str) -> PathBuf {
        let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gyakubiki-dataset-{label}-{}-{sequence}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale scratch directory");
        }
        fs::create_dir_all(&root).expect("create scratch directory");
        root
    }

    fn write_raw(root: &Path, codes: &[&str]) -> PathBuf {
        let records: Vec<_> = codes
            .iter()
            .enumerate()
            .map(|(index, code)| {
                json!({
                    "kogiCd": code,
                    "kogiNm": format!("Mutation boundary {index}"),
                    "tantoKyoin": "Test",
                    "jikanwari": if index % 2 == 0 {
                        "1学期: 月曜日１時限"
                    } else {
                        "1学期: 集中講義"
                    },
                    "kogiKaikojikiNm": "1学期",
                    "kogiKubunNm": "講義",
                    "sekininBushoNm": "Test",
                    "kochiNm": "朝倉キャンパス",
                    "syllabusKomokuPatternId": "4",
                    "kaikoNendo": "2026"
                })
            })
            .collect();
        let path = root.join("raw.json");
        fs::write(&path, serde_json::to_vec(&records).unwrap()).unwrap();
        path
    }

    fn write_detail(directory: &Path, code: &str) {
        fs::create_dir_all(directory).unwrap();
        let detail = SanshoDetail {
            cd: code.to_owned(),
            summary: Some(format!("{code} detail")),
            ..Default::default()
        };
        fs::write(
            directory.join(format!("{code}.json")),
            serde_json::to_vec(&detail).unwrap(),
        )
        .unwrap();
    }

    fn options(raw: &Path, output: &Path) -> BuildDatasetOptions {
        BuildDatasetOptions {
            files: vec![raw.to_owned()],
            details_dir: None,
            output_dir: output.to_owned(),
            generated_at: "2026-01-01T00:00:00Z".to_owned(),
            source_commit: "0".repeat(40),
            compact: true,
            allow_incomplete_details: true,
        }
    }

    fn generation(output: &Path, manifest: &DatasetManifest) -> PathBuf {
        output.join("datasets").join(&manifest.dataset_id)
    }

    fn dummy_asset(path: &str) -> Asset {
        Asset {
            path: path.to_owned(),
            bytes: 1,
            sha256: "a".repeat(64),
            decoded_bytes: None,
            decoded_sha256: None,
        }
    }

    #[test]
    fn index_transport_limits_are_exact() {
        assert_eq!(BROTLI_BUFFER_BYTES, 32_768);
        assert_eq!(MAX_DECODED_INDEX_BYTES, 134_217_728);
        assert_eq!(MAX_COMPRESSED_INDEX_BYTES, 33_554_432);
        assert_eq!(MAX_DECODED_INDEX_READ_BYTES, 134_217_729);
    }

    #[test]
    fn complete_and_partial_details_preserve_coverage() {
        let root = scratch("detail-coverage");
        let details = root.join("details");
        write_detail(&details, "A0001");
        fs::write(details.join("notes.txt"), b"not json").unwrap();
        fs::write(details.join("_crawler.json"), b"not json").unwrap();
        fs::write(details.join("B9999.json"), b"not json").unwrap();

        let strict_raw = write_raw(&root, &["A0001"]);
        let strict_output = root.join("strict-public");
        let mut strict = options(&strict_raw, &strict_output);
        strict.details_dir = Some(details.clone());
        strict.allow_incomplete_details = false;
        let strict_manifest = build(&strict).expect("complete detail set publishes");
        assert_eq!(strict_manifest.counts.details, 1);
        assert_eq!(strict_manifest.counts.detail_coverage, 1.0);

        let partial_root = root.join("partial");
        fs::create_dir_all(&partial_root).unwrap();
        let partial_raw = write_raw(&partial_root, &["A0001", "A0002"]);
        let partial_output = partial_root.join("public");
        let mut partial = options(&partial_raw, &partial_output);
        partial.details_dir = Some(details);
        let partial_manifest = build(&partial).expect("explicit incomplete fixture publishes");
        assert_eq!(partial_manifest.counts.courses, 2);
        assert_eq!(partial_manifest.counts.details, 1);
        assert_eq!(partial_manifest.counts.detail_coverage, 0.5);

        let detail_index: BTreeMap<String, Asset> = serde_json::from_slice(
            &fs::read(
                generation(&partial_output, &partial_manifest)
                    .join(&partial_manifest.assets.details.path),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(detail_index.keys().collect::<Vec<_>>(), vec!["A0001"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_file_helpers_preserve_overlapping_flags_and_bool_semantics() {
        assert_eq!(combine_move_file_flags(0b0011, 0b0101), 0b0111);
        assert!(move_file_ex_failed(0));
        assert!(!move_file_ex_failed(1));
    }

    #[test]
    fn atomic_replace_moves_source_and_replaces_destination() {
        let root = scratch("atomic-replace");
        let source = root.join("manifest.next.json");
        let destination = root.join("manifest.json");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old").unwrap();

        #[cfg(windows)]
        {
            assert_ne!(ATOMIC_REPLACE_FLAGS & MOVEFILE_REPLACE_EXISTING, 0);
            assert_ne!(ATOMIC_REPLACE_FLAGS & MOVEFILE_WRITE_THROUGH, 0);
        }
        replace_file_atomic(&source, &destination).expect("atomic replacement succeeds");
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promotion_failure_rolls_back_new_generation_but_preserves_existing_one() {
        let new_root = scratch("promotion-new");
        let new_raw = write_raw(&new_root, &["A0001"]);
        let new_output = new_root.join("public");
        fs::create_dir_all(new_output.join("manifest.json")).unwrap();
        let error =
            build(&options(&new_raw, &new_output)).expect_err("manifest directory blocks replace");
        assert!(error.to_string().contains("atomically promote"));
        assert!(!new_output.join("manifest.next.json").exists());
        assert_eq!(
            fs::read_dir(new_output.join("datasets")).unwrap().count(),
            0
        );

        let existing_root = scratch("promotion-existing");
        let existing_raw = write_raw(&existing_root, &["A0001"]);
        let existing_output = existing_root.join("public");
        let existing_options = options(&existing_raw, &existing_output);
        let manifest = build(&existing_options).expect("initial generation publishes");
        let existing_generation = generation(&existing_output, &manifest);
        fs::remove_file(existing_output.join("manifest.json")).unwrap();
        fs::create_dir(existing_output.join("manifest.json")).unwrap();

        assert!(build(&existing_options).is_err());
        assert!(existing_generation.is_dir());
        assert!(!existing_output.join("manifest.next.json").exists());
        assert!(
            fs::read_dir(existing_output.join("datasets"))
                .unwrap()
                .all(|entry| !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".stage-"))
        );
        fs::remove_dir_all(new_root).unwrap();
        fs::remove_dir_all(existing_root).unwrap();
    }

    #[test]
    fn staged_verifier_and_asset_validators_reject_tampering() {
        let root = scratch("verify");
        let raw = write_raw(&root, &["A0001"]);
        let output = root.join("public");
        let manifest = build(&options(&raw, &output)).unwrap();
        let mut tampered = manifest.clone();
        tampered.counts.details = tampered.counts.courses + 1;
        assert!(verify_staged(&generation(&output, &manifest), &tampered).is_err());

        assert!(validate_asset_path("data.abcdef.json").is_ok());
        for invalid in [
            "",
            "../data.json",
            "/data.json",
            "dir\\data.json",
            "bad\nname",
        ] {
            assert!(
                validate_asset_path(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }

        let valid = dummy_asset("data.json");
        assert!(validate_asset_identity(&valid).is_ok());
        let mut bad_hash = valid.clone();
        bad_hash.sha256 = "x".repeat(64);
        assert!(validate_asset_identity(&bad_hash).is_err());
        let mut empty = valid.clone();
        empty.bytes = 0;
        assert!(validate_asset_identity(&empty).is_err());
        let mut incomplete_decoded = valid;
        incomplete_decoded.decoded_bytes = Some(1);
        assert!(validate_asset_identity(&incomplete_decoded).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_read_legacy_cleanup_and_generation_prune_are_exact() {
        let root = scratch("cleanup");
        let manifest_path = root.join("manifest.json");
        let manifest = DatasetManifest {
            schema_version: 4,
            app_compat_version: 4,
            dataset_id: "b".repeat(64),
            source_commit: "0".repeat(40),
            year: "2026".to_owned(),
            generated_at: "2026-01-01T00:00:00Z".to_owned(),
            base_path: format!("datasets/{}/", "b".repeat(64)),
            counts: DatasetCounts {
                courses: 0,
                details: 0,
                detail_coverage: 1.0,
                scheduled_courses: 0,
                unscheduled_courses: 0,
            },
            range: DatasetRange {
                min_day: None,
                max_day: None,
                min_period: None,
                max_period: None,
            },
            assets: DatasetAssets {
                data: dummy_asset("data.json"),
                index: dummy_asset("search.idx.br"),
                details: dummy_asset("details.json"),
            },
        };
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(read_dataset_id(&manifest_path), Some("b".repeat(64)));
        assert_eq!(read_dataset_id(&root.join("missing.json")), None);
        fs::write(&manifest_path, b"not json").unwrap();
        assert_eq!(read_dataset_id(&manifest_path), None);

        fs::write(root.join("data.json"), b"legacy").unwrap();
        fs::write(root.join("search.idx"), b"legacy").unwrap();
        fs::create_dir(root.join("details")).unwrap();
        fs::write(root.join("details/legacy.json"), b"legacy").unwrap();
        remove_legacy_outputs(&root).unwrap();
        assert!(!root.join("data.json").exists());
        assert!(!root.join("search.idx").exists());
        assert!(!root.join("details").exists());

        let datasets = root.join("datasets");
        for directory in ["current", "previous", "obsolete", ".stage-interrupted"] {
            fs::create_dir_all(datasets.join(directory)).unwrap();
        }
        fs::write(datasets.join("README.txt"), b"not a generation").unwrap();
        prune_dataset_generations(&datasets, "current", Some("previous")).unwrap();
        assert!(datasets.join("current").is_dir());
        assert!(datasets.join("previous").is_dir());
        assert!(datasets.join(".stage-interrupted").is_dir());
        assert!(datasets.join("README.txt").is_file());
        assert!(!datasets.join("obsolete").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
