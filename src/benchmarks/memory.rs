use crate::testing::BenchmarkResult;

/// Memory-focused measurements associated with a benchmark run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMetrics {
    /// Estimated peak bytes touched during benchmark execution.
    pub peak_bytes: usize,
    /// Estimated allocation events performed by benchmark execution.
    pub allocations: usize,
}

/// Benchmark result annotated with memory metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasuredBenchmark {
    /// Timing statistics for the benchmark.
    pub result: BenchmarkResult,
    /// Estimated memory metrics for the benchmark.
    pub memory: MemoryMetrics,
}

impl MeasuredBenchmark {
    /// Creates a measured benchmark payload from timing and memory data.
    #[must_use]
    pub const fn new(result: BenchmarkResult, memory: MemoryMetrics) -> Self {
        Self { result, memory }
    }
}

/// Creates a memory estimate from byte and allocation counters.
#[must_use]
pub const fn estimate_memory_metrics(peak_bytes: usize, allocations: usize) -> MemoryMetrics {
    MemoryMetrics {
        peak_bytes,
        allocations,
    }
}
