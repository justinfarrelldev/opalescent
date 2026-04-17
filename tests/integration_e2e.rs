#![cfg(feature = "integration")]

use opalescent::compiler::{
    compile_program, compile_to_module, emit_object_file, link_object_file,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
    fn smoke_void_program_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/_smoke/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "smoke temp directory should be created");

        let source = "entry main = f(): void => { return void }";
        let binary_result = compile_program(source, temp_dir);
        assert!(
            binary_result.is_ok(),
            "smoke source should compile to a runnable binary"
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = Command::new(&binary_path).output();
        assert!(
            output_result.is_ok(),
            "compiled smoke binary should execute"
        );
        let Ok(run_output) = output_result else {
            return;
        };

        assert!(
            run_output.status.success(),
            "compiled smoke binary should exit successfully"
        );
        assert!(
            run_output.stdout.is_empty(),
            "compiled smoke binary should not print anything"
        );

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "smoke temp directory should be removed");
    }

    #[test]
    fn emit_object_file_creates_valid_object() {
        let temp_dir = Path::new("test-projects/_emit/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "emit temp directory should be created");

        let context = inkwell::context::Context::create();
        let source = "entry main = f(): void => { return void }";
        let module_result = compile_to_module(&context, source);
        assert!(
            module_result.is_ok(),
            "source should compile into an LLVM module for object emission"
        );
        let Ok(module) = module_result else {
            return;
        };

        let object_path = temp_dir.join("program.o");
        let emit_result = emit_object_file(&module, &object_path);
        assert!(emit_result.is_ok(), "object emission should succeed");

        assert!(
            object_path.exists(),
            "object file should exist after emission"
        );

        let metadata_result = fs::metadata(&object_path);
        assert!(
            metadata_result.is_ok(),
            "object metadata should be readable"
        );
        let Ok(metadata) = metadata_result else {
            return;
        };
        assert!(
            metadata.len() > 0,
            "object file should be non-empty after emission"
        );

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "emit temp directory should be removed");
    }

    #[test]
    fn link_produces_executable() {
        let temp_dir = Path::new("test-projects/_link/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "link temp directory should be created");

        let context = inkwell::context::Context::create();
        let source = "entry main = f(): void => { return void }";
        let module_result = compile_to_module(&context, source);
        assert!(
            module_result.is_ok(),
            "source should compile into an LLVM module for linking"
        );
        let Ok(module) = module_result else {
            return;
        };

        let object_path = temp_dir.join("program.o");
        let binary_path = temp_dir.join("program");

        let emit_result = emit_object_file(&module, &object_path);
        assert!(
            emit_result.is_ok(),
            "object emission should succeed before linking"
        );

        let link_result = link_object_file(&object_path, &binary_path);
        assert!(
            link_result.is_ok(),
            "link step should produce an executable"
        );
        let Ok(linked_binary) = link_result else {
            return;
        };

        assert!(
            linked_binary.exists(),
            "linked binary should exist at requested output path"
        );

        #[cfg(unix)]
        {
            let metadata_result = fs::metadata(&linked_binary);
            assert!(
                metadata_result.is_ok(),
                "linked binary metadata should be readable on unix"
            );
            let Ok(metadata) = metadata_result else {
                return;
            };
            let mode = metadata.permissions().mode();
            assert!(
                mode & 0o111 != 0,
                "linked output should have executable bits on unix"
            );
        }

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "link temp directory should be removed");
    }

    #[test]
    fn hello_world_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/hello-world/target");
        let prepare = prepare_dir(temp_dir);
        assert!(
            prepare.is_ok(),
            "hello-world target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/hello-world/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "hello-world source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "hello-world source should compile and link into a binary: {error}"
                    ));
                }
            };

            let output_result = Command::new(&binary_path).output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "hello-world compiled binary should execute: {error}"
                    ));
                }
            };

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains("Hello world") {
                return Err(format!(
                    "hello-world binary stdout should contain exact greeting 'Hello world', got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "hello-world binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(temp_dir);
        assert!(
            cleanup.is_ok(),
            "hello-world target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "hello-world end-to-end flow should compile, link, run, print greeting, and exit cleanly: {failure_message}"
        );
    }

    #[test]
    fn fib_recursive_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/fib-recursive/target");
        let prepare = prepare_dir(temp_dir);
        assert!(
            prepare.is_ok(),
            "fib-recursive target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/fib-recursive/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "fib-recursive source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "fib-recursive source should compile and link into a binary: {error}"
                    ));
                }
            };

            let output_result = Command::new(&binary_path).output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "fib-recursive compiled binary should execute: {error}"
                    ));
                }
            };

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains("fib(10) = 55") {
                return Err(format!(
                    "fib-recursive binary stdout should contain 'fib(10) = 55', got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "fib-recursive binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(temp_dir);
        assert!(
            cleanup.is_ok(),
            "fib-recursive target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "fib-recursive end-to-end flow should compile, link, run, print fibonacci result, and exit cleanly: {failure_message}"
        );
    }

    #[test]
    fn fib_iterative_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/fib-iterative/target");
        let prepare = prepare_dir(temp_dir);
        assert!(
            prepare.is_ok(),
            "fib-iterative target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/fib-iterative/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "fib-iterative source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "fib-iterative source should compile and link into a binary: {error}"
                    ));
                }
            };

            let output_result = Command::new(&binary_path).output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "fib-iterative compiled binary should execute: {error}"
                    ));
                }
            };

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains("fib(10) = 55") {
                return Err(format!(
                    "fib-iterative binary stdout should contain 'fib(10) = 55', got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "fib-iterative binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(temp_dir);
        assert!(
            cleanup.is_ok(),
            "fib-iterative target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "fib-iterative end-to-end flow should compile, link, run, print fibonacci result, and exit cleanly: {failure_message}"
        );
    }

    #[cfg(feature = "integration")]
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "integration test covers full stdin/stdout quiz flow"
    )]
    fn simple_quiz_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/simple-quiz/target");
        let prepare = prepare_dir(temp_dir);
        assert!(
            prepare.is_ok(),
            "simple-quiz target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/simple-quiz/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "simple-quiz source file should be readable from disk: {error}"
                    ));
                }
            };

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            let binary_path = match binary_result {
                Ok(path) => path,
                Err(error) => {
                    return Err(format!(
                        "simple-quiz source should compile and link into a binary: {error}"
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
                        "simple-quiz compiled binary should spawn with piped stdio: {error}"
                    ));
                }
            };

            if let Some(ref mut stdin) = child.stdin {
                let write_result = stdin.write_all(b"TestUser\n3\n");
                if let Err(error) = write_result {
                    return Err(format!(
                        "simple-quiz stdin should accept scripted user input: {error}"
                    ));
                }
            } else {
                return Err(
                    "simple-quiz process stdin should be piped so test input can be written"
                        .to_owned(),
                );
            }

            let output_result = child.wait_with_output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "simple-quiz compiled binary should complete and produce output: {error}"
                    ));
                }
            };

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains("What is your name?") {
                return Err(format!(
                    "simple-quiz stdout should contain prompt 'What is your name?', got: '{stdout}'"
                ));
            }

            if !stdout.contains("Hello, TestUser!") {
                return Err(format!(
                    "simple-quiz stdout should contain greeting 'Hello, TestUser!', got: '{stdout}'"
                ));
            }

            if !stdout.contains("Guess a number") {
                return Err(format!(
                    "simple-quiz stdout should contain prompt 'Guess a number', got: '{stdout}'"
                ));
            }

            if !stdout.contains("you guessed correctly")
                && !stdout.contains("too low")
                && !stdout.contains("Too high")
            {
                return Err(format!(
                    "simple-quiz stdout should contain one of ['you guessed correctly', 'too low', 'Too high'] due to random outcome, got: '{stdout}'"
                ));
            }

            if !run_output.status.success() {
                return Err(format!(
                    "simple-quiz binary should exit with status code 0, got: {:?}",
                    run_output.status.code()
                ));
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(temp_dir);
        assert!(
            cleanup.is_ok(),
            "simple-quiz target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "simple-quiz end-to-end flow should compile, link, run with stdin, print prompts/results, and exit cleanly: {failure_message}"
        );
    }

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

            let output_result = Command::new(&binary_path).output();
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

            let output_result = Command::new(&binary_path).output();
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

            let output_result = Command::new(&binary_path).output();
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

            let output_result = Command::new(&binary_path).output();
            let run_output = match output_result {
                Ok(output) => output,
                Err(error) => {
                    return Err(format!(
                        "string-interp-long compiled binary should execute: {error}"
                    ));
                }
            };

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            if !stdout.contains("prefix:") {
                return Err(format!(
                    "string-interp-long binary stdout should contain 'prefix:', got: '{stdout}'"
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
    fn immutability_compile_error() {
        let temp_dir = Path::new("test-projects/immutability/target");
        let prepare = prepare_dir(temp_dir);
        assert!(
            prepare.is_ok(),
            "immutability target directory should be created"
        );

        let execution_result: Result<(), String> = (|| {
            let source_path = Path::new("test-projects/immutability/src/main.op");
            let source_result = fs::read_to_string(source_path);
            let source_str = match source_result {
                Ok(contents) => contents,
                Err(error) => {
                    return Err(format!(
                        "immutability source file should be readable: {error}"
                    ));
                }
            };

            let binary_result = compile_program(source_str.as_str(), temp_dir);
            if binary_result.is_ok() {
                return Err(
                    "immutability source should fail to compile (assignment to immutable variable), but compilation succeeded".to_owned()
                );
            }

            Ok(())
        })();

        let cleanup = cleanup_dir(temp_dir);
        assert!(
            cleanup.is_ok(),
            "immutability target directory should be removed"
        );

        let failure_message = match execution_result {
            Ok(()) => String::new(),
            Err(message) => message,
        };
        assert!(
            failure_message.is_empty(),
            "immutability source should be rejected at compile time: {failure_message}"
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

            let output_result = Command::new(&binary_path).output();
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
}
