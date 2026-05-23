#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;

const RC_STORE_HARNESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_rc_store_binary(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<PathBuf, String> {
    let context = inkwell::context::Context::create();
    let module = compile_to_module(&context, Path::new(fixture_name), source)
        .map_err(|error| format!("{fixture_name} should compile into an LLVM module: {error:?}"))?;

    let object_path = temp_dir.join(format!("{fixture_name}.o"));
    emit_object_file(&module, &object_path, &TargetTriple::host())
        .map_err(|error| format!("{fixture_name} object emission should succeed: {error}"))?;

    let harness_bin = temp_dir.join(format!("{fixture_name}_rc_store_harness"));
    let fixture_path = repo_root().join("tests/integration_e2e/fixtures/rc_store_leak_regressions.c");

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
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
        .arg(&fixture_path)
        .arg("-o")
        .arg(&harness_bin)
        .current_dir(repo_root());

    let compile = run_command_output_with_timeout(
        &mut compile_command,
        RC_STORE_HARNESS_TIMEOUT,
        "rc store leak harness compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "rc store leak harness compile should succeed for {fixture_name}, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(harness_bin)
}

fn run_rc_store_case(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<String, String> {
    let harness_bin = compile_rc_store_binary(temp_dir, fixture_name, source)?;

    let output = run_binary_output_with_timeout(
        &harness_bin,
        RC_STORE_HARNESS_TIMEOUT,
        "rc store leak harness",
    )?;

    drop(std::fs::remove_file(&harness_bin));

    if !output.status.success() {
        return Err(format!(
            "rc store leak harness should exit 0 for {fixture_name}, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_counter_line(stdout: &str, label: &str) -> Result<(usize, usize, usize), String> {
    let Some(line) = stdout
        .lines()
        .find(|line| line.starts_with(&format!("rc_store_counter:{label} alloc=")))
    else {
        return Err(format!(
            "rc store output should include 'rc_store_counter:{label} alloc=', got stdout: {stdout}"
        ));
    };

    let mut alloc: Option<usize> = None;
    let mut free: Option<usize> = None;
    let mut live_count: Option<usize> = None;

    for token in line.split_whitespace() {
        if let Some(value) = token.strip_prefix("alloc=") {
            alloc = value.parse::<usize>().ok();
        } else if let Some(value) = token.strip_prefix("free=") {
            free = value.parse::<usize>().ok();
        } else if let Some(value) = token.strip_prefix("live=") {
            live_count = value.parse::<usize>().ok();
        }
    }

    let Some(alloc_count) = alloc else {
        return Err(format!(
            "rc store output line should include parseable alloc count for {label}, line='{line}', stdout: {stdout}"
        ));
    };
    let Some(free_count) = free else {
        return Err(format!(
            "rc store output line should include parseable free count for {label}, line='{line}', stdout: {stdout}"
        ));
    };
    let Some(live_count) = live_count else {
        return Err(format!(
            "rc store output line should include parseable live count for {label}, line='{line}', stdout: {stdout}"
        ));
    };

    Ok((alloc_count, free_count, live_count))
}

fn parse_metric(stdout: &str, prefix: &str) -> Result<usize, String> {
    let Some(line) = stdout.lines().find(|line| line.starts_with(prefix)) else {
        return Err(format!(
            "rc store output should include '{prefix}<number>', got stdout: {stdout}"
        ));
    };

    let value = line
        .split_once('=')
        .and_then(|(_, parsed)| parsed.trim().parse::<usize>().ok())
        .ok_or_else(|| {
            format!(
                "rc store metric line should include parseable usize for prefix '{prefix}', line='{line}', stdout: {stdout}"
            )
        })?;

    Ok(value)
}

fn assert_rc_store_balanced(stdout: &str, test_name: &str) -> Result<(), String> {
    let (array_alloc, array_free, array_live) = parse_counter_line(stdout, "arrays")?;
    let live_heap = parse_metric(stdout, "rc_store_live_heap_bytes=")?;
    let peak_heap = parse_metric(stdout, "rc_store_peak_heap_bytes=")?;

    if array_alloc == 0 {
        return Err(format!(
            "{test_name} should exercise array allocations, got stdout: {stdout}"
        ));
    }

    if array_free != array_alloc || array_live != 0 {
        return Err(format!(
            "{test_name} should balance array counters (alloc={array_alloc}, free={array_free}, live={array_live}), stdout: {stdout}"
        ));
    }

    if live_heap != 0 {
        return Err(format!(
            "{test_name} should end with zero live heap bytes, got live_heap={live_heap}, stdout: {stdout}"
        ));
    }

    if peak_heap == 0 {
        return Err(format!(
            "{test_name} should report non-zero peak heap bytes for exercised allocations, got stdout: {stdout}"
        ));
    }

    if !stdout.contains("rc_store_counter_status=balanced") {
        return Err(format!(
            "{test_name} should report rc_store_counter_status=balanced, got stdout: {stdout}"
        ));
    }

    if !stdout.contains("rc_store_heap_status=balanced") {
        return Err(format!(
            "{test_name} should report rc_store_heap_status=balanced, got stdout: {stdout}"
        ));
    }

    Ok(())
}

fn run_rc_store_test_case(test_name: &str, fixture_name: &str, source: &str) {
    let temp_dir = unique_probe_target_dir(test_name);
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "{test_name} temp directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let stdout = run_rc_store_case(&temp_dir, fixture_name, source)?;
        assert_rc_store_balanced(&stdout, test_name)
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "{test_name} temp directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "{test_name} should complete with balanced rc store counters and heap accounting: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn rc_store_direct_assignment() {
    let source = "
##
  Description: RC store regression for direct assignment from a fresh array value.
##
entry main = f(args: string[]): void =>
    let mutable board: int8[] = [0]
    board = [1, 0, 1, 0]
    return void
";

    run_rc_store_test_case(
        "rc_store_direct_assignment",
        "rc_store_direct_assignment",
        source,
    );
}

#[test]
#[serial(fs)]
fn rc_store_push_no_grow() {
    let source = "
##
  Description: RC store regression for push path with reserved capacity and no growth.
##
entry main = f(args: string[]): void =>
    let mutable values: int64[] = [1, 2]
    values.push(3)
    return void
";

    run_rc_store_test_case("rc_store_push_no_grow", "rc_store_push_no_grow", source);
}

#[test]
#[serial(fs)]
fn rc_store_push_grow() {
    let source = "
##
  Description: RC store regression for push path that grows backing capacity.
##
entry main = f(args: string[]): void =>
    let mutable values: int64[] = [1]
    values.push(2)
    values.push(3)
    return void
";

    run_rc_store_test_case("rc_store_push_grow", "rc_store_push_grow", source);
}

#[test]
#[serial(fs)]
fn rc_store_self_overwrite() {
    let source = "
##
  Description: RC store regression for self-overwrite assignment.
##
entry main = f(args: string[]): void =>
    let mutable values: int64[] = [1, 2, 3]
    values = values
    return void
";

    run_rc_store_test_case(
        "rc_store_self_overwrite",
        "rc_store_self_overwrite",
        source,
    );
}

#[test]
#[serial(fs)]
fn rc_store_aliased_source_safety() {
    let source = "
##
  Description: RC store regression for aliased source assignment safety.
##
entry main = f(args: string[]): void =>
    let mutable source: int64[] = [9, 8, 7]
    let alias = source
    source = alias
    if alias.length > 0:
        print('alias-live')
    return void
";

    run_rc_store_test_case(
        "rc_store_aliased_source_safety",
        "rc_store_aliased_source_safety",
        source,
    );
}

#[test]
#[serial(fs)]
fn rc_store_second_class_ref_adjacent() {
    let source = "
##
  Description: Returns the first scalar cell for ref-adjacent assignment coverage.
##
let identity_value = f(x: int32): int32 =>
    return x

##
  Description: RC store regression near second-class-reference adjacency and assignment overwrite.
##
entry main = f(args: string[]): void =>
    let mutable values: int32[] = [4, 5]
    let first_cell = values[0]
    let head = identity_value(first_cell)
    if head > 0:
        values = [head, 6]
    return void
";

    run_rc_store_test_case(
        "rc_store_second_class_ref_adjacent",
        "rc_store_second_class_ref_adjacent",
        source,
    );
}
