#![cfg(feature = "integration")]

//! End-to-end validation of the `Bytes` stdlib surface.
//!
//! Compiles `test-projects/bytes-hex-roundtrip`, links it against the
//! bundled C runtime (now including `opal_bytes.c`), executes the resulting
//! binary, and asserts the observable output covers every bytes built-in
//! that was promoted from the Rust stdlib into the Opalescent language.

use super::*;

/// Covered surface: `bytes_from_hex`, `Bytes.length`, `bytes_to_hex`,
/// `bytes_concatenate`, and `bytes_slice` (both via `guard`).
///
/// The script encodes and decodes `deadbeef`, doubles the buffer, slices
/// back to the original length, and prints each intermediate observation.
/// Any regression in the shared struct-return convention or in the opaque
/// `Bytes` handle representation should surface as a failure here.
#[test]
fn bytes_hex_roundtrip_compiles_and_runs() {
    let temp_dir = Path::new("test-projects/bytes-hex-roundtrip/target");
    let prepare = prepare_dir(temp_dir);
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
                    "bytes-hex-roundtrip source should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "bytes-hex-roundtrip compiled binary should execute: {error}"
                ));
            }
        };

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

    let cleanup = cleanup_dir(temp_dir);
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
