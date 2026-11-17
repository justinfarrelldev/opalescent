//! Build-once test-only native executables, reused by fresh attempt processes.
extern crate alloc;
use super::checking::CheckedNativeProject;
use super::identity::CaseIdentity;
use super::project::NativeTestProject;
use crate::codegen::context::CodegenContext;
use alloc::fmt::Write;
use inkwell::context::Context;
use inkwell::targets::FileType;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Test runtime/descriptor/worker protocol version, separate from production ABI.
pub const NATIVE_TEST_ABI_VERSION: u32 = 1;

/// A reusable executable artifact; this is never a production cache entry.
#[derive(Debug)]
pub struct NativeTestArtifact {
    /// Absolute native worker executable path.
    pub executable: PathBuf,
    /// Stable selected identities; listing never runs setup or worker code.
    pub cases: Vec<CaseIdentity>,
}

/// Compilation/infrastructure failure, distinct from an executed test failure.
#[derive(Debug, Error)]
pub enum NativeArtifactError {
    /// Native source/LLVM/static consistency failure.
    #[error("native test compilation failed: {0}")]
    Compilation(String),
    /// Artifact materialization or compiler invocation failure.
    #[error("native test artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// Compiler/linker diagnostics are retained, not interpreted as test failures.
    #[error("native test artifact link failed: {0}")]
    Link(String),
}

/// Build a complete checked suite into one native executable in a private directory.
///
/// The caller owns the private build-product directory. The higher-level
/// `artifact_cache` publishes and verifies persistent test-only entries; this
/// low-level builder never reads or overwrites another build's products.
///
/// # Errors
/// Returns compilation or infrastructure errors; never runs application/setup code.
pub fn build_native_artifact(
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
    directory: &Path,
) -> Result<NativeTestArtifact, NativeArtifactError> {
    if checked.suite.cases.is_empty() {
        return Err(NativeArtifactError::Compilation(
            "no selected native cases".to_owned(),
        ));
    }
    std::fs::create_dir_all(directory)?;
    let directory = std::fs::canonicalize(directory)?;
    // Refuse directory reuse rather than overwrite an artifact another worker uses.
    let _build_claim = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("native-build.lock"))?;
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "opalescent_native_tests");
    let compiled = crate::codegen::native_project::emit_checked_project(&codegen, project, checked)
        .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    codegen
        .module
        .verify()
        .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    let object = directory.join("native-tests.o");
    let machine = codegen.target_machine.as_ref().ok_or_else(|| {
        NativeArtifactError::Compilation("native target machine is unavailable".to_owned())
    })?;
    machine
        .write_to_file(&codegen.module, FileType::Object, &object)
        .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    let runtime = crate::compiler::RuntimeTempFile::create()
        .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    let mut inputs = vec![object];
    for &(name, contents) in TEST_SOURCES {
        let path = runtime.include_dir().join(name);
        std::fs::write(&path, contents)?;
        if Path::new(name)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("c"))
        {
            inputs.push(path);
        }
    }
    let registry = directory.join("native-registry.c");
    std::fs::write(&registry, registry_source(&compiled)?)?;
    inputs.push(registry);
    let executable = directory.join(if cfg!(windows) {
        "native-tests.exe"
    } else {
        "native-tests"
    });
    let staged = directory.join(if cfg!(windows) {
        "native-tests.pending.exe"
    } else {
        "native-tests.pending"
    });
    let output = crate::compiler::build_linker_command(
        &codegen.target,
        &inputs,
        runtime.path(),
        &staged,
        runtime.include_dir(),
    )
    .output()?;
    if !output.status.success() {
        return Err(NativeArtifactError::Link(format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    std::fs::rename(staged, &executable)?;
    Ok(NativeTestArtifact {
        executable,
        cases: compiled
            .cases
            .iter()
            .map(|case| case.identity.clone())
            .collect(),
    })
}

/// Static registry strings encode complete identities, never example positions.
fn registry_source(
    compiled: &crate::codegen::native_project::CompiledNativeProject<'_>,
) -> Result<String, NativeArtifactError> {
    let mut source = "#include \"opal_testing_worker.h\"\n".to_owned();
    for case in &compiled.cases {
        writeln!(
            source,
            "extern const OpalTestCaseDescriptor {};",
            case.symbol
        )
        .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    }
    source.push_str("const OpalTestCompiledCase opal_test_compiled_cases[] = {\n");
    for case in &compiled.cases {
        let identity = serde_json::to_string(&case.identity)
            .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
        // JSON escaping of the serialized JSON identity is also a safe C string
        // literal: the identity's own control characters are already escaped.
        let literal = serde_json::to_string(&identity)
            .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
        writeln!(source, "{{{literal}, &{}}},", case.symbol)
            .map_err(|error| NativeArtifactError::Compilation(error.to_string()))?;
    }
    source.push_str("};\nconst size_t opal_test_compiled_case_count = sizeof(opal_test_compiled_cases)/sizeof(opal_test_compiled_cases[0]);\n");
    Ok(source)
}

/// Embedded test-only units are linked in addition to the unchanged application runtime.
const TEST_SOURCES: &[(&str, &str)] = &[
    (
        "opal_testing_runtime.h",
        include_str!("../../../runtime/opal_testing_runtime.h"),
    ),
    (
        "opal_testing_runtime.c",
        include_str!("../../../runtime/opal_testing_runtime.c"),
    ),
    (
        "opal_testing_lifecycle.h",
        include_str!("../../../runtime/opal_testing_lifecycle.h"),
    ),
    (
        "opal_testing_lifecycle.c",
        include_str!("../../../runtime/opal_testing_lifecycle.c"),
    ),
    (
        "opal_testing_values.h",
        include_str!("../../../runtime/opal_testing_values.h"),
    ),
    (
        "opal_testing_values.c",
        include_str!("../../../runtime/opal_testing_values.c"),
    ),
    (
        "opal_testing_doubles.h",
        include_str!("../../../runtime/opal_testing_doubles.h"),
    ),
    (
        "opal_testing_doubles.c",
        include_str!("../../../runtime/opal_testing_doubles.c"),
    ),
    (
        "opal_testing_polling.h",
        include_str!("../../../runtime/opal_testing_polling.h"),
    ),
    (
        "opal_testing_polling.c",
        include_str!("../../../runtime/opal_testing_polling.c"),
    ),
    (
        "opal_testing_matchers.h",
        include_str!("../../../runtime/opal_testing_matchers.h"),
    ),
    (
        "opal_testing_matchers.c",
        include_str!("../../../runtime/opal_testing_matchers.c"),
    ),
    (
        "opal_testing_worker.h",
        include_str!("../../../runtime/opal_testing_worker.h"),
    ),
    (
        "opal_testing_worker.c",
        include_str!("../../../runtime/opal_testing_worker.c"),
    ),
];
