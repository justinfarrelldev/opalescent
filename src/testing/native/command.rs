//! Actual compiler CLI discovery, selection, native scheduling and report dispatch.
extern crate alloc;
mod options;
mod report;
use super::{
    artifact_cache::cached_native_artifact,
    checking::check_project,
    identity::DeclarationIdentity,
    project::NativeTestProject,
    scheduler::{CaseOutcome, NativeRunOptions, ScheduledCase, run_cases},
};
use alloc::collections::BTreeSet;
use options::{Options, Reporter};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// CLI output is separate from user test stdout/stderr captures.
pub struct NativeCommandOutput {
    /// Shared text/JSON/JUnit rendering.
    pub text: String,
    /// Zero success, one failed/empty selection, two compiler/infrastructure failure.
    pub exit_code: i32,
    /// Optional user-requested report file already written by the coordinator.
    pub written_to: Option<PathBuf>,
}

/// A compiler/argument/infrastructure error is never an assertion failure.
#[derive(Debug, Error)]
#[error("{kind}: {message}")]
pub struct NativeCommandError {
    /// Machine-readable error category.
    kind: &'static str,
    /// Source/compiler/process diagnostic.
    message: String,
}

impl NativeCommandError {
    /// Invalid finite CLI option grammar.
    fn argument(message: impl Into<String>) -> Self {
        Self {
            kind: "argument",
            message: message.into(),
        }
    }
    /// Invalid source or compiler diagnostics.
    fn compilation(message: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self {
            kind: "compilation",
            message: message.into().to_string(),
        }
    }
    /// Coordinator build/transport/reporting failure.
    fn infrastructure(message: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self {
            kind: "infrastructure",
            message: message.into().to_string(),
        }
    }
    /// Formatting/serialization failures are coordinator infrastructure errors.
    fn report(message: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self::infrastructure(message)
    }
}

/// Execute the real native test command without mixing source output into reports.
///
/// # Errors
/// Returns malformed argument or report-file errors; source/build failures are
/// rendered with a machine-distinguishable category and nonzero exit status.
pub fn execute(args: &[String]) -> Result<NativeCommandOutput, NativeCommandError> {
    let options = Options::parse(args)?;
    let (text, exit_code) = match execute_options(&options) {
        Ok(output) => output,
        Err(error) => (error_report(&error, options.reporter)?, 2_i32),
    };
    if let Some(ref path) = options.output {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(NativeCommandError::infrastructure)?;
        }
        std::fs::write(path, &text).map_err(NativeCommandError::infrastructure)?;
    }
    Ok(NativeCommandOutput {
        text,
        exit_code,
        written_to: options.output,
    })
}

/// Compiler CLI adapter; the reusable command logic is directly testable.
pub fn run(args: &[String]) -> Result<(), i32> {
    match execute(args) {
        Ok(output) => {
            if output.written_to.is_none() {
                println!("{}", output.text);
            }
            if output.exit_code == 0 {
                Ok(())
            } else {
                Err(output.exit_code)
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            Err(2)
        }
    }
}

/// Preserve one canonical selected identity and its complete scheduler exclusion set.
fn scheduled_cases(
    checked: &super::checking::CheckedNativeProject,
    selected: &BTreeSet<super::identity::CaseIdentity>,
) -> Vec<ScheduledCase> {
    checked
        .suite
        .cases
        .iter()
        .filter(|case| selected.contains(&case.identity))
        .map(|case| ScheduledCase {
            identity: case.identity.clone(),
            exclusion_keys: checked.suite.bodies[&case.body_identity]
                .exclusion_keys
                .clone(),
        })
        .collect()
}

/// Required E2E subjects use the unchanged normal project compiler exactly once.
fn application_for_cases(
    project: &NativeTestProject,
    checked: &super::checking::CheckedNativeProject,
    selected: &BTreeSet<super::identity::CaseIdentity>,
    output: &Path,
) -> Result<Option<PathBuf>, NativeCommandError> {
    let required = checked.suite.cases.iter().any(|case| {
        selected.contains(&case.identity)
            && checked.suite.bodies[&case.body_identity].kind
                == crate::ast::testing::TestKind::EndToEnd
    });
    if !required {
        return Ok(None);
    }
    crate::compiler::compile_project(
        &project.root,
        output,
        &crate::build_system::targets::TargetTriple::host(),
    )
    .map(Some)
    .map_err(NativeCommandError::compilation)
}

/// Preserve compiler failures separately from cache/link/tool infrastructure failures.
fn cached_worker(
    project: &NativeTestProject,
    checked: &super::checking::CheckedNativeProject,
    root: &Path,
) -> Result<super::artifact::NativeTestArtifact, NativeCommandError> {
    cached_native_artifact(project, checked, &root.join("target/native-testing/cache")).map_err(
        |error| match error {
            super::artifact::NativeArtifactError::Compilation(_) => {
                NativeCommandError::compilation(error)
            }
            _ => NativeCommandError::infrastructure(error),
        },
    )
}

/// Discover/check once, build once, then schedule selected cases independently.
fn execute_options(options: &Options) -> Result<(String, i32), NativeCommandError> {
    let root = std::fs::canonicalize(&options.root).map_err(NativeCommandError::infrastructure)?;
    let (project, checked) = load_project(&root)?;
    let mut selected = BTreeSet::new();
    for case in &checked.suite.cases {
        let body = &checked.suite.bodies[&case.body_identity];
        let id = serde_json::to_string(&case.identity).map_err(NativeCommandError::report)?;
        if options
            .case
            .as_ref()
            .is_some_and(|expected| expected != &id)
            || options
                .filter
                .as_ref()
                .is_some_and(|filter| !report::display(&case.identity).contains(filter))
            || options
                .kind
                .as_ref()
                .is_some_and(|kind| kind != report::kind(body.kind))
            || options.unit.as_ref().is_some_and(|unit| {
                !body
                    .subjects
                    .iter()
                    .any(|subject| matches_identity(&subject.0, unit))
            })
            || options.world.as_ref().is_some_and(|world| {
                !body
                    .world_origins
                    .keys()
                    .any(|identity| matches_identity(&identity.0, world))
            })
        {
            continue;
        }
        selected.insert(case.identity.clone());
    }
    if options.list || selected.is_empty() {
        let code = i32::from(selected.is_empty() && !options.allow_empty);
        return Ok((
            report::render(&checked, &selected, None, options.list, options.reporter)?,
            code,
        ));
    }
    if options.target.as_ref().is_some_and(|target| {
        target.to_llvm_string()
            != crate::build_system::targets::TargetTriple::host().to_llvm_string()
    }) {
        return Err(NativeCommandError::compilation(
            "native worker execution currently requires the host target",
        ));
    }
    let directory = run_directory(&root)?;
    let artifact = cached_worker(&project, &checked, &root)?;
    let cases = scheduled_cases(&checked, &selected);
    let controls = NativeRunOptions {
        jobs: options.jobs,
        retries: options.retries,
        timeout: options.timeout,
        application_executable: application_for_cases(
            &project,
            &checked,
            &selected,
            &directory.join("application"),
        )?,
        working_directory: root,
        output_directory: directory.join("attempts"),
    };
    let executions =
        run_cases(&artifact, &cases, &controls).map_err(NativeCommandError::infrastructure)?;
    let code = if executions
        .cases
        .iter()
        .any(|case| case.outcome == CaseOutcome::InfrastructureFailed)
    {
        2_i32
    } else {
        i32::from(
            executions
                .cases
                .iter()
                .any(|case| !case.outcome.is_success()),
        )
    };
    Ok((
        report::render(
            &checked,
            &selected,
            Some(&executions),
            false,
            options.reporter,
        )?,
        code,
    ))
}

/// Build and result products are separate from production cache entries.
fn run_directory(root: &Path) -> Result<PathBuf, NativeCommandError> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(NativeCommandError::infrastructure)?
        .as_nanos();
    Ok(root
        .join("target/native-testing")
        .join(format!("run-{}-{stamp}", std::process::id())))
}

/// Validate the actual project manifest and ordinary native source graph once.
fn load_project(
    root: &Path,
) -> Result<(NativeTestProject, super::checking::CheckedNativeProject), NativeCommandError> {
    match std::fs::read_to_string(root.join("opal.toml")) {
        Ok(config) => {
            crate::build_system::config::parse_config(&config)
                .map_err(|error| NativeCommandError::compilation(format!("{error:?}")))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(NativeCommandError::infrastructure(error)),
    }
    let mut files = Vec::new();
    discover(&root.join("tests"), &mut files)?;
    files.sort();
    let project = NativeTestProject::load(root, &files).map_err(NativeCommandError::compilation)?;
    let checked = check_project(&project).map_err(NativeCommandError::compilation)?;
    Ok((project, checked))
}

/// Discovery never executes source, follows no directory symlink loops and uses
/// the initial tests/**/*.test.op convention.
fn discover(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), NativeCommandError> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(NativeCommandError::infrastructure(error)),
    };
    for entry in entries {
        let entry = entry.map_err(NativeCommandError::infrastructure)?;
        let kind = entry
            .file_type()
            .map_err(NativeCommandError::infrastructure)?;
        if kind.is_dir() {
            discover(&entry.path(), files)?;
        } else if entry.path().is_file()
            && entry.file_name().to_str().is_some_and(|name| {
                name.strip_suffix(".op").is_some_and(|stem| {
                    Path::new(stem)
                        .extension()
                        .is_some_and(|extension| extension == "test")
                })
            })
        {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Exact canonical qualification is supported without confusing aliases with IDs.
fn matches_identity(identity: &DeclarationIdentity, selection: &str) -> bool {
    selection == identity.name || selection == format!("{}::{}", identity.module, identity.name)
}

/// Machine-readable compiler/infrastructure failures cannot look like test failures.
fn error_report(
    error: &NativeCommandError,
    reporter: Reporter,
) -> Result<String, NativeCommandError> {
    match reporter {
        Reporter::Json => serde_json::to_string_pretty(&serde_json::json!({"schema_version":1,"coverage":"not_collected","cases":[],"error":{"kind":error.kind,"message":error.message}})).map_err(NativeCommandError::report),
        Reporter::Text => Ok(format!("error: {error}\n")),
        Reporter::Junit => Ok(format!("<?xml version=\"1.0\"?><testsuite name=\"Opalescent native tests\" tests=\"1\" errors=\"1\"><testcase classname=\"framework\" name=\"native compilation or infrastructure\"><error type=\"{}\">{}</error></testcase></testsuite>", error.kind, report::xml(&error.message))),
    }
}
