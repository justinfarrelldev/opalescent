#![cfg(feature = "integration")]

//! End-to-end validation of the `Bytes` stdlib surface.
//!
//! Compiles `test-projects/bytes-hex-roundtrip`, links it against the
//! bundled C runtime (now including `opal_bytes.c`), executes the resulting
//! binary, and asserts the observable output covers every bytes built-in
//! that was promoted from the Rust stdlib into the Opalescent language.

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Covered surface: `bytes_from_hex`, `Bytes.length`, `bytes_to_hex`,
/// `bytes_concatenate`, and `bytes_slice` (both via `guard`).
///
/// The script encodes and decodes `deadbeef`, doubles the buffer, slices
/// back to the original length, and prints each intermediate observation.
/// Any regression in the shared struct-return convention or in the opaque
/// `Bytes` handle representation should surface as a failure here.
#[test]
fn bytes_hex_roundtrip_compiles_and_runs() {
    let temp_dir = unique_probe_target_dir("bytes-hex-roundtrip");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "bytes-hex-roundtrip target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/bytes-hex-roundtrip/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "bytes-hex-roundtrip source file should be readable: {error}"
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
                    "bytes-hex-roundtrip source should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "bytes-hex-roundtrip compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "bytes-hex-roundtrip binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout).into_owned();
        let required_lines = [
            "length: 4",
            "first: deadbeef",
            "doubled: 8",
            "slice: deadbeef",
        ];
        for expected in required_lines {
            if !stdout.contains(expected) {
                return Err(format!(
                    "bytes-hex-roundtrip stdout should contain '{expected}', got:\n{stdout}"
                ));
            }
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "bytes-hex-roundtrip target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "bytes-hex-roundtrip should compile, run, and print the expected sequence: \
         {failure_message}"
    );
}

/// Covered surface: the legacy `bytes_new()` constructor and `Bytes.length`.
///
/// The script constructs an empty buffer, prints its length, and should
/// surface any regression in the legacy zero-argument Bytes initialization
/// path.
#[test]
fn empty_bytes_via_bytes_new() {
    let temp_dir = unique_probe_target_dir("bytes-empty-construct-legacy");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "bytes-empty-construct-legacy target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/bytes-empty-construct-legacy/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "bytes-empty-construct-legacy source file should be readable: {error}"
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
                    "bytes-empty-construct-legacy source should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "bytes-empty-construct-legacy compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "bytes-empty-construct-legacy binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout).into_owned();
        let expected_line = "legacy length: 0";
        if !stdout.contains(expected_line) {
            return Err(format!(
                "bytes-empty-construct-legacy stdout should contain '{expected_line}', got:\n{stdout}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "bytes-empty-construct-legacy target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "bytes-empty-construct-legacy should compile, run, and print the expected sequence: \
         {failure_message}"
    );
}

/// Covered surface: the proposed `new Bytes` constructor syntax and `Bytes.length`.
///
/// The script constructs an empty buffer with the new syntax, prints its length,
/// and should surface any regression in parser, typechecker, or codegen handling
/// before implementation lands.
#[test]
fn empty_bytes_via_new_bytes() {
    let temp_dir = unique_probe_target_dir("bytes-empty-construct-new-syntax");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "bytes-empty-construct-new-syntax target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/bytes-empty-construct-new-syntax/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "bytes-empty-construct-new-syntax source file should be readable: {error}"
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
                    "bytes-empty-construct-new-syntax source should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "bytes-empty-construct-new-syntax compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "bytes-empty-construct-new-syntax binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout).into_owned();
        let expected_line = "new syntax length: 0";
        if !stdout.contains(expected_line) {
            return Err(format!(
                "bytes-empty-construct-new-syntax stdout should contain '{expected_line}', got:\n{stdout}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "bytes-empty-construct-new-syntax target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "bytes-empty-construct-new-syntax should compile, run, and print the expected sequence: \
         {failure_message}"
    );
}

/// Covered surface: inferred constructor typing for `let buffer = new Bytes`
/// plus `Bytes.length` member access on the inferred binding.
///
/// The script constructs an empty buffer without an explicit annotation,
/// reads `buffer.length`, and prints it. This should surface regressions in
/// constructor inference and downstream member access typing.
#[test]
fn bytes_stdlib_inferred_constructor() {
    let temp_dir = unique_probe_target_dir("bytes-empty-construct-inferred-new-syntax");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "bytes-empty-construct-inferred-new-syntax target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path =
            Path::new("test-projects/bytes-empty-construct-inferred-new-syntax/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "bytes-empty-construct-inferred-new-syntax source file should be readable: {error}"
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
                    "bytes-empty-construct-inferred-new-syntax source should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "bytes-empty-construct-inferred-new-syntax compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "bytes-empty-construct-inferred-new-syntax binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout).into_owned();
        let expected_line = "inferred syntax length: 0";
        if !stdout.contains(expected_line) {
            return Err(format!(
                "bytes-empty-construct-inferred-new-syntax stdout should contain '{expected_line}', got:\n{stdout}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "bytes-empty-construct-inferred-new-syntax target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "bytes-empty-construct-inferred-new-syntax should compile, run, and print the expected sequence: \
         {failure_message}"
    );
}
