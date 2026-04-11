//! Coverage map and report computation for Opalescent tests.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::convert::TryFrom;

/// Mapping of source line number to execution marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageMap {
    /// Per-line execution state.
    pub line_hits: BTreeMap<u32, bool>,
}

impl CoverageMap {
    /// Create an empty coverage map.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            line_hits: BTreeMap::new(),
        }
    }

    /// Record one line as executed.
    pub fn mark_executed(&mut self, line: u32) {
        self.line_hits.insert(line, true);
    }

    /// Record one line as not executed.
    pub fn mark_unexecuted(&mut self, line: u32) {
        self.line_hits.insert(line, false);
    }
}

impl Default for CoverageMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate coverage summary.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageReport {
    /// Total tracked lines.
    pub total_lines: usize,
    /// Lines executed at least once.
    pub executed_lines: usize,
    /// Coverage percentage in range `0.0..=100.0`.
    pub coverage_percent: f64,
}

/// Generate a coverage report from `CoverageMap`.
#[expect(
    clippy::float_arithmetic,
    reason = "coverage percentages are represented as floating-point values"
)]
#[must_use]
pub fn generate_coverage_report(coverage_map: &CoverageMap) -> CoverageReport {
    let total_lines = coverage_map.line_hits.len();
    let mut executed_lines = 0_usize;
    for was_executed in coverage_map.line_hits.values() {
        if *was_executed {
            executed_lines = executed_lines.saturating_add(1);
        }
    }

    let coverage_percent = if total_lines == 0 {
        0.0_f64
    } else {
        let executed = usize_to_f64(executed_lines);
        let total = usize_to_f64(total_lines);
        (executed / total) * 100.0_f64
    };

    CoverageReport {
        total_lines,
        executed_lines,
        coverage_percent,
    }
}

/// Convert `usize` into `f64` using checked conversion.
fn usize_to_f64(value: usize) -> f64 {
    match u64::try_from(value) {
        Ok(converted) => u64_to_f64(converted),
        Err(_error) => f64::from(u32::MAX),
    }
}

/// Convert `u64` into `f64` without `as` casting.
#[expect(
    clippy::float_arithmetic,
    reason = "u64-to-f64 conversion combines high and low chunks with float math"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "conversion formula prioritizes readability over fused operations"
)]
fn u64_to_f64(value: u64) -> f64 {
    let low_mask = u64::from(u32::MAX);
    let low_bits_u64 = value & low_mask;
    let high_bits_u64 = value >> 32_u32;

    let low_bits = match u32::try_from(low_bits_u64) {
        Ok(converted) => converted,
        Err(_error) => u32::MAX,
    };
    let high_bits = match u32::try_from(high_bits_u64) {
        Ok(converted) => converted,
        Err(_error) => u32::MAX,
    };

    f64::from(high_bits) * 4_294_967_296.0 + f64::from(low_bits)
}
