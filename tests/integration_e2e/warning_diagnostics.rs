#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::PathBuf;
use std::process::Command;

const CLI_WARNING_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const REPLACEABLE_ERROR_LIST_CODE: &str =
    "opalescent::type_system::warning::replaceable_error_list";
const REPLACEABLE_ERROR_LIST_LABEL: &str = "complete replaceable error list";
const BYTES_ERROR_HELP: &str =
    "Replace `errors HexDecodeError, SliceRangeError` with `errors BytesError`.";

fn opalescent_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("opalescent")
}

fn assert_replaceable_error_list_warning_renders(
    fixture_name: &str,
    source: &str,
) -> Result<(), String> {
    let temp_dir = unique_probe_target_dir(fixture_name);
    prepare_dir(&temp_dir).map_err(|error| {
        format!("{fixture_name} temporary directory should be created: {error}")
    })?;

    let execution_result = (|| {
        let source_path = temp_dir.join("main.op");
        fs::write(&source_path, source)
            .map_err(|error| format!("{fixture_name} source should be written: {error}"))?;

        let mut command = Command::new(opalescent_binary_path());
        command.args(["check", source_path.to_string_lossy().as_ref()]);
        let output = run_command_output_with_timeout(
            &mut command,
            CLI_WARNING_TEST_TIMEOUT,
            &format!("{fixture_name} opal check"),
        )?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(format!(
                "{fixture_name} warning-only source should exit successfully, stdout: {stdout}, stderr: {stderr}"
            ));
        }
        if !stdout.contains("check passed") {
            return Err(format!(
                "{fixture_name} warning-only source should report successful checking, stdout: {stdout}"
            ));
        }
        for expected in [
            "warning",
            REPLACEABLE_ERROR_LIST_CODE,
            REPLACEABLE_ERROR_LIST_LABEL,
            BYTES_ERROR_HELP,
            "BytesError",
            "HexDecodeError",
            "SliceRangeError",
            "main.op",
        ] {
            if !stderr.contains(expected) {
                return Err(format!(
                    "{fixture_name} rendered warning should contain {expected:?}, stderr: {stderr}"
                ));
            }
        }

        Ok(())
    })();

    cleanup_dir(&temp_dir).map_err(|error| {
        format!("{fixture_name} temporary directory should be removed: {error}")
    })?;
    execution_result
}

#[test]
fn stdlib_error_family_warning_function_renders() {
    let source = "##\n  Description: Main function declares every Bytes error leaf for warning rendering.\n##\nentry main = f(): void errors HexDecodeError, SliceRangeError =>\n    return void\n";
    let result =
        assert_replaceable_error_list_warning_renders("error-family-warning-function", source);
    assert!(
        result.is_ok(),
        "function warning should render without failing check: {result:?}"
    );
}

#[test]
fn stdlib_error_family_warning_lambda_renders() {
    let source = "let _decode = f(): void errors HexDecodeError, SliceRangeError =>\n    return void\n\n##\n  Description: Entry point keeps the lambda warning fixture executable.\n##\nentry main = f(): void =>\n    return void\n";
    let result =
        assert_replaceable_error_list_warning_renders("error-family-warning-lambda", source);
    assert!(
        result.is_ok(),
        "lambda warning should render without failing check: {result:?}"
    );
}
