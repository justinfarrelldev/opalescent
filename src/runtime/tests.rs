#![allow(
    warnings,
    clippy::all,
    clippy::panic,
    reason = "test harness uses panic-based assertions"
)]
extern crate alloc;

use crate::runtime::arrays::{allocate_array, array_index, array_length};
use crate::runtime::errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
use crate::runtime::io::{IoHandler, print, take_input};
use crate::runtime::memory::{OpalArray, OpalString, OpalWeakRef, RuntimeAllocator};
use crate::runtime::reporting::format_runtime_error;
use crate::runtime::stdlib::{
    RandomIntSource, format_interpolated_string, opal_array_slice, random_int32_with_source,
    string_to_int32,
};
use crate::runtime::strings::{string_compare, string_concat, string_equals, string_length};
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use std::fs;
use std::process::Command;

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
