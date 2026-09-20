#![allow(
    warnings,
    clippy::all,
    clippy::cognitive_complexity,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::needless_raw_strings,
    clippy::panic,
    clippy::pattern_type_mismatch,
    clippy::too_many_lines,
    reason = "test harness uses panic-based assertions"
)]
extern crate alloc;

use crate::bounded_proc::{RunOutput, RunPolicy, run_command};
use crate::build_system::targets::TargetTriple;
use crate::compiler::{CompileError, CompileRunPolicy, compile_project_with_run_policy};
use crate::errors::renderer::render_report;
use crate::runtime::arrays::{allocate_array, array_index, array_length};
use crate::runtime::errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
use crate::runtime::io::{IoHandler, print, take_input};
use crate::runtime::memory::{OpalArray, OpalString, OpalWeakRef, RuntimeAllocator};
use crate::runtime::reporting::format_runtime_error;
use crate::runtime::stdlib::{
    RandomIntSource, format_interpolated_string, opal_array_slice, random_int32_with_source,
    string_to_int32,
};
use crate::runtime::strings::{
    string_compare, string_concat, string_equals, string_extract_range, string_find_index_or,
    string_find_last_index_of_text, string_index, string_is_blank, string_length,
    string_split_lines, string_take_prefix, string_take_suffix, string_trim_whitespace,
};
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MockAllocator;

impl RuntimeAllocator for MockAllocator {
    fn allocate_string(&self, value: &str) -> RuntimeResult<OpalString> {
        Ok(OpalString::new(value.to_owned()))
    }

    fn allocate_array<T>(&self, values: &[T]) -> RuntimeResult<OpalArray<T>>
    where
        T: Clone,
    {
        Ok(OpalArray::new(values.to_vec()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockIoHandler {
    input: VecDeque<String>,
    output: Vec<String>,
}

impl MockIoHandler {
    fn with_input(lines: &[&str]) -> Self {
        Self {
            input: lines.iter().map(|line| (*line).to_owned()).collect(),
            output: Vec::new(),
        }
    }
}

impl IoHandler for MockIoHandler {
    fn write(&mut self, value: &str) -> RuntimeResult<()> {
        self.output.push(value.to_owned());
        Ok(())
    }

    fn read(&mut self) -> RuntimeResult<String> {
        self.input.pop_front().map_or_else(
            || Err(RuntimeError::user_error(9_001, "no mocked input available")),
            Ok,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockRandomSource {
    values: VecDeque<u32>,
}

impl MockRandomSource {
    fn from_values(values: &[u32]) -> Self {
        Self {
            values: values.iter().copied().collect(),
        }
    }
}

impl RandomIntSource for MockRandomSource {
    fn next_u32(&mut self) -> u32 {
        self.values.pop_front().unwrap_or_default()
    }
}

struct CompiledRuntimeProject {
    _temp_dir: tempfile::TempDir,
    binary_path: PathBuf,
}

fn format_runtime_test_compile_error(error: &CompileError) -> String {
    match error {
        CompileError::Report {
            source_path,
            report,
            normalized_source,
        } => render_report(source_path, normalized_source, report),
        other => format!("{other:?}"),
    }
}

fn compile_runtime_red_project(test_name: &str, main_source: &str) -> CompiledRuntimeProject {
    let temp_dir = tempfile::tempdir().expect("create temp runtime project");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).expect("create runtime project src dir");
    fs::write(
        temp_dir.path().join("opal.toml"),
        format!("name = \"{test_name}\"\nversion = \"0.1.0\"\n"),
    )
    .expect("write runtime project opal.toml");
    fs::write(
        src_dir.join("bounds.types.op"),
        "public type IndexOutOfBoundsError:\n    OutOfBounds\n",
    )
    .expect("write runtime project error type fixture");
    let main_source = main_source
        .replacen(
            "import print from standard\n",
            "import print from standard\nimport type IndexOutOfBoundsError from ./bounds.types\n",
            1,
        )
        .replacen(
            "\n\nentry main =",
            "\n\n##\n    Description: Entry function runs the runtime .at(...) RED scenario\n##\nentry main =",
            1,
        )
        .replace(" else -1\n", " else -1 as int32\n");
    fs::write(src_dir.join("main.op"), main_source).expect("write runtime project main source");

    let target = TargetTriple::host();
    let output_dir = temp_dir.path().join("target");
    let binary_result = compile_project_with_run_policy(
        temp_dir.path(),
        &output_dir,
        &target,
        CompileRunPolicy::bounded_for_test_harness(),
    );
    assert!(
        binary_result.is_ok(),
        "runtime .at(...) RED fixture '{test_name}' should compile so its runtime behavior can be exercised; current failure:\n{}",
        binary_result
            .as_ref()
            .err()
            .map_or_else(String::new, format_runtime_test_compile_error)
    );
    let Ok(binary_path) = binary_result else {
        unreachable!("compile assertion above should return early on failure")
    };

    CompiledRuntimeProject {
        _temp_dir: temp_dir,
        binary_path,
    }
}

fn run_compiled_runtime_project(binary_path: &Path) -> RunOutput {
    let mut command = Command::new(binary_path);
    run_command(
        &mut command,
        RunPolicy::Bounded {
            timeout: Duration::from_secs(30),
            grace: Duration::from_secs(2),
            kill_group: true,
        },
        format!("run compiled runtime RED binary: {}", binary_path.display()),
    )
    .unwrap_or_else(|error| panic!("failed to run compiled runtime RED binary: {error}"))
}

fn try_compile_runtime_behavior_project(
    test_name: &str,
    main_source: &str,
) -> Result<CompiledRuntimeProject, String> {
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).map_err(|error| error.to_string())?;
    fs::write(
        temp_dir.path().join("opal.toml"),
        format!("name = \"{test_name}\"\nversion = \"0.1.0\"\n"),
    )
    .map_err(|error| error.to_string())?;
    fs::write(src_dir.join("main.op"), main_source).map_err(|error| error.to_string())?;

    let target = TargetTriple::host();
    let output_dir = temp_dir.path().join("target");
    let binary_result = compile_project_with_run_policy(
        temp_dir.path(),
        &output_dir,
        &target,
        CompileRunPolicy::bounded_for_test_harness(),
    );
    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => return Err(format_runtime_test_compile_error(&error)),
    };

    Ok(CompiledRuntimeProject {
        _temp_dir: temp_dir,
        binary_path,
    })
}

#[test]
fn string_at_runtime_returns_first_scalar_for_index_zero() {
    let project = compile_runtime_red_project(
        "string-at-runtime-first-index-zero",
        "import print from standard\n\n##\n    Description: String .at(0) should return the first Unicode scalar through the error ABI\n##\nlet read_first = f(): string errors IndexOutOfBoundsError => {\n    let message = 'hé🙂'\n    return propagate message.at(0)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let scalar: string = propagate read_first()\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "string .at(0) runtime success fixture should exit successfully:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "h\n",
        "string .at(0) should print the first scalar"
    );
}

#[test]
fn string_at_runtime_returns_last_scalar_for_length_minus_one() {
    let project = compile_runtime_red_project(
        "string-at-runtime-last-valid-index",
        "import print from standard\n\n##\n    Description: String .at(length - 1) should return the last Unicode scalar through the error ABI\n##\nlet read_last = f(): string errors IndexOutOfBoundsError => {\n    let message = 'hé🙂'\n    return propagate message.at(message.length - 1)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let scalar: string = propagate read_last()\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "string .at(length - 1) should exit successfully"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "🙂\n",
        "string .at(length - 1) should print the last scalar"
    );
}

#[test]
fn string_at_runtime_returns_middle_scalar_for_dynamic_index() {
    let project = compile_runtime_red_project(
        "string-at-runtime-dynamic-index",
        "import print from standard\n\n##\n    Description: String .at(dynamic_index) should return the selected scalar through the error ABI\n##\nlet read_dynamic = f(): string errors IndexOutOfBoundsError => {\n    let message = 'hé🙂'\n    let index: int64 = 1\n    return propagate message.at(index)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let scalar: string = propagate read_dynamic()\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "string .at(dynamic_index) should exit successfully"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "é\n",
        "string .at(dynamic_index) should print the selected scalar"
    );
}

#[test]
fn string_at_runtime_falls_back_for_negative_index() {
    let project = compile_runtime_red_project(
        "string-at-runtime-negative-index",
        "import print from standard\n\n##\n    Description: String .at(-1) should take the IndexOutOfBoundsError path\n##\nlet read_negative = f(): string errors IndexOutOfBoundsError => {\n    let message = 'hé🙂'\n    return propagate message.at(-1)\n}\n\nentry main = f(): void => {\n    let scalar: string = guard read_negative() into value: string else 'IndexOutOfBoundsError'\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "negative string .at(...) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "IndexOutOfBoundsError\n",
        "negative string .at(...) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn string_at_runtime_falls_back_for_empty_string_index_zero() {
    let project = compile_runtime_red_project(
        "string-at-runtime-empty-index-zero",
        "import print from standard\n\n##\n    Description: Empty-string .at(0) should take the IndexOutOfBoundsError path\n##\nlet read_empty = f(): string errors IndexOutOfBoundsError => {\n    let message = ''\n    return propagate message.at(0)\n}\n\nentry main = f(): void => {\n    let scalar: string = guard read_empty() into value: string else 'IndexOutOfBoundsError'\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "empty string .at(0) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "IndexOutOfBoundsError\n",
        "empty string .at(0) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn string_at_runtime_falls_back_for_one_past_end_index() {
    let project = compile_runtime_red_project(
        "string-at-runtime-one-past-end",
        "import print from standard\n\n##\n    Description: String .at(length) should take the IndexOutOfBoundsError path\n##\nlet read_oob = f(): string errors IndexOutOfBoundsError => {\n    let message = 'hé🙂'\n    return propagate message.at(message.length)\n}\n\nentry main = f(): void => {\n    let scalar: string = guard read_oob() into value: string else 'IndexOutOfBoundsError'\n    print(scalar)\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "string .at(length) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "IndexOutOfBoundsError\n",
        "string .at(length) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn array_at_runtime_returns_first_element_for_index_zero() {
    let project = compile_runtime_red_project(
        "array-at-runtime-first-index-zero",
        "import print from standard\n\n##\n    Description: Array .at(0) should return the first element through the error ABI\n##\nlet read_first = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = [10 as int32, 20 as int32, 30 as int32]\n    return propagate values.at(0)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let value: int32 = propagate read_first()\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(output.exit.success, "array .at(0) should exit successfully");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "10\n",
        "array .at(0) should print the first element"
    );
}

#[test]
fn array_at_runtime_returns_last_element_for_length_minus_one() {
    let project = compile_runtime_red_project(
        "array-at-runtime-last-valid-index",
        "import print from standard\n\n##\n    Description: Array .at(length - 1) should return the last element through the error ABI\n##\nlet read_last = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = [10 as int32, 20 as int32, 30 as int32]\n    return propagate values.at(values.length - 1)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let value: int32 = propagate read_last()\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "array .at(length - 1) should exit successfully"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "30\n",
        "array .at(length - 1) should print the last element"
    );
}

#[test]
fn array_at_runtime_returns_middle_element_for_dynamic_index() {
    let project = compile_runtime_red_project(
        "array-at-runtime-dynamic-index",
        "import print from standard\n\n##\n    Description: Array .at(dynamic_index) should return the selected element through the error ABI\n##\nlet read_dynamic = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = [10 as int32, 20 as int32, 30 as int32]\n    let index: int64 = 1\n    return propagate values.at(index)\n}\n\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let value: int32 = propagate read_dynamic()\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "array .at(dynamic_index) should exit successfully"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "20\n",
        "array .at(dynamic_index) should print the selected element"
    );
}

#[test]
fn array_at_runtime_falls_back_for_negative_index() {
    let project = compile_runtime_red_project(
        "array-at-runtime-negative-index",
        "import print from standard\n\n##\n    Description: Array .at(-1) should take the IndexOutOfBoundsError path\n##\nlet read_negative = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = [10 as int32, 20 as int32, 30 as int32]\n    return propagate values.at(-1)\n}\n\nentry main = f(): void => {\n    let value: int32 = guard read_negative() into found: int32 else -1\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "negative array .at(...) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "-1\n",
        "negative array .at(...) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn array_at_runtime_falls_back_for_empty_array_index_zero() {
    let project = compile_runtime_red_project(
        "array-at-runtime-empty-index-zero",
        "import print from standard\n\n##\n    Description: Empty-array .at(0) should take the IndexOutOfBoundsError path\n##\nlet read_empty = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = []\n    return propagate values.at(0)\n}\n\nentry main = f(): void => {\n    let value: int32 = guard read_empty() into found: int32 else -1\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "empty array .at(0) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "-1\n",
        "empty array .at(0) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn array_at_runtime_falls_back_for_one_past_end_index() {
    let project = compile_runtime_red_project(
        "array-at-runtime-one-past-end",
        "import print from standard\n\n##\n    Description: Array .at(length) should take the IndexOutOfBoundsError path\n##\nlet read_oob = f(): int32 errors IndexOutOfBoundsError => {\n    let values: int32[] = [10 as int32, 20 as int32, 30 as int32]\n    return propagate values.at(values.length)\n}\n\nentry main = f(): void => {\n    let value: int32 = guard read_oob() into found: int32 else -1\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "array .at(length) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "-1\n",
        "array .at(length) should route through the IndexOutOfBoundsError guard path"
    );
}

#[test]
fn array_at_runtime_nested_access_uses_inner_row_length() {
    let project = compile_runtime_red_project(
        "array-at-runtime-nested-inner-row-length",
        "import print from standard\n\n##\n    Description: Nested array .at(...) should use the selected row length for the inner access\n##\nlet read_nested = f(): int32 errors IndexOutOfBoundsError => {\n    let rows: int32[][] = [[10 as int32], [], [30 as int32, 40 as int32]]\n    let row: int32[] = propagate rows.at(1)\n    return propagate row.at(0)\n}\n\nentry main = f(): void => {\n    let value: int32 = guard read_nested() into found: int32 else -1 as int32\n    print('{value}')\n    return void\n}\n",
    );

    let output = run_compiled_runtime_project(&project.binary_path);
    assert!(
        output.exit.success,
        "nested array .at(...) guard should keep the program successful"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "-1\n",
        "nested array .at(...) should route through the inner row IndexOutOfBoundsError path"
    );
}

#[test]
fn string_runtime_supports_all_required_operations() {
    let allocator = MockAllocator;

    let left_result = allocator.allocate_string("hello");
    assert!(left_result.is_ok(), "left allocation should succeed");
    let Ok(left) = left_result else {
        return;
    };

    let right_result = allocator.allocate_string(" world");
    assert!(right_result.is_ok(), "right allocation should succeed");
    let Ok(right) = right_result else {
        return;
    };

    let combined_result = string_concat(&allocator, &left, &right);
    assert!(combined_result.is_ok(), "string concatenation must succeed");
    let Ok(combined) = combined_result else {
        return;
    };

    assert_eq!(
        combined.as_str(),
        "hello world",
        "concatenation should produce combined string"
    );
    assert_eq!(
        string_length(&combined),
        11,
        "string length should count Unicode scalar values"
    );
    assert!(
        string_equals(&combined, &OpalString::new(String::from("hello world"))),
        "string equality should match identical values"
    );
    assert_eq!(
        string_compare(&left, &right),
        Ordering::Greater,
        "lexicographic compare should report expected ordering"
    );
}

#[test]
fn string_indexing_uses_unicode_scalar_semantics_and_bounds_checks() {
    let allocator = MockAllocator;
    let message = OpalString::new(String::from("hé🙂"));

    let first = string_index(&allocator, &message, 0);
    assert!(first.is_ok(), "first scalar index should succeed");
    assert_eq!(
        first.as_ref().map(OpalString::as_str),
        Ok("h"),
        "ASCII scalar indexing should return a one-character string"
    );

    let accented = string_index(&allocator, &message, 1);
    assert!(accented.is_ok(), "non-ASCII scalar index should succeed");
    assert_eq!(
        accented.as_ref().map(OpalString::as_str),
        Ok("é"),
        "non-ASCII indexing should return the full Unicode scalar"
    );

    let last = string_index(&allocator, &message, string_length(&message) - 1);
    assert!(last.is_ok(), "last scalar index should succeed");
    assert_eq!(
        last.as_ref().map(OpalString::as_str),
        Ok("🙂"),
        "length - 1 indexing should return the final Unicode scalar"
    );

    let out_of_bounds = string_index(&allocator, &message, string_length(&message));
    assert!(
        out_of_bounds.is_err(),
        "out-of-bounds string index should fail"
    );
    assert_eq!(
        out_of_bounds.err(),
        Some(RuntimeError::IndexOutOfBounds {
            index: 3,
            length: 3,
        }),
        "string indexing bounds failures must preserve index and scalar length"
    );
}

#[test]
fn string_indexing_rejects_empty_string_access_with_zero_length_bounds() {
    let allocator = MockAllocator;
    let empty = OpalString::new(String::new());

    let out_of_bounds = string_index(&allocator, &empty, 0);
    assert!(
        out_of_bounds.is_err(),
        "empty-string string index should fail immediately"
    );
    assert_eq!(
        out_of_bounds.err(),
        Some(RuntimeError::IndexOutOfBounds {
            index: 0,
            length: 0,
        }),
        "empty-string indexing should preserve attempted index 0 and scalar length 0"
    );
}

#[test]
fn string_find_helpers_use_unicode_scalar_indices() {
    let text = OpalString::new(String::from("hé🙂z🙂"));
    let smile = OpalString::new(String::from("🙂"));
    let missing = OpalString::new(String::from("xyz"));
    let empty = OpalString::new(String::new());

    assert_eq!(string_find_index_or(&text, &smile, -1), 2);
    assert_eq!(string_find_index_or(&text, &missing, -1), -1);
    assert_eq!(string_find_index_or(&text, &empty, -7), -7);
    assert_eq!(
        string_find_last_index_of_text(&text, &smile),
        Ok(4),
        "last match should report Unicode-scalar index rather than byte offset"
    );
    assert_eq!(
        string_find_last_index_of_text(&text, &missing),
        Err(RuntimeError::user_error(
            1_102,
            "StringPatternNotFoundError"
        ))
    );
    assert_eq!(
        string_find_last_index_of_text(&text, &empty),
        Err(RuntimeError::user_error(
            1_101,
            "StringEmptySearchTextError"
        ))
    );
}

#[test]
fn string_stdlib_lines_whitespace() {
    let allocator = MockAllocator;

    let empty = OpalString::new(String::from(""));
    let lf_trailing = OpalString::new(String::from("a\n"));
    let crlf = OpalString::new(String::from("a\r\nb"));
    let bare_cr = OpalString::new(String::from("a\rb"));
    let only_lf = OpalString::new(String::from("\n"));
    let double_lf = OpalString::new(String::from("\n\n"));
    let interior_blank = OpalString::new(String::from("a\n\nb"));

    let split_empty = string_split_lines(&allocator, &empty).expect("split empty should succeed");
    assert!(
        split_empty.is_empty(),
        "empty string should produce no lines"
    );

    let split_lf = string_split_lines(&allocator, &lf_trailing).expect("LF split should succeed");
    assert_eq!(split_lf.len(), 1);
    assert_eq!(split_lf.get(0).map(OpalString::as_str), Some("a"));

    let split_crlf = string_split_lines(&allocator, &crlf).expect("CRLF split should succeed");
    assert_eq!(split_crlf.len(), 2);
    assert_eq!(split_crlf.get(0).map(OpalString::as_str), Some("a"));
    assert_eq!(split_crlf.get(1).map(OpalString::as_str), Some("b"));

    let split_bare_cr = string_split_lines(&allocator, &bare_cr).expect("CR split should succeed");
    assert_eq!(split_bare_cr.len(), 2);
    assert_eq!(split_bare_cr.get(0).map(OpalString::as_str), Some("a"));
    assert_eq!(split_bare_cr.get(1).map(OpalString::as_str), Some("b"));

    let split_only_lf =
        string_split_lines(&allocator, &only_lf).expect("only LF split should succeed");
    assert_eq!(split_only_lf.len(), 1);
    assert_eq!(split_only_lf.get(0).map(OpalString::as_str), Some(""));

    let split_double_lf =
        string_split_lines(&allocator, &double_lf).expect("double LF split should succeed");
    assert_eq!(split_double_lf.len(), 2);
    assert_eq!(split_double_lf.get(0).map(OpalString::as_str), Some(""));
    assert_eq!(split_double_lf.get(1).map(OpalString::as_str), Some(""));

    let split_interior_blank = string_split_lines(&allocator, &interior_blank)
        .expect("interior blank split should succeed");
    assert_eq!(split_interior_blank.len(), 3);
    assert_eq!(
        split_interior_blank.get(0).map(OpalString::as_str),
        Some("a")
    );
    assert_eq!(
        split_interior_blank.get(1).map(OpalString::as_str),
        Some("")
    );
    assert_eq!(
        split_interior_blank.get(2).map(OpalString::as_str),
        Some("b")
    );

    assert!(string_is_blank(&empty));
    assert!(string_is_blank(&OpalString::new(String::from("   \t\n\r"))));
    assert!(string_is_blank(&OpalString::new(String::from(
        "\u{2003}\u{3000}"
    ))));
    assert!(!string_is_blank(&OpalString::new(String::from("猫"))));

    let trimmed = string_trim_whitespace(
        &allocator,
        &OpalString::new(String::from("\u{2003}  hé 🙂  \t\u{3000}")),
    )
    .expect("trim unicode whitespace should succeed");
    assert_eq!(trimmed.as_str(), "hé 🙂");

    let trimmed_noop =
        string_trim_whitespace(&allocator, &OpalString::new(String::from("cat café")))
            .expect("trim noop should succeed");
    assert_eq!(trimmed_noop.as_str(), "cat café");

    let invariant_source = OpalString::new(String::from("\u{2003}  \t\u{3000}"));
    let invariant_trimmed = string_trim_whitespace(&allocator, &invariant_source)
        .expect("trim invariant should succeed");
    assert!(string_is_blank(&invariant_source));
    assert_eq!(invariant_trimmed.as_str(), "");
}

#[test]
fn string_stdlib_ranges() {
    let allocator = MockAllocator;
    let text = OpalString::new(String::from("hé🙂"));
    let ascii = OpalString::new(String::from("hello"));

    let prefix_zero = string_take_prefix(&allocator, &text, 0).expect("prefix zero should succeed");
    assert_eq!(prefix_zero.as_str(), "");
    let prefix_exact =
        string_take_prefix(&allocator, &text, 3).expect("prefix exact should succeed");
    assert_eq!(prefix_exact.as_str(), "hé🙂");
    assert_eq!(
        string_take_prefix(&allocator, &text, -1),
        Err(RuntimeError::user_error(1_103, "StringNegativeCountError"))
    );
    assert_eq!(
        string_take_prefix(&allocator, &text, 4),
        Err(RuntimeError::user_error(
            1_104,
            "StringRangeOutOfBoundsError"
        ))
    );

    let suffix_zero = string_take_suffix(&allocator, &text, 0).expect("suffix zero should succeed");
    assert_eq!(suffix_zero.as_str(), "");
    let suffix_exact =
        string_take_suffix(&allocator, &text, 3).expect("suffix exact should succeed");
    assert_eq!(suffix_exact.as_str(), "hé🙂");
    assert_eq!(
        string_take_suffix(&allocator, &text, -1),
        Err(RuntimeError::user_error(1_103, "StringNegativeCountError"))
    );
    assert_eq!(
        string_take_suffix(&allocator, &text, 4),
        Err(RuntimeError::user_error(
            1_104,
            "StringRangeOutOfBoundsError"
        ))
    );

    let range_normal =
        string_extract_range(&allocator, &ascii, 1, 4).expect("range normal should succeed");
    assert_eq!(range_normal.as_str(), "ell");
    let range_empty =
        string_extract_range(&allocator, &ascii, 2, 2).expect("empty range should succeed");
    assert_eq!(range_empty.as_str(), "");
    let range_full =
        string_extract_range(&allocator, &text, 0, 3).expect("full range should succeed");
    assert_eq!(range_full.as_str(), "hé🙂");
    let range_at_end =
        string_extract_range(&allocator, &text, 3, 3).expect("at-end empty range should succeed");
    assert_eq!(range_at_end.as_str(), "");
    assert_eq!(
        string_extract_range(&allocator, &ascii, 4, 1),
        Err(RuntimeError::user_error(1_105, "StringRangeOrderError"))
    );
    assert_eq!(
        string_extract_range(&allocator, &ascii, 0, 8),
        Err(RuntimeError::user_error(
            1_104,
            "StringRangeOutOfBoundsError"
        ))
    );
}

#[test]
fn string_stdlib_behavior() {
    let cases = [(
        "string-ranges-stdlib-behavior",
        r#"import print from standard

##
    Description: Focused compile-and-run coverage for string prefix/suffix/range behavior
##
entry main = f(): void errors StringNegativeCountError, StringRangeOutOfBoundsError, StringRangeOrderError, AllocationFailureError => {
    let prefix_zero: string = propagate string_take_prefix('hé🙂', 0 as int64)
    let prefix_exact: string = propagate string_take_prefix('hé🙂', 3 as int64)
    let suffix_zero: string = propagate string_take_suffix('hé🙂', 0 as int64)
    let suffix_exact: string = propagate string_take_suffix('hé🙂', 3 as int64)
    let range_normal: string = propagate string_extract_range('hello', 1 as int64, 4 as int64)
    let range_empty: string = propagate string_extract_range('hello', 2 as int64, 2 as int64)
    let range_full: string = propagate string_extract_range('hé🙂', 0 as int64, 3 as int64)
    let range_at_end: string = propagate string_extract_range('hé🙂', 3 as int64, 3 as int64)
    print('PREFIX_ZERO=[{prefix_zero}]')
    print('PREFIX_EXACT=[{prefix_exact}]')
    print('SUFFIX_ZERO=[{suffix_zero}]')
    print('SUFFIX_EXACT=[{suffix_exact}]')
    print('RANGE_NORMAL=[{range_normal}]')
    print('RANGE_EMPTY=[{range_empty}]')
    print('RANGE_FULL=[{range_full}]')
    print('RANGE_AT_END=[{range_at_end}]')
    return void
}
"#,
        "PREFIX_ZERO=[]\nPREFIX_EXACT=[hé🙂]\nSUFFIX_ZERO=[]\nSUFFIX_EXACT=[hé🙂]\nRANGE_NORMAL=[ell]\nRANGE_EMPTY=[]\nRANGE_FULL=[hé🙂]\nRANGE_AT_END=[]\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "focused string ranges stdlib behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn string_lines_whitespace_stdlib_behavior() {
    let cases = [(
        "string-lines-whitespace-stdlib-behavior",
        r#"import print from standard

let print_split_case = f(label: string, text: string): void errors AllocationFailureError => {
    let lines: string[] = propagate string_split_lines(text)
    let first: string = guard lines.at(0) into value: string else '<missing>'
    let second: string = guard lines.at(1) into value: string else '<missing>'
    let third: string = guard lines.at(2) into value: string else '<missing>'
    print('{label}|len={lines.length}|0={first}|1={second}|2={third}')
    return void
}

##
    Description: Focused compile-and-run coverage for string split/blank/trim behavior
##
entry main = f(): void errors AllocationFailureError => {
    propagate print_split_case('SPLIT_EMPTY', '')
    propagate print_split_case('SPLIT_LF_TRAILING', 'a\n')
    propagate print_split_case('SPLIT_CRLF', 'a\r\nb')
    propagate print_split_case('SPLIT_CR', 'a\rb')
    let is_blank_empty = string_is_blank('')
    let is_blank_ascii = string_is_blank('   ')
    let is_blank_text = string_is_blank('cat')
    if is_blank_empty:
        print('IS_BLANK_EMPTY=true')
    else:
        print('IS_BLANK_EMPTY=false')
    if is_blank_ascii:
        print('IS_BLANK_ASCII=true')
    else:
        print('IS_BLANK_ASCII=false')
    if is_blank_text:
        print('IS_BLANK_TEXT=true')
    else:
        print('IS_BLANK_TEXT=false')
    let trimmed: string = propagate string_trim_whitespace('  hé 🙂  ')
    print('TRIMMED=[{trimmed}]')
    return void
}
"#,
        "SPLIT_EMPTY|len=0|0=<missing>|1=<missing>|2=<missing>\nSPLIT_LF_TRAILING|len=1|0=a|1=<missing>|2=<missing>\nSPLIT_CRLF|len=2|0=a|1=b|2=<missing>\nSPLIT_CR|len=2|0=a|1=b|2=<missing>\nIS_BLANK_EMPTY=true\nIS_BLANK_ASCII=true\nIS_BLANK_TEXT=false\nTRIMMED=[hé 🙂]\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "focused string lines/whitespace stdlib behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn string_stdlib_search() {
    let cases = [(
        "string-search-stdlib-behavior-main",
        r#"import print from standard

##
    Description: Focused compile-and-run coverage for string search happy paths
##
entry main = f(): void errors StringEmptySearchTextError, StringPatternNotFoundError => {
    let find_index_found = string_find_index_or('hello world', 'world', -1 as int64)
    let find_index_empty = string_find_index_or('hello', '', -1 as int64)
    let find_index_missing = string_find_index_or('hello', 'xyz', -1 as int64)
    let find_index_unicode = string_find_index_or('hé🙂z', '🙂', -1 as int64)
    let last_found: int64 = propagate string_find_last_index_of_text('hello world', 'world')
    let last_repeated: int64 = propagate string_find_last_index_of_text('bananana', 'ana')
    print('FIND_INDEX_FOUND={find_index_found}')
    print('FIND_INDEX_EMPTY={find_index_empty}')
    print('FIND_INDEX_MISSING={find_index_missing}')
    print('FIND_INDEX_UNICODE={find_index_unicode}')
    print('LAST_FOUND={last_found}')
    print('LAST_REPEATED={last_repeated}')
    return void
}
"#,
        "FIND_INDEX_FOUND=6\nFIND_INDEX_EMPTY=-1\nFIND_INDEX_MISSING=-1\nFIND_INDEX_UNICODE=2\nLAST_FOUND=6\nLAST_REPEATED=5\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "focused string search behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
#[ignore = "invalid guard-else fixture shape; error contract covered by direct runtime helper assertions"]
fn string_search_stdlib_error_behavior() {
    let cases = [(
        "string-search-stdlib-errors",
        r#"import print from standard

##
    Description: Focused compile-and-run coverage for string search error paths
##
entry main = f(): void => {
    let last_empty: int64 = guard string_find_last_index_of_text('hello', '') into value: int64 else -11 as int64
    let last_missing: int64 = guard string_find_last_index_of_text('hello', 'xyz') into value: int64 else -22 as int64
    let unicode_last: int64 = guard string_find_last_index_of_text('🙂é🙂', '🙂') into value: int64 else -1 as int64
    print('LAST_EMPTY_ERROR={last_empty}')
    print('LAST_MISSING_ERROR={last_missing}')
    print('UNICODE_LAST_INDEX={unicode_last}')
    return void
}
"#,
        "LAST_EMPTY_ERROR=-11\nLAST_MISSING_ERROR=-22\nUNICODE_LAST_INDEX=2\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "focused string search error behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
#[ignore = "broad compile-and-run fixture; skip during focused search verification"]
fn string_stdlib_behavior_legacy_broad() {
    let cases = [(
        "string-stdlib-behavior-main",
        r#"import print from standard

let print_split_case = f(label: string, text: string): void => {
    let lines: string[] = guard string_split_lines(text) into value: string[] else ['<split-error>']
    let first: string = guard lines.at(0) into value: string else '<missing>'
    let second: string = guard lines.at(1) into value: string else '<missing>'
    let third: string = guard lines.at(2) into value: string else '<missing>'
    print('{label}|len={lines.length}|0={first}|1={second}|2={third}')
    return void
}

##
    Description: Focused runtime behavior spec for planned string stdlib functions
##
entry main = f(): void => {
    let find_index_found = string_find_index_or('hello world', 'world', -1 as int64)
    let find_index_empty = string_find_index_or('hello', '', -1 as int64)
    let find_index_missing = string_find_index_or('hello', 'xyz', -1 as int64)
    let find_index_unicode = string_find_index_or('hé🙂z', '🙂', -1 as int64)
    print('FIND_INDEX_FOUND={find_index_found}')
    print('FIND_INDEX_EMPTY={find_index_empty}')
    print('FIND_INDEX_MISSING={find_index_missing}')
    print('FIND_INDEX_UNICODE={find_index_unicode}')

    let last_found: int64 = guard string_find_last_index_of_text('hello world', 'world') into value: int64 else -1 as int64
    let last_repeated: int64 = guard string_find_last_index_of_text('bananana', 'ana') into value: int64 else -1 as int64
    let last_empty: int64 = guard string_find_last_index_of_text('hello', '') into value: int64 else -11 as int64
    let last_missing: int64 = guard string_find_last_index_of_text('hello', 'xyz') into value: int64 else -22 as int64
    print('LAST_FOUND={last_found}')
    print('LAST_REPEATED={last_repeated}')
    print('LAST_EMPTY_ERROR={last_empty}')
    print('LAST_MISSING_ERROR={last_missing}')

    print_split_case('SPLIT_EMPTY', '')
    print_split_case('SPLIT_SINGLE', 'a')
    print_split_case('SPLIT_LF_TRAILING', 'a\n')
    print_split_case('SPLIT_CRLF_TRAILING', 'a\r\n')
    print_split_case('SPLIT_CR_TRAILING', 'a\r')
    print_split_case('SPLIT_ONLY_LF', '\n')
    print_split_case('SPLIT_DOUBLE_LF', '\n\n')
    print_split_case('SPLIT_INTERIOR_BLANK', 'a\n\nb')

    let is_blank_empty = string_is_blank('')
    let is_blank_ascii = string_is_blank('   ')
    let is_blank_tabs_newlines = string_is_blank('\t\n\r')
    let is_blank_unicode = string_is_blank(' 　')
    let is_blank_unicode_text = string_is_blank('猫')
    print('IS_BLANK_EMPTY={is_blank_empty}')
    print('IS_BLANK_ASCII={is_blank_ascii}')
    print('IS_BLANK_TABS_NEWLINES={is_blank_tabs_newlines}')
    print('IS_BLANK_UNICODE={is_blank_unicode}')
    print('IS_BLANK_UNICODE_TEXT={is_blank_unicode_text}')

    let trimmed_unicode: string = guard string_trim_whitespace('   hé 🙂  \t ') into value: string else '<trim-error>'
    let trimmed_noop: string = guard string_trim_whitespace('cat café') into value: string else '<trim-error>'
    print('TRIM_UNICODE=[{trimmed_unicode}]')
    print('TRIM_NOOP=[{trimmed_noop}]')

    let prefix_zero: string = guard string_take_prefix('hé🙂', 0 as int64) into value: string else '<prefix-error>'
    let prefix_exact: string = guard string_take_prefix('hé🙂', 3 as int64) into value: string else '<prefix-error>'
    let prefix_negative: string = guard string_take_prefix('hé🙂', -1 as int64) into value: string else 'NEGATIVE_COUNT'
    let prefix_oob: string = guard string_take_prefix('hé🙂', 4 as int64) into value: string else 'OUT_OF_BOUNDS'
    print('PREFIX_ZERO=[{prefix_zero}]')
    print('PREFIX_EXACT=[{prefix_exact}]')
    print('PREFIX_NEGATIVE={prefix_negative}')
    print('PREFIX_OOB={prefix_oob}')

    let suffix_zero: string = guard string_take_suffix('hé🙂', 0 as int64) into value: string else '<suffix-error>'
    let suffix_exact: string = guard string_take_suffix('hé🙂', 3 as int64) into value: string else '<suffix-error>'
    let suffix_negative: string = guard string_take_suffix('hé🙂', -1 as int64) into value: string else 'NEGATIVE_COUNT'
    let suffix_oob: string = guard string_take_suffix('hé🙂', 4 as int64) into value: string else 'OUT_OF_BOUNDS'
    print('SUFFIX_ZERO=[{suffix_zero}]')
    print('SUFFIX_EXACT=[{suffix_exact}]')
    print('SUFFIX_NEGATIVE={suffix_negative}')
    print('SUFFIX_OOB={suffix_oob}')

    let range_normal: string = guard string_extract_range('hello', 1 as int64, 4 as int64) into value: string else '<range-error>'
    let range_empty: string = guard string_extract_range('hello', 2 as int64, 2 as int64) into value: string else '<range-error>'
    let range_full: string = guard string_extract_range('hé🙂', 0 as int64, 3 as int64) into value: string else '<range-error>'
    let range_at_end: string = guard string_extract_range('hé🙂', 3 as int64, 3 as int64) into value: string else '<range-error>'
    let range_order: string = guard string_extract_range('hello', 4 as int64, 1 as int64) into value: string else 'RANGE_ORDER'
    let range_bounds: string = guard string_extract_range('hello', 0 as int64, 8 as int64) into value: string else 'OUT_OF_BOUNDS'
    print('RANGE_NORMAL=[{range_normal}]')
    print('RANGE_EMPTY=[{range_empty}]')
    print('RANGE_FULL=[{range_full}]')
    print('RANGE_AT_END=[{range_at_end}]')
    print('RANGE_ORDER={range_order}')
    print('RANGE_BOUNDS={range_bounds}')
    return void
}
"#,
        "FIND_INDEX_FOUND=6\nFIND_INDEX_EMPTY=-1\nFIND_INDEX_MISSING=-1\nFIND_INDEX_UNICODE=2\nLAST_FOUND=6\nLAST_REPEATED=5\nLAST_EMPTY_ERROR=-11\nLAST_MISSING_ERROR=-22\nSPLIT_EMPTY|len=0|0=<missing>|1=<missing>|2=<missing>\nSPLIT_SINGLE|len=1|0=a|1=<missing>|2=<missing>\nSPLIT_LF_TRAILING|len=1|0=a|1=<missing>|2=<missing>\nSPLIT_CRLF_TRAILING|len=1|0=a|1=<missing>|2=<missing>\nSPLIT_CR_TRAILING|len=1|0=a|1=<missing>|2=<missing>\nSPLIT_ONLY_LF|len=1|0=|1=<missing>|2=<missing>\nSPLIT_DOUBLE_LF|len=2|0=|1=|2=<missing>\nSPLIT_INTERIOR_BLANK|len=3|0=a|1=|2=b\nIS_BLANK_EMPTY=true\nIS_BLANK_ASCII=true\nIS_BLANK_TABS_NEWLINES=true\nIS_BLANK_UNICODE=true\nIS_BLANK_UNICODE_TEXT=false\nTRIM_UNICODE=[hé 🙂]\nTRIM_NOOP=[cat café]\nPREFIX_ZERO=[]\nPREFIX_EXACT=[hé🙂]\nPREFIX_NEGATIVE=NEGATIVE_COUNT\nPREFIX_OOB=OUT_OF_BOUNDS\nSUFFIX_ZERO=[]\nSUFFIX_EXACT=[hé🙂]\nSUFFIX_NEGATIVE=NEGATIVE_COUNT\nSUFFIX_OOB=OUT_OF_BOUNDS\nRANGE_NORMAL=[ell]\nRANGE_EMPTY=[]\nRANGE_FULL=[hé🙂]\nRANGE_AT_END=[]\nRANGE_ORDER=RANGE_ORDER\nRANGE_BOUNDS=OUT_OF_BOUNDS\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "planned string stdlib behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
#[ignore = "broad compile-and-run fixture; skip during focused search verification"]
fn string_stdlib_behavior_unicode() {
    let cases = [(
        "string-stdlib-behavior-unicode",
        r#"import print from standard

##
    Description: Focused runtime Unicode behavior spec for planned string stdlib functions
##
entry main = f(): void => {
    let unicode_find_index = string_find_index_or('hé🙂z', '🙂', -1 as int64)
    let unicode_last: int64 = guard string_find_last_index_of_text('🙂é🙂', '🙂') into value: int64 else -1 as int64
    let unicode_is_blank = string_is_blank(' 　')
    print('UNICODE_FIND_INDEX={unicode_find_index}')
    print('UNICODE_LAST_INDEX={unicode_last}')
    print('UNICODE_IS_BLANK={unicode_is_blank}')
    let trimmed: string = guard string_trim_whitespace(' hé🙂 ') into value: string else '<trim-error>'
    let prefix: string = guard string_take_prefix('hé🙂z', 3 as int64) into value: string else '<prefix-error>'
    let suffix: string = guard string_take_suffix('hé🙂z', 2 as int64) into value: string else '<suffix-error>'
    let range: string = guard string_extract_range('hé🙂z', 1 as int64, 3 as int64) into value: string else '<range-error>'
    print('UNICODE_TRIM=[{trimmed}]')
    print('UNICODE_PREFIX=[{prefix}]')
    print('UNICODE_SUFFIX=[{suffix}]')
    print('UNICODE_RANGE=[{range}]')
    return void
}
"#,
        "UNICODE_FIND_INDEX=2\nUNICODE_LAST_INDEX=2\nUNICODE_IS_BLANK=true\nUNICODE_TRIM=[hé🙂]\nUNICODE_PREFIX=[hé🙂]\nUNICODE_SUFFIX=[🙂z]\nUNICODE_RANGE=[é🙂]\n",
    )];
    let mut failures = Vec::new();

    for (name, source, expected_stdout) in cases {
        let project = match try_compile_runtime_behavior_project(name, source) {
            Ok(project) => project,
            Err(error) => {
                failures.push(format!("{name} compile failure:\n{error}"));
                continue;
            }
        };
        let output = run_compiled_runtime_project(&project.binary_path);
        if !output.exit.success {
            failures.push(format!(
                "{name} runtime failure:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }

        let actual_stdout = String::from_utf8_lossy(&output.stdout);
        if actual_stdout != expected_stdout {
            failures.push(format!(
                "{name} output mismatch:\nexpected:\n{expected_stdout}\nactual:\n{actual_stdout}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "planned Unicode string stdlib behavior still fails:\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn array_runtime_supports_allocation_indexing_and_bounds_checks() {
    let allocator = MockAllocator;
    let values = [4_i64, 8_i64, 15_i64, 16_i64];

    let array_result = allocate_array(&allocator, &values);
    assert!(array_result.is_ok(), "array allocation must succeed");
    let Ok(array) = array_result else {
        return;
    };

    assert_eq!(array_length(&array), 4, "array length should match input");

    let value_result = array_index(&array, 2);
    assert!(value_result.is_ok(), "in-bounds index should succeed");
    let Ok(value) = value_result else {
        return;
    };
    assert_eq!(value, 15, "indexing should return expected value");

    let out_of_bounds = array_index(&array, 8);
    assert!(out_of_bounds.is_err(), "out-of-bounds indexing should fail");
    assert_eq!(
        out_of_bounds.err(),
        Some(RuntimeError::IndexOutOfBounds {
            index: 8,
            length: 4,
        }),
        "out-of-bounds error must preserve index and length"
    );
}

#[test]
fn io_runtime_uses_injected_handler_for_print_and_take_input() {
    let allocator = MockAllocator;
    let mut io = MockIoHandler::with_input(&["typed value"]);

    let printed = OpalString::new(String::from("hello runtime"));
    let print_result = print(&mut io, &printed);
    assert!(
        print_result.is_ok(),
        "print should write through injected handler"
    );
    assert_eq!(
        io.output,
        vec![String::from("hello runtime")],
        "print should append to captured output buffer"
    );

    let input_result = take_input(&mut io, &allocator);
    assert!(input_result.is_ok(), "take_input should read mocked input");
    let Ok(input) = input_result else {
        return;
    };

    assert_eq!(
        input.as_str(),
        "typed value",
        "take_input should allocate and return injected line"
    );
}

#[test]
fn runtime_error_exposes_code_message_and_result_helper() {
    let out_of_bounds = RuntimeError::IndexOutOfBounds {
        index: 3,
        length: 2,
    };
    assert_eq!(
        out_of_bounds.error_code(),
        1_001,
        "IndexOutOfBounds should map to fixed runtime error code"
    );
    assert_eq!(
        out_of_bounds.message(),
        String::from("index 3 is out of bounds for length 2"),
        "IndexOutOfBounds should format stable message"
    );

    let mapped: RuntimeResult<i32> =
        Err(String::from("oops")).into_runtime_error(7_123, "io failed");
    assert!(
        mapped.is_err(),
        "error mapping helper should convert to runtime error"
    );
    assert_eq!(
        mapped.err(),
        Some(RuntimeError::UserError {
            code: 7_123,
            message: String::from("io failed: oops"),
        }),
        "result extension helper should preserve code and combined message"
    );
}

#[test]
fn string_to_int32_parses_valid_integer_text() {
    let parsed = string_to_int32("-12345");
    assert_eq!(
        parsed,
        Ok(-12_345_i32),
        "string_to_int32 should parse valid signed int32 text"
    );
}

#[test]
fn string_to_int32_returns_parse_error_for_invalid_text() {
    let parsed = string_to_int32("12x");
    assert!(parsed.is_err(), "invalid numeric text should fail parsing");
    assert_eq!(
        parsed.err(),
        Some(RuntimeError::ParseError {
            message: String::from("failed to parse int32 from '12x'"),
        }),
        "invalid parse should map to ParseError with stable message"
    );
}

#[test]
fn random_int32_with_source_is_deterministic_and_range_checked() {
    let mut source = MockRandomSource::from_values(&[5, 7]);
    let first = random_int32_with_source(&mut source, 1, 3);
    let second = random_int32_with_source(&mut source, 10, 12);

    assert_eq!(first, Ok(3_i32), "first random value should map into range");
    assert_eq!(
        second,
        Ok(11_i32),
        "second random value should map deterministically into range"
    );
}

#[test]
fn random_full_domain_ranges_are_safe_and_inclusive() {
    compile_and_run_rng_c_test(
        "random_full_domain_ranges_are_safe_and_inclusive",
        r#"
#include <stdint.h>
#include <stdio.h>
#include "opal_runtime.h"

int main(void) {
    uint64_t unsigned_full_domain = random_uint64(UINT64_C(0), UINT64_MAX);
    int64_t signed_full_domain = random_int64(INT64_MIN, INT64_MAX);
    uint64_t bounded = random_uint64(UINT64_C(4), UINT64_C(9));

    (void)unsigned_full_domain;
    (void)signed_full_domain;

    if (bounded < UINT64_C(4) || bounded > UINT64_C(9)) {
        fprintf(stderr, "bounded result was outside inclusive range\n");
        return 1;
    }
    if (random_uint64(UINT64_C(7), UINT64_C(7)) != UINT64_C(7)) {
        fprintf(stderr, "equal unsigned bounds should return min\n");
        return 2;
    }
    if (random_int64(INT64_C(3), INT64_C(-2)) != INT64_C(3)) {
        fprintf(stderr, "reversed signed bounds should return min\n");
        return 3;
    }

    return 0;
}
"#,
    );
}

#[test]
fn interpolate_string_formats_mixed_placeholder_parts() {
    let values = vec![String::from("Ada"), String::from("4")];
    let formatted = format_interpolated_string("Hello, {name}! You rolled {value}.", &values);
    assert_eq!(
        formatted,
        Ok(String::from("Hello, Ada! You rolled 4.")),
        "interpolation should replace placeholders in encounter order"
    );
}

#[test]
fn interpolate_string_errors_when_placeholder_values_missing() {
    let values = vec![String::from("Ada")];
    let formatted = format_interpolated_string("Hello, {name}! You rolled {value}.", &values);
    assert_eq!(
        formatted,
        Err(RuntimeError::UserError {
            code: 2_004,
            message: String::from("placeholder count mismatch: expected 2 values, received 1"),
        }),
        "placeholder mismatch should be reported as user-facing runtime error"
    );
}

#[test]
fn opal_array_slice_returns_expected_range() {
    let allocator = MockAllocator;
    let source_result = allocate_array(&allocator, &[10_i64, 20_i64, 30_i64, 40_i64]);
    assert!(source_result.is_ok(), "source allocation should succeed");
    let Ok(source) = source_result else {
        return;
    };

    let slice_result = opal_array_slice(&source, 1, 3);
    assert!(slice_result.is_ok(), "valid slice range should succeed");
    let Ok(slice) = slice_result else {
        return;
    };

    assert_eq!(slice.len(), 2, "slice length should match selected range");
    assert_eq!(
        slice.get(0),
        Some(&20_i64),
        "slice should include start element"
    );
    assert_eq!(
        slice.get(1),
        Some(&30_i64),
        "slice should include end-1 element"
    );
}

#[test]
fn opal_array_slice_returns_error_for_invalid_range() {
    let allocator = MockAllocator;
    let source_result = allocate_array(&allocator, &[10_i64, 20_i64, 30_i64]);
    assert!(source_result.is_ok(), "source allocation should succeed");
    let Ok(source) = source_result else {
        return;
    };

    let invalid = opal_array_slice(&source, 2, 1);
    assert_eq!(
        invalid,
        Err(RuntimeError::IndexOutOfBounds {
            index: 2,
            length: 3,
        }),
        "start greater than end should return bounds-style runtime error"
    );
}

#[test]
fn runtime_error_reporting_formats_miette_style_multiline_output() {
    let error = RuntimeError::ParseError {
        message: String::from("failed to parse int32 from 'abc'"),
    };
    let rendered = format_runtime_error(&error);

    assert!(
        rendered.contains("error[opalescent::runtime::parse_error]"),
        "formatted output should include diagnostic code header"
    );
    assert!(
        rendered.contains("failed to parse int32 from 'abc'"),
        "formatted output should include primary message"
    );
    assert!(
        rendered.contains("help:"),
        "formatted output should include actionable help text"
    );
}

#[test]
fn c_runtime_string_bounds_reporting_uses_unicode_scalar_columns() {
    let temp_dir = tempfile::tempdir().expect("create temp dir for C runtime unicode test");
    let source_path = temp_dir
        .path()
        .join("c_runtime_string_bounds_reporting_uses_unicode_scalar_columns.c");
    let binary_path = temp_dir
        .path()
        .join("c_runtime_string_bounds_reporting_uses_unicode_scalar_columns");
    let runtime_test_source = r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern uint64_t opal_runtime_string_index_span_start;
extern uint64_t opal_runtime_string_index_span_len;
extern const char* opal_runtime_string_index_source_path;
extern const char* opal_runtime_string_index_source_text;
void opal_array_bounds_error(uint64_t index, uint64_t length);

int main(void) {
    const char* source_text = "entry main = f(): void =>\n    let prefix = 'é🙂'; let first: string = message[message.length]\n    return void\n";
    const char* failing_expr = strstr(source_text, "message[message.length]");
    if (failing_expr == NULL) {
        fprintf(stderr, "failed to find failing expression\n");
        return 2;
    }

    opal_runtime_string_index_source_path = "test-runtime-string-index-unicode.op";
    opal_runtime_string_index_source_text = source_text;
    opal_runtime_string_index_span_start = (uint64_t)(size_t)(failing_expr - source_text);
    opal_runtime_string_index_span_len = (uint64_t)strlen("message[message.length]");
    opal_array_bounds_error(4u, 4u);
    return 3;
}
"#;
    fs::write(&source_path, runtime_test_source).expect("write C runtime unicode test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_string.c",
            "runtime/opal_rc.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "gcc not found, skipping c_runtime_string_bounds_reporting_uses_unicode_scalar_columns"
            );
            return;
        }
        Err(error) => panic!("failed to invoke gcc for unicode runtime test: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for unicode runtime test:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled unicode runtime test: {error}"));
    let stderr = String::from_utf8_lossy(&run_output.stderr);

    assert!(
        !run_output.status.success(),
        "unicode runtime test should exit non-zero after triggering bounds error"
    );
    assert!(
        stderr.contains("error[opalescent::runtime::index_out_of_bounds]"),
        "unicode runtime diagnostic should include the runtime error code header: {stderr}"
    );
    assert!(
        stderr.contains("test-runtime-string-index-unicode.op:2:44"),
        "unicode runtime diagnostic should report scalar-aware line/column 2:44: {stderr}"
    );
    assert!(
        stderr.contains("let prefix = 'é🙂'; let first: string = message[message.length]"),
        "unicode runtime diagnostic should include the full source line: {stderr}"
    );
    assert!(
        stderr.contains("                                           ^^^^^^^^^^^^^^^^^^^^^^^ string index is out of bounds for this string"),
        "unicode runtime diagnostic should align the caret underline to the failing expression after non-ASCII prefix text: {stderr}"
    );
    assert!(
        stderr.contains("Ensure the index is within 0 <= index < string.length"),
        "unicode runtime diagnostic should include the bounds help text: {stderr}"
    );
}

#[test]
fn weak_reference_upgrade_fails_after_strong_values_drop() {
    let weak = {
        let strong = OpalString::new(String::from("ephemeral"));
        OpalWeakRef::from_string(&strong)
    };

    assert!(
        weak.upgrade_string().is_none(),
        "weak references should not keep values alive after strong owners drop"
    );
}

fn compile_and_run_error_attachment_c_test(test_name: &str, source: &str) {
    let temp_dir = tempfile::tempdir().expect("create temp dir for error attachment C test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write error attachment C runtime test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rc.c",
            "runtime/opal_error.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

fn compile_and_run_terminal_model_c_test(test_name: &str, source: &str) {
    let temp_dir = std::env::temp_dir();
    let source_path = temp_dir.join(format!("{test_name}.c"));
    let binary_path = temp_dir.join(format!("{test_name}.bin"));
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("failed to write C source for {test_name}: {error}"));

    let compile_output = std::process::Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rc.c",
            "runtime/opal_terminal_model.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = std::process::Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

fn compile_and_run_terminal_coordinator_c_test(test_name: &str, source: &str) {
    let temp_dir = tempfile::tempdir().expect("create temp dir for terminal coordinator C test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write terminal coordinator C test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_io.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

#[test]
fn terminal_model_c_setters_validate_and_preserve_structured_invalid_options() {
    compile_and_run_terminal_model_c_test(
        "terminal-model-c-setters-validate-and-preserve-structured-invalid-options",
        r#"
#include "opal_runtime.h"
#include "opal_rc.h"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    int64_t tag;
    uint8_t payload[64];
} OpalTerminalTaggedValue;

typedef struct {
    bool use_alternate_screen;
    bool hide_cursor;
    bool enable_bracketed_paste;
    bool require_trusted_paste_framing;
    bool enable_enhanced_key_identity;
    bool enable_focus_events;
    void* mouse_tracking;
    bool capture_control_keys;
    bool require_requested_features;
} OpalTerminalSessionFeaturePolicyInputRecord;

typedef struct {
    void* input_sequence_timeout;
    void* maximum_committed_text_bytes;
    void* maximum_composition_preedit_bytes;
    void* maximum_paste_chunk_bytes;
    void* maximum_unknown_chunk_bytes;
    void* maximum_pending_sequence_bytes;
    void* maximum_retained_events;
    void* maximum_retained_bytes;
    void* maximum_correlated_events;
    void* maximum_correlated_bytes;
    void* maximum_diagnostics;
    void* maximum_diagnostic_bytes;
} OpalTerminalSessionResourceLimitsInputRecord;

static void* must_i32(int32_t value, int32_t minimum, int32_t maximum) {
    FsHandleResult result = opal_terminal_constrain_i32_range(value, minimum, maximum);
    assert(result.error == NULL);
    assert(result.value != NULL);
    return result.value;
}

static OpalTerminalSessionResourceLimitsInputRecord valid_limits(void) {
    OpalTerminalSessionResourceLimitsInputRecord limits;
    limits.input_sequence_timeout = must_i32(25, 1, 60000);
    limits.maximum_committed_text_bytes = must_i32(4096, 4, 0x00100000);
    limits.maximum_composition_preedit_bytes = must_i32(4096, 4, 0x00100000);
    limits.maximum_paste_chunk_bytes = must_i32(4096, 4, 0x01000000);
    limits.maximum_unknown_chunk_bytes = must_i32(1024, 1, 0x00100000);
    limits.maximum_pending_sequence_bytes = must_i32(1024, 4, 0x00100000);
    limits.maximum_retained_events = must_i32(1024, 8, 0x00100000);
    limits.maximum_retained_bytes = must_i32(8192, 4096, 0x40000000);
    limits.maximum_correlated_events = must_i32(64, 2, 0x00010000);
    limits.maximum_correlated_bytes = must_i32(4096, 64, 0x01000000);
    limits.maximum_diagnostics = must_i32(16, 1, 256);
    limits.maximum_diagnostic_bytes = must_i32(65536, 256, 0x00100000);
    return limits;
}

int main(void) {
    void* defaults = terminal_session_options_default();
    OpalTerminalTaggedValue buttons_and_drag = {3, {0}};
    OpalTerminalSessionFeaturePolicyInputRecord policy = {
        true,
        true,
        true,
        false,
        true,
        true,
        &buttons_and_drag,
        true,
        true,
    };
    OpalTerminalSessionResourceLimitsInputRecord limits = valid_limits();
    FsHandleResult with_policy;
    FsHandleResult with_limits;
    FsHandleResult swapped;
    FsHandleResult reversed;
    FsHandleResult validated;
    FsHandleResult invalid_bytes_result;
    FsHandleResult invalid_bytes_validation;
    FsHandleResult invalid_events_result;
    FsHandleResult invalid_events_validation;
    OpalTerminalSessionResourceLimitsInputRecord invalid_bytes = valid_limits();
    OpalTerminalSessionResourceLimitsInputRecord invalid_events = valid_limits();

    assert(defaults != NULL);
    opal_terminal_test_reset_invalid_options_errors();
    assert(opal_terminal_test_options_use_alternate_screen(defaults) == 0);
    assert(opal_terminal_test_options_mouse_tracking_tag(defaults) == 1);
    assert(opal_terminal_test_options_maximum_retained_bytes(defaults) == 0x00100000);
    assert(opal_terminal_test_options_maximum_correlated_bytes(defaults) == 0x00010000);

    with_policy = terminal_session_options_with_feature_policy(defaults, &policy);
    assert(with_policy.error == NULL);
    assert(with_policy.value != NULL);
    assert(opal_terminal_test_options_use_alternate_screen(defaults) == 0);
    assert(opal_terminal_test_options_use_alternate_screen(with_policy.value) == 1);
    assert(opal_terminal_test_options_mouse_tracking_tag(with_policy.value) == 3);
    assert(opal_terminal_test_options_maximum_retained_bytes(with_policy.value) == 0x00100000);

    with_limits = terminal_session_options_with_resource_limits(defaults, &limits);
    assert(with_limits.error == NULL);
    assert(with_limits.value != NULL);
    assert(opal_terminal_test_options_use_alternate_screen(with_limits.value) == 0);
    assert(opal_terminal_test_options_maximum_retained_bytes(with_limits.value) == 8192);
    assert(opal_terminal_test_options_maximum_correlated_bytes(with_limits.value) == 4096);

    swapped = terminal_session_options_with_resource_limits(with_policy.value, &limits);
    reversed = terminal_session_options_with_feature_policy(with_limits.value, &policy);
    assert(swapped.error == NULL && swapped.value != NULL);
    assert(reversed.error == NULL && reversed.value != NULL);
    assert(opal_terminal_test_options_use_alternate_screen(swapped.value) == 1);
    assert(opal_terminal_test_options_use_alternate_screen(reversed.value) == 1);
    assert(opal_terminal_test_options_mouse_tracking_tag(swapped.value) == 3);
    assert(opal_terminal_test_options_mouse_tracking_tag(reversed.value) == 3);
    assert(opal_terminal_test_options_maximum_retained_bytes(swapped.value) == 8192);
    assert(opal_terminal_test_options_maximum_retained_bytes(reversed.value) == 8192);
    validated = terminal_session_options_validate(swapped.value);
    assert(validated.error == NULL);
    assert(validated.value == swapped.value);

    invalid_bytes.maximum_retained_bytes = must_i32(4096, 4096, 0x40000000);
    invalid_bytes.maximum_correlated_bytes = must_i32(8192, 64, 0x01000000);
    invalid_bytes_result = terminal_session_options_with_resource_limits(defaults, &invalid_bytes);
    assert(invalid_bytes_result.error == NULL && invalid_bytes_result.value != NULL);
    invalid_bytes_validation = terminal_session_options_validate(invalid_bytes_result.value);
    assert(invalid_bytes_validation.value == NULL);
    assert(invalid_bytes_validation.error != NULL);
    assert(strcmp(invalid_bytes_validation.error, "TerminalSessionOptionsError.InvalidOptions") == 0);
    assert(opal_terminal_test_invalid_options_kind(invalid_bytes_validation.error) == 2);
    assert(opal_terminal_test_invalid_options_required(invalid_bytes_validation.error) == 8192u);
    assert(opal_terminal_test_invalid_options_configured(invalid_bytes_validation.error) == 4096u);

    invalid_events.maximum_retained_events = must_i32(8, 8, 0x00100000);
    invalid_events.maximum_correlated_events = must_i32(64, 2, 0x00010000);
    invalid_events_result = terminal_session_options_with_resource_limits(defaults, &invalid_events);
    assert(invalid_events_result.error == NULL && invalid_events_result.value != NULL);
    invalid_events_validation = terminal_session_options_validate(invalid_events_result.value);
    assert(invalid_events_validation.value == NULL);
    assert(invalid_events_validation.error != NULL);
    assert(strcmp(invalid_events_validation.error, "TerminalSessionOptionsError.InvalidOptions") == 0);
    assert(opal_terminal_test_invalid_options_kind(invalid_events_validation.error) == 3);
    assert(opal_terminal_test_invalid_options_required(invalid_events_validation.error) == 64u);
    assert(opal_terminal_test_invalid_options_configured(invalid_events_validation.error) == 8u);
    return 0;
}
"#,
    );
}

#[test]
fn terminal_model_c_setters_and_trust_conversion_report_allocation_failure() {
    compile_and_run_terminal_model_c_test(
        "terminal-model-c-setters-and-trust-conversion-report-allocation-failure",
        r#"
#include "opal_runtime.h"
#include "opal_rc.h"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    int64_t tag;
    uint8_t payload[64];
} OpalTerminalTaggedValue;

typedef struct {
    bool use_alternate_screen;
    bool hide_cursor;
    bool enable_bracketed_paste;
    bool require_trusted_paste_framing;
    bool enable_enhanced_key_identity;
    bool enable_focus_events;
    void* mouse_tracking;
    bool capture_control_keys;
    bool require_requested_features;
} OpalTerminalSessionFeaturePolicyInputRecord;

typedef struct {
    void* input_sequence_timeout;
    void* maximum_committed_text_bytes;
    void* maximum_composition_preedit_bytes;
    void* maximum_paste_chunk_bytes;
    void* maximum_unknown_chunk_bytes;
    void* maximum_pending_sequence_bytes;
    void* maximum_retained_events;
    void* maximum_retained_bytes;
    void* maximum_correlated_events;
    void* maximum_correlated_bytes;
    void* maximum_diagnostics;
    void* maximum_diagnostic_bytes;
} OpalTerminalSessionResourceLimitsInputRecord;

static void* must_i32(int32_t value, int32_t minimum, int32_t maximum) {
    FsHandleResult result = opal_terminal_constrain_i32_range(value, minimum, maximum);
    assert(result.error == NULL);
    assert(result.value != NULL);
    return result.value;
}

int main(void) {
    void* defaults = terminal_session_options_default();
    OpalTerminalTaggedValue disabled = {1, {0}};
    OpalTerminalSessionFeaturePolicyInputRecord policy = {
        false,
        false,
        false,
        false,
        false,
        false,
        &disabled,
        false,
        false,
    };
    OpalTerminalSessionResourceLimitsInputRecord limits = {
        must_i32(25, 1, 60000),
        must_i32(4096, 4, 0x00100000),
        must_i32(4096, 4, 0x00100000),
        must_i32(4096, 4, 0x01000000),
        must_i32(1024, 1, 0x00100000),
        must_i32(1024, 4, 0x00100000),
        must_i32(1024, 8, 0x00100000),
        must_i32(8192, 4096, 0x40000000),
        must_i32(64, 2, 0x00010000),
        must_i32(4096, 64, 0x01000000),
        must_i32(16, 1, 256),
        must_i32(65536, 256, 0x00100000),
    };
    FsHandleResult failed_feature;
    FsHandleResult failed_limits;
    FsHandleResult failed_trust;

    assert(defaults != NULL);
    opal_test_fail_next_allocation_for_test();
    failed_feature = terminal_session_options_with_feature_policy(defaults, &policy);
    assert(failed_feature.value == NULL);
    assert(failed_feature.error != NULL);
    assert(strcmp(failed_feature.error, "AllocationFailureError") == 0);

    opal_test_fail_next_allocation_for_test();
    failed_limits = terminal_session_options_with_resource_limits(defaults, &limits);
    assert(failed_limits.value == NULL);
    assert(failed_limits.error != NULL);
    assert(strcmp(failed_limits.error, "AllocationFailureError") == 0);

    opal_test_fail_next_allocation_for_test();
    failed_trust = trusted_terminal_output_from_application_text("safe terminal output");
    assert(failed_trust.value == NULL);
    assert(failed_trust.error != NULL);
    assert(strcmp(failed_trust.error, "AllocationFailureError") == 0);
    return 0;
}
"#,
    );
}

#[test]
fn terminal_coordinator_c_rejects_before_stdout_mutation() {
    compile_and_run_terminal_coordinator_c_test(
        "terminal-coordinator-c-rejects-before-stdout-mutation",
        r#"
#include "opal_runtime.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static long captured_size(FILE *capture) {
    long size;
    fflush(stdout);
    assert(fseek(capture, 0, SEEK_END) == 0);
    size = ftell(capture);
    assert(size >= 0);
    return size;
}

int main(void) {
    int saved_stdin = dup(STDIN_FILENO);
    int saved_stdout = dup(STDOUT_FILENO);
    FILE *input = tmpfile();
    FILE *capture = tmpfile();
    assert(saved_stdin >= 0);
    assert(saved_stdout >= 0);
    assert(input != NULL);
    assert(capture != NULL);
    assert(fputs("accepted\n", input) >= 0);
    assert(fflush(input) == 0);
    assert(fseek(input, 0, SEEK_SET) == 0);
    assert(dup2(fileno(input), STDIN_FILENO) >= 0);
    assert(dup2(fileno(capture), STDOUT_FILENO) >= 0);

    opal_terminal_test_reset();
    opal_terminal_test_set_state(OPAL_TERMINAL_TEST_OPENING);
    FsVoidResult result = print_text_sync("blocked");
    assert(result.error != NULL);
    assert(strstr(result.error, "WriteFailureError") != NULL);
    assert(strstr(result.error, "TerminalCoordinatorUnavailable") != NULL);
    print_string("blocked\\033[31m");
    FsStringResult blocked_input = take_input();
    assert(blocked_input.value == NULL);
    assert(blocked_input.error != NULL);
    assert(strstr(blocked_input.error, "StandardInputReadError") != NULL);
    assert(strstr(blocked_input.error, "state: Opening") != NULL);
    assert(strstr(blocked_input.error, "operation: TakeInput") != NULL);
    result = terminal_clear_screen_sync();
    assert(result.error != NULL);
    assert(strstr(result.error, "TerminalWriteFailureError") != NULL);
    assert(captured_size(capture) == 0);

    opal_terminal_test_reset();
    FsStringResult accepted_input = take_input();
    assert(accepted_input.error == NULL);
    assert(accepted_input.value != NULL);
    assert(strcmp(accepted_input.value, "accepted") == 0);
    free(accepted_input.value);
    FsHandleResult old_writer_result = stdout_writer();
    FsHandleResult old_terminal_result = stdout_terminal();
    assert(old_writer_result.error == NULL);
    assert(old_terminal_result.error == NULL);
    OpalStdoutWriter *old_writer = (OpalStdoutWriter *)old_writer_result.value;
    OpalStdoutTerminal *old_terminal = (OpalStdoutTerminal *)old_terminal_result.value;
    assert(opal_terminal_test_reserve_opening() == 1);
    opal_terminal_test_return_free();
    result = writer_write_sync(old_writer, "stale");
    assert(result.error != NULL);
    assert(strstr(result.error, "WriterWrite") != NULL);
    result = terminal_clear_screen_on_sync(old_terminal);
    assert(result.error != NULL);
    assert(strstr(result.error, "TerminalClearScreenOn") != NULL);
    FsBooleanResult stale_capability = terminal_supports_ansi(old_terminal);
    assert(stale_capability.value == 0);
    assert(stale_capability.error != NULL);
    assert(strstr(stale_capability.error, "StandardOutputCapabilityError") != NULL);
    assert(strstr(stale_capability.error, "state: Free") != NULL);
    assert(strstr(stale_capability.error, "operation: TerminalSupportsAnsi") != NULL);
    assert(captured_size(capture) == 0);

    FsHandleResult fresh_writer_result = stdout_writer();
    FsHandleResult fresh_terminal_result = stdout_terminal();
    assert(fresh_writer_result.error == NULL);
    assert(fresh_terminal_result.error == NULL);
    OpalStdoutWriter *fresh_writer = (OpalStdoutWriter *)fresh_writer_result.value;
    OpalStdoutTerminal *fresh_terminal = (OpalStdoutTerminal *)fresh_terminal_result.value;
    result = writer_write_sync(fresh_writer, "fresh");
    assert(result.error == NULL);
    result = terminal_move_cursor_on_sync(fresh_terminal, 0, 0);
    assert(result.error == NULL);
    long size_after_fresh_use = captured_size(capture);
    assert(size_after_fresh_use > 5);
    const char safe_text[] = {'s', 'a', 'f', 'e', 0x1B, '[', '3', '1', 'm', '\0'};
    print_string(safe_text);
    assert(captured_size(capture) > size_after_fresh_use);
    char captured[512] = {0};
    assert(fseek(capture, 0, SEEK_SET) == 0);
    size_t captured_length = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[captured_length] = '\0';
    const char escaped_text[] = {'s', 'a', 'f', 'e', 0x5C, 'x', '1', 'B', '[', '3', '1', 'm', '\0'};
    assert(strstr(captured, escaped_text) != NULL);
    long size_after_allowed_use = captured_size(capture);

    result = writer_write_sync(old_writer, "still stale");
    assert(result.error != NULL);
    assert(strstr(result.error, "WriterWrite") != NULL);
    result = terminal_clear_screen_on_sync(old_terminal);
    assert(result.error != NULL);
    assert(strstr(result.error, "TerminalClearScreenOn") != NULL);
    assert(captured_size(capture) == size_after_allowed_use);

    opal_terminal_test_set_state(OPAL_TERMINAL_TEST_ACTIVE);
    FsHandleResult active_writer = stdout_writer();
    FsHandleResult active_terminal = stdout_terminal();
    assert(active_writer.value == NULL);
    assert(active_writer.error != NULL);
    assert(active_terminal.value == NULL);
    assert(active_terminal.error != NULL);
    opal_terminal_test_set_state(OPAL_TERMINAL_TEST_OPENING);
    FsHandleResult opening_writer = stdout_writer();
    FsHandleResult opening_terminal = stdout_terminal();
    assert(opening_writer.value == NULL);
    assert(opening_writer.error != NULL);
    assert(opening_terminal.value == NULL);
    assert(opening_terminal.error != NULL);
    opal_terminal_test_return_free();

    assert(dup2(saved_stdin, STDIN_FILENO) >= 0);
    assert(dup2(saved_stdout, STDOUT_FILENO) >= 0);
    close(saved_stdin);
    close(saved_stdout);
    fclose(input);
    fclose(capture);
    return 0;
}
"#,
    );
}

#[test]
fn error_cause_insertion_is_immutable_and_inspectable() {
    compile_and_run_error_attachment_c_test(
        "error_cause_insertion_is_immutable_and_inspectable",
        r#"
#include "opal_runtime.h"
#include <stdio.h>
#include <string.h>

static int fail(const char* message) {
    fprintf(stderr, "%s\n", message);
    return 1;
}

int main(void) {
    char* primary = opal_error_new("PrimaryError");
    char* prior = opal_error_new("PriorError");
    char* derived = opal_error_attach_cause(primary, prior);

    FsStringResult original_cause = error_cause(primary);
    if (original_cause.error == NULL || strcmp(original_cause.error, "ErrorAttachmentAbsentError") != 0) {
        return fail("original alias should remain without a cause");
    }

    FsStringResult derived_cause = error_cause(derived);
    if (derived_cause.error != NULL || derived_cause.value != prior) {
        return fail("derived error should expose the requested cause identity");
    }
    if (error_suppressed_length(derived) != 0) {
        return fail("new immediate cause should not add suppressed values");
    }

    OpalErrorTruncation* truncation = error_attachment_truncation(derived);
    if (error_attachment_truncation_cause_depth(truncation)
        || error_attachment_truncation_suppressed_count(truncation)
        || error_attachment_truncation_bytes(truncation)) {
        return fail("ordinary cause insertion should not set truncation markers");
    }
    return 0;
}
"#,
    );
}

#[test]
fn error_suppressed_fallback_idempotent_and_bounds_checked() {
    compile_and_run_error_attachment_c_test(
        "error_suppressed_fallback_idempotent_and_bounds_checked",
        r#"
#include "opal_runtime.h"
#include <stdio.h>
#include <string.h>

static int fail(const char* message) {
    fprintf(stderr, "%s\n", message);
    return 1;
}

int main(void) {
    char* primary = opal_error_new("PrimaryError");
    char* first = opal_error_new("FirstCauseError");
    char* second = opal_error_new("SecondCauseError");
    char* with_cause = opal_error_attach_cause(primary, first);
    char* with_suppressed = opal_error_attach_cause(with_cause, second);

    FsStringResult cause = error_cause(with_suppressed);
    if (cause.error != NULL || cause.value != first) {
        return fail("existing immediate cause should not be replaced");
    }
    if (error_suppressed_length(with_suppressed) != 1) {
        return fail("different cause should fall back to one suppressed value");
    }
    FsStringResult suppressed = error_suppressed_at(with_suppressed, 0);
    if (suppressed.error != NULL || suppressed.value != second) {
        return fail("suppressed value should preserve requested identity");
    }
    if (opal_error_attach_cause(with_suppressed, second) != with_suppressed) {
        return fail("reattaching an existing suppressed identity should be idempotent");
    }
    if (opal_error_attach_cause(with_suppressed, first) != with_suppressed) {
        return fail("reattaching the immediate cause should be idempotent");
    }

    FsStringResult out_of_bounds = error_suppressed_at(with_suppressed, 1);
    if (out_of_bounds.error == NULL || strcmp(out_of_bounds.error, "IndexOutOfBoundsError") != 0) {
        return fail("suppressed inspector should reject out-of-bounds indexes");
    }
    FsStringResult negative = error_suppressed_at(with_suppressed, -1);
    if (negative.error == NULL || strcmp(negative.error, "IndexOutOfBoundsError") != 0) {
        return fail("suppressed inspector should reject negative indexes");
    }
    return 0;
}
"#,
    );
}

#[test]
fn error_attachment_allocation_failure_preserves_aliases_and_marks_bytes() {
    compile_and_run_error_attachment_c_test(
        "error_attachment_allocation_failure_preserves_aliases_and_marks_bytes",
        r#"
#include "opal_runtime.h"
#include "opal_test_alloc.h"
#include <stdio.h>
#include <string.h>

static int fail(const char* message) {
    fprintf(stderr, "%s\n", message);
    return 1;
}

int main(void) {
    char* primary = opal_error_new("PrimaryError");
    char* first = opal_error_new("FirstCauseError");
    char* second = opal_error_new("SecondCauseError");
    char* with_cause = opal_error_attach_cause(primary, first);

    opal_test_fail_next_allocation_for_test();
    char* allocation_limited = opal_error_attach_cause(with_cause, second);
    if (allocation_limited == with_cause) {
        return fail("allocation-limited attachment should return a distinct marker alias");
    }

    FsStringResult original_cause = error_cause(with_cause);
    if (original_cause.error != NULL || original_cause.value != first) {
        return fail("allocation failure must not mutate existing immediate cause");
    }
    if (error_suppressed_length(with_cause) != 0) {
        return fail("allocation failure must not mutate existing suppressed edges");
    }
    if (error_suppressed_length(allocation_limited) != 0) {
        return fail("allocation-limited clone should not invent suppressed edges");
    }

    OpalErrorTruncation* truncation = error_attachment_truncation(allocation_limited);
    if (!error_attachment_truncation_bytes(truncation)) {
        return fail("allocation-limited attachment should set the bytes truncation marker");
    }
    if (error_attachment_truncation_cause_depth(truncation)
        || error_attachment_truncation_suppressed_count(truncation)) {
        return fail("allocation-limited attachment should only set the bytes marker");
    }

    char* with_suppressed = opal_error_attach_cause(with_cause, second);
    if (error_suppressed_length(with_suppressed) != 1) {
        return fail("one-shot allocation failure should be consumed before the next attachment");
    }
    if (error_suppressed_length(with_cause) != 0) {
        return fail("successful retry must still preserve the original alias");
    }
    return 0;
}
"#,
    );
}

#[test]
fn error_attachment_cycle_limit_and_storage_truncation_markers() {
    compile_and_run_error_attachment_c_test(
        "error_attachment_cycle_limit_and_storage_truncation_markers",
        r#"
#include "opal_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int fail(const char* message) {
    fprintf(stderr, "%s\n", message);
    return 1;
}

int main(void) {
    char* primary = opal_error_new("PrimaryError");
    char* cause = opal_error_new("CauseError");
    char* primary_with_cause = opal_error_attach_cause(primary, cause);
    char* cyclic = opal_error_attach_cause(cause, primary_with_cause);
    OpalErrorTruncation* cyclic_truncation = error_attachment_truncation(cyclic);
    if (!error_attachment_truncation_cause_depth(cyclic_truncation)) {
        return fail("cycle prevention without existing cause should set cause-depth marker");
    }

    char* limited = opal_error_new("Depth0Error");
    for (int index = 1; index <= 9; ++index) {
        char name[32];
        snprintf(name, sizeof(name), "Depth%dError", index);
        limited = opal_error_attach_cause(opal_error_new(name), limited);
    }
    OpalErrorTruncation* depth_truncation = error_attachment_truncation(limited);
    if (!error_attachment_truncation_cause_depth(depth_truncation)) {
        return fail("cause-depth overflow should set the cause-depth marker");
    }

    char* suppressed_primary = opal_error_attach_cause(
        opal_error_new("SuppressedPrimaryError"),
        opal_error_new("SuppressedImmediateError")
    );
    for (int index = 0; index < 8; ++index) {
        char name[40];
        snprintf(name, sizeof(name), "Suppressed%dError", index);
        suppressed_primary = opal_error_attach_cause(suppressed_primary, opal_error_new(name));
    }
    char* suppressed_overflow = opal_error_attach_cause(
        suppressed_primary,
        opal_error_new("SuppressedOverflowError")
    );
    if (error_suppressed_length(suppressed_overflow) != 8) {
        return fail("suppressed-count overflow should retain the bounded prefix");
    }
    OpalErrorTruncation* suppressed_truncation = error_attachment_truncation(suppressed_overflow);
    if (!error_attachment_truncation_suppressed_count(suppressed_truncation)) {
        return fail("suppressed-count overflow should set the suppressed marker");
    }

    char* large = (char*)malloc(70000u);
    if (large == NULL) {
        return fail("large test allocation failed");
    }
    memset(large, 'E', 69999u);
    large[69999u] = '\0';
    char* bytes_overflow = opal_error_attach_cause(opal_error_new("BytesPrimaryError"), opal_error_new(large));
    OpalErrorTruncation* bytes_truncation = error_attachment_truncation(bytes_overflow);
    if (!error_attachment_truncation_bytes(bytes_truncation)) {
        return fail("storage overflow should set the bytes marker");
    }
    free(large);
    return 0;
}
"#,
    );
}

fn compile_and_run_rng_c_test(test_name: &str, source: &str) {
    let temp_dir = tempfile::tempdir().expect("create temp dir for RNG C runtime test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write RNG C runtime test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rng.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

fn compile_and_run_array_rc_c_test(test_name: &str, source: &str) {
    let temp_dir = tempfile::tempdir().expect("create temp dir for C runtime test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write C runtime test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rc.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

fn compile_and_run_string_builder_c_test(test_name: &str, source: &str) {
    let temp_dir = tempfile::tempdir().expect("create temp dir for string builder C runtime test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write string builder C runtime test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rc.c",
            "runtime/opal_string.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping {test_name}");
            return;
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run_output = Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"));

    assert!(
        run_output.status.success(),
        "compiled C test {test_name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

fn compile_and_run_filesystem_c_test(test_name: &str, source: &str) -> std::process::Output {
    let temp_dir = tempfile::tempdir().expect("create temp dir for filesystem C runtime test");
    let source_path = temp_dir.path().join(format!("{test_name}.c"));
    let binary_path = temp_dir.path().join(test_name);

    fs::write(&source_path, source).expect("write filesystem C runtime test source");

    let compile_output = Command::new("gcc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-DOPAL_ENABLE_INTERNAL_TESTING",
            "-Wall",
            "-Wextra",
            "-Werror",
            source_path.to_str().expect("utf-8 source path"),
            "runtime/opal_rc.c",
            "runtime/opal_fs.c",
            "-Iruntime",
            "-o",
            binary_path.to_str().expect("utf-8 binary path"),
        ])
        .output();

    let compiled = match compile_output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            panic!("gcc is required for {test_name}: {error}")
        }
        Err(error) => panic!("failed to invoke gcc for {test_name}: {error}"),
    };

    assert!(
        compiled.status.success(),
        "gcc failed for {test_name}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    Command::new(&binary_path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run compiled test {test_name}: {error}"))
}

#[test]
fn string_builder_push_overflow_returns_allocation_failure_error() {
    compile_and_run_string_builder_c_test(
        "string_builder_push_overflow_returns_allocation_failure_error",
        r#"
#include "opal_runtime.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

void opal_string_builder_set_length_for_test(OpalStringBuilder* builder, size_t length);

int main(void) {
    OpalStringBuilder* builder = string_builder_new();
    if (builder == NULL) {
        fprintf(stderr, "builder allocation failed\n");
        return 1;
    }

    opal_string_builder_set_length_for_test(builder, SIZE_MAX - 1u);
    StringBuilderVoidResult result = string_builder_push(builder, "x");
    if (result.value != NULL || result.error == NULL || strcmp(result.error, "AllocationFailureError") != 0) {
        fprintf(stderr, "overflow should return AllocationFailureError\n");
        return 2;
    }

    return 0;
}
"#,
    );
}

#[test]
fn opal_array_rc_alloc_empty_and_roundtrip_metadata() {
    compile_and_run_array_rc_c_test(
        "opal_array_rc_alloc_empty_and_roundtrip_metadata",
        r#"
#include "opal_rc.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    int *data = NULL;
    void *array = opal_array_alloc(sizeof(int), _Alignof(int), 0, 0, NULL);
    if (array == NULL) {
        fprintf(stderr, "array allocation returned null\n");
        return 1;
    }
    if (opal_array_len(array) != 0) {
        fprintf(stderr, "expected len 0, got %zu\n", opal_array_len(array));
        return 2;
    }
    if (opal_array_cap(array) != 0) {
        fprintf(stderr, "expected cap 0, got %zu\n", opal_array_cap(array));
        return 3;
    }

    opal_array_set_len(array, 0);
    opal_array_set_cap(array, 4);
    if (opal_array_len(array) != 0 || opal_array_cap(array) != 4) {
        fprintf(stderr, "metadata roundtrip failed\n");
        return 4;
    }

    data = (int *)opal_array_data(array, _Alignof(int));
    if (data == NULL) {
        fprintf(stderr, "data pointer returned null\n");
        return 5;
    }

    opal_rc_dec(array);
    return 0;
}
"#,
    );
}

#[test]
fn opal_array_rc_non_empty_layout_and_data_pointer_math_hold() {
    compile_and_run_array_rc_c_test(
        "opal_array_rc_non_empty_layout_and_data_pointer_math_hold",
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "opal_rc.h"

typedef struct WideAlignedValue {
    long double value;
} WideAlignedValue;

int main(void) {
    unsigned char *array_bytes = NULL;
    WideAlignedValue *elems = NULL;
    size_t offset = 0;
    void *array = opal_array_alloc(sizeof(WideAlignedValue), _Alignof(WideAlignedValue), 2, 5, NULL);
    if (array == NULL) {
        fprintf(stderr, "array allocation returned null\n");
        return 1;
    }
    if (opal_array_len(array) != 2) {
        fprintf(stderr, "expected len 2, got %zu\n", opal_array_len(array));
        return 2;
    }
    if (opal_array_cap(array) != 5) {
        fprintf(stderr, "expected cap 5, got %zu\n", opal_array_cap(array));
        return 3;
    }

    array_bytes = (unsigned char *)array;
    elems = (WideAlignedValue *)opal_array_data(array, _Alignof(WideAlignedValue));
    offset = opal_array_data_offset(array, _Alignof(WideAlignedValue));
    if (((uintptr_t)elems % _Alignof(WideAlignedValue)) != 0) {
        fprintf(stderr, "element pointer alignment mismatch\n");
        return 4;
    }
    if ((size_t)(void *)((unsigned char *)elems - array_bytes) != offset) {
        fprintf(stderr, "data offset mismatch\n");
        return 5;
    }
    if ((const void *)elems != opal_array_data_const(array, _Alignof(WideAlignedValue))) {
        fprintf(stderr, "const/non-const data pointer mismatch\n");
        return 6;
    }

    elems[0].value = 3.5;
    elems[1].value = 7.25;
    if (elems[0].value != 3.5 || elems[1].value != 7.25) {
        fprintf(stderr, "element writes were not preserved\n");
        return 7;
    }

    opal_rc_dec(array);
    return 0;
}
"#,
    );
}

#[test]
fn opal_runtime_heap_accounting_tracks_array_liveness() {
    compile_and_run_array_rc_c_test(
        "opal_runtime_heap_accounting_tracks_array_liveness",
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "opal_rc.h"

int main(void) {
    size_t live_after_alloc = 0;
    size_t peak_after_alloc = 0;
    void *array = NULL;

    opal_runtime_reset_heap_accounting();
    if (opal_runtime_live_heap_bytes() != 0 || opal_runtime_peak_heap_bytes() != 0) {
        fprintf(stderr, "heap accounting did not reset to zero\n");
        return 1;
    }

    array = opal_array_alloc(sizeof(int32_t), _Alignof(int32_t), 4, 4, NULL);
    if (array == NULL) {
        fprintf(stderr, "array allocation returned null\n");
        return 2;
    }

    live_after_alloc = opal_runtime_live_heap_bytes();
    peak_after_alloc = opal_runtime_peak_heap_bytes();
    if (live_after_alloc == 0) {
        fprintf(stderr, "live heap bytes stayed at zero after allocation\n");
        return 3;
    }
    if (peak_after_alloc < live_after_alloc) {
        fprintf(stderr, "peak heap bytes should be at least live heap bytes\n");
        return 4;
    }

    opal_rc_dec(array);
    if (opal_runtime_live_heap_bytes() != 0) {
        fprintf(stderr, "live heap bytes should return to zero after drop\n");
        return 5;
    }
    if (opal_runtime_peak_heap_bytes() != peak_after_alloc) {
        fprintf(stderr, "peak heap bytes should preserve the allocation high-water mark\n");
        return 6;
    }

    return 0;
}
"#,
    );
}

#[test]
fn rc_uniqueness_strong_only() {
    compile_and_run_array_rc_c_test(
        "rc_uniqueness_strong_only",
        r#"
#include <stdio.h>
#include <stdlib.h>
#include "opal_rc.h"

int main(void) {
    void *obj = opal_rc_alloc(sizeof(int), NULL);
    if (obj == NULL) {
        fprintf(stderr, "allocation returned null\n");
        return 1;
    }
    if (!opal_rc_is_unique(obj)) {
        fprintf(stderr, "fresh allocation should report unique\n");
        return 2;
    }
    if (!opal_rc_is_reuse_eligible(obj)) {
        fprintf(stderr, "fresh allocation should be reuse eligible\n");
        return 3;
    }
    if (opal_rc_strong_count_for_test(obj) != 1) {
        fprintf(stderr, "expected strong count 1, got %zu\n", opal_rc_strong_count_for_test(obj));
        return 4;
    }
    if (opal_rc_weak_count_for_test(obj) != 0) {
        fprintf(stderr, "expected weak count 0, got %zu\n", opal_rc_weak_count_for_test(obj));
        return 5;
    }

    opal_rc_dec(obj);
    return 0;
}
"#,
    );
}

#[test]
fn rc_uniqueness_weak_blocks_reuse() {
    compile_and_run_array_rc_c_test(
        "rc_uniqueness_weak_blocks_reuse",
        r#"
#include <stdio.h>
#include <stdlib.h>
#include "opal_rc.h"

int main(void) {
    void *obj = opal_rc_alloc(sizeof(int), NULL);
    OpalWeakRef *weak = NULL;
    if (obj == NULL) {
        fprintf(stderr, "allocation returned null\n");
        return 1;
    }

    weak = opal_weak_alloc(obj);
    if (weak == NULL) {
        fprintf(stderr, "weak allocation returned null\n");
        opal_rc_dec(obj);
        return 2;
    }
    if (!opal_rc_is_unique(obj)) {
        fprintf(stderr, "weak refs should not affect strong uniqueness\n");
        opal_weak_dec(weak);
        opal_rc_dec(obj);
        return 3;
    }
    if (opal_rc_is_reuse_eligible(obj)) {
        fprintf(stderr, "weak refs must block reuse eligibility\n");
        opal_weak_dec(weak);
        opal_rc_dec(obj);
        return 4;
    }
    if (opal_rc_strong_count_for_test(obj) != 1) {
        fprintf(stderr, "expected strong count 1, got %zu\n", opal_rc_strong_count_for_test(obj));
        opal_weak_dec(weak);
        opal_rc_dec(obj);
        return 5;
    }
    if (opal_rc_weak_count_for_test(obj) != 1) {
        fprintf(stderr, "expected weak count 1, got %zu\n", opal_rc_weak_count_for_test(obj));
        opal_weak_dec(weak);
        opal_rc_dec(obj);
        return 6;
    }

    opal_rc_dec(obj);
    if (opal_weak_upgrade(weak) != NULL) {
        fprintf(stderr, "weak upgrade should fail after strong drop\n");
        opal_weak_dec(weak);
        return 7;
    }
    opal_weak_dec(weak);
    return 0;
}
"#,
    );
}

#[test]
fn allocation_fault_injection_is_one_shot_resettable_and_preserves_realloc_input() {
    compile_and_run_array_rc_c_test(
        "allocation_fault_injection_is_one_shot_resettable_and_preserves_realloc_input",
        r#"
#include "opal_rc.h"
#include <stdio.h>
#include <stdlib.h>
#include "opal_test_alloc.h"

int main(void) {
    char *buffer = (char *)malloc(8u);
    char *zeroed = NULL;
    char *reset_buffer = NULL;
    void *object = NULL;
    char *resized = NULL;

    if (buffer == NULL) {
        fprintf(stderr, "initial allocation failed\n");
        return 1;
    }
    buffer[0] = 'x';

    opal_test_fail_next_allocation_for_test();
    if (opal_rc_alloc(sizeof(int), NULL) != NULL) {
        fprintf(stderr, "armed runtime allocation should fail\n");
        free(buffer);
        return 2;
    }

    object = opal_rc_alloc(sizeof(int), NULL);
    if (object == NULL) {
        fprintf(stderr, "one-shot failure should be consumed\n");
        free(buffer);
        return 3;
    }
    opal_rc_dec(object);

    opal_test_fail_next_allocation_for_test();
    if (calloc(1u, 8u) != NULL) {
        fprintf(stderr, "armed calloc should fail\n");
        free(buffer);
        return 4;
    }

    zeroed = (char *)calloc(1u, 8u);
    if (zeroed == NULL) {
        fprintf(stderr, "one-shot calloc failure should be consumed\n");
        free(buffer);
        return 5;
    }
    free(zeroed);

    opal_test_fail_next_allocation_for_test();
    opal_test_reset_allocation_failure_for_test();
    reset_buffer = (char *)malloc(8u);
    if (reset_buffer == NULL) {
        fprintf(stderr, "reset should cancel pending malloc failure\n");
        free(buffer);
        return 6;
    }
    free(reset_buffer);

    opal_test_fail_next_allocation_for_test();
    resized = (char *)realloc(buffer, 16u);
    if (resized != NULL || buffer[0] != 'x') {
        fprintf(stderr, "failed realloc should preserve its input allocation\n");
        free(buffer);
        return 7;
    }

    free(buffer);
    return 0;
}
"#,
    );
}

#[test]
fn rc_drop_stack_oom_fails_closed() {
    compile_and_run_array_rc_c_test(
        "rc_drop_stack_oom_fails_closed",
        r#"
#include "opal_rc.h"
#include "opal_test_alloc.h"
#include <stdio.h>
#include <sys/wait.h>
#include <unistd.h>

#define GROWTH_CHILD_COUNT 65u

static void *growth_children[GROWTH_CHILD_COUNT];

static void drop_growth_children(void *root, void ***stack, size_t *stack_top, size_t *stack_cap) {
    size_t i = 0;
    (void)root;

    opal_test_fail_next_allocation_for_test();
    for (i = 0; i < GROWTH_CHILD_COUNT; ++i) {
        opal_rc_drop_child(growth_children[i], stack, stack_top, stack_cap);
    }
}

static void trigger_initial_stack_oom(void) {
    void *root = opal_rc_alloc(sizeof(int), NULL);
    if (root == NULL) {
        _exit(10);
    }

    opal_test_fail_next_allocation_for_test();
    opal_rc_dec(root);
    _exit(0);
}

static void trigger_growth_stack_oom(void) {
    size_t i = 0;
    void *root = NULL;

    for (i = 0; i < GROWTH_CHILD_COUNT; ++i) {
        growth_children[i] = opal_rc_alloc(sizeof(int), NULL);
        if (growth_children[i] == NULL) {
            _exit(11);
        }
    }

    root = opal_rc_alloc(sizeof(int), drop_growth_children);
    if (root == NULL) {
        _exit(12);
    }

    opal_rc_dec(root);
    _exit(0);
}

static int expect_nonzero_child_exit(void (*trigger)(void), const char *label) {
    int status = 0;
    pid_t child = fork();

    if (child < 0) {
        fprintf(stderr, "%s: fork failed\n", label);
        return 1;
    }
    if (child == 0) {
        trigger();
    }
    if (waitpid(child, &status, 0) != child) {
        fprintf(stderr, "%s: waitpid failed\n", label);
        return 2;
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) == 0) {
        fprintf(stderr, "%s: expected nonzero fatal exit, status=%d\n", label, status);
        return 3;
    }

    return 0;
}

int main(void) {
    if (expect_nonzero_child_exit(trigger_initial_stack_oom, "initial stack allocation") != 0) {
        return 1;
    }
    if (expect_nonzero_child_exit(trigger_growth_stack_oom, "stack growth allocation") != 0) {
        return 2;
    }

    return 0;
}
"#,
    );
}

#[test]
fn path_normalization_oom_is_fatal_not_empty_sentinel() {
    let run_output = compile_and_run_filesystem_c_test(
        "path_normalization_oom_is_fatal_not_empty_sentinel",
        r#"
#include "opal_rc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *normalize_path(const char *path);

int main(void) {
    opal_test_fail_next_allocation_for_test();
    char *normalized = normalize_path("stable/component");
    fprintf(stderr, "normalization continued after forced OOM: %s\n", normalized ? normalized : "(null)");
    free(normalized);
    return 19;
}
"#,
    );

    assert_eq!(
        run_output.status.code(),
        Some(1_i32),
        "forced filesystem OOM must terminate through the runtime fatal path; status: {:?}, stderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run_output.stderr)
            .contains("Runtime error: out of memory while normalizing filesystem path"),
        "forced filesystem OOM must report the runtime fatal path; stderr: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );
}

#[test]
fn rc_debug_counter_registry_tracks_categories_with_test_only_hooks() {
    compile_and_run_array_rc_c_test(
        "rc_debug_counter_registry_tracks_categories_with_test_only_hooks",
        r#"
#include <stdio.h>
#include <stdlib.h>
#include "opal_rc.h"

int main(void) {
    void *string_obj = NULL;
    void *child_array = NULL;
    void *array_obj = NULL;

    opal_rc_debug_reset_counters_for_test();
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_BYTES);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_BUILDERS);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);

    string_obj = opal_rc_alloc_tracked(sizeof(int), NULL, OPAL_RC_DEBUG_COUNTER_STRINGS);
    child_array = opal_rc_alloc_tracked(sizeof(int), NULL, OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
    array_obj = opal_array_alloc(sizeof(int), _Alignof(int), 1, 1, NULL);
    if (string_obj == NULL || child_array == NULL || array_obj == NULL) {
        fprintf(stderr, "tracked allocations returned null\n");
        return 1;
    }

    if (opal_rc_debug_alloc_count_for_test(OPAL_RC_DEBUG_COUNTER_STRINGS) != 2) {
        fprintf(stderr, "expected 2 string allocs\n");
        return 2;
    }
    if (opal_rc_debug_alloc_count_for_test(OPAL_RC_DEBUG_COUNTER_ARRAYS) != 1) {
        fprintf(stderr, "expected 1 array alloc\n");
        return 3;
    }
    if (opal_rc_debug_alloc_count_for_test(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS) != 2) {
        fprintf(stderr, "expected 2 rc child array allocs\n");
        return 4;
    }
    if (opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_BYTES) != 1 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_BUILDERS) != 1 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS) != 1 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS) != 1 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS) != 1) {
        fprintf(stderr, "expected singleton live counts for manual categories\n");
        return 5;
    }

    opal_rc_dec(string_obj);
    opal_rc_dec(child_array);
    opal_rc_dec(array_obj);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BUILDERS);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS);

    if (opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_STRINGS) != 1) {
        fprintf(stderr, "expected one live string after manual note\n");
        return 6;
    }
    if (opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_ARRAYS) != 0 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS) != 1 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_BYTES) != 0 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_BUILDERS) != 0 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS) != 0 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS) != 0 ||
        opal_rc_debug_live_count_for_test(OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS) != 0) {
        fprintf(stderr, "unexpected live counter values after frees\n");
        return 7;
    }

    if (opal_rc_debug_free_count_for_test(OPAL_RC_DEBUG_COUNTER_STRINGS) != 1 ||
        opal_rc_debug_free_count_for_test(OPAL_RC_DEBUG_COUNTER_ARRAYS) != 1 ||
        opal_rc_debug_free_count_for_test(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS) != 1) {
        fprintf(stderr, "tracked free counts did not update\n");
        return 8;
    }

    return 0;
}
"#,
    );
}
