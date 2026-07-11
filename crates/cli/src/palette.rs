//! Derivation of the "Macaron" course-tile palette and the eval colours, ported
//! from the OKLCH scrap so it is source-controlled and drift-guarded (like
//! `fields`). Tiles are 10 hues on a shared ring, each at its lightest tone that
//! still holds one fixed chroma; the tinted inks (text / mutedText / accentText)
//! are solved to a WCAG target on the tile. `gen-palette --check` asserts the
//! committed `colors.ts` / `syllabus-icons.ts` still match this derivation.
//!
//! The maths mirrors the reference exactly (same constants, same scan steps), so
//! the output is byte-identical to the committed hexes — a test pins that.

use std::f64::consts::PI;
use std::path::Path;

use anyhow::{Context, Result, bail};

// ---------- OKLCH → sRGB ----------

fn oklch_to_linear(l: f64, c: f64, h_deg: f64) -> [f64; 3] {
    let h = h_deg * PI / 180.0;
    let a = c * h.cos();
    let b = c * h.sin();
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_662_1 * lc - 3.307_711_591_3 * mc + 0.230_969_929_2 * sc,
        -1.268_438_004_6 * lc + 2.609_757_401_1 * mc - 0.341_319_396_5 * sc,
        -0.004_196_086_3 * lc - 0.703_418_614_7 * mc + 1.707_614_701 * sc,
    ]
}

fn in_gamut(l: f64, c: f64, h: f64) -> bool {
    oklch_to_linear(l, c, h)
        .iter()
        .all(|&v| (-1e-4..=1.0 + 1e-4).contains(&v))
}

fn gamma(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn oklch_to_hex(l: f64, c: f64, h: f64) -> String {
    let rgb = oklch_to_linear(l, c, h);
    let mut out = String::from("#");
    for v in rgb {
        let byte = (gamma(v) * 255.0).round() as i64;
        out.push_str(&format!("{:02x}", byte.clamp(0, 255)));
    }
    out
}

/// Max in-gamut chroma at `(l, h)`, by 40-step bisection over `[0, 0.4]`.
fn max_chroma(l: f64, h: f64) -> f64 {
    let (mut lo, mut hi) = (0.0f64, 0.4f64);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if in_gamut(l, mid, h) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

// ---------- WCAG ----------

fn parse_hex(s: &str) -> [f64; 3] {
    let s = s.trim_start_matches('#');
    [0, 2, 4].map(|i| i64::from_str_radix(&s[i..i + 2], 16).unwrap_or(0) as f64)
}

fn luminance(rgb: [f64; 3]) -> f64 {
    let lin = rgb.map(|c| {
        let s = c / 255.0;
        if s <= 0.039_28 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]
}

fn contrast(fg: &str, bg: &str) -> f64 {
    let a = luminance(parse_hex(fg)) + 0.05;
    let b = luminance(parse_hex(bg)) + 0.05;
    if a > b { a / b } else { b / a }
}

// ---------- Tone / ink solvers ----------

/// Lightest (dir<0) / darkest (dir>0) L that still holds chroma `c` at hue `h`,
/// scanning by 0.004 within `[lo_l, hi_l]`.
fn tone_at_chroma(h: f64, c: f64, dir: i32, lo_l: f64, hi_l: f64) -> f64 {
    let step = 0.004;
    if dir < 0 {
        let mut l = hi_l;
        while l >= lo_l {
            if max_chroma(l, h) >= c {
                return l;
            }
            l -= step;
        }
        lo_l
    } else {
        let mut l = lo_l;
        while l <= hi_l {
            if max_chroma(l, h) >= c {
                return l;
            }
            l += step;
        }
        hi_l
    }
}

/// Solve a tinted text lightness (hue `h`, chroma `max*chroma_frac`) that clears
/// `target` contrast on `bg`, scanning L toward the ink end (dir).
fn solve_text_l(bg: &str, h: f64, chroma_frac: f64, target: f64, dir: i32) -> String {
    let mut l = if dir < 0 { 0.7 } else { 0.55 };
    loop {
        if dir < 0 {
            if l < 0.2 {
                break;
            }
        } else if l > 0.95 {
            break;
        }
        let c = max_chroma(l, h) * chroma_frac;
        let hex = oklch_to_hex(l, c, h);
        if contrast(&hex, bg) >= target {
            return hex;
        }
        l += f64::from(dir) * 0.005;
    }
    oklch_to_hex(if dir < 0 { 0.2 } else { 0.95 }, 0.0, h)
}

/// The tile's name ink: a same-hue tone solved to a crisp target (8 → … → 4.5).
fn tinted_ink(bg: &str, h: f64, dir: i32) -> String {
    for target in [8.0, 7.0, 6.0, 5.0, 4.5] {
        let hex = solve_text_l(bg, h, 0.35, target, dir);
        if contrast(&hex, bg) >= target {
            return hex;
        }
    }
    solve_text_l(bg, h, 0.35, 4.5, dir)
}

// ---------- Palette ----------

/// The five tokens of one tile.
#[derive(Debug, PartialEq, Eq)]
pub struct Tint {
    pub bg: String,
    pub border: String,
    pub text: String,
    pub muted_text: String,
    pub accent_text: String,
}

/// The derived palette: 10 tiles (light + dark) and the 6 eval colours.
pub struct Palette {
    pub light: Vec<Tint>,
    pub dark: Vec<Tint>,
    /// `(name, light_hex, dark_hex)` in eval order.
    pub eval: Vec<(&'static str, String, String)>,
}

const C_BG: f64 = 0.07;
const C_BORDER: f64 = 0.15;
const C_BG_DARK: f64 = 0.06;
const C_BORDER_DARK: f64 = 0.14;

/// Derive the whole palette. Deterministic — the source of truth for the tile
/// and eval colours.
#[must_use]
pub fn derive() -> Palette {
    let hues: Vec<f64> = (0..10)
        .map(|i| (350.0 + f64::from(i) * 36.0) % 360.0)
        .collect();

    let mut light = Vec::new();
    let mut dark = Vec::new();
    for &h in &hues {
        let lbg = oklch_to_hex(tone_at_chroma(h, C_BG, -1, 0.86, 0.96), C_BG, h);
        let lborder = oklch_to_hex(tone_at_chroma(h, C_BORDER, -1, 0.55, 0.8), C_BORDER, h);
        light.push(Tint {
            text: tinted_ink(&lbg, h, -1),
            accent_text: solve_text_l(&lbg, h, 0.9, 4.5, -1),
            muted_text: solve_text_l(&lbg, h, 0.6, 4.5, -1),
            border: lborder,
            bg: lbg,
        });
        let dbg = oklch_to_hex(tone_at_chroma(h, C_BG_DARK, 1, 0.22, 0.4), C_BG_DARK, h);
        let dborder = oklch_to_hex(
            tone_at_chroma(h, C_BORDER_DARK, 1, 0.5, 0.72),
            C_BORDER_DARK,
            h,
        );
        dark.push(Tint {
            text: tinted_ink(&dbg, h, 1),
            accent_text: solve_text_l(&dbg, h, 0.9, 4.5, 1),
            muted_text: solve_text_l(&dbg, h, 0.6, 4.5, 1),
            border: dborder,
            bg: dbg,
        });
    }

    // The kinds mirror KULAS's fixed grade-table vocabulary (see classify.rs).
    // minireport sits between report (blue) and attendance (green) as a teal --
    // kin to report, distinct at a glance.
    let eval_hues: [(&str, f64); 6] = [
        ("exam", 15.0),
        ("report", 250.0),
        ("minireport", 190.0),
        ("attendance", 150.0),
        ("quiz", 300.0),
        ("other", 250.0),
    ];
    let eval = eval_hues
        .iter()
        .map(|&(k, h)| {
            let cf = if k == "other" { 0.25 } else { 0.95 };
            (
                k,
                oklch_to_hex(0.64, max_chroma(0.64, h) * cf, h),
                oklch_to_hex(0.74, max_chroma(0.74, h) * cf, h),
            )
        })
        .collect();

    Palette { light, dark, eval }
}

// ---------- Drift check ----------

/// Verify the committed `colors.ts` and `syllabus-icons.ts` still match the
/// derivation. `--check` for CI.
///
/// # Errors
/// Returns an error listing any tile/eval colour that has drifted from the
/// derived value (a hand-edit that bypassed the generator).
pub fn check(root: &Path) -> Result<()> {
    let palette = derive();
    let colors = std::fs::read_to_string(root.join("web/src/lib/colors.ts"))
        .context("cannot read web/src/lib/colors.ts")?;
    let icons = std::fs::read_to_string(root.join("web/src/lib/syllabus-icons.ts"))
        .context("cannot read web/src/lib/syllabus-icons.ts")?;

    let mut drift: Vec<String> = Vec::new();

    // Every derived tile hex must appear in colors.ts (the file lists them inline).
    for (theme, tints) in [("light", &palette.light), ("dark", &palette.dark)] {
        for (i, t) in tints.iter().enumerate() {
            for (name, hex) in [
                ("bg", &t.bg),
                ("border", &t.border),
                ("text", &t.text),
                ("mutedText", &t.muted_text),
                ("accentText", &t.accent_text),
            ] {
                if !colors.contains(hex.as_str()) {
                    drift.push(format!(
                        "colors.ts {theme} tile {i} {name}: derived {hex} not found"
                    ));
                }
            }
        }
    }
    for (k, l, d) in &palette.eval {
        if !icons.contains(l.as_str()) {
            drift.push(format!(
                "syllabus-icons.ts eval {k} light: derived {l} not found"
            ));
        }
        if !icons.contains(d.as_str()) {
            drift.push(format!(
                "syllabus-icons.ts eval {k} dark: derived {d} not found"
            ));
        }
    }

    if drift.is_empty() {
        Ok(())
    } else {
        bail!(
            "palette drift — regenerate colours from the derivation (do not hand-edit):\n  {}",
            drift.join("\n  ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::derive;

    #[test]
    fn reproduces_the_committed_macaron_tiles() {
        let p = derive();
        // Spot-check the first, middle, and last light tiles against the committed
        // colors.ts (the whole point: the derivation is byte-exact).
        assert_eq!(p.light[0].bg, "#fec7df"); // rose
        assert_eq!(p.light[0].border, "#ff8ec6");
        assert_eq!(p.light[0].text, "#4d2e3d");
        assert_eq!(p.light[0].muted_text, "#94406c");
        assert_eq!(p.light[0].accent_text, "#ac1f74");
        assert_eq!(p.light[7].bg, "#acdafe"); // blue
        assert_eq!(p.light[9].accent_text, "#9723c3"); // magenta
        // A dark tile, and an eval colour.
        assert_eq!(p.dark[0].bg, "#2e0d1e");
        assert_eq!(p.eval.iter().find(|e| e.0 == "exam").unwrap().1, "#fa285c");
    }

    #[test]
    fn every_tint_clears_wcag_aa_on_its_tile() {
        let p = derive();
        for tints in [&p.light, &p.dark] {
            for t in tints {
                for ink in [&t.text, &t.muted_text, &t.accent_text] {
                    assert!(
                        super::contrast(ink, &t.bg) >= 4.5,
                        "{ink} on {} is below AA",
                        t.bg
                    );
                }
            }
        }
    }
}
