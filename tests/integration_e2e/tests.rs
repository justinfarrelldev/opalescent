#![cfg(feature = "integration")]

use super::*;

mod project_execution;
mod interactive_io;
mod compile_failures;

#[test]
fn smoke_void_program_compiles_links_and_runs() {
    let temp_dir = Path::new("test-projects/_smoke/target");
    let prepare = prepare_dir(temp_dir);
    assert!(prepare.is_ok(), "smoke temp directory should be created");

    let source = "##\n    Description: Entry point for the smoke test program run\n##\nentry main = f(): void => { return void }";
    let binary_result = compile_program(source, temp_dir);
    assert!(
        binary_result.is_ok(),
        "smoke source should compile to a runnable binary"
    );
    let Ok(binary_path) = binary_result else {
        return;
    };

    let output_result = std::process::Command::new(&binary_path).output();
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
    let source = "##\n    Description: Entry point used for object emission validation\n##\nentry main = f(): void => { return void }";
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
    let source = "##\n    Description: Entry point used for linker executable validation\n##\nentry main = f(): void => { return void }";
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

        let output_result = std::process::Command::new(&binary_path).output();
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

        let output_result = std::process::Command::new(&binary_path).output();
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

        let output_result = std::process::Command::new(&binary_path).output();
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

#[test]
fn loop_expression_break_value_compiles_and_runs() {
    let temp_dir = Path::new("test-projects/_loop_expr/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "loop-expr temp directory should be created"
    );

    let source = "
import int64_to_string from standard

##
    Description: Entry point for expression loop break value test
##
entry main = f(args: string[]): void =>
    let result = loop =>
        break result: 42
    println(int64_to_string(result))
    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program(source, temp_dir);
        if binary_result.is_err() {
            return Err(format!(
                "loop expression source should compile: {:?}",
                binary_result.err()
            ));
        }
        let binary_path = binary_result.expect("compile succeeded");

        let output_result = std::process::Command::new(&binary_path).output();
        if output_result.is_err() {
            return Err(format!(
                "loop expression binary should execute: {:?}",
                output_result.err()
            ));
        }
        let run_output = output_result.expect("execution succeeded");

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("42") {
            return Err(format!(
                "loop expression binary stdout should contain '42', got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "loop expression binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
    assert!(
        cleanup.is_ok(),
        "loop-expr target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "loop expression should compile, run, print '42', and exit cleanly: {failure_message}"
    );
}
