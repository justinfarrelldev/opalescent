#![cfg(feature = "integration")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

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

fn run_opal_source(source: &std::path::Path) -> std::process::Output {
    let _guard = array_test_lock()
        .lock()
        .expect("array integration lock should not be poisoned");
    let binary = binary_path();
    Command::new(&binary)
        .arg("run")
        .arg(source)
        .output()
        .expect("opalescent run command should spawn and complete")
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
    Command::new(&binary)
        .arg("check")
        .arg(source)
        .output()
        .expect("opalescent check command should spawn and complete")
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
    fn array_push_runs() {
        assert_stdout("array-push", &read_expected_stdout("array-push"));
    }

    #[test]
    fn array_push_on_immutable_receiver_fails_at_compile_time() {
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
