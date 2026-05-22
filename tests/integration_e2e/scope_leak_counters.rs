#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCOPE_LEAK_HARNESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

const SCOPE_LEAK_COUNTER_HARNESS_C: &str = r#"#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_rc.h"

typedef struct {
    OpalRcDebugCounterKind kind;
    const char* label;
} CounterLabel;

static const CounterLabel REQUIRED_COUNTERS[] = {
    { OPAL_RC_DEBUG_COUNTER_STRINGS, "strings" },
    { OPAL_RC_DEBUG_COUNTER_ARRAYS, "arrays" },
};

static void report_counters(void) {
    size_t i = 0;
    int balanced = 1;
    for (i = 0; i < (sizeof(REQUIRED_COUNTERS) / sizeof(REQUIRED_COUNTERS[0])); ++i) {
        const CounterLabel* counter = &REQUIRED_COUNTERS[i];
        size_t alloc_count = opal_rc_debug_alloc_count_for_test(counter->kind);
        size_t free_count = opal_rc_debug_free_count_for_test(counter->kind);
        size_t live_count = opal_rc_debug_live_count_for_test(counter->kind);
        if (alloc_count == 0 || free_count != alloc_count || live_count != 0) {
            balanced = 0;
        }
        printf(
            "counter:%s alloc=%zu free=%zu live=%zu\n",
            counter->label,
            alloc_count,
            free_count,
            live_count
        );
    }

    printf("counter_status=%s\n", balanced ? "balanced" : "imbalanced");
}

__attribute__((constructor))
static void init_counter_harness(void) {
    opal_rc_debug_reset_counters_for_test();
    if (atexit(report_counters) != 0) {
        fprintf(stderr, "failed to register scope leak counter reporter\n");
    }
}
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_scope_leak_binary(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<PathBuf, String> {
    let context = inkwell::context::Context::create();
    let module = compile_to_module(&context, Path::new(fixture_name), source)
        .map_err(|error| format!("{fixture_name} should compile into an LLVM module: {error:?}"))?;

    let object_path = temp_dir.join(format!("{fixture_name}.o"));
    emit_object_file(&module, &object_path, &TargetTriple::host())
        .map_err(|error| format!("{fixture_name} object emission should succeed: {error}"))?;

    let harness_source_path = temp_dir.join(format!("{fixture_name}_counter_harness.c"));
    fs::write(&harness_source_path, SCOPE_LEAK_COUNTER_HARNESS_C)
        .map_err(|error| format!("{fixture_name} harness source should be written: {error}"))?;

    let harness_bin = temp_dir.join(format!("{fixture_name}_counter_harness"));
    let repo_root = repo_root();

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
        .arg(&harness_source_path)
        .arg("-o")
        .arg(&harness_bin)
        .current_dir(&repo_root);

    let compile = run_command_output_with_timeout(
        &mut compile_command,
        SCOPE_LEAK_HARNESS_TIMEOUT,
        "scope leak counter harness compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "scope leak counter harness compile should succeed for {fixture_name}, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(harness_bin)
}

fn run_scope_leak_case(temp_dir: &Path, fixture_name: &str, source: &str) -> Result<String, String> {
    let harness_bin = compile_scope_leak_binary(temp_dir, fixture_name, source)?;

    let output = run_binary_output_with_timeout(
        &harness_bin,
        SCOPE_LEAK_HARNESS_TIMEOUT,
        "scope leak counter harness",
    )?;

    drop(std::fs::remove_file(&harness_bin));

    if !output.status.success() {
        return Err(format!(
            "scope leak counter harness should exit 0 for {fixture_name}, status={:?}, stdout='{}', stderr='{}'",
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
        .find(|line| line.starts_with(&format!("counter:{label} alloc=")))
    else {
        return Err(format!(
            "scope leak output should include 'counter:{label} alloc=', got stdout: {stdout}"
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
            "scope leak output line should include parseable alloc count for {label}, line='{line}', stdout: {stdout}"
        ));
    };
    let Some(free_count) = free else {
        return Err(format!(
            "scope leak output line should include parseable free count for {label}, line='{line}', stdout: {stdout}"
        ));
    };
    let Some(live_count) = live_count else {
        return Err(format!(
            "scope leak output line should include parseable live count for {label}, line='{line}', stdout: {stdout}"
        ));
    };

    Ok((alloc_count, free_count, live_count))
}

fn assert_balanced_scope_counters(stdout: &str, test_name: &str) -> Result<(), String> {
    let (string_alloc, string_free, string_live) = parse_counter_line(stdout, "strings")?;
    let (array_alloc, array_free, array_live) = parse_counter_line(stdout, "arrays")?;

    if string_alloc == 0 {
        return Err(format!(
            "{test_name} should exercise string allocations, got stdout: {stdout}"
        ));
    }
    if array_alloc == 0 {
        return Err(format!(
            "{test_name} should exercise array allocations, got stdout: {stdout}"
        ));
    }

    if string_free != string_alloc || string_live != 0 {
        return Err(format!(
            "{test_name} should balance string counters (alloc={string_alloc}, free={string_free}, live={string_live}), stdout: {stdout}"
        ));
    }

    if array_free != array_alloc || array_live != 0 {
        return Err(format!(
            "{test_name} should balance array counters (alloc={array_alloc}, free={array_free}, live={array_live}), stdout: {stdout}"
        ));
    }

    if !stdout.contains("counter_status=balanced") {
        return Err(format!(
            "{test_name} should report counter_status=balanced, got stdout: {stdout}"
        ));
    }

    Ok(())
}

fn run_scope_leak_test_case(test_name: &str, fixture_name: &str, source: &str) {
    let temp_dir = unique_probe_target_dir(test_name);
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "{test_name} temp directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let stdout = run_scope_leak_case(&temp_dir, fixture_name, source)?;
        assert_balanced_scope_counters(&stdout, test_name)
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "{test_name} temp directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "{test_name} should complete with balanced scope counters: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn scope_leak_block_exit() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for block exit cleanup.
##
entry main = f(args: string[]): void =>
    let outer: string[] = ['outer']
    if outer.length > 0:
        let scoped: string[] = ['block', 'exit']
        let joined = string_join(scoped, '-')
        print(joined)
    print(outer[0])
    return void
";

    run_scope_leak_test_case("scope_leak_block_exit", "scope_leak_block_exit", source);
}

#[test]
#[serial(fs)]
fn scope_leak_for_iter_var() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for for-loop iteration binding cleanup.
##
entry main = f(args: string[]): void =>
    let values: string[] = ['alpha', 'beta', 'gamma']
    for value in values:
        let row: string[] = [value, 'iter']
        let joined = string_join(row, '-')
        print(joined)
    return void
";

    run_scope_leak_test_case("scope_leak_for_iter_var", "scope_leak_for_iter_var", source);
}

#[test]
#[serial(fs)]
fn scope_leak_early_return() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for early return cleanup.
##
entry main = f(args: string[]): void =>
    let values: string[] = ['early', 'return']
    if values.length > 0:
        let payload: string[] = [values[0], 'path']
        let joined = string_join(payload, '-')
        print(joined)
        return void
    print('unreachable')
    return void
";

    run_scope_leak_test_case(
        "scope_leak_early_return",
        "scope_leak_early_return",
        source,
    );
}

#[test]
#[serial(fs)]
fn scope_leak_break() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for break cleanup.
##
entry main = f(args: string[]): void =>
    let outcome = loop =>
        let parts: string[] = ['break', 'scope']
        let text = string_join(parts, '-')
        print(text)
        break outcome: text
    print(outcome)
    return void
";

    run_scope_leak_test_case("scope_leak_break", "scope_leak_break", source);
}

#[test]
#[serial(fs)]
fn scope_leak_continue() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for continue cleanup.
##
entry main = f(args: string[]): void =>
    let values: string[] = ['one', 'two', 'three']
    for value in values:
        let parts: string[] = [value, 'continue']
        let joined = string_join(parts, '-')
        print(joined)
        continue
    return void
";

    run_scope_leak_test_case("scope_leak_continue", "scope_leak_continue", source);
}

#[test]
#[serial(fs)]
fn scope_leak_return_transfer() {
    let source = "
import string_join from standard

##
  Description: Scope leak fixture for return transfer ownership.
##
let make_message = f(): string =>
    let parts: string[] = ['owned', 'value']
    let message = string_join(parts, '-')
    return message

##
  Description: Exercises returned ownership in caller scope.
##
entry main = f(args: string[]): void =>
    let returned = make_message()
    print(returned)
    return void
";

    run_scope_leak_test_case(
        "scope_leak_return_transfer",
        "scope_leak_return_transfer",
        source,
    );
}

#[test]
#[serial(fs)]
fn scope_leak_string_array_drop() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for string array element drop cleanup.
##
entry main = f(args: string[]): void =>
    let first = 'alpha'
    let second = 'beta'
    let values: string[] = [first, second, 'gamma']
    let combined = string_join(values, ',')
    print(combined)
    return void
";

    run_scope_leak_test_case(
        "scope_leak_string_array_drop",
        "scope_leak_string_array_drop",
        source,
    );
}

#[test]
#[serial(fs)]
fn scope_leak_propagated_string_local() {
    let source = "
import string_builder_push, string_builder_finish from standard

##
  Description: Scope leak fixture for propagate-wrapped owned string bound inside a lexical loop scope.
##
entry main = f(args: string[]): void errors BuilderFinishedError, AllocationFailureError =>
    let mutable tick: int64 = 0
    while tick < 1:
        let builder: StringBuilder = new StringBuilder
        propagate string_builder_push(builder, 'hello')
        let rendered = propagate string_builder_finish(builder)
        print(rendered)
        tick = tick + 1
    return void
";

    run_scope_leak_test_case(
        "scope_leak_propagated_string_local",
        "scope_leak_propagated_string_local",
        source,
    );
}

#[test]
#[serial(fs)]
fn scope_leak_nested_if_else() {
    let source = "
import string_join from standard

##
  Description: Scope leak red fixture for nested if/else branch cleanup.
##
entry main = f(args: string[]): void =>
    let values: string[] = ['nested']
    if values.length > 0:
        if values.length > 1:
            let branch: string[] = ['inner', 'if']
            let joined = string_join(branch, '-')
            print(joined)
        else:
            let branch: string[] = ['inner', 'else']
            let joined = string_join(branch, '-')
            print(joined)
    else:
        let branch: string[] = ['outer', 'else']
        let joined = string_join(branch, '-')
        print(joined)
    return void
";

    run_scope_leak_test_case(
        "scope_leak_nested_if_else",
        "scope_leak_nested_if_else",
        source,
    );
}
