//! Test-only content-addressed artifacts; publication never overwrites running workers.
use super::{
    artifact::{
        NATIVE_TEST_ABI_VERSION, NativeArtifactError, NativeTestArtifact, build_native_artifact,
    },
    checking::CheckedNativeProject,
    identity::CaseIdentity,
    project::NativeTestProject,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// On-disk manifest/key schema, independent of the worker ABI.
const SCHEMA: u32 = 1;
#[cfg(test)]
mod key_tests;
/// Domain separation prevents production artifacts from being mistaken for tests.
const DOMAIN: &str = "opalescent-instrumented-native-test-artifact";
/// Actual profile currently used by the native test backend.
const PROFILE: &str = "native-default";
/// Explicit mandatory instrumentation features, in addition to binary identity.
const CONFIGURATION: &str = "test-outcomes;source-edge-interception;fresh-worker-protocol";
/// Platform-specific worker filename, never supplied by cache contents.
const EXECUTABLE: &str = if cfg!(windows) {
    "native-tests.exe"
} else {
    "native-tests"
};

/// Every field participates in the key; source bytes include interception declarations.
#[derive(Serialize)]
struct Inputs {
    /// Test-only cache namespace.
    domain: &'static str,
    /// Key/manifest layout version.
    schema: u32,
    /// Human compiler release version.
    compiler_version: &'static str,
    /// Exact running compiler image, including embedded runtime sources/features.
    compiler_binary: String,
    /// Actual backend target triple.
    target: String,
    /// Actual backend profile.
    profile: &'static str,
    /// Test callback/descriptor/transport ABI.
    runtime_abi: u32,
    /// Required test instrumentation features.
    testing_configuration: &'static str,
    /// Canonical source root affects paths embedded in diagnostics.
    source_root: PathBuf,
    /// Selected native linker/compiler program.
    linker_program: OsString,
    /// Native linker arguments with stable placeholder build paths.
    linker_arguments: Vec<OsString>,
    /// Compiler/linker version probe bytes.
    linker_version: Vec<u8>,
    /// Relevant toolchain and header/library search environment.
    tool_environment: Vec<(OsString, Option<OsString>)>,
    /// Canonical identities and exact original bytes, including double declarations.
    modules: Vec<(PathBuf, String, String)>,
    /// Complete independent case identities compiled into the registry.
    cases: Vec<CaseIdentity>,
}

/// Integrity-checked publication record written only after a successful link.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Test-only namespace.
    domain: String,
    /// Supported on-disk schema.
    schema: u32,
    /// Digest of all compilation inputs.
    key: String,
    /// Digest of the linked worker's exact bytes.
    executable_sha256: String,
    /// Independent registry identities expected by the coordinator.
    cases: Vec<CaseIdentity>,
}

/// Build on a miss; verify manifest, case identities and executable bytes on every hit.
///
/// Failed builds remain private. Concurrent builders may duplicate work, but only
/// complete immutable directories become visible; no cache artifact is overwritten.
///
/// # Errors
/// Returns build/I/O errors or rejects corrupt/foreign/incompatible cache entries.
pub fn cached_native_artifact(
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
    cache: &Path,
) -> Result<NativeTestArtifact, NativeArtifactError> {
    std::fs::create_dir_all(cache)?;
    let cache = std::fs::canonicalize(cache)?;
    let inputs = inputs(project, checked)?;
    let key = key(&inputs)?;
    let entry = cache.join(&key);
    if entry.try_exists()? {
        return load(&entry, &key, &inputs.cases);
    }
    let staging = tempfile::Builder::new()
        .prefix(".native-test-build-")
        .tempdir_in(&cache)?;
    let artifact = build_native_artifact(project, checked, staging.path())?;
    let manifest = Manifest {
        domain: DOMAIN.to_owned(),
        schema: SCHEMA,
        key: key.clone(),
        executable_sha256: hash_file(&artifact.executable)?,
        cases: artifact.cases,
    };
    let data = serde_json::to_vec(&manifest).map_err(invalid)?;
    std::fs::write(staging.path().join("native-test-manifest.json"), data)?;
    match std::fs::rename(staging.path(), &entry) {
        Ok(()) => load(&entry, &key, &inputs.cases),
        Err(_) if entry.try_exists()? => load(&entry, &key, &inputs.cases),
        Err(error) => Err(error.into()),
    }
}

/// Cache keys encode the actual fixed native profile, not a requested but ignored one.
fn inputs(
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
) -> Result<Inputs, NativeArtifactError> {
    let target = crate::build_system::targets::TargetTriple::host();
    let command = crate::compiler::build_linker_command(
        &target,
        &[],
        Path::new("runtime.c"),
        Path::new("binary"),
        Path::new("include"),
    );
    let program = command.get_program().to_owned();
    let version = std::process::Command::new(&program)
        .arg(if program == "cl.exe" {
            "/?"
        } else {
            "--version"
        })
        .output()?;
    let mut linker_version = version.stdout;
    linker_version.extend(version.stderr);
    Ok(Inputs {
        domain: DOMAIN,
        schema: SCHEMA,
        compiler_version: env!("CARGO_PKG_VERSION"),
        compiler_binary: compiler_identity()?,
        target: target.to_llvm_string(),
        profile: PROFILE,
        runtime_abi: NATIVE_TEST_ABI_VERSION,
        testing_configuration: CONFIGURATION,
        source_root: project.root.clone(),
        linker_program: program,
        linker_arguments: command.get_args().map(std::ffi::OsStr::to_owned).collect(),
        linker_version,
        tool_environment: [
            "PATH",
            "CC",
            "CFLAGS",
            "CPPFLAGS",
            "LDFLAGS",
            "CPATH",
            "C_INCLUDE_PATH",
            "LIBRARY_PATH",
            "SDKROOT",
            "MACOSX_DEPLOYMENT_TARGET",
            "INCLUDE",
            "LIB",
            "LIBPATH",
            "OPAL_XWIN_SYSROOT",
            "XWIN_CACHE",
        ]
        .into_iter()
        .map(|name| (OsString::from(name), std::env::var_os(name)))
        .collect(),
        modules: project
            .modules
            .values()
            .map(|module| {
                (
                    module.path.clone(),
                    module.identity.clone(),
                    module.source.clone(),
                )
            })
            .collect(),
        cases: checked
            .suite
            .cases
            .iter()
            .map(|case| case.identity.clone())
            .collect(),
    })
}

/// Length-delimited canonical serialization prevents ambiguous input concatenation.
fn key(inputs: &Inputs) -> Result<String, NativeArtifactError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(inputs).map_err(invalid)?)
    ))
}

/// Hash the running image, including embedded runtimes and enabled compiler features.
fn compiler_identity() -> Result<String, NativeArtifactError> {
    static IDENTITY: OnceLock<String> = OnceLock::new();
    if let Some(value) = IDENTITY.get() {
        return Ok(value.clone());
    }
    #[cfg(target_os = "linux")]
    let executable = PathBuf::from("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let executable = std::env::current_exe()?;
    let digest = hash_file(&executable)?;
    Ok(IDENTITY.get_or_init(|| digest).clone())
}

/// Stream executable bytes without loading entire compiler binaries into memory.
fn hash_file(path: &Path) -> Result<String, NativeArtifactError> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 0x4000];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Validate published metadata and product integrity before returning a worker.
fn load(
    entry: &Path,
    key: &str,
    cases: &[CaseIdentity],
) -> Result<NativeTestArtifact, NativeArtifactError> {
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(entry.join("native-test-manifest.json"))?)
            .map_err(invalid)?;
    let executable = entry.join(EXECUTABLE);
    if manifest.domain != DOMAIN
        || manifest.schema != SCHEMA
        || manifest.key != key
        || manifest.cases != cases
        || !std::fs::metadata(&executable)?.is_file()
        || manifest.executable_sha256 != hash_file(&executable)?
    {
        return Err(invalid(
            "incompatible or corrupt native-test cache entry; remove this entry before rebuilding",
        ));
    }
    Ok(NativeTestArtifact {
        executable,
        cases: manifest.cases,
    })
}

/// Malformed cache state is infrastructure data failure, not a test assertion.
fn invalid(message: impl std::fmt::Display) -> NativeArtifactError {
    NativeArtifactError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.to_string(),
    ))
}
