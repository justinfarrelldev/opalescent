extern crate alloc;

use crate::runtime::arrays::{allocate_array, array_index, array_length};
use crate::runtime::errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
use crate::runtime::io::{print, take_input, IoHandler};
use crate::runtime::memory::{OpalArray, OpalString, RuntimeAllocator};
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
    assert!(print_result.is_ok(), "print should write through injected handler");
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
    assert!(mapped.is_err(), "error mapping helper should convert to runtime error");
    assert_eq!(
        mapped.err(),
        Some(RuntimeError::UserError {
            code: 7_123,
            message: String::from("io failed: oops"),
        }),
        "result extension helper should preserve code and combined message"
    );
}
