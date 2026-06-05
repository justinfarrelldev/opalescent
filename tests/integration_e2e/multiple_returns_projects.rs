#![cfg(feature = "integration")]
#![allow(
    clippy::match_same_arms,
    clippy::pattern_type_mismatch,
    clippy::single_char_lifetime_names,
    clippy::uninlined_format_args,
    reason = "integration fixture helpers prioritize readable compile-failure assertions over Clippy-preferred rewrites during blocker cleanup"
)]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn opalescent_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("opalescent")
}

fn fixture_project_dir(project_name: &str) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("current working directory should be readable for integration tests: {error}")
    })?;
    Ok(cwd.join("test-projects").join(project_name))
}

fn read_expected_stdout(project_name: &str) -> Result<String, String> {
    fs::read_to_string(
        fixture_project_dir(project_name)?
            .join("expected")
            .join("stdout.txt"),
    )
    .map_err(|error| format!("{project_name} expected stdout fixture should be readable: {error}"))
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "destination directory {} should be created before copying fixture contents: {error}",
            destination.display()
        )
    })?;

    for entry_result in fs::read_dir(source).map_err(|error| {
        format!(
            "fixture source directory {} should be readable: {error}",
            source.display()
        )
    })? {
        let entry = entry_result.map_err(|error| {
            format!(
                "fixture directory entry under {} should be readable: {error}",
                source.display()
            )
        })?;
        let entry_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "fixture entry type for {} should be readable: {error}",
                entry_path.display()
            )
        })?;

        if file_type.is_dir() {
            copy_dir_recursive(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path).map_err(|error| {
                format!(
                    "fixture file {} should copy into {}: {error}",
                    entry_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn copy_fixture_project(project_name: &str) -> Result<(PathBuf, PathBuf), String> {
    let fixture_dir = fixture_project_dir(project_name)?;
    let temp_dir = unique_probe_target_dir(&format!("multiple-returns-{project_name}"));
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be created: {error}"))?;
    let sandbox_project_dir = temp_dir.join(project_name);
    copy_dir_recursive(&fixture_dir, &sandbox_project_dir)?;
    Ok((temp_dir, sandbox_project_dir))
}

fn cleanup_temp_dir(temp_dir: &Path, project_name: &str) -> Result<(), String> {
    cleanup_dir(temp_dir)
        .map_err(|error| format!("{project_name} target directory should be removed: {error}"))
}

fn run_opal_command(project_dir: &Path, args: &[&str], context: &str) -> Result<Output, String> {
    let mut command = std::process::Command::new(opalescent_binary_path());
    command.current_dir(project_dir);
    for arg in args {
        command.arg(arg);
    }
    run_command_output_with_timeout(&mut command, GENERATED_BINARY_TEST_TIMEOUT, context)
}

fn strip_run_prefix(stdout: &str) -> &str {
    stdout.strip_prefix("target/program\n").unwrap_or(stdout)
}

fn assert_project_check_build_and_run(project_name: &str) -> Result<(), String> {
    let expected_stdout = read_expected_stdout(project_name)?;
    let (temp_dir, project_dir) = copy_fixture_project(project_name)?;

    let execution_result: Result<(), String> = (|| {
        if project_name == "multiple-returns-cross-module" {
            let compile_check_dir = temp_dir.join("project-check");
            prepare_dir(&compile_check_dir).map_err(|error| {
                format!("{project_name} project-check directory should be created: {error}")
            })?;
            compile_project_for_tests(&project_dir, &compile_check_dir, &TargetTriple::host())
                .map_err(|error| {
                    format!(
                        "{project_name} project-aware check should succeed before CLI build/run coverage: {error}"
                    )
                })?;
        } else {
            let check_output = run_opal_command(
                &project_dir,
                &["check", "src/main.op"],
                &format!("{project_name} opal check"),
            )?;
            if !check_output.status.success() {
                return Err(format!(
                    "{project_name} opal check should succeed, stderr: {}",
                    String::from_utf8_lossy(&check_output.stderr)
                ));
            }
        }

        let build_output = run_opal_command(
            &project_dir,
            &["build"],
            &format!("{project_name} opal build"),
        )?;
        if !build_output.status.success() {
            return Err(format!(
                "{project_name} opal build should succeed, stderr: {}",
                String::from_utf8_lossy(&build_output.stderr)
            ));
        }

        let built_binary = project_dir.join("target").join("program");
        if !built_binary.exists() {
            return Err(format!(
                "{project_name} opal build should create {}, but it was missing",
                built_binary.display()
            ));
        }

        let run_output =
            run_opal_command(&project_dir, &["run"], &format!("{project_name} opal run"))?;
        if !run_output.status.success() {
            return Err(format!(
                "{project_name} opal run should succeed, stderr: {}",
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let actual_stdout = strip_run_prefix(stdout.as_ref());
        if actual_stdout != expected_stdout {
            return Err(format!(
                "{project_name} opal run stdout should exactly match fixture output, expected {:?}, got {:?}",
                expected_stdout, actual_stdout
            ));
        }

        Ok(())
    })();

    let cleanup_result = cleanup_temp_dir(&temp_dir, project_name);
    match (execution_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(exec_error), Err(cleanup_error)) => Err(format!("{exec_error}; {cleanup_error}")),
    }
}

fn compile_inline_failure(
    source_path: &Path,
    source: &str,
    label: &str,
) -> Result<CompileError, String> {
    let temp_dir = unique_probe_target_dir(&format!("multiple-returns-inline-{label}"));
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{label} target directory should be created: {error}"))?;

    let compile_result =
        compile_program_for_tests(source_path, source, &temp_dir, &TargetTriple::host());
    let cleanup_result = cleanup_temp_dir(&temp_dir, label);
    match (compile_result, cleanup_result) {
        (Ok(_), Ok(())) => Err(format!(
            "{label} source should fail to compile, but compilation succeeded"
        )),
        (Err(error), Ok(())) => Ok(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(_), Err(error)) => Err(error),
    }
}

fn compile_cross_module_failure(
    sandbox_name: &str,
    replacement_main: &str,
) -> Result<CompileError, String> {
    let fixture_name = "multiple-returns-cross-module";
    let (temp_dir, project_dir) = copy_fixture_project(fixture_name)?;

    let execution_result: Result<CompileError, String> = (|| {
        fs::write(project_dir.join("src").join("main.op"), replacement_main).map_err(|error| {
            format!(
                "{sandbox_name} replacement main.op should be writable in temp fixture: {error}"
            )
        })?;
        compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .err()
            .ok_or_else(|| {
                format!("{sandbox_name} cross-module project should fail to compile, but compilation succeeded")
            })
    })();

    let cleanup_result = cleanup_temp_dir(&temp_dir, sandbox_name);
    match (execution_result, cleanup_result) {
        (Ok(error), Ok(())) => Ok(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(exec_error), Err(cleanup_error)) => Err(format!("{exec_error}; {cleanup_error}")),
    }
}

fn with_report_type_errors<'a>(
    compile_error: &'a CompileError,
    label: &str,
) -> Result<Vec<&'a TypeError>, String> {
    match compile_error {
        CompileError::Report { report, .. } => Ok(report
            .entries()
            .iter()
            .filter_map(|entry| match &entry.1 {
                CompilerError::TypeChecker(type_error) => Some(type_error),
                _ => None,
            })
            .collect()),
        CompileError::Type(type_error) => Ok(vec![type_error]),
        other => Err(format!(
            "{label} should fail with a type-check report, got: {other}"
        )),
    }
}

#[test]
fn multiple_returns_basic_project_checks_builds_and_runs() {
    let result = assert_project_check_build_and_run("multiple-returns-basic");
    assert!(
        result.is_ok(),
        "multiple-returns-basic project should check, build, and run with exact output: {result:?}"
    );
}

#[test]
fn multiple_returns_cross_module_project_checks_builds_and_runs() {
    let result = assert_project_check_build_and_run("multiple-returns-cross-module");
    assert!(
        result.is_ok(),
        "multiple-returns-cross-module project should check, build, and run with exact output: {result:?}"
    );
}

#[test]
fn multiple_returns_fallible_project_checks_builds_and_runs() {
    let result = assert_project_check_build_and_run("multiple-returns-fallible");
    assert!(
        result.is_ok(),
        "multiple-returns-fallible project should check, build, and run with exact output: {result:?}"
    );
}

#[test]
fn multiple_return_missing_signature_labels_fail_with_required_label_diagnostic() {
    let source = "##\n  Description: Intentionally invalid fixture proving multi-return signatures must label every slot.\n##\nlet pair = f(): int32, string =>\n    return x: 1 as int32, y: 'two'\n\n##\n  Description: Entry point for missing multi-return signature label diagnostic coverage.\n##\nentry main = f(args: string[]): void =>\n    return void\n";

    let compile_error = compile_inline_failure(
        Path::new("test-projects/multiple-returns-basic/src/main.op"),
        source,
        "multiple-return-missing-signature-labels",
    )
    .expect("missing signature labels fixture should fail to compile");

    let errors =
        with_report_type_errors(&compile_error, "multiple-return-missing-signature-labels")
            .expect("missing signature labels fixture should surface type errors");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::MissingMultiReturnLabels { .. })),
        "missing signature labels fixture should emit MissingMultiReturnLabels, got: {errors:?}"
    );
}

#[test]
fn multiple_return_cross_module_unlabeled_mismatch_is_rejected() {
    let source = "import pair from './producer'\n\n##\n  Description: Intentionally invalid caller proving imported label metadata rejects unlabeled renaming.\n##\nentry main = f(args: string[]): void =>\n    let left_value, right_value = pair()\n    print('UNEXPECTED={left_value},{right_value}')\n    return void\n";

    let compile_error =
        compile_cross_module_failure("multiple-return-cross-module-unlabeled-mismatch", source)
            .expect("cross-module unlabeled mismatch fixture should fail to compile");
    let errors = with_report_type_errors(
        &compile_error,
        "multiple-return-cross-module-unlabeled-mismatch",
    )
    .expect("cross-module unlabeled mismatch should surface type errors");
    assert!(
        errors.iter().any(|error| match error {
            TypeError::ReturnLabelMismatch { expected, .. } => {
                expected.contains("must exactly match returned labels")
            }
            _ => false,
        }),
        "cross-module unlabeled mismatch should mention exact returned-label matching, got: {errors:?}"
    );
}

#[test]
fn multiple_return_cross_module_reversed_explicit_labels_are_rejected() {
    let source = "import pair from './producer'\n\n##\n  Description: Intentionally invalid caller proving explicit imported labels do not reorder positions.\n##\nentry main = f(args: string[]): void =>\n    let right: rhs, left: lhs = pair()\n    print('UNEXPECTED={lhs},{rhs}')\n    return void\n";

    let compile_error =
        compile_cross_module_failure("multiple-return-cross-module-reversed-labels", source)
            .expect("cross-module reversed explicit labels fixture should fail to compile");
    let errors = with_report_type_errors(
        &compile_error,
        "multiple-return-cross-module-reversed-labels",
    )
    .expect("cross-module reversed explicit labels should surface type errors");
    assert!(
        errors.iter().any(|error| match error {
            TypeError::ReturnLabelMismatch { expected, .. } => {
                expected.contains("do not reorder returned values")
            }
            _ => false,
        }),
        "cross-module reversed explicit labels should mention non-reordering intent, got: {errors:?}"
    );
}

#[test]
fn multiple_return_cross_module_unknown_label_is_rejected() {
    let source = "import pair from './producer'\n\n##\n  Description: Intentionally invalid caller proving imported label metadata rejects unknown explicit labels.\n##\nentry main = f(args: string[]): void =>\n    let missing: lhs, right: rhs = pair()\n    print('UNEXPECTED={lhs},{rhs}')\n    return void\n";

    let compile_error =
        compile_cross_module_failure("multiple-return-cross-module-unknown-label", source)
            .expect("cross-module unknown label fixture should fail to compile");
    let errors =
        with_report_type_errors(&compile_error, "multiple-return-cross-module-unknown-label")
            .expect("cross-module unknown label should surface type errors");
    assert!(
        errors.iter().any(|error| match error {
            TypeError::ReturnLabelMismatch {
                expected, found, ..
            } => expected.contains("expected label 'left'") && found == "missing",
            _ => false,
        }),
        "cross-module unknown label should report the imported expected label and unknown actual label, got: {errors:?}"
    );
}
