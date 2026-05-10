#![cfg(feature = "integration")]

use opalescent::build_system::targets::TargetTriple;
use opalescent::compiler::{CompileRunPolicy, compile_program_with_run_policy};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const INTERACTIVE_TEST_TIMEOUT: Duration = Duration::from_secs(15);

fn run_binary_with_timeout(binary_path: &Path, context: &str) -> Result<std::process::Output, String> {
    let child = Command::new(binary_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{context} should execute: {error}"))?;

    opalescent::bounded_proc::wait_for_child_output_with_timeout(child, GENERATED_BINARY_TEST_TIMEOUT, context)
        .map_err(|error| error.to_string())
}
fn prepare_dir(path: &Path) -> Result<PathBuf, std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
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

fn unique_test_target_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-integration-print-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn compile_program_for_tests(
    source_path: &Path,
    source: &str,
    output_dir: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, opalescent::compiler::CompileError> {
    compile_program_with_run_policy(
        source_path,
        source,
        output_dir,
        target,
        CompileRunPolicy::bounded_for_test_harness(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_types_compiles_links_and_runs() {
        let temp_dir = unique_test_target_dir("print-types");
        let prepare = prepare_dir(&temp_dir);
        assert!(
            prepare.is_ok(),
            "print-types target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/print-types/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "print-types source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program_for_tests(
                source_path,
                source_str.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            );
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "print-types source should compile and link into a binary: {error}"
                    ));
                }
            };

            let run_output = run_binary_with_timeout(&binary_path, "print-types compiled binary")?;

            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("42") {
                return Err(format!(
                    "print-types stdout should contain '42' (integer print), got: '{stdout}'"
                ));
            }
            if !stdout.contains("true") {
                return Err(format!(
                    "print-types stdout should contain 'true' (bool print), got: '{stdout}'"
                ));
            }
            if !stdout.contains("false") {
                return Err(format!(
                    "print-types stdout should contain 'false' (bool print), got: '{stdout}'"
                ));
            }
            if !stdout.contains("hello") {
                return Err(format!(
                    "print-types stdout should contain 'hello' (string print regression), got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "print-types binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(&temp_dir);
        assert!(
            cleanup.is_ok(),
            "print-types target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "print-types end-to-end flow should compile, link, run, print all types, and exit cleanly: {failure_message}"
        );
    }

    #[test]
    fn should_print_final_result_outputs_sum() {
        let temp_dir = unique_test_target_dir("should-print-final-result");
        let prepare = prepare_dir(&temp_dir);
        assert!(
            prepare.is_ok(),
            "should-print-final-result target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/should-print-final-result/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "should-print-final-result source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program_for_tests(
                source_path,
                source_str.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            );
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "should-print-final-result source should compile and link into a binary: {error}"
                    ));
                }
            };

            let child_result = Command::new(&binary_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let mut child = match child_result {
                Ok(child_process) => child_process,
                Err(error) => {
                    return Err(format!(
                        "should-print-final-result compiled binary should spawn with piped stdio: {error}"
                    ));
                }
            };

            if let Some(ref mut stdin) = child.stdin {
                let write_result = stdin.write_all(b"3\n4\n");
                if let Err(error) = write_result {
                    return Err(format!(
                        "should-print-final-result stdin should accept scripted input: {error}"
                    ));
                }
            } else {
                return Err(
                    "should-print-final-result process stdin should be piped so test input can be written"
                        .to_owned(),
                );
            }

            let run_output = opalescent::bounded_proc::wait_for_child_output_with_timeout(
                child,
                INTERACTIVE_TEST_TIMEOUT,
                "should-print-final-result compiled binary",
            )
            .map_err(|error| error.to_string())?;

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains('7') {
                return Err(format!(
                    "should-print-final-result stdout should contain '7' (sum of 3+4), got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "should-print-final-result binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(&temp_dir);
        assert!(
            cleanup.is_ok(),
            "should-print-final-result target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "should-print-final-result end-to-end flow should compile, link, run, print sum, and exit cleanly: {failure_message}"
        );
    }
}
