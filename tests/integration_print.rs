#![cfg(feature = "integration")]

use opalescent::compiler::compile_program;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_types_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/print-types/target");
        let prepare = prepare_dir(temp_dir);
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

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "print-types source should compile and link into a binary: {error}"
                    ));
                }
            };

            let output_result = Command::new(&binary_path).output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "print-types compiled binary should execute: {error}"
                    ));
                }
            };

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

        let cleanup = cleanup_dir(temp_dir);
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
        let temp_dir = Path::new("test-projects/should-print-final-result/target");
        let prepare = prepare_dir(temp_dir);
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

            let binary_result = compile_program(source_str.as_str(), temp_dir);
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

            let output_result = child.wait_with_output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "should-print-final-result compiled binary should complete and produce output: {error}"
                    ));
                }
            };

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

        let cleanup = cleanup_dir(temp_dir);
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
