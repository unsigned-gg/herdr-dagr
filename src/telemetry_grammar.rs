//! Pure, zero-I/O `SPEC-0009` telemetry grammar and `BaobabView` proportional
//! flow helpers (Layer 4 of `odsc-dataviz-estate-upgrade-synthesis.md`).
//!
//! Enforces three visualization invariants across TUI and snapshot surfaces:
//! 1. Redundant dual-encoding (glyph + text label) for gate posture so status
//!    never relies on ANSI hue alone.
//! 2. Proportional flow stroke tiers (`"━"`, `"═"`, `"█"`) keyed by basis points
//!    (`0..=10_000`) for `BaobabView` tributary ribbons.
//! 3. Datasaurus-guarded quantile strips (`P50`/`P95`/`P99` + `SLO CAP`) that
//!    reject low-sample or bare-mean summaries.

#![allow(dead_code)]

use std::fmt;

/// Maximum proportional flow in basis points (100.00%).
pub const MAX_FLOW_BPS: u16 = 10_000;

/// Minimum sample count required before rendering a quantile strip.
pub const MIN_DATASAURUS_SAMPLES: u64 = 10;

/// Gate posture outcomes mapped to redundant colorblind-safe glyph + label tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GatePostureGlyph {
    Green,
    Anchored,
    Shadow,
    Red,
    Never,
}

impl GatePostureGlyph {
    /// Standalone geometric/kanji glyph for compact column cells.
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Green => "✓",
            Self::Anchored => "◈",
            Self::Shadow => "◎",
            Self::Red => "▲",
            Self::Never => "封",
        }
    }

    /// Uppercase canonical posture label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Green => "GREEN",
            Self::Anchored => "ANCHORED",
            Self::Shadow => "SHADOW",
            Self::Red => "RED",
            Self::Never => "NEVER",
        }
    }

    /// Redundant glyph + label token (`SPEC-0009` dual-encoding contract).
    pub const fn token(self) -> &'static str {
        match self {
            Self::Green => "✓ GREEN",
            Self::Anchored => "◈ ANCHORED",
            Self::Shadow => "◎ SHADOW",
            Self::Red => "▲ RED",
            Self::Never => "封 NEVER",
        }
    }

    /// Parse a gate outcome string into its canonical posture glyph.
    pub fn from_outcome(outcome: &str) -> Option<Self> {
        match outcome.trim().to_ascii_lowercase().as_str() {
            "green" | "pass" | "passed" | "ok" => Some(Self::Green),
            "anchored" | "anchor" => Some(Self::Anchored),
            "shadow" | "advisory" => Some(Self::Shadow),
            "red" | "fail" | "failed" | "blocked" => Some(Self::Red),
            "never" | "sealed" | "forbid" | "forbidden" => Some(Self::Never),
            _ => None,
        }
    }
}

impl fmt::Display for GatePostureGlyph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// Maps proportional flow basis points (`0..=10_000`) to discrete TUI/SVG
/// stroke tiers for `BaobabView` tributary ribbons:
/// - `< 2000 bps` -> `"━"` (thin)
/// - `2000..6000 bps` -> `"═"` (medium)
/// - `>= 6000 bps` -> `"█"` (heavy)
pub const fn baobab_ribbon_stroke_tier(flow_bps: u16) -> &'static str {
    if flow_bps < 2_000 {
        "━"
    } else if flow_bps < 6_000 {
        "═"
    } else {
        "█"
    }
}

/// Quantile summary for attempt durations with Datasaurus guard validation
/// and `SPEC-0009` header strip formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptDurationQuantiles {
    pub sample_count: u64,
    pub min_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
    pub mean_ms: u64,
    pub slo_cap_ms: u64,
}

impl AttemptDurationQuantiles {
    /// Validates the Datasaurus guard:
    /// 1. Requires `sample_count >= 10`.
    /// 2. Rejects bare means where `max_ms == 0 && mean_ms > 0`.
    /// 3. Enforces monotonic quantile ordering `min_ms <= p50_ms <= p95_ms <= p99_ms <= max_ms`.
    pub const fn validate_datasaurus_guard(&self) -> Result<(), &'static str> {
        if self.sample_count < MIN_DATASAURUS_SAMPLES {
            return Err("datasaurus guard: sample_count must be at least 10");
        }
        if self.max_ms == 0 && self.mean_ms > 0 {
            return Err("datasaurus guard: bare mean rejected when max_ms is 0");
        }
        if !(self.min_ms <= self.p50_ms
            && self.p50_ms <= self.p95_ms
            && self.p95_ms <= self.p99_ms
            && self.p99_ms <= self.max_ms)
        {
            return Err("datasaurus guard: quantiles must satisfy min <= p50 <= p95 <= p99 <= max");
        }
        Ok(())
    }

    /// Formats the `SPEC-0009` telemetry header strip:
    /// `[✓ P50 ..ms] [◈ P95 ..ms] [▲ P99 ..ms] [SLO CAP ..ms] [■■■■■■■■■□ ..% HIT]`
    pub fn format_header_strip(&self, hit_rate_pct: u8) -> String {
        let clamped_pct = hit_rate_pct.min(100);
        let filled = (clamped_pct / 10) as usize;
        let empty = 10usize.saturating_sub(filled);
        let bar = format!("{}{}", "■".repeat(filled), "□".repeat(empty));
        format!(
            "[✓ P50 {}ms] [◈ P95 {}ms] [▲ P99 {}ms] [SLO CAP {}ms] [{} {}% HIT]",
            self.p50_ms, self.p95_ms, self.p99_ms, self.slo_cap_ms, bar, clamped_pct
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_posture_glyphs_map_to_redundant_tokens() {
        assert_eq!(GatePostureGlyph::Green.token(), "✓ GREEN");
        assert_eq!(GatePostureGlyph::Anchored.token(), "◈ ANCHORED");
        assert_eq!(GatePostureGlyph::Shadow.token(), "◎ SHADOW");
        assert_eq!(GatePostureGlyph::Red.token(), "▲ RED");
        assert_eq!(GatePostureGlyph::Never.token(), "封 NEVER");

        assert_eq!(GatePostureGlyph::from_outcome("green"), Some(GatePostureGlyph::Green));
        assert_eq!(GatePostureGlyph::from_outcome("ANCHORED"), Some(GatePostureGlyph::Anchored));
        assert_eq!(GatePostureGlyph::from_outcome("Shadow"), Some(GatePostureGlyph::Shadow));
        assert_eq!(GatePostureGlyph::from_outcome("red"), Some(GatePostureGlyph::Red));
        assert_eq!(GatePostureGlyph::from_outcome("never"), Some(GatePostureGlyph::Never));
        assert_eq!(GatePostureGlyph::from_outcome("unknown"), None);
    }

    #[test]
    fn baobab_ribbon_stroke_tiers_cover_boundaries() {
        assert_eq!(baobab_ribbon_stroke_tier(0), "━");
        assert_eq!(baobab_ribbon_stroke_tier(1_999), "━");
        assert_eq!(baobab_ribbon_stroke_tier(2_000), "═");
        assert_eq!(baobab_ribbon_stroke_tier(5_999), "═");
        assert_eq!(baobab_ribbon_stroke_tier(6_000), "█");
        assert_eq!(baobab_ribbon_stroke_tier(10_000), "█");
    }

    #[test]
    fn datasaurus_guard_accepts_valid_quantiles_and_formats_header_strip() {
        let valid = AttemptDurationQuantiles {
            sample_count: 42,
            min_ms: 12,
            p50_ms: 45,
            p95_ms: 120,
            p99_ms: 240,
            max_ms: 310,
            mean_ms: 68,
            slo_cap_ms: 250,
        };
        assert_eq!(valid.validate_datasaurus_guard(), Ok(()));
        assert_eq!(
            valid.format_header_strip(94),
            "[✓ P50 45ms] [◈ P95 120ms] [▲ P99 240ms] [SLO CAP 250ms] [■■■■■■■■■□ 94% HIT]"
        );
    }

    #[test]
    fn datasaurus_guard_negative_controls_reject_low_samples_bare_means_and_inversions() {
        let too_few_samples = AttemptDurationQuantiles {
            sample_count: 9,
            min_ms: 10,
            p50_ms: 20,
            p95_ms: 30,
            p99_ms: 40,
            max_ms: 50,
            mean_ms: 25,
            slo_cap_ms: 100,
        };
        assert!(too_few_samples.validate_datasaurus_guard().is_err());

        let bare_mean = AttemptDurationQuantiles {
            sample_count: 25,
            min_ms: 0,
            p50_ms: 0,
            p95_ms: 0,
            p99_ms: 0,
            max_ms: 0,
            mean_ms: 85,
            slo_cap_ms: 100,
        };
        assert!(bare_mean.validate_datasaurus_guard().is_err());

        let inverted_quantiles = AttemptDurationQuantiles {
            sample_count: 25,
            min_ms: 10,
            p50_ms: 90,
            p95_ms: 60,
            p99_ms: 120,
            max_ms: 150,
            mean_ms: 70,
            slo_cap_ms: 200,
        };
        assert!(inverted_quantiles.validate_datasaurus_guard().is_err());
    }
}
