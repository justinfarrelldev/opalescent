#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;

const CALL_TEMP_HARNESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const CALL_TEMP_SANITIZER_RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_call_temp_binary(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<PathBuf, String> {
    let context = inkwell::context::Context::create();
    let module = compile_to_module(&context, Path::new(fixture_name), source)
        .map_err(|error| format!("{fixture_name} should compile into an LLVM module: {error:?}"))?;

    let object_path = temp_dir.join(format!("{fixture_name}.o"));
    emit_object_file(&module, &object_path, &TargetTriple::host())
        .map_err(|error| format!("{fixture_name} object emission should succeed: {error}"))?;

    let harness_bin = temp_dir.join(format!("{fixture_name}_call_temp_harness"));
    let repo_root = repo_root();

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
        .arg("-fsanitize=address,leak")
        .arg("-fno-omit-frame-pointer")
        .arg("-DOPAL_ENABLE_INTERNAL_TESTING")
        .arg("-no-pie")
        .arg("-I.")
        .arg("runtime/opal_runtime.c")
        .arg("runtime/opal_error.c")
        .arg("runtime/opal_io.c")
        .arg("runtime/opal_print.c")
        .arg("runtime/opal_rng.c")
        .arg("runtime/opal_parse.c")
        .arg("runtime/opal_string.c")
        .arg("runtime/opal_bytes.c")
        .arg("runtime/opal_rc.c")
        .arg("runtime/opal_fs.c")
        .arg(&object_path)
        .arg("-o")
        .arg(&harness_bin)
        .current_dir(&repo_root);

    let compile = run_command_output_with_timeout(
        &mut compile_command,
        CALL_TEMP_HARNESS_TIMEOUT,
        "call temp leak harness compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "call temp leak harness compile should succeed for {fixture_name}, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(harness_bin)
}

fn assert_no_sanitizer_markers(stdout: &str, stderr: &str, test_name: &str) -> Result<(), String> {
    let combined = format!("{stdout}\n{stderr}");
    let forbidden_markers = [
        "ERROR: AddressSanitizer",
        "LeakSanitizer",
        "heap-use-after-free",
        "double-free",
        "detected memory leaks",
    ];

    for marker in forbidden_markers {
        if combined.contains(marker) {
            return Err(format!(
                "{test_name} should not include sanitizer marker '{marker}', stdout='{stdout}', stderr='{stderr}'"
            ));
        }
    }

    Ok(())
}

fn run_call_temp_case(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<(), String> {
    let harness_bin = compile_call_temp_binary(temp_dir, fixture_name, source)?;

    let mut run_command = Command::new(&harness_bin);
    run_command.env(
        "ASAN_OPTIONS",
        "detect_leaks=1:halt_on_error=1:strict_string_checks=1:check_initialization_order=1",
    );
    run_command.env("LSAN_OPTIONS", "halt_on_error=1:print_suppressions=0");

    let output = run_command_output_with_timeout(
        &mut run_command,
        CALL_TEMP_SANITIZER_RUN_TIMEOUT,
        "call temp sanitizer harness",
    )?;

    drop(std::fs::remove_file(&harness_bin));

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert_no_sanitizer_markers(&stdout, &stderr, fixture_name)
}

fn run_call_temp_test_case(test_name: &str, fixture_name: &str, source: &str) {
    let temp_dir = unique_probe_target_dir(test_name);
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "{test_name} temp directory should be created"
    );

    let execution_result: Result<(), String> =
        run_call_temp_case(&temp_dir, fixture_name, source);

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "{test_name} temp directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "{test_name} should complete with no sanitizer leak markers for call temporaries: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn call_temp_owned_arg_freed_on_return() {
    let source = "
import string_to_int32 from standard

##
  Description: Direct interpolation call-arg temporary should be freed on normal return.
##
entry main = f(args: string[]): void errors ParseError =>
    let parsed = propagate string_to_int32('{args.length}')
    if parsed >= 0:
        return void
    return void
";

    run_call_temp_test_case(
        "call_temp_owned_arg_freed_on_return",
        "call_temp_owned_arg_freed_on_return",
        source,
    );
}

#[test]
#[serial(fs)]
fn call_temp_owned_arg_freed_on_propagate() {
    let source = "
import string_to_int32 from standard

##
  Description: Direct interpolation call-arg temporary should be cleaned when propagate exits early.
##
entry main = f(args: string[]): void errors ParseError =>
    let _parsed = propagate string_to_int32('propagate-leak:{args.length}')
    return void
";

    run_call_temp_test_case(
        "call_temp_owned_arg_freed_on_propagate",
        "call_temp_owned_arg_freed_on_propagate",
        source,
    );
}

#[test]
#[serial(fs)]
fn call_temp_mixed_disposition() {
    let source = "
import stdout_writer, writer_write_sync, writer_flush_sync from standard

##
  Description: Mixed borrowed and owned string dispositions should not leak call temporaries.
##
entry main = f(args: string[]): void errors WriteFailureError, FlushFailureError, SinkClosedError =>
    let writer = stdout_writer()
    let borrowed = 'borrowed-text'
    propagate writer_write_sync(writer, borrowed)
    propagate writer_write_sync(writer, 'owned-text:{args.length}')
    propagate writer_flush_sync(writer)
    return void
";

    run_call_temp_test_case(
        "call_temp_mixed_disposition",
        "call_temp_mixed_disposition",
        source,
    );
}

#[test]
#[serial(fs)]
fn call_temp_nested_later_failure_cleanup() {
    let source = "
import stdout_writer, writer_write_sync from standard
import string_to_int32 from standard

##
  Description: Earlier direct call-temp allocations should be cleaned when a later propagate fails.
##
entry main = f(args: string[]): void errors WriteFailureError, SinkClosedError, ParseError =>
    let writer = stdout_writer()
    propagate writer_write_sync(writer, 'first:{args.length}')
    let _forced_fail = propagate string_to_int32('not-a-number')
    propagate writer_write_sync(writer, 'second:unreachable')
    return void
";

    run_call_temp_test_case(
        "call_temp_nested_later_failure_cleanup",
        "call_temp_nested_later_failure_cleanup",
        source,
    );
}

#[test]
#[serial(fs)]
fn call_temp_take_owned_no_double_free() {
    let source = "
import stdout_writer, writer_write_sync, writer_flush_sync from standard

##
  Description: Ownership-transfer call path should not double-free a direct interpolation temporary.
##
entry main = f(args: string[]): void errors WriteFailureError, FlushFailureError, SinkClosedError =>
    let writer = stdout_writer()
    let rendered: string = 'transfer:{args.length}'
    propagate writer_write_sync(writer, rendered)
    propagate writer_flush_sync(writer)
    return void
";

    run_call_temp_test_case(
        "call_temp_take_owned_no_double_free",
        "call_temp_take_owned_no_double_free",
        source,
    );
}
