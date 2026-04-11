use crate::testing::BenchmarkResult;
use alloc::string::String;

extern crate alloc;

/// Regression acceptance threshold for one benchmark scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionThreshold {
    /// Benchmark name the threshold applies to.
    pub name: String,
    /// Maximum allowed benchmark mean in nanoseconds.
    pub max_mean_ns: u64,
    /// Maximum allowed benchmark standard deviation in nanoseconds.
    pub max_stddev_ns: u64,
}

/// Regression violation details describing threshold failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionViolation {
    /// Benchmark name that violated thresholds.
    pub name: String,
    /// Measured mean in nanoseconds.
    pub mean_ns: u64,
    /// Measured standard deviation in nanoseconds.
    pub stddev_ns: u64,
    /// Threshold maximum mean in nanoseconds.
    pub max_mean_ns: u64,
    /// Threshold maximum standard deviation in nanoseconds.
    pub max_stddev_ns: u64,
}

/// Regression check result type alias.
pub type RegressionResult = Result<(), RegressionViolation>;

/// Evaluates a benchmark result against a regression threshold.
///
/// # Errors
///
/// Returns [`RegressionViolation`] when `result` exceeds `threshold` limits.
pub fn check_regression(
    result: &BenchmarkResult,
    threshold: &RegressionThreshold,
) -> RegressionResult {
    let mean_exceeded = result.mean_ns > threshold.max_mean_ns;
    let stddev_exceeded = result.stddev_ns > threshold.max_stddev_ns;

    if !mean_exceeded && !stddev_exceeded {
        return Ok(());
    }

    Err(RegressionViolation {
        name: result.name.clone(),
        mean_ns: result.mean_ns,
        stddev_ns: result.stddev_ns,
        max_mean_ns: threshold.max_mean_ns,
        max_stddev_ns: threshold.max_stddev_ns,
    })
}
