#![cfg(feature = "integration")]

use super::*;

#[test]
fn overflow_trap_exits_nonzero() {
    let temp_dir = Path::new("test-projects/overflow-trap/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "overflow-trap target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/overflow-trap/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "overflow-trap source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "overflow-trap source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "overflow-trap compiled binary should execute: {error}"
                ));
            }
        };

        if run_output.status.success() {
            return Err(
                "overflow-trap binary should exit with non-zero status (overflow trap), but it exited successfully".to_owned()
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "overflow-trap target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "overflow-trap should compile, run, and exit with non-zero status: {failure_message}"
    );
}

#[test]
fn lambda_basic_compiles_and_returns_correct_value() {
    let temp_dir = Path::new("test-projects/lambda-basic/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "lambda-basic target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/lambda-basic/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "lambda-basic source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "lambda-basic source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "lambda-basic compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("3 + 4 = 7") {
            return Err(format!(
                "lambda-basic binary stdout should contain '3 + 4 = 7', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "lambda-basic binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "lambda-basic target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "lambda-basic should compile, run, print correct sum, and exit cleanly: {failure_message}"
    );
}

#[test]
fn array_bounds_trap_exits_nonzero() {
    let temp_dir = Path::new("test-projects/array-bounds/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "array-bounds target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/array-bounds/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "array-bounds source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "array-bounds source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "array-bounds compiled binary should execute: {error}"
                ));
            }
        };

        if run_output.status.success() {
            return Err(
                "array-bounds binary should exit with non-zero status (bounds trap), but it exited successfully".to_owned()
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "array-bounds target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "array-bounds should compile, run, and exit with non-zero status: {failure_message}"
    );
}

#[test]
fn string_interp_long_does_not_crash() {
    let temp_dir = Path::new("test-projects/string-interp-long/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "string-interp-long target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/string-interp-long/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "string-interp-long source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "string-interp-long source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "string-interp-long compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("Hello") {
            return Err(format!(
                "string-interp-long binary stdout should contain 'Hello', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "string-interp-long binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-interp-long target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "string-interp-long should compile, run without crash, print output, and exit cleanly: {failure_message}"
    );
}

#[test]
fn should_print_final_result_compiles_and_runs() {
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
                    "should-print-final-result source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "should-print-final-result source should compile into a binary: {error}"
                ));
            }
        };

        let child_result = std::process::Command::new(&binary_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
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
            let write_result = std::io::Write::write_all(stdin, b"2\n3\n");
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
        if !stdout.contains('5') {
            return Err(format!(
                "should-print-final-result stdout should contain computed final result '5', got: '{stdout}'"
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
        "should-print-final-result should compile, run, and produce final numeric output: {failure_message}"
    );
}

#[test]
fn cast_safety_compiles_and_runs() {
    let temp_dir = Path::new("test-projects/cast-safety/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "cast-safety target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/cast-safety/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "cast-safety source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "cast-safety source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "cast-safety compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("float64:") {
            return Err(format!(
                "cast-safety binary stdout should contain 'float64:', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "cast-safety binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "cast-safety target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "cast-safety should compile, run, print float output, and exit cleanly: {failure_message}"
    );
}
