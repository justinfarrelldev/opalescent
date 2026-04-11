extern crate alloc;

use crate::hot_reload::abi::{
    generate_abi_signature, ExportedFunction, FunctionSignature, ModuleVTable,
};
use crate::hot_reload::loader::{
    hot_swap_module, HostProcess, HotReloadError, LoadedModule, ModuleLoader,
};
use crate::testing::bench::summarize_samples;
use crate::testing::BenchmarkResult;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::convert::TryFrom;
use std::time::Instant;

/// Aggregated benchmark suite state.
#[derive(Debug, Default, Clone)]
pub struct BenchmarkSuite {
    /// Collected benchmark results.
    results: Vec<BenchmarkResult>,
}

/// Report generated from a [`BenchmarkSuite`].
#[derive(Debug, Clone)]
pub struct SuiteReport {
    /// Results in insertion order.
    pub results: Vec<BenchmarkResult>,
    /// Results indexed by benchmark name.
    pub by_name: BTreeMap<String, BenchmarkResult>,
}

impl BenchmarkSuite {
    /// Creates an empty benchmark suite.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Adds a benchmark result to the suite.
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Returns all currently collected results.
    #[must_use]
    pub fn results(&self) -> &[BenchmarkResult] {
        self.results.as_slice()
    }

    /// Builds a report for current suite results.
    #[must_use]
    pub fn report(&self) -> SuiteReport {
        let mut by_name = BTreeMap::new();

        for result in &self.results {
            by_name.insert(result.name.clone(), result.clone());
        }

        SuiteReport {
            results: self.results.clone(),
            by_name,
        }
    }

    /// Measures mock hot-reload ABI swap latency.
    #[must_use]
    pub fn measure_hot_reload_swap() -> BenchmarkResult {
        measure_iterations("hot_reload_swap", 20, || {
            let mut loader = MockModuleLoader::new();
            let mut host = HostProcess::new();

            consume(hot_swap_module(&mut host, &mut loader, "logic_v0001.so"));
            consume(hot_swap_module(&mut host, &mut loader, "logic_v0002.so"));
        })
    }
}

/// Executes an operation repeatedly and summarizes timing statistics.
#[must_use]
pub fn measure_iterations<F>(name: &str, iterations: usize, mut operation: F) -> BenchmarkResult
where
    F: FnMut(),
{
    if iterations == 0 {
        return BenchmarkResult {
            name: name.to_owned(),
            mean_ns: 0,
            stddev_ns: 0,
            iterations,
        };
    }

    let mut samples = Vec::with_capacity(iterations);
    let mut count = 0_usize;
    while count < iterations {
        let start = Instant::now();
        operation();
        let elapsed = elapsed_nanos_u64(start);
        samples.push(elapsed);
        count = count.saturating_add(1);
    }

    let (mean_ns, stddev_ns) = summarize_samples(samples.as_slice());

    BenchmarkResult {
        name: name.to_owned(),
        mean_ns,
        stddev_ns,
        iterations,
    }
}

/// Converts elapsed duration to bounded nanoseconds.
#[must_use]
fn elapsed_nanos_u64(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Minimal in-memory module loader used by hot-reload benchmark timing.
#[derive(Debug, Clone)]
struct MockModuleLoader {
    /// Preloaded modules by versioned module name.
    modules: BTreeMap<String, LoadedModule>,
}

impl MockModuleLoader {
    /// Builds loader with two ABI-compatible modules.
    #[must_use]
    fn new() -> Self {
        let mut modules = BTreeMap::new();

        let exported = vec![ExportedFunction {
            name: String::from("compute"),
            signature: FunctionSignature {
                parameters: vec![String::from("int32")],
                return_types: vec![String::from("int32")],
            },
        }];
        let signature = generate_abi_signature(exported.as_slice(), &BTreeMap::new());

        modules.insert(
            String::from("logic_v0001.so"),
            LoadedModule {
                module_name: String::from("logic_v0001.so"),
                vtable: ModuleVTable {
                    module_entry: noop_entry,
                },
                abi_signature: signature.clone(),
            },
        );
        modules.insert(
            String::from("logic_v0002.so"),
            LoadedModule {
                module_name: String::from("logic_v0002.so"),
                vtable: ModuleVTable {
                    module_entry: noop_entry,
                },
                abi_signature: signature,
            },
        );

        Self { modules }
    }
}

impl ModuleLoader for MockModuleLoader {
    fn load_module(&mut self, module_name: &str) -> Result<LoadedModule, HotReloadError> {
        self.modules
            .get(module_name)
            .cloned()
            .ok_or_else(|| HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module not found in benchmark loader"),
            })
    }

    fn unload_module(&mut self, module_name: &str) -> Result<(), HotReloadError> {
        if self.modules.contains_key(module_name) {
            Ok(())
        } else {
            Err(HotReloadError::ModuleUnloadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module not found during benchmark unload"),
            })
        }
    }
}

/// No-op module entry used for mock vtable values.
const extern "C" fn noop_entry() {}

/// Consumes a value to make benchmark side effects explicit.
fn consume<T>(value: T) {
    drop(value);
}
