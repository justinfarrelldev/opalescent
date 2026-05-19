#![cfg(feature = "integration")]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("opalescent")
}

fn array_project_src(project: &str, filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join(project)
        .join("src")
        .join(filename)
}

fn array_project_expected(project: &str, filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join(project)
        .join("expected")
        .join(filename)
}

fn array_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn run_opal_source(source: &std::path::Path) -> std::process::Output {
    let _guard = array_test_lock()
        .lock()
        .expect("array integration lock should not be poisoned");
    let binary = binary_path();
    let child = Command::new(&binary)
        .arg("run")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("opalescent run command should spawn and complete");

    opalescent::bounded_proc::wait_for_child_output_with_timeout(
        child,
        GENERATED_BINARY_TEST_TIMEOUT,
        "array integration opalescent run command",
    )
    .expect("opalescent run command should complete")
}

fn run_opal_project(project: &str) -> std::process::Output {
    let source = array_project_src(project, "main.op");
    run_opal_source(&source)
}

fn assert_stdout(project: &str, expected: &str) {
    let output = run_opal_project(project);
    assert!(
        output.status.success(),
        "{project} integration run should exit successfully, got: {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let raw_stdout = String::from_utf8_lossy(&output.stdout);
    let actual = raw_stdout
        .strip_prefix("target/program\n")
        .unwrap_or_else(|| raw_stdout.as_ref());
    assert_eq!(
        actual, expected,
        "{project} stdout should match expected output"
    );
}

fn run_opal_check(source: &std::path::Path) -> std::process::Output {
    let _guard = array_test_lock()
        .lock()
        .expect("array integration lock should not be poisoned");
    let binary = binary_path();
    let child = Command::new(&binary)
        .arg("check")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("opalescent check command should spawn and complete");

    opalescent::bounded_proc::wait_for_child_output_with_timeout(
        child,
        GENERATED_BINARY_TEST_TIMEOUT,
        "array integration opalescent check command",
    )
    .expect("opalescent check command should complete")
}

fn write_temp_project_source(project_name: &str, source: &str) -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("tempdir for temporary array fixture");
    let project_root = temp_dir.path();
    fs::create_dir_all(project_root.join("src"))
        .expect("temporary fixture should create src directory");
    fs::write(
        project_root.join("opal.toml"),
        format!("name = \"{project_name}\"\nversion = \"0.1.0\"\n"),
    )
    .expect("temporary fixture should write opal.toml");
    fs::write(project_root.join("src").join("main.op"), source)
        .expect("temporary fixture should write source file");
    temp_dir
}

fn read_expected_stdout(project: &str) -> String {
    fs::read_to_string(array_project_expected(project, "stdout.txt"))
        .expect("expected stdout fixture should be readable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_append_runs() {
        assert_stdout("array-append", &read_expected_stdout("array-append"));
    }

    #[test]
    fn array_pair_runs() {
        assert_stdout("array-pair", &read_expected_stdout("array-pair"));
    }

    #[test]
    fn array_zip_runs() {
        assert_stdout("array-zip", &read_expected_stdout("array-zip"));
    }

    #[test]
    fn array_double_runs() {
        assert_stdout("array-double", &read_expected_stdout("array-double"));
    }

    #[test]
    fn array_double_nested_out_of_bounds_reports_row_length() {
        let temp_dir = write_temp_project_source(
            "array-double-nested-bounds",
            "##\n  Description: Verifies nested array bounds checks use the inner row length.\n##\nentry main = f(args: string[]): void =>\n    let jagged: int32[][] = [[1 as int32, 2 as int32], [], [3 as int32, 4 as int32, 5 as int32]]\n    let value = jagged[1][0]\n    print('value {value}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        assert!(
            !output.status.success() || combined.contains("index 0 is out of bounds for length 0"),
            "nested bounds fixture should report a runtime bounds error, stdout/stderr: {combined}"
        );
        assert!(
            combined.contains("index 0 is out of bounds for length 0"),
            "nested bounds output should mention the inner row length, stdout/stderr: {combined}"
        );
    }

    #[test]
    fn array_zip_equal_lengths() {
        let temp_dir = write_temp_project_source(
            "array-zip-equal",
            "##\n  Description: Verifies zip preserves all pairs when both arrays have equal length.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: string[] = ['a', 'b']\n    let pairs = left.zip(right)\n    print('length {pairs.length}')\n    print('pair0 {pairs[0].first} {pairs[0].second}')\n    print('pair1 {pairs[1].first} {pairs[1].second}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "equal-length zip fixture should exit successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "length 2\npair0 1 a\npair1 2 b\n",
            "equal-length zip stdout should match"
        );
    }

    #[test]
    fn array_zip_empty_side() {
        for (project_name, source) in [
            (
                "array-zip-empty-left",
                "##\n  Description: Verifies zip returns empty output when the left array is empty.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = []\n    let right: string[] = ['a', 'b']\n    let pairs = left.zip(right)\n    print('length {pairs.length}')\n    return void\n",
            ),
            (
                "array-zip-empty-right",
                "##\n  Description: Verifies zip returns empty output when the right array is empty.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: string[] = []\n    let pairs = left.zip(right)\n    print('length {pairs.length}')\n    return void\n",
            ),
        ] {
            let temp_dir = write_temp_project_source(project_name, source);
            let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
            assert!(
                output.status.success(),
                "{project_name} fixture should exit successfully, stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let raw_stdout = String::from_utf8_lossy(&output.stdout);
            let actual = raw_stdout
                .strip_prefix("target/program\n")
                .unwrap_or_else(|| raw_stdout.as_ref());
            assert_eq!(actual, "length 0\n", "{project_name} stdout should match");
        }
    }

    #[test]
    fn array_push_runs() {
        assert_stdout("array-push", &read_expected_stdout("array-push"));
    }

    #[test]
    fn array_push_cow_alias() {
        let temp_dir = write_temp_project_source(
            "array-push-cow-alias",
            "##\n  Description: Verifies push uses alias-preserving COW rebinding.\n##\nentry main = f(args: string[]): void =>\n    let base: int32[] = [1 as int32, 2 as int32]\n    let mutable grown = base\n    grown.push(3 as int32)\n    print('base length {base.length}')\n    print('base values {base[0]} {base[1]}')\n    print('grown length {grown.length}')\n    print('grown values {grown[0]} {grown[1]} {grown[2]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "push COW alias fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base length 2\nbase values 1 2\ngrown length 3\ngrown values 1 2 3\n",
            "push COW alias output should preserve the alias and rebind only the mutable receiver"
        );
    }

    #[test]
    fn array_push_immutable_rejected() {
        let temp_dir = write_temp_project_source(
            "array-push-immutable",
            "##\n  Description: Verifies immutable array push is rejected at compile time.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32]\n    xs.push(1 as int32)\n    return void\n",
        );
        let project_root = temp_dir.path();

        let output = run_opal_check(&project_root.join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "immutable push fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("mutable") && stderr.contains("push"),
            "immutable push diagnostic should mention mutable receiver and push, stderr: {stderr}"
        );
    }

    #[test]
    fn array_push_on_immutable_receiver_fails_at_compile_time() {
        array_push_immutable_rejected();
    }

    #[test]
    fn array_push_cannot_be_used_as_a_value() {
        let temp_dir = write_temp_project_source(
            "array-push-void-misuse",
            "##\n  Description: Verifies array push remains a void-returning expression.\n##\nentry main = f(args: string[]): void =>\n    let mutable xs: int32[] = []\n    let length_after_push: int32 = xs.push(1 as int32)\n    print('length {length_after_push}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "push used as a value should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            (stderr.contains("Cannot unify types") || stderr.contains("Type mismatch"))
                && ((stderr.contains("void") || stderr.contains("unit"))
                    && stderr.contains("int32")),
            "push value misuse should mention void/unit versus int32 mismatch, stderr: {stderr}"
        );
    }

    #[test]
    fn array_index_assignment() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment",
            "##\n  Description: Verifies identifier-backed indexed assignment lowers via COW rebinding.\n##\nentry main = f(args: string[]): void =>\n    let mutable xs: int32[] = [1 as int32, 2 as int32, 3 as int32]\n    xs[1] = 9 as int32\n    print('length {xs.length}')\n    print('values {xs[0]} {xs[1]} {xs[2]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "indexed assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "length 3\nvalues 1 9 3\n");
    }

    #[test]
    fn array_index_assignment_cow_alias() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment-cow-alias",
            "##\n  Description: Verifies indexed assignment only rebinds the mutable identifier.\n##\nentry main = f(args: string[]): void =>\n    let base: int32[] = [1 as int32, 2 as int32, 3 as int32]\n    let mutable xs = base\n    xs[1] = 9 as int32\n    xs[0] = 7 as int32\n    print('base length {base.length}')\n    print('base values {base[0]} {base[1]} {base[2]}')\n    print('xs length {xs.length}')\n    print('xs values {xs[0]} {xs[1]} {xs[2]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "indexed assignment COW alias fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base length 3\nbase values 1 2 3\nxs length 3\nxs values 7 9 3\n"
        );
    }

    #[test]
    fn array_index_assignment_rc_nested_row_rebind() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment-rc-nested-row-rebind",
            "##\n  Description: Verifies indexed assignment handles RC-backed nested array elements via COW rebinding.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: int32[] = [8 as int32, 9 as int32]\n    let base: int32[][] = [left, left]\n    let mutable xs = base\n    xs[1] = right\n    print('base left {base[0][0]} {base[0][1]}')\n    print('base right {base[1][0]} {base[1][1]}')\n    print('xs left {xs[0][0]} {xs[0][1]}')\n    print('xs right {xs[1][0]} {xs[1][1]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "indexed assignment nested-row fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base left 1 2\nbase right 1 2\nxs left 1 2\nxs right 8 9\n"
        );
    }

    #[test]
    fn array_index_assignment_unsupported_target_rejected() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment-unsupported-target",
            "##\n  Description: Verifies non-identifier indexed assignment targets are rejected.\n##\nentry main = f(args: string[]): void =>\n    let mutable rows: int32[][] = [[1 as int32, 2 as int32], [3 as int32, 4 as int32]]\n    rows[0][0] = 7 as int32\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "unsupported indexed-assignment target should fail during compile/run"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("indexed assignment currently requires identifier array receiver"),
            "unsupported target diagnostic should mention identifier-only indexed assignment, stderr: {stderr}"
        );
    }

    #[test]
    fn array_append_type_mismatch_fails_at_check_time() {
        let temp_dir = write_temp_project_source(
            "array-append-type-mismatch",
            "import append from standard\n\n##\n  Description: Verifies append rejects incompatible element types.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32]\n    let grown = append(xs, 'x')\n    print('grown length {grown.length}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "append type mismatch fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Cannot unify types 'int32' and 'string'")
                || (stderr.contains("int32")
                    && stderr.contains("string")
                    && stderr.contains("incompatible")),
            "append type mismatch diagnostic should mention incompatible element types, stderr: {stderr}"
        );
    }

    #[test]
    fn array_filled() {
        let temp_dir = write_temp_project_source(
            "array-filled",
            "import array_filled from standard\n\n##\n  Description: Verifies array_filled allocates len=cap=length and repeats values.\n##\nentry main = f(args: string[]): void =>\n    let values: int32[] = array_filled(3 as int64, 7 as int32)\n    print('length {values.length}')\n    print('values {values[0]} {values[1]} {values[2]}')\n    let row: int32[] = [9 as int32]\n    let nested: int32[][] = array_filled(2 as int64, row)\n    print('nested length {nested.length}')\n    print('nested values {nested[0][0]} {nested[1][0]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_filled fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "length 3\nvalues 7 7 7\nnested length 2\nnested values 9 9\n"
        );
    }

    #[test]
    fn array_reserve() {
        let temp_dir = write_temp_project_source(
            "array-reserve",
            "import reserve from standard\n\n##\n  Description: Verifies reserve is functional and preserves source aliases.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32, 2 as int32]\n    let reserved: int32[] = reserve(xs, 10 as int64)\n    print('xs length {xs.length}')\n    print('xs values {xs[0]} {xs[1]}')\n    print('reserved length {reserved.length}')\n    print('reserved values {reserved[0]} {reserved[1]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_reserve fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "xs length 2\nxs values 1 2\nreserved length 2\nreserved values 1 2\n"
        );
    }

    #[test]
    fn array_clear() {
        let temp_dir = write_temp_project_source(
            "array-clear",
            "import clear from standard\n\n##\n  Description: Verifies clear returns fresh len=0 array and keeps source unchanged.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [4 as int32, 5 as int32, 6 as int32]\n    let emptied: int32[] = clear(xs)\n    print('xs length {xs.length}')\n    print('xs values {xs[0]} {xs[1]} {xs[2]}')\n    print('emptied length {emptied.length}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_clear fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "xs length 3\nxs values 4 5 6\nemptied length 0\n");
    }

    #[test]
    fn array_rc_elements() {
        let temp_dir = write_temp_project_source(
            "array-rc-elements",
            "import append, array_filled, reserve, clear from standard\n\n##\n  Description: Verifies RC-bearing nested-array elements survive literal, append, push, array_filled, reserve, and clear paths.\n##\nentry main = f(args: string[]): void =>\n    let row: int32[] = [7 as int32]\n    let literal: int32[][] = [row]\n    let appended: int32[][] = append(literal, row)\n    let mutable pushed: int32[][] = literal\n    pushed.push(row)\n    let filled: int32[][] = array_filled(2 as int64, row)\n    let reserved: int32[][] = reserve(appended, 6 as int64)\n    let cleared: int32[][] = clear(reserved)\n    print('literal {literal.length} {literal[0][0]}')\n    print('appended {appended.length} {appended[0][0]} {appended[1][0]}')\n    print('pushed {pushed.length} {pushed[0][0]} {pushed[1][0]}')\n    print('filled {filled.length} {filled[0][0]} {filled[1][0]}')\n    print('reserved {reserved.length} {reserved[0][0]} {reserved[1][0]}')\n    print('cleared {cleared.length}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_rc_elements fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "literal 1 7\nappended 2 7 7\npushed 2 7 7\nfilled 2 7 7\nreserved 2 7 7\ncleared 0\n"
        );
    }

    #[test]
    fn array_memory_churn_sanitizer_fixture() {
        let temp_dir = write_temp_project_source(
            "array-memory-churn-sanitizer-fixture",
            "import append, array_filled, reserve, clear from standard\n\n##\n  Description: Exercises RC array churn paths for sanitizer coverage: append, push, indexed overwrite, nested arrays, array_filled, reserve, and clear.\n##\nentry main = f(args: string[]): void =>\n    let row_a: int32[] = [1 as int32]\n    let row_b: int32[] = [9 as int32]\n    let base: int32[][] = [row_a, row_a]\n    let appended: int32[][] = append(base, row_b)\n    let mutable pushed: int32[][] = appended\n    pushed.push(row_a)\n    pushed[1] = row_b\n    let filled: int32[][] = array_filled(2 as int64, row_a)\n    let reserved: int32[][] = reserve(pushed, 8 as int64)\n    let cleared: int32[][] = clear(reserved)\n    print('base {base.length} {base[0][0]} {base[1][0]}')\n    print('appended {appended.length} {appended[0][0]} {appended[1][0]} {appended[2][0]}')\n    print('pushed {pushed.length} {pushed[0][0]} {pushed[1][0]} {pushed[2][0]} {pushed[3][0]}')\n    print('filled {filled.length} {filled[0][0]} {filled[1][0]}')\n    print('reserved {reserved.length} {reserved[0][0]} {reserved[1][0]} {reserved[2][0]} {reserved[3][0]}')\n    print('cleared {cleared.length}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array memory churn sanitizer fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base 2 1 1\nappended 3 1 1 9\npushed 4 1 9 9 1\nfilled 2 1 1\nreserved 4 1 9 9 1\ncleared 0\n"
        );
    }

    #[test]
    fn array_nested_rc_drop() {
        let temp_dir = write_temp_project_source(
            "array-nested-rc-drop",
            "##\n  Description: Verifies nested RC-backed child arrays survive until parent-array drop at program exit.\n##\nentry main = f(args: string[]): void =>\n    let child_a: int32[] = [1 as int32, 2 as int32]\n    let child_b: int32[] = [3 as int32, 4 as int32]\n    let rows: int32[][] = [child_a, child_b]\n    print('rows {rows.length} {rows[0][0]} {rows[1][1]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_nested_rc_drop fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "rows 2 1 4\n");
    }

    #[test]
    fn array_index_assignment_rc_elements() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment-rc-elements",
            "##\n  Description: Verifies identifier-backed indexed assignment preserves RC-bearing nested arrays across overwrite.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = [1 as int32]\n    let middle: int32[] = [2 as int32]\n    let right: int32[] = [3 as int32]\n    let mutable rows: int32[][] = [left, middle]\n    rows[1] = right\n    print('rows {rows.length} {rows[0][0]} {rows[1][0]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array_index_assignment_rc_elements fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "rows 2 1 3\n");
    }

    #[test]
    fn array_pop_runs() {
        assert_stdout("array-pop", &read_expected_stdout("array-pop"));
    }

    #[test]
    fn array_pop_on_empty_array_traps() {
        let temp_dir = write_temp_project_source(
            "array-pop-empty",
            "##
  Description: Verifies empty array pop traps with a clear runtime error.
##
entry main = f(args: string[]): void =>
    let mutable xs: int32[] = []
    let popped = xs.pop()
    print('popped {popped}')
    return void
",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "empty pop fixture should exit with a runtime failure"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("pop on empty array"),
            "empty pop stderr should mention the runtime trap, stderr: {stderr}"
        );
    }

    #[test]
    fn array_map_runs() {
        assert_stdout("array-map", &read_expected_stdout("array-map"));
    }

    #[test]
    fn array_map_empty_returns_empty_array() {
        let temp_dir = write_temp_project_source(
            "array-map-empty",
            "##\n  Description: Verifies array map over an empty input yields an empty output.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = []\n    let out = xs.map(f(x: int32): int32 => x * (2 as int32))\n    print('length {out.length}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "empty map fixture should exit successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "length 0\n", "empty map stdout should match");
    }

    #[test]
    fn array_map_callback_return_mismatch_fails_at_check_time() {
        let temp_dir = write_temp_project_source(
            "array-map-type-mismatch",
            "##\n  Description: Verifies array map result type stays tied to the callback return type.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32]\n    let doubled: int32[] = xs.map(f(x: int32): string => 'wrong type')\n    print('length {doubled.length}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "map callback return mismatch fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            (stderr.contains("Cannot unify types") || stderr.contains("Type mismatch"))
                && stderr.contains("int32")
                && stderr.contains("string"),
            "map callback mismatch diagnostic should mention int32 and string, stderr: {stderr}"
        );
    }

    #[test]
    fn array_filter_runs() {
        assert_stdout("array-filter", &read_expected_stdout("array-filter"));
    }

    #[test]
    fn array_filter_all_pass_preserves_order() {
        let temp_dir = write_temp_project_source(
            "array-filter-all-pass",
            "##\n  Description: Verifies array filter keeps all elements in original order when every predicate check passes.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32, 2 as int32, 3 as int32, 4 as int32]\n    let all_values = xs.filter(f(x: int32): boolean => x > (0 as int32))\n    print('length {all_values.length}')\n    print('values {all_values[0]} {all_values[1]} {all_values[2]} {all_values[3]}')\n    print('source length {xs.length}')\n    print('source values {xs[0]} {xs[1]} {xs[2]} {xs[3]}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "all-pass filter fixture should exit successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "length 4\nvalues 1 2 3 4\nsource length 4\nsource values 1 2 3 4\n",
            "all-pass filter should preserve order and leave the source array unchanged"
        );
    }

    #[test]
    fn array_filter_empty_input_returns_empty_array() {
        let temp_dir = write_temp_project_source(
            "array-filter-empty",
            "##\n  Description: Verifies array filter over an empty input yields an empty output.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = []\n    let out = xs.filter(f(x: int32): boolean => x > (0 as int32))\n    print('length {out.length}')\n    print('source length {xs.length}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "empty filter fixture should exit successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "length 0\nsource length 0\n",
            "empty filter stdout should match"
        );
    }

    #[test]
    fn array_filter_non_boolean_predicate_fails_at_check_time() {
        let temp_dir = write_temp_project_source(
            "array-filter-non-boolean",
            "##\n  Description: Verifies array filter rejects predicates that do not return boolean values.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32, 2 as int32]\n    let filtered = xs.filter(f(x: int32): int32 => x)\n    print('length {filtered.length}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "non-boolean filter predicate fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("filter") && stderr.contains("boolean"),
            "filter predicate diagnostic should mention filter and boolean, stderr: {stderr}"
        );
    }

    #[test]
    fn array_reduce_runs() {
        assert_stdout("array-reduce", &read_expected_stdout("array-reduce"));
    }

    #[test]
    fn array_reduce_accumulator_mismatch_fails_at_check_time() {
        let temp_dir = write_temp_project_source(
            "array-reduce-accumulator-mismatch",
            "##\n  Description: Verifies array reduce rejects reducer return types that do not match the seeded accumulator.\n##\nentry main = f(args: string[]): void =>\n    let xs: int32[] = [1 as int32, 2 as int32]\n    let sum = xs.reduce(0 as int32, f(acc: int32, x: int32): string => 'wrong type')\n    print('sum {sum}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "reduce accumulator mismatch fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            ((stderr.contains("Cannot unify types") || stderr.contains("Type mismatch"))
                && stderr.contains("int32")
                && stderr.contains("string"))
                || (stderr.contains("reduce")
                    && stderr.contains("int32")
                    && stderr.contains("string")),
            "reduce accumulator mismatch diagnostic should mention int32 and string, stderr: {stderr}"
        );
    }
}
