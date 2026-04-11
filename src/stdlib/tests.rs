//! Test suite for the Opalescent core standard library.
//!
//! All tests are inline with no filesystem I/O. Tests follow TDD red-green-refactor
//! discipline: they were written before the corresponding implementation.
#[cfg(test)]
#[expect(
    clippy::module_inception,
    reason = "test modules nested in test files follow this pattern throughout the codebase"
)]
mod tests {
    use crate::stdlib::fs::{FileSystem, FsError, MockFileSystem};
    use crate::stdlib::io::MockStdlibIoHandler;
    use crate::stdlib::math;
    use crate::stdlib::strings;
    use crate::stdlib::types;

    // =========================================================================
    // types — checked arithmetic
    // =========================================================================

    /// Verify `checked_add_i64` returns `Some` for non-overflowing addition.
    #[test]
    fn test_checked_add_i64_normal() {
        assert_eq!(
            types::checked_add_i64(10_i64, 20_i64),
            Some(30_i64),
            "10 + 20 should equal 30"
        );
    }

    /// Verify `checked_add_i64` returns `None` on overflow.
    #[test]
    fn test_checked_add_i64_overflow() {
        assert_eq!(
            types::checked_add_i64(i64::MAX, 1_i64),
            None,
            "i64::MAX + 1 must overflow and return None"
        );
    }

    /// Verify `checked_sub_i64` underflow returns `None`.
    #[test]
    fn test_checked_sub_i64_underflow() {
        assert_eq!(
            types::checked_sub_i64(i64::MIN, 1_i64),
            None,
            "i64::MIN - 1 must underflow and return None"
        );
    }

    /// Verify `checked_mul_i64` returns `Some` for a normal multiplication.
    #[test]
    fn test_checked_mul_i64_normal() {
        assert_eq!(
            types::checked_mul_i64(6_i64, 7_i64),
            Some(42_i64),
            "6 * 7 should equal 42"
        );
    }

    /// Verify `checked_div_i64` returns `None` when dividing by zero.
    #[test]
    fn test_checked_div_i64_by_zero() {
        assert_eq!(
            types::checked_div_i64(10_i64, 0_i64),
            None,
            "division by zero must return None"
        );
    }

    /// Verify `checked_add_i32` returns `Some` for normal addition.
    #[test]
    fn test_checked_add_i32_normal() {
        assert_eq!(
            types::checked_add_i32(100_i32, 200_i32),
            Some(300_i32),
            "100 + 200 should equal 300 for i32"
        );
    }

    /// Verify `checked_add_i32` returns `None` on overflow.
    #[test]
    fn test_checked_add_i32_overflow() {
        assert_eq!(
            types::checked_add_i32(i32::MAX, 1_i32),
            None,
            "i32::MAX + 1 must overflow"
        );
    }

    /// Verify `saturating_add_i64` clamps to `i64::MAX` on overflow.
    #[test]
    fn test_saturating_add_i64_overflow_clamps() {
        assert_eq!(
            types::saturating_add_i64(i64::MAX, 1_i64),
            i64::MAX,
            "saturating add must clamp at i64::MAX"
        );
    }

    /// Verify `saturating_sub_i64` clamps to `i64::MIN` on underflow.
    #[test]
    fn test_saturating_sub_i64_underflow_clamps() {
        assert_eq!(
            types::saturating_sub_i64(i64::MIN, 1_i64),
            i64::MIN,
            "saturating sub must clamp at i64::MIN"
        );
    }

    /// Verify `saturating_mul_i64` normal case.
    #[test]
    fn test_saturating_mul_i64_normal() {
        assert_eq!(
            types::saturating_mul_i64(3_i64, 4_i64),
            12_i64,
            "3 * 4 should equal 12 for saturating mul"
        );
    }

    /// Verify `checked_add_u64` returns `Some` for normal addition.
    #[test]
    fn test_checked_add_u64_normal() {
        assert_eq!(
            types::checked_add_u64(10_u64, 20_u64),
            Some(30_u64),
            "10 + 20 should be 30 for u64"
        );
    }

    /// Verify `checked_add_u64` returns `None` on overflow.
    #[test]
    fn test_checked_add_u64_overflow() {
        assert_eq!(
            types::checked_add_u64(u64::MAX, 1_u64),
            None,
            "u64::MAX + 1 must overflow"
        );
    }

    /// Verify `checked_add_u32` for unsigned 32-bit.
    #[test]
    fn test_checked_add_u32_normal() {
        assert_eq!(
            types::checked_add_u32(50_u32, 50_u32),
            Some(100_u32),
            "50 + 50 should equal 100 for u32"
        );
    }

    /// Verify `saturating_add_u32` clamps at max.
    #[test]
    fn test_saturating_add_u32_overflow_clamps() {
        assert_eq!(
            types::saturating_add_u32(u32::MAX, 1_u32),
            u32::MAX,
            "saturating add must clamp at u32::MAX"
        );
    }

    /// Verify `checked_add_i8` overflow.
    #[test]
    fn test_checked_add_i8_overflow() {
        assert_eq!(
            types::checked_add_i8(i8::MAX, 1_i8),
            None,
            "i8::MAX + 1 must overflow"
        );
    }

    /// Verify `checked_add_u8` normal.
    #[test]
    fn test_checked_add_u8_normal() {
        assert_eq!(
            types::checked_add_u8(10_u8, 20_u8),
            Some(30_u8),
            "10 + 20 should equal 30 for u8"
        );
    }

    /// Verify `checked_add_i16` normal.
    #[test]
    fn test_checked_add_i16_normal() {
        assert_eq!(
            types::checked_add_i16(100_i16, 200_i16),
            Some(300_i16),
            "100 + 200 should equal 300 for i16"
        );
    }

    /// Verify `checked_add_u16` overflow.
    #[test]
    fn test_checked_add_u16_overflow() {
        assert_eq!(
            types::checked_add_u16(u16::MAX, 1_u16),
            None,
            "u16::MAX + 1 must overflow"
        );
    }

    // =========================================================================
    // math — constants
    // =========================================================================

    /// Verify PI constant approximation.
    #[test]
    fn test_math_pi_constant() {
        let delta = (math::PI - core::f64::consts::PI).abs();
        assert!(delta < 1e-10_f64, "PI should equal core::f64::consts::PI");
    }

    /// Verify E constant approximation.
    #[test]
    fn test_math_e_constant() {
        let delta = (math::E - core::f64::consts::E).abs();
        assert!(delta < 1e-10_f64, "E should equal core::f64::consts::E");
    }

    /// Verify INFINITY is positive infinity.
    #[test]
    fn test_math_infinity() {
        assert!(
            math::INFINITY.is_infinite() && math::INFINITY.is_sign_positive(),
            "INFINITY must be positive infinity"
        );
    }

    /// Verify NAN is not a number.
    #[test]
    fn test_math_nan() {
        assert!(math::NAN.is_nan(), "NAN must satisfy is_nan()");
    }

    // =========================================================================
    // math — integer functions
    // =========================================================================

    /// Verify `abs_i64` of negative value returns absolute value.
    #[test]
    fn test_math_abs_i64_negative() {
        assert_eq!(math::abs_i64(-42_i64), 42_i64, "abs_i64(-42) should be 42");
    }

    /// Verify `abs_i64` of positive value is unchanged.
    #[test]
    fn test_math_abs_i64_positive() {
        assert_eq!(math::abs_i64(7_i64), 7_i64, "abs_i64(7) should be 7");
    }

    /// Verify `abs_i64` of `i64::MIN` clamps at `i64::MAX` via `saturating_abs`.
    #[test]
    fn test_math_abs_i64_min_saturates() {
        assert_eq!(
            math::abs_i64(i64::MIN),
            i64::MAX,
            "abs_i64(i64::MIN) must saturate at i64::MAX"
        );
    }

    /// Verify `min_i64` returns smaller value.
    #[test]
    fn test_math_min_i64() {
        assert_eq!(
            math::min_i64(10_i64, 20_i64),
            10_i64,
            "min_i64(10, 20) should be 10"
        );
    }

    /// Verify `max_i64` returns larger value.
    #[test]
    fn test_math_max_i64() {
        assert_eq!(
            math::max_i64(10_i64, 20_i64),
            20_i64,
            "max_i64(10, 20) should be 20"
        );
    }

    // =========================================================================
    // math — floating point functions
    // =========================================================================

    /// Verify `abs_f64` of negative float.
    #[test]
    fn test_math_abs_f64_negative() {
        let result = math::abs_f64(-3.5_f64);
        let delta = (result - 3.5_f64).abs();
        assert!(delta < 1e-10_f64, "abs_f64(-3.5) should be 3.5");
    }

    /// Verify ceil rounds up.
    #[test]
    fn test_math_ceil() {
        let result = math::ceil(1.2_f64);
        let delta = (result - 2.0_f64).abs();
        assert!(delta < 1e-10_f64, "ceil(1.2) should be 2.0");
    }

    /// Verify floor rounds down.
    #[test]
    fn test_math_floor() {
        let result = math::floor(1.8_f64);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "floor(1.8) should be 1.0");
    }

    /// Verify round rounds to nearest.
    #[test]
    fn test_math_round_up() {
        let result = math::round(1.5_f64);
        let delta = (result - 2.0_f64).abs();
        assert!(delta < 1e-10_f64, "round(1.5) should be 2.0");
    }

    /// Verify round rounds down for < 0.5.
    #[test]
    fn test_math_round_down() {
        let result = math::round(1.4_f64);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "round(1.4) should be 1.0");
    }

    /// Verify sqrt of 4.0 is 2.0.
    #[test]
    fn test_math_sqrt() {
        let result = math::sqrt(4.0_f64);
        let delta = (result - 2.0_f64).abs();
        assert!(delta < 1e-10_f64, "sqrt(4.0) should be 2.0");
    }

    /// Verify pow(2.0, 10.0) is 1024.0.
    #[test]
    fn test_math_pow() {
        let result = math::pow(2.0_f64, 10.0_f64);
        let delta = (result - 1_024.0_f64).abs();
        assert!(delta < 1e-6_f64, "pow(2.0, 10.0) should be 1024.0");
    }

    /// Verify log(E) is approximately 1.0.
    #[test]
    fn test_math_log_e() {
        let result = math::log(math::E);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "log(E) should be 1.0");
    }

    /// Verify log10(100.0) is approximately 2.0.
    #[test]
    fn test_math_log10() {
        let result = math::log10(100.0_f64);
        let delta = (result - 2.0_f64).abs();
        assert!(delta < 1e-10_f64, "log10(100.0) should be 2.0");
    }

    /// Verify sin(PI/2) is approximately 1.0.
    #[test]
    fn test_math_sin() {
        let result = math::sin(math::PI / 2.0_f64);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "sin(PI/2) should be 1.0");
    }

    /// Verify cos(0.0) is approximately 1.0.
    #[test]
    fn test_math_cos() {
        let result = math::cos(0.0_f64);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "cos(0.0) should be 1.0");
    }

    /// Verify tan(PI/4) is approximately 1.0.
    #[test]
    fn test_math_tan() {
        let result = math::tan(math::PI / 4.0_f64);
        let delta = (result - 1.0_f64).abs();
        assert!(delta < 1e-10_f64, "tan(PI/4) should be 1.0");
    }

    /// Verify atan2(1.0, 1.0) is approximately PI/4.
    #[test]
    fn test_math_atan2() {
        let result = math::atan2(1.0_f64, 1.0_f64);
        let expected = math::PI / 4.0_f64;
        let delta = (result - expected).abs();
        assert!(delta < 1e-10_f64, "atan2(1.0, 1.0) should be PI/4");
    }

    /// Verify `min_f64` returns smaller value.
    #[test]
    fn test_math_min_f64() {
        let result = math::min_f64(1.5_f64, 2.5_f64);
        let delta = (result - 1.5_f64).abs();
        assert!(delta < 1e-10_f64, "min_f64(1.5, 2.5) should be 1.5");
    }

    /// Verify `max_f64` returns larger value.
    #[test]
    fn test_math_max_f64() {
        let result = math::max_f64(1.5_f64, 2.5_f64);
        let delta = (result - 2.5_f64).abs();
        assert!(delta < 1e-10_f64, "max_f64(1.5, 2.5) should be 2.5");
    }

    // =========================================================================
    // strings — operations
    // =========================================================================

    /// Verify concat joins two strings.
    #[test]
    fn test_strings_concat() {
        let result = strings::concat("hello", " world");
        assert_eq!(result, "hello world", "concat should join strings");
    }

    /// Verify concat with empty strings.
    #[test]
    fn test_strings_concat_empty() {
        let result = strings::concat("", "abc");
        assert_eq!(result, "abc", "concat with empty left should return right");
    }

    /// Verify length returns character count (Unicode-aware).
    #[test]
    fn test_strings_length() {
        assert_eq!(strings::length("hello"), 5_usize, "length of 'hello' is 5");
    }

    /// Verify length of empty string.
    #[test]
    fn test_strings_length_empty() {
        assert_eq!(strings::length(""), 0_usize, "length of empty string is 0");
    }

    /// Verify length counts Unicode scalar values, not bytes.
    #[test]
    fn test_strings_length_unicode() {
        // "é" is one Unicode scalar value but two bytes in UTF-8
        assert_eq!(strings::length("é"), 1_usize, "é should count as 1 char");
    }

    /// Verify find returns `Some(index)` when substring is found.
    #[test]
    fn test_strings_find_found() {
        let result = strings::find("hello world", "world");
        assert_eq!(result, Some(6_usize), "find 'world' in 'hello world'");
    }

    /// Verify find returns `None` when substring is absent.
    #[test]
    fn test_strings_find_not_found() {
        let result = strings::find("hello world", "xyz");
        assert_eq!(result, None, "find 'xyz' in 'hello world' should be None");
    }

    /// Verify find on empty source returns `None` unless needle is also empty.
    #[test]
    fn test_strings_find_empty_needle() {
        let result = strings::find("hello", "");
        assert_eq!(result, Some(0_usize), "empty needle always found at 0");
    }

    /// Verify replace substitutes occurrences.
    #[test]
    fn test_strings_replace() {
        let result = strings::replace("hello world", "world", "Opalescent");
        assert_eq!(
            result, "hello Opalescent",
            "replace should substitute 'world'"
        );
    }

    /// Verify replace with no match returns original.
    #[test]
    fn test_strings_replace_no_match() {
        let result = strings::replace("hello", "xyz", "abc");
        assert_eq!(result, "hello", "replace with no match returns original");
    }

    /// Verify split divides by delimiter.
    #[test]
    fn test_strings_split() {
        let result = strings::split("a,b,c", ",");
        assert_eq!(result, vec!["a", "b", "c"], "split by comma");
    }

    /// Verify split with no delimiter returns single-element vec.
    #[test]
    fn test_strings_split_no_match() {
        let result = strings::split("abc", ",");
        assert_eq!(
            result,
            vec!["abc"],
            "split with no match returns whole string"
        );
    }

    /// Verify trim removes leading/trailing whitespace.
    #[test]
    fn test_strings_trim() {
        let result = strings::trim("  hello  ");
        assert_eq!(result, "hello", "trim should remove surrounding whitespace");
    }

    /// Verify trim on clean string is unchanged.
    #[test]
    fn test_strings_trim_clean() {
        let result = strings::trim("hello");
        assert_eq!(result, "hello", "trim on clean string should be unchanged");
    }

    /// Verify `to_upper` converts lowercase to uppercase.
    #[test]
    fn test_strings_to_upper() {
        let result = strings::to_upper("hello");
        assert_eq!(result, "HELLO", "to_upper('hello') should be 'HELLO'");
    }

    /// Verify `to_lower` converts uppercase to lowercase.
    #[test]
    fn test_strings_to_lower() {
        let result = strings::to_lower("HELLO");
        assert_eq!(result, "hello", "to_lower('HELLO') should be 'hello'");
    }

    /// Verify slice returns substring for valid range.
    #[test]
    fn test_strings_slice_valid() {
        let result = strings::slice("hello world", 0_usize, 5_usize);
        assert_eq!(
            result,
            Some(String::from("hello")),
            "slice(0, 5) of 'hello world'"
        );
    }

    /// Verify slice returns `None` for out-of-range end.
    #[test]
    fn test_strings_slice_out_of_range() {
        let result = strings::slice("hello", 0_usize, 100_usize);
        assert_eq!(result, None, "slice out of range should return None");
    }

    /// Verify slice with start > end returns `None`.
    #[test]
    fn test_strings_slice_inverted_range() {
        let result = strings::slice("hello", 5_usize, 2_usize);
        assert_eq!(result, None, "slice with start > end should return None");
    }

    // =========================================================================
    // io — mockable StdlibIoHandler
    // =========================================================================

    /// Verify `println` writes message with newline via mock handler.
    #[test]
    fn test_io_println_uses_handler() {
        let mut handler = MockStdlibIoHandler::new();
        handler.queue_input("ignored");
        crate::stdlib::io::println(&mut handler, "hello").expect("println should succeed");
        let output = handler.take_output();
        assert_eq!(output, "hello\n", "println should append newline");
    }

    /// Verify `print` writes message without newline via mock handler.
    #[test]
    fn test_io_print_no_newline() {
        let mut handler = MockStdlibIoHandler::new();
        crate::stdlib::io::print(&mut handler, "hello").expect("print should succeed");
        let output = handler.take_output();
        assert_eq!(output, "hello", "print should not append newline");
    }

    /// Verify `read_line` returns queued input from mock handler.
    #[test]
    fn test_io_read_line_returns_input() {
        let mut handler = MockStdlibIoHandler::new();
        handler.queue_input("user input");
        let result = crate::stdlib::io::read_line(&mut handler).expect("read_line should succeed");
        assert_eq!(result, "user input", "read_line should return queued input");
    }

    /// Verify multiple prints accumulate in output buffer.
    #[test]
    fn test_io_multiple_prints_accumulate() {
        let mut handler = MockStdlibIoHandler::new();
        crate::stdlib::io::print(&mut handler, "a").expect("first print");
        crate::stdlib::io::print(&mut handler, "b").expect("second print");
        let output = handler.take_output();
        assert_eq!(output, "ab", "multiple prints should accumulate output");
    }

    // =========================================================================
    // fs — FileSystem trait with MockFileSystem
    // =========================================================================

    /// Verify `write_file` + `read_file` round-trip via mock filesystem.
    #[test]
    fn test_fs_write_then_read() {
        let mut fs = MockFileSystem::new();
        fs.write_file("test.txt", "content")
            .expect("write should succeed");
        let result = fs.read_file("test.txt").expect("read should succeed");
        assert_eq!(result, "content", "read should return written content");
    }

    /// Verify `read_file` returns error for nonexistent file.
    #[test]
    fn test_fs_read_nonexistent() {
        let fs = MockFileSystem::new();
        let result = fs.read_file("missing.txt");
        assert!(result.is_err(), "reading missing file should return Err");
        assert!(
            matches!(result, Err(FsError::NotFound { .. })),
            "error should be NotFound variant"
        );
    }

    /// Verify `file_exists` returns true for written file.
    #[test]
    fn test_fs_file_exists_after_write() {
        let mut fs = MockFileSystem::new();
        fs.write_file("existing.txt", "data")
            .expect("write should succeed");
        assert!(
            fs.file_exists("existing.txt"),
            "file_exists should return true after write"
        );
    }

    /// Verify `file_exists` returns false for nonexistent file.
    #[test]
    fn test_fs_file_exists_missing() {
        let fs = MockFileSystem::new();
        assert!(
            !fs.file_exists("nope.txt"),
            "file_exists should return false for missing file"
        );
    }

    /// Verify `list_dir` returns files added to a directory.
    #[test]
    fn test_fs_list_dir() {
        let mut fs = MockFileSystem::new();
        fs.write_file("dir/a.txt", "a").expect("write a");
        fs.write_file("dir/b.txt", "b").expect("write b");
        let mut result = fs.list_dir("dir").expect("list_dir should succeed");
        result.sort();
        assert!(
            result.contains(&String::from("dir/a.txt")),
            "list_dir should include dir/a.txt"
        );
        assert!(
            result.contains(&String::from("dir/b.txt")),
            "list_dir should include dir/b.txt"
        );
    }

    /// Verify `list_dir` returns empty vec for empty directory.
    #[test]
    fn test_fs_list_dir_empty() {
        let fs = MockFileSystem::new();
        let result = fs.list_dir("empty_dir");
        assert!(
            result.is_err() || result.is_ok_and(|v| v.is_empty()),
            "list_dir on unknown dir should return empty or error"
        );
    }

    /// Verify `write_file` overwrites existing content.
    #[test]
    fn test_fs_write_overwrites() {
        let mut fs = MockFileSystem::new();
        fs.write_file("file.txt", "original").expect("first write");
        fs.write_file("file.txt", "updated").expect("second write");
        let result = fs.read_file("file.txt").expect("read should succeed");
        assert_eq!(result, "updated", "second write should overwrite first");
    }
}
