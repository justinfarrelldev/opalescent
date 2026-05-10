#![cfg(feature = "integration")]

use opalescent::build_system::targets::TargetTriple;
use opalescent::compiler::{
    CompileError, CompileRunPolicy, compile_program_with_run_policy,
    compile_project_with_run_policy, compile_to_module, emit_object_file, link_object_file_with_policy,
};
use opalescent::errors::reporter::CompilerError;
use opalescent::type_system::errors::TypeError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn prepare_dir(path: &Path) -> Result<PathBuf, std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::create_dir_all(path)?;
    Ok(path.to_path_buf())
}

fn cleanup_dir(path: &Path) -> Result<(), std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn compile_program_for_tests(
    source_path: &Path,
    source: &str,
    output_dir: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    compile_program_with_run_policy(
        source_path,
        source,
        output_dir,
        target,
        CompileRunPolicy::bounded_for_test_harness(),
    )
}

fn compile_project_for_tests(
    project_dir: &Path,
    output_dir: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    compile_project_with_run_policy(
        project_dir,
        output_dir,
        target,
        CompileRunPolicy::bounded_for_test_harness(),
    )
}

fn link_object_file_for_tests(
    object_path: &Path,
    output_path: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    link_object_file_with_policy(
        object_path,
        output_path,
        target,
        CompileRunPolicy::bounded_for_test_harness(),
    )
}

fn run_command_output_with_timeout(
    command: &mut Command,
    timeout: Duration,
    context: &str,
) -> Result<Output, String> {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{context} should execute: {error}"))?;

    tests::fs_helpers::wait_for_child_output_with_timeout(child, timeout, context)
}

fn run_binary_output_with_timeout(
    binary_path: impl AsRef<Path>,
    timeout: Duration,
    context: &str,
) -> Result<Output, String> {
    let mut command = Command::new(binary_path.as_ref());
    run_command_output_with_timeout(&mut command, timeout, context)
}

fn run_binary_in_dir_output_with_timeout(
    binary_path: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
    timeout: Duration,
    context: &str,
) -> Result<Output, String> {
    let mut command = Command::new(binary_path.as_ref());
    command.current_dir(cwd.as_ref());
    run_command_output_with_timeout(&mut command, timeout, context)
}

#[cfg(test)]
#[path = "integration_e2e/tests.rs"]
mod tests;
