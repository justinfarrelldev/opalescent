extern crate alloc;

use crate::runtime::arrays::{allocate_array, array_index, array_length};
use crate::runtime::errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
use crate::runtime::io::{print, take_input, IoHandler};
use crate::runtime::memory::{OpalArray, OpalString, OpalWeakRef, RuntimeAllocator};
use crate::runtime::reporting::format_runtime_error;
use crate::runtime::stdlib::{
    format_interpolated_string, opal_array_slice, random_int32_with_source, string_to_int32,
    RandomIntSource,
};
use crate::runtime::strings::{string_compare, string_concat, string_equals, string_length};
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

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
