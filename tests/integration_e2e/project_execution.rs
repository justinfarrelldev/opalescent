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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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
        if !stdout.contains("Part1=") || !stdout.contains("Part2=") {
            return Err(format!(
                "string-interp-long binary stdout should contain both 'Part1=' and 'Part2=', got: '{stdout}'"
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
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
        if !stdout.contains("float64:") || !stdout.contains("float32:") {
            return Err(format!(
                "cast-safety binary stdout should contain both 'float64:' and 'float32:', got: '{stdout}'"
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

#[test]
fn multi_file_project_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/multi-file");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "multi-file target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "multi-file project should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "multi-file compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "7" {
            return Err(format!(
                "multi-file binary stdout should equal '7', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "multi-file binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "multi-file target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "multi-file project should compile, run, and print computed sum: {failure_message}"
    );
}

#[test]
fn entry_in_wrong_file_fails_with_entry_not_in_main_module() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/_entry-wrong-file");
    let src_dir = project_dir.join("src");
    let output_dir = project_dir.join("target");

    let prepare = prepare_dir(&project_dir);
    assert!(
        prepare.is_ok(),
        "entry-wrong-file project directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let src_create_result = fs::create_dir_all(&src_dir);
        if let Err(error) = src_create_result {
            return Err(format!(
                "entry-wrong-file src directory should be created: {error}"
            ));
        }

        let toml_write = fs::write(
            project_dir.join("opal.toml"),
            "name = \"entry-wrong-file\"\nversion = \"1.0.0\"\n",
        );
        if let Err(error) = toml_write {
            return Err(format!(
                "entry-wrong-file opal.toml should be written: {error}"
            ));
        }

        let main_write = fs::write(
            src_dir.join("main.op"),
            "import { helper } from './worker'\n\nlet call_helper = f(): int32 =>\n    return helper()\n",
        );
        if let Err(error) = main_write {
            return Err(format!(
                "entry-wrong-file main.op should be written: {error}"
            ));
        }

        let worker_write = fs::write(
            src_dir.join("worker.op"),
            "##\n  Description: wrong module entry used for validation error path\n##\nentry worker = f(args: string[]): void =>\n    return void\n\n##\n  Description: helper function exported from worker module\n##\npublic let helper = f(): int32 =>\n    return 1\n",
        );
        if let Err(error) = worker_write {
            return Err(format!(
                "entry-wrong-file worker.op should be written: {error}"
            ));
        }

        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &output_dir, &TargetTriple::host());
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "entry-wrong-file project should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let error_message = compile_error.to_string();
        let contains_expected = error_message.to_ascii_lowercase().contains("entry")
            && error_message.to_ascii_lowercase().contains("main");
        if !contains_expected {
            return Err(format!(
                "entry-wrong-file compile error should mention entry not in main module, got: {error_message}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&project_dir);
    assert!(
        cleanup.is_ok(),
        "entry-wrong-file project directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "entry-wrong-file project should fail with entry-not-in-main-module style error: {failure_message}"
    );
}

#[test]
fn package_import_fails_with_not_supported() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/_package-import-not-supported");
    let src_dir = project_dir.join("src");
    let output_dir = project_dir.join("target");

    let prepare = prepare_dir(&project_dir);
    assert!(
        prepare.is_ok(),
        "package-import-not-supported project directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let src_create_result = fs::create_dir_all(&src_dir);
        if let Err(error) = src_create_result {
            return Err(format!(
                "package-import-not-supported src directory should be created: {error}"
            ));
        }

        let toml_write = fs::write(
            project_dir.join("opal.toml"),
            "name = \"package-import-not-supported\"\nversion = \"1.0.0\"\n",
        );
        if let Err(error) = toml_write {
            return Err(format!(
                "package-import-not-supported opal.toml should be written: {error}"
            ));
        }

        let main_write = fs::write(
            src_dir.join("main.op"),
            "import { foo } from '@scope/package'\n\n##\n  Description: entrypoint used for package import error validation\n##\nentry main = f(args: string[]): void =>\n    print('{foo}')\n    return void\n",
        );
        if let Err(error) = main_write {
            return Err(format!(
                "package-import-not-supported main.op should be written: {error}"
            ));
        }

        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &output_dir, &TargetTriple::host());
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "package-import-not-supported project should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let error_message = compile_error.to_string();
        let lowercase_message = error_message.to_ascii_lowercase();
        let contains_expected = lowercase_message.contains("package")
            && lowercase_message.contains("import")
            && lowercase_message.contains("support");
        if !contains_expected {
            return Err(format!(
                "package-import-not-supported compile error should mention package imports are not supported, got: {error_message}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&project_dir);
    assert!(
        cleanup.is_ok(),
        "package-import-not-supported project directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "package-import-not-supported project should fail with not-supported import error: {failure_message}"
    );
}

#[test]
fn import_types_basic_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/import-types-basic");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "import-types-basic target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "import-types-basic project should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "import-types-basic compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim_end() != "Alice is 30 years old" {
            return Err(format!(
                "import-types-basic stdout should equal 'Alice is 30 years old', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "import-types-basic binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "import-types-basic target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "import-types-basic should compile, run, print expected output, and exit cleanly: {failure_message}"
    );
}

#[test]
fn import_types_aliased_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/import-types-aliased");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "import-types-aliased target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "import-types-aliased project should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "import-types-aliased compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim_end() != "User 42: Bob" {
            return Err(format!(
                "import-types-aliased stdout should equal 'User 42: Bob', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "import-types-aliased binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "import-types-aliased target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "import-types-aliased should compile, run, print expected output, and exit cleanly: {failure_message}"
    );
}

#[test]
fn import_types_multiple_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/import-types-multiple");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "import-types-multiple target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "import-types-multiple project should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "import-types-multiple compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim_end() != "Rect 10x20 at (0,0)" {
            return Err(format!(
                "import-types-multiple stdout should equal 'Rect 10x20 at (0,0)', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "import-types-multiple binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "import-types-multiple target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "import-types-multiple should compile, run, print expected output, and exit cleanly: {failure_message}"
    );
}

