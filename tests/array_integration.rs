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

fn generated_binary_test_timeout() -> Duration {
    if std::env::var_os("ASAN_OPTIONS").is_some() || std::env::var_os("LSAN_OPTIONS").is_some() {
        return Duration::from_secs(120);
    }
    Duration::from_secs(30)
}

fn project_root_for_source(source: &std::path::Path) -> PathBuf {
    source
        .ancestors()
        .find(|candidate| candidate.join("opal.toml").is_file())
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            std::path::Path::to_path_buf,
        )
}

fn run_opal_source(source: &std::path::Path) -> std::process::Output {
    let _guard = array_test_lock()
        .lock()
        .expect("array integration lock should not be poisoned");
    let binary = binary_path();
    let project_root = project_root_for_source(source);
    let child = Command::new(&binary)
        .current_dir(project_root)
        .arg("run")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("opalescent run command should spawn and complete");

    opalescent::bounded_proc::wait_for_child_output_with_timeout(
        child,
        generated_binary_test_timeout(),
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
        generated_binary_test_timeout(),
        "array integration opalescent check command",
    )
    .expect("opalescent check command should complete")
}

fn temp_fixture_root() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".sisyphus")
        .join("tmp")
        .join("array-integration-fixtures");
    fs::create_dir_all(&root).expect("array integration temp fixture root should exist");
    root
}

fn write_temp_project_source(project_name: &str, source: &str) -> tempfile::TempDir {
    let temp_dir = tempfile::Builder::new()
        .prefix(project_name)
        .tempdir_in(temp_fixture_root())
        .expect("tempdir for temporary array fixture");
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
    fn array_append_unique_input_pure() {
        let temp_dir = write_temp_project_source(
            "array-append-unique-input-pure",
            "import append from standard\n\n##\n  Description: Verifies append returns a new array without mutating a unique receiver.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let xs: int32[] = [1 as int32, 2 as int32]\n    let grown = append(xs, 3 as int32)\n    print('xs length {xs.length}')\n    print('xs values {propagate xs.at(0)} {propagate xs.at(1)}')\n    print('grown length {grown.length}')\n    print('grown values {propagate grown.at(0)} {propagate grown.at(1)} {propagate grown.at(2)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "unique append fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "xs length 2\nxs values 1 2\ngrown length 3\ngrown values 1 2 3\n"
        );
    }

    #[test]
    fn array_append_shared_input_pure() {
        let temp_dir = write_temp_project_source(
            "array-append-shared-input-pure",
            "import append from standard\n\n##\n  Description: Verifies append leaves all shared aliases unchanged.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let base: int32[] = [4 as int32, 5 as int32]\n    let shared = base\n    let grown = append(base, 6 as int32)\n    print('base length {base.length}')\n    print('base values {propagate base.at(0)} {propagate base.at(1)}')\n    print('shared length {shared.length}')\n    print('shared values {propagate shared.at(0)} {propagate shared.at(1)}')\n    print('grown length {grown.length}')\n    print('grown values {propagate grown.at(0)} {propagate grown.at(1)} {propagate grown.at(2)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "shared append fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base length 2\nbase values 4 5\nshared length 2\nshared values 4 5\ngrown length 3\ngrown values 4 5 6\n"
        );
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
            "##\n  Description: Verifies nested array .at(...) uses the inner row length semantics.\n##\nentry main = f(args: string[]): void =>\n    let jagged: int32[][] = [[1 as int32, 2 as int32], [], [3 as int32, 4 as int32, 5 as int32]]\n    let row: int32[] = guard jagged.at(1) into found_row: int32[] else []\n    let value: int32 = guard row.at(0) into found_value: int32 else -1 as int32\n    print('value {value}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested .at(...) bounds fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "value -1\n",
            "nested .at(...) fixture should use the inner row length and fall back for the empty row"
        );
    }

    #[test]
    fn array_zip_equal_lengths() {
        let temp_dir = write_temp_project_source(
            "array-zip-equal",
            "##\n  Description: Verifies zip preserves all pairs when both arrays have equal length.\n##\nentry main = f(args: string[]): void =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: string[] = ['a', 'b']\n    let pairs = left.zip(right)\n    let mutable pair_index: int64 = 0\n    print('length {pairs.length}')\n    for pair in pairs:\n        if pair_index is 0:\n            print('pair0 {pair.first} {pair.second}')\n        if pair_index is 1:\n            print('pair1 {pair.first} {pair.second}')\n        pair_index = pair_index + 1\n    return void\n",
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
    fn ordinary_multiple_return_destructure_and_pass_through_runs() {
        let temp_dir = write_temp_project_source(
            "ordinary-multiple-return-destructure",
            "##\n  Description: Verifies ordinary labeled multi-return calls destructure safely and pass through return values unchanged.\n##\nlet pair = f(): x: int32, y: int32 =>\n    return x: 7 as int32, y: 9 as int32\n\nlet forward = f(): x: int32, y: int32 =>\n    return pair()\n\n##\n  Description: Exercises direct destructuring and return pass-through for ordinary labeled multi-return calls.\n##\nentry main = f(args: string[]): void =>\n    let x, y = forward()\n    let x: left, y: right = pair()\n    print('{x} {y}')\n    print('{left} {right}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "ordinary multi-return fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "7 9\n7 9\n");
    }

    #[test]
    fn ordinary_multiple_return_reversed_explicit_labels_rejected() {
        let temp_dir = write_temp_project_source(
            "ordinary-multiple-return-reversed-labels",
            "##\n  Description: Verifies explicit multi-return labels validate positional intent and never reorder.\n##\nlet pair = f(): x: int32, y: int32 =>\n    return x: 1 as int32, y: 2 as int32\n\n##\n  Description: Ensures reversed explicit labels fail before ordinary multi-return code generation runs.\n##\nentry main = f(args: string[]): void =>\n    let y: y_val, x: x_val = pair()\n    print('{y_val} {x_val}')\n    return void\n",
        );
        let output = run_opal_check(&temp_dir.path().join("src").join("main.op"));
        assert!(
            !output.status.success(),
            "reversed explicit-label fixture should fail compilation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("do not reorder"),
            "reversed explicit-label diagnostic should mention non-reordering intent, stderr: {stderr}"
        );
    }

    #[test]
    fn array_push_runs() {
        assert_stdout("array-push", &read_expected_stdout("array-push"));
    }

    #[test]
    fn array_push_cow_alias() {
        let temp_dir = write_temp_project_source(
            "array-push-cow-alias",
            "##\n  Description: Verifies push uses alias-preserving COW rebinding.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let base: int32[] = [1 as int32, 2 as int32]\n    let mutable grown = base\n    grown.push(3 as int32)\n    print('base length {base.length}')\n    print('base values {propagate base.at(0)} {propagate base.at(1)}')\n    print('grown length {grown.length}')\n    print('grown values {propagate grown.at(0)} {propagate grown.at(1)} {propagate grown.at(2)}')\n    return void\n",
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
            actual, "base length 2\nbase values 1 2\ngrown length 3\ngrown values 1 2 3\n",
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
    fn array_index_assignment_unique_in_place() {
        // Indexed assignment remains a supported compile-pass regression while legacy bracket reads are retired separately.
        let temp_dir = write_temp_project_source(
            "array-index-assignment-unique-in-place",
            "##\n  Description: Verifies identifier-backed indexed assignment mutates in-place when the receiver is uniquely owned.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let mutable xs: int32[] = [1 as int32, 2 as int32, 3 as int32]\n    xs[1] = 9 as int32\n    print('length {xs.length}')\n    print('values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)}')\n    return void\n",
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
            "##\n  Description: Verifies indexed assignment only rebinds the mutable identifier.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let base: int32[] = [1 as int32, 2 as int32, 3 as int32]\n    let mutable xs = base\n    xs[1] = 9 as int32\n    xs[0] = 7 as int32\n    print('base length {base.length}')\n    print('base values {propagate base.at(0)} {propagate base.at(1)} {propagate base.at(2)}')\n    print('xs length {xs.length}')\n    print('xs values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)}')\n    return void\n",
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
            "##\n  Description: Verifies indexed assignment handles RC-backed nested array elements via COW rebinding.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: int32[] = [8 as int32, 9 as int32]\n    let base: int32[][] = [left, left]\n    let mutable xs = base\n    xs[1] = right\n    let base_left: int32[] = propagate base.at(0)\n    let base_right: int32[] = propagate base.at(1)\n    let xs_left: int32[] = propagate xs.at(0)\n    let xs_right: int32[] = propagate xs.at(1)\n    print('base left {propagate base_left.at(0)} {propagate base_left.at(1)}')\n    print('base right {propagate base_right.at(0)} {propagate base_right.at(1)}')\n    print('xs left {propagate xs_left.at(0)} {propagate xs_left.at(1)}')\n    print('xs right {propagate xs_right.at(0)} {propagate xs_right.at(1)}')\n    return void\n",
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
    fn array_self_assignment_rc_safe() {
        let temp_dir = write_temp_project_source(
            "array-self-assignment-rc-safe",
            "##\n  Description: Verifies self-assignment keeps RC-bearing array binding stable and value-preserving.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let row: int32[] = [4 as int32]\n    let mutable rows: int32[][] = [row]\n    rows = rows\n    let row0: int32[] = propagate rows.at(0)\n    print('rows length {rows.length}')\n    print('rows first {propagate row0.at(0)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "self-assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "rows length 1\nrows first 4\n");
    }

    #[test]
    fn array_rebind_releases_old_preserves_alias() {
        let temp_dir = write_temp_project_source(
            "array-rebind-releases-old-preserves-alias",
            "##\n  Description: Verifies rebinding a mutable array variable releases old binding while preserving aliases.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let base: int32[] = [1 as int32, 2 as int32]\n    let mutable xs = base\n    let replacement: int32[] = [7 as int32, 8 as int32, 9 as int32]\n    xs = replacement\n    print('base length {base.length}')\n    print('base values {propagate base.at(0)} {propagate base.at(1)}')\n    print('xs length {xs.length}')\n    print('xs values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "rebind alias fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base length 2\nbase values 1 2\nxs length 3\nxs values 7 8 9\n"
        );
    }

    #[test]
    fn array_param_local_alias_mutation_rc_safe() {
        let temp_dir = write_temp_project_source(
            "array-param-local-alias-mutation-rc-safe",
            "##\n  Description: Verifies function parameter/local array alias mutation uses COW rebinding and preserves caller aliases.\n##\n##\n  Description: Mutates a local alias of the parameter and prints local values.\n##\nlet grow_once = f(source: int32[]): void errors IndexOutOfBoundsError =>\n    let mutable local = source\n    local.push(3 as int32)\n    print('local length {local.length}')\n    print('local values {propagate local.at(0)} {propagate local.at(1)} {propagate local.at(2)}')\n    return void\n\n##\n  Description: Calls grow_once then prints the original array alias.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let base: int32[] = [1 as int32, 2 as int32]\n    propagate grow_once(base)\n    print('base length {base.length}')\n    print('base values {propagate base.at(0)} {propagate base.at(1)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "parameter/local alias mutation fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "local length 3\nlocal values 1 2 3\nbase length 2\nbase values 1 2\n"
        );
    }

    #[test]
    fn array_nested_read_returns_correct_jagged_value() {
        let temp_dir = write_temp_project_source(
            "array-nested-read-returns-correct-jagged-value",
            "##\n  Description: Verifies jagged nested array reads load the selected row before loading the selected cell.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let rows: int32[][] = [[1 as int32], [4 as int32, 5 as int32, 6 as int32], [9 as int32, 8 as int32]]\n    let row1: int32[] = propagate rows.at(1)\n    let row2: int32[] = propagate rows.at(2)\n    print('row1col2 {propagate row1.at(2)}')\n    print('row2col0 {propagate row2.at(0)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested read fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "row1col2 6\nrow2col0 9\n");
    }

    #[test]
    fn array_nested_assignment_unique_row() {
        let temp_dir = write_temp_project_source(
            "array-nested-assignment-unique-row",
            "##\n  Description: Verifies nested indexed assignment mutates a unique outer array and unique inner row in place.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let mutable rows: int32[][] = [[1 as int32, 2 as int32], [3 as int32, 4 as int32, 5 as int32]]\n    rows[1][1] = 9 as int32\n    let row0: int32[] = propagate rows.at(0)\n    let row1: int32[] = propagate rows.at(1)\n    print('row0 {propagate row0.at(0)} {propagate row0.at(1)}')\n    print('row1 {propagate row1.at(0)} {propagate row1.at(1)} {propagate row1.at(2)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested unique-row assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "row0 1 2\nrow1 3 9 5\n");
    }

    #[test]
    fn array_nested_assignment_shared_inner_row_cow() {
        let temp_dir = write_temp_project_source(
            "array-nested-assignment-shared-inner-row-cow",
            "##\n  Description: Verifies nested indexed assignment clones the inner row before writing when the row is shared inside a unique outer array.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let shared: int32[] = [1 as int32, 2 as int32, 3 as int32]\n    let mutable rows: int32[][] = [shared, shared]\n    rows[1][1] = 9 as int32\n    let row0: int32[] = propagate rows.at(0)\n    let row1: int32[] = propagate rows.at(1)\n    print('row0 {propagate row0.at(0)} {propagate row0.at(1)} {propagate row0.at(2)}')\n    print('row1 {propagate row1.at(0)} {propagate row1.at(1)} {propagate row1.at(2)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested shared-inner-row assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "row0 1 2 3\nrow1 1 9 3\n");
    }

    #[test]
    fn array_nested_assignment_shared_outer_unique_inner_row_cow() {
        let temp_dir = write_temp_project_source(
            "array-nested-assignment-shared-outer-unique-inner-row-cow",
            "##\n  Description: Verifies nested indexed assignment clones through the outer array when the outer array is shared even if the selected inner row is unique.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let left: int32[] = [1 as int32, 2 as int32]\n    let right: int32[] = [7 as int32, 8 as int32]\n    let base: int32[][] = [left, right]\n    let mutable rows = base\n    rows[1][0] = 5 as int32\n    let base_row1: int32[] = propagate base.at(1)\n    let rows_row1: int32[] = propagate rows.at(1)\n    print('base row1 {propagate base_row1.at(0)} {propagate base_row1.at(1)}')\n    print('rows row1 {propagate rows_row1.at(0)} {propagate rows_row1.at(1)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested shared-outer assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(actual, "base row1 7 8\nrows row1 5 8\n");
    }

    #[test]
    fn array_nested_assignment_shared_outer_and_inner_cow() {
        let temp_dir = write_temp_project_source(
            "array-nested-assignment-shared-outer-and-inner-cow",
            "##\n  Description: Verifies nested indexed assignment preserves aliases when both the outer array and selected inner row are shared.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let shared: int32[] = [1 as int32, 2 as int32]\n    let base: int32[][] = [shared, shared]\n    let mutable rows = base\n    rows[1][0] = 6 as int32\n    let base_row0: int32[] = propagate base.at(0)\n    let base_row1: int32[] = propagate base.at(1)\n    let rows_row0: int32[] = propagate rows.at(0)\n    let rows_row1: int32[] = propagate rows.at(1)\n    print('base row0 {propagate base_row0.at(0)} {propagate base_row0.at(1)}')\n    print('base row1 {propagate base_row1.at(0)} {propagate base_row1.at(1)}')\n    print('rows row0 {propagate rows_row0.at(0)} {propagate rows_row0.at(1)}')\n    print('rows row1 {propagate rows_row1.at(0)} {propagate rows_row1.at(1)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "nested shared-outer-and-inner assignment fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "base row0 1 2\nbase row1 1 2\nrows row0 1 2\nrows row1 6 2\n"
        );
    }

    #[test]
    fn array_index_assignment_unsupported_target_rejected() {
        let temp_dir = write_temp_project_source(
            "array-index-assignment-unsupported-target",
            "##\n  Description: Verifies non-identifier indexed assignment targets are rejected.\n##\nentry main = f(args: string[]): void =>\n    [[1 as int32, 2 as int32], [3 as int32, 4 as int32]][0 as int32][0 as int32] = 7 as int32\n    return void\n",
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
            "import array_filled from standard\n\n##\n  Description: Verifies array_filled allocates len=cap=length and repeats values.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let values: int32[] = array_filled(3 as int64, 7 as int32)\n    let nested_row0 = propagate values.at(0)\n    let nested_row1 = propagate values.at(1)\n    print('length {values.length}')\n    print('values {propagate values.at(0)} {propagate values.at(1)} {propagate values.at(2)}')\n    let row: int32[] = [9 as int32]\n    let nested: int32[][] = array_filled(2 as int64, row)\n    let nested0: int32[] = propagate nested.at(0)\n    let nested1: int32[] = propagate nested.at(1)\n    print('nested length {nested.length}')\n    print('nested values {propagate nested0.at(0)} {propagate nested1.at(0)}')\n    return void\n",
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
            "import reserve from standard\n\n##\n  Description: Verifies reserve is functional and preserves source aliases.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let xs: int32[] = [1 as int32, 2 as int32]\n    let reserved: int32[] = reserve(xs, 10 as int64)\n    print('xs length {xs.length}')\n    print('xs values {propagate xs.at(0)} {propagate xs.at(1)}')\n    print('reserved length {reserved.length}')\n    print('reserved values {propagate reserved.at(0)} {propagate reserved.at(1)}')\n    return void\n",
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
            "import clear from standard\n\n##\n  Description: Verifies clear returns fresh len=0 array and keeps source unchanged.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let xs: int32[] = [4 as int32, 5 as int32, 6 as int32]\n    let emptied: int32[] = clear(xs)\n    print('xs length {xs.length}')\n    print('xs values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)}')\n    print('emptied length {emptied.length}')\n    return void\n",
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
            "import append, array_filled, reserve, clear from standard\n\n##\n  Description: Verifies RC-bearing nested-array elements survive literal, append, push, array_filled, reserve, and clear paths.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let row: int32[] = [7 as int32]\n    let literal: int32[][] = [row]\n    let appended: int32[][] = append(literal, row)\n    let mutable pushed: int32[][] = literal\n    pushed.push(row)\n    let filled: int32[][] = array_filled(2 as int64, row)\n    let reserved: int32[][] = reserve(appended, 6 as int64)\n    let cleared: int32[][] = clear(reserved)\n    let literal_row: int32[] = propagate literal.at(0)\n    let appended_row0: int32[] = propagate appended.at(0)\n    let appended_row1: int32[] = propagate appended.at(1)\n    let pushed_row0: int32[] = propagate pushed.at(0)\n    let pushed_row1: int32[] = propagate pushed.at(1)\n    let filled_row0: int32[] = propagate filled.at(0)\n    let filled_row1: int32[] = propagate filled.at(1)\n    let reserved_row0: int32[] = propagate reserved.at(0)\n    let reserved_row1: int32[] = propagate reserved.at(1)\n    print('literal {literal.length} {propagate literal_row.at(0)}')\n    print('appended {appended.length} {propagate appended_row0.at(0)} {propagate appended_row1.at(0)}')\n    print('pushed {pushed.length} {propagate pushed_row0.at(0)} {propagate pushed_row1.at(0)}')\n    print('filled {filled.length} {propagate filled_row0.at(0)} {propagate filled_row1.at(0)}')\n    print('reserved {reserved.length} {propagate reserved_row0.at(0)} {propagate reserved_row1.at(0)}')\n    print('cleared {cleared.length}')\n    return void\n",
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
            "import append, array_filled, reserve, clear from standard\n\n##\n  Description: Exercises RC array churn paths for sanitizer coverage: append, push, indexed overwrite, nested arrays, array_filled, reserve, and clear.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let row_a: int32[] = [1 as int32]\n    let row_b: int32[] = [9 as int32]\n    let base: int32[][] = [row_a, row_a]\n    let appended: int32[][] = append(base, row_b)\n    let mutable pushed: int32[][] = appended\n    pushed.push(row_a)\n    pushed[1] = row_b\n    let filled: int32[][] = array_filled(2 as int64, row_a)\n    let reserved: int32[][] = reserve(pushed, 8 as int64)\n    let cleared: int32[][] = clear(reserved)\n    let base_row0: int32[] = propagate base.at(0)\n    let base_row1: int32[] = propagate base.at(1)\n    let appended_row0: int32[] = propagate appended.at(0)\n    let appended_row1: int32[] = propagate appended.at(1)\n    let appended_row2: int32[] = propagate appended.at(2)\n    let pushed_row0: int32[] = propagate pushed.at(0)\n    let pushed_row1: int32[] = propagate pushed.at(1)\n    let pushed_row2: int32[] = propagate pushed.at(2)\n    let pushed_row3: int32[] = propagate pushed.at(3)\n    let filled_row0: int32[] = propagate filled.at(0)\n    let filled_row1: int32[] = propagate filled.at(1)\n    let reserved_row0: int32[] = propagate reserved.at(0)\n    let reserved_row1: int32[] = propagate reserved.at(1)\n    let reserved_row2: int32[] = propagate reserved.at(2)\n    let reserved_row3: int32[] = propagate reserved.at(3)\n    print('base {base.length} {propagate base_row0.at(0)} {propagate base_row1.at(0)}')\n    print('appended {appended.length} {propagate appended_row0.at(0)} {propagate appended_row1.at(0)} {propagate appended_row2.at(0)}')\n    print('pushed {pushed.length} {propagate pushed_row0.at(0)} {propagate pushed_row1.at(0)} {propagate pushed_row2.at(0)} {propagate pushed_row3.at(0)}')\n    print('filled {filled.length} {propagate filled_row0.at(0)} {propagate filled_row1.at(0)}')\n    print('reserved {reserved.length} {propagate reserved_row0.at(0)} {propagate reserved_row1.at(0)} {propagate reserved_row2.at(0)} {propagate reserved_row3.at(0)}')\n    print('cleared {cleared.length}')\n    return void\n",
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
    fn array_game_of_life_churn_sanitizer_fixture() {
        let temp_dir = write_temp_project_source(
            "array-game-of-life-churn-sanitizer-fixture",
            "##\n  Description: Exercises Game-of-Life-style two-board churn with indexed updates and board swapping for sanitizer coverage.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let mutable board: int32[] = [0 as int32, 1 as int32, 0 as int32, 0 as int32, 0 as int32, 1 as int32, 1 as int32, 1 as int32, 1 as int32]\n    let mutable next: int32[] = [0 as int32, 0 as int32, 0 as int32, 0 as int32, 0 as int32, 0 as int32, 0 as int32, 0 as int32, 0 as int32]\n    let mutable tick: int32 = 0 as int32\n    while tick < (10 as int32):\n        next[0] = propagate board.at(2)\n        next[1] = propagate board.at(1)\n        next[2] = propagate board.at(0)\n        next[3] = propagate board.at(5)\n        next[4] = propagate board.at(4)\n        next[5] = propagate board.at(3)\n        next[6] = propagate board.at(8)\n        next[7] = propagate board.at(7)\n        next[8] = propagate board.at(6)\n        let swap = board\n        board = next\n        next = swap\n        tick = tick + (1 as int32)\n    print('board {propagate board.at(0)} {propagate board.at(1)} {propagate board.at(2)}')\n    print('board {propagate board.at(3)} {propagate board.at(4)} {propagate board.at(5)}')\n    print('board {propagate board.at(6)} {propagate board.at(7)} {propagate board.at(8)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "array Game-of-Life churn sanitizer fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "board 0 1 0\nboard 0 0 1\nboard 1 1 1\n",
            "Game-of-Life churn fixture should preserve original board after even mirror swaps"
        );
    }

    #[test]
    fn array_nested_rc_drop() {
        let temp_dir = write_temp_project_source(
            "array-nested-rc-drop",
            "##\n  Description: Verifies nested RC-backed child arrays survive until parent-array drop at program exit.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let child_a: int32[] = [1 as int32, 2 as int32]\n    let child_b: int32[] = [3 as int32, 4 as int32]\n    let rows: int32[][] = [child_a, child_b]\n    let row0: int32[] = propagate rows.at(0)\n    let row1: int32[] = propagate rows.at(1)\n    print('rows {rows.length} {propagate row0.at(0)} {propagate row1.at(1)}')\n    return void\n",
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
            "##\n  Description: Verifies identifier-backed indexed assignment preserves RC-bearing nested arrays across overwrite.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let left: int32[] = [1 as int32]\n    let middle: int32[] = [2 as int32]\n    let right: int32[] = [3 as int32]\n    let mutable rows: int32[][] = [left, middle]\n    rows[1] = right\n    let row0: int32[] = propagate rows.at(0)\n    let row1: int32[] = propagate rows.at(1)\n    print('rows {rows.length} {propagate row0.at(0)} {propagate row1.at(0)}')\n    return void\n",
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
    fn array_push_unique_reuses_capacity() {
        let temp_dir = write_temp_project_source(
            "array-push-unique-reuses-capacity",
            "import reserve from standard\n\n##\n  Description: Verifies unique push can consume reserved capacity without changing values.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let mutable xs: int32[] = [1 as int32, 2 as int32]\n    xs = reserve(xs, 8 as int64)\n    xs.push(3 as int32)\n    xs.push(4 as int32)\n    print('length {xs.length}')\n    print('values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)} {propagate xs.at(3)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "unique push reserve fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "length 4\nvalues 1 2 3 4\n",
            "unique push reserve fixture should append in order"
        );
    }

    #[test]
    fn array_pop_rc_element_ownership_transfer() {
        let temp_dir = write_temp_project_source(
            "array-pop-rc-element-ownership-transfer",
            "##\n  Description: Verifies popped RC-bearing row remains valid after receiver rebind and clear.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let left: int32[] = [7 as int32]\n    let right: int32[] = [9 as int32]\n    let mutable rows: int32[][] = [left, right]\n    let popped: int32[] = rows.pop()\n    rows = []\n    print('rows length {rows.length}')\n    print('popped length {popped.length}')\n    print('popped value {propagate popped.at(0)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "pop RC ownership fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual, "rows length 0\npopped length 1\npopped value 9\n",
            "popped RC-bearing element should stay alive after receiver mutation"
        );
    }

    #[test]
    fn array_reserve_noop_when_within_capacity() {
        let temp_dir = write_temp_project_source(
            "array-reserve-noop-when-within-capacity",
            "import reserve from standard\n\n##\n  Description: Verifies reserve with <= current capacity keeps logical contents unchanged.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let mutable xs: int32[] = [1 as int32, 2 as int32]\n    xs = reserve(xs, 8 as int64)\n    let same_cap: int32[] = reserve(xs, 8 as int64)\n    let lower_cap: int32[] = reserve(xs, 0 as int64)\n    print('xs length {xs.length}')\n    print('same length {same_cap.length}')\n    print('lower length {lower_cap.length}')\n    print('same values {propagate same_cap.at(0)} {propagate same_cap.at(1)}')\n    print('lower values {propagate lower_cap.at(0)} {propagate lower_cap.at(1)}')\n    return void\n",
        );
        let output = run_opal_source(&temp_dir.path().join("src").join("main.op"));
        assert!(
            output.status.success(),
            "reserve noop fixture should run successfully, stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let actual = raw_stdout
            .strip_prefix("target/program\n")
            .unwrap_or_else(|| raw_stdout.as_ref());
        assert_eq!(
            actual,
            "xs length 2\nsame length 2\nlower length 2\nsame values 1 2\nlower values 1 2\n",
            "reserve noop fixture should preserve values for requested <= cap"
        );
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
            "##\n  Description: Verifies array filter keeps all elements in original order when every predicate check passes.\n##\nentry main = f(args: string[]): void errors IndexOutOfBoundsError =>\n    let xs: int32[] = [1 as int32, 2 as int32, 3 as int32, 4 as int32]\n    let all_values = xs.filter(f(x: int32): boolean => x > (0 as int32))\n    print('length {all_values.length}')\n    print('values {propagate all_values.at(0)} {propagate all_values.at(1)} {propagate all_values.at(2)} {propagate all_values.at(3)}')\n    print('source length {xs.length}')\n    print('source values {propagate xs.at(0)} {propagate xs.at(1)} {propagate xs.at(2)} {propagate xs.at(3)}')\n    return void\n",
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
