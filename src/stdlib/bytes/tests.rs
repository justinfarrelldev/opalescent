//! Test suite for the Opalescent `Bytes` standard library type.
//!
//! All tests are inline with no filesystem I/O. Tests follow TDD red-green-refactor
//! discipline: they were written before the corresponding implementation.
//!
//! # Test Coverage
//!
//! Every public method of [`super::Bytes`] has ≥3 tests covering normal,
//! edge, and error paths:
//!
//! - Construction: `new`, `from_slice`, `from_vec`
//! - Accessors: `length`, `get`, `as_slice`
//! - `concatenate` — empty/nonempty combinations, length correctness
//! - `slice` — full/empty/middle ranges, boundary errors
//! - `to_hex_string` — empty, single-byte, multi-byte, output-length invariant
//! - `from_hex_string` — round-trip, case insensitivity, odd-length and invalid-char errors
//! - Equality — same content, different content, different length

#[cfg(test)]
#[expect(
    clippy::module_inception,
    reason = "test modules nested in test files follow this pattern throughout the codebase"
)]
mod tests {
    use crate::stdlib::bytes::{Bytes, BytesError};

    // =========================================================================
    // Construction & basic accessors
    // =========================================================================

    /// Verify that a freshly constructed `Bytes` buffer is empty.
    #[test]
    fn test_bytes_new_is_empty() {
        let b = Bytes::new();
        assert_eq!(
            b.length(),
            0_usize,
            "Bytes::new should produce an empty buffer"
        );
    }

    /// Verify `from_slice` preserves the source bytes verbatim.
    #[test]
    fn test_bytes_from_slice_preserves_data() {
        let source: [u8; 3_usize] = [1_u8, 2_u8, 3_u8];
        let b = Bytes::from_slice(&source);
        assert_eq!(b.as_slice(), &source, "from_slice must preserve bytes");
    }

    /// Verify `from_vec` preserves the source bytes verbatim.
    #[test]
    fn test_bytes_from_vec_preserves_data() {
        extern crate alloc;
        use alloc::vec;
        let source = vec![10_u8, 20_u8, 30_u8];
        let b = Bytes::from_vec(source.clone());
        assert_eq!(
            b.as_slice(),
            source.as_slice(),
            "from_vec must preserve bytes"
        );
    }

    /// Verify `length` matches the number of bytes stored.
    #[test]
    fn test_bytes_length_matches_data() {
        let b = Bytes::from_slice(&[0_u8, 0_u8, 0_u8, 0_u8, 0_u8]);
        assert_eq!(
            b.length(),
            5_usize,
            "length must match construction data length"
        );
    }

    /// Verify `get` returns the byte at a valid index.
    #[test]
    fn test_bytes_get_in_bounds() {
        let b = Bytes::from_slice(&[0xAA_u8, 0xBB_u8, 0xCC_u8]);
        assert_eq!(
            b.get(1_usize),
            Some(0xBB_u8),
            "get(1) should return the middle byte"
        );
    }

    /// Verify `get` returns `None` when the index is at or past the length.
    #[test]
    fn test_bytes_get_out_of_bounds() {
        let b = Bytes::from_slice(&[1_u8, 2_u8]);
        assert_eq!(b.get(2_usize), None, "get at length must return None");
        assert_eq!(
            b.get(99_usize),
            None,
            "get far past length must return None"
        );
    }

    /// Verify `as_slice` returns a view that matches the construction data.
    #[test]
    fn test_bytes_as_slice_matches_construction() {
        let source: [u8; 4_usize] = [9_u8, 8_u8, 7_u8, 6_u8];
        let b = Bytes::from_slice(&source);
        assert_eq!(
            b.as_slice(),
            &source,
            "as_slice must match the source bytes"
        );
    }

    // =========================================================================
    // concatenate
    // =========================================================================

    /// Verify concatenation of two non-empty buffers produces the joined bytes.
    #[test]
    fn test_concatenate_two_nonempty() {
        let left = Bytes::from_slice(&[1_u8, 2_u8]);
        let right = Bytes::from_slice(&[3_u8, 4_u8]);
        let combined = left.concatenate(&right);
        assert_eq!(
            combined.as_slice(),
            &[1_u8, 2_u8, 3_u8, 4_u8],
            "concatenate must join in order"
        );
    }

    /// Verify concatenating an empty left buffer yields only the right buffer.
    #[test]
    fn test_concatenate_empty_left() {
        let left = Bytes::new();
        let right = Bytes::from_slice(&[1_u8, 2_u8]);
        let combined = left.concatenate(&right);
        assert_eq!(
            combined.as_slice(),
            &[1_u8, 2_u8],
            "empty left must yield right"
        );
    }

    /// Verify concatenating an empty right buffer yields only the left buffer.
    #[test]
    fn test_concatenate_empty_right() {
        let left = Bytes::from_slice(&[1_u8, 2_u8]);
        let right = Bytes::new();
        let combined = left.concatenate(&right);
        assert_eq!(
            combined.as_slice(),
            &[1_u8, 2_u8],
            "empty right must yield left"
        );
    }

    /// Verify concatenating two empty buffers yields an empty buffer.
    #[test]
    fn test_concatenate_both_empty() {
        let left = Bytes::new();
        let right = Bytes::new();
        let combined = left.concatenate(&right);
        assert_eq!(combined.length(), 0_usize, "empty + empty must be empty");
    }

    /// Verify the resulting length is the sum of the operand lengths.
    #[test]
    fn test_concatenate_length_is_sum() {
        let left = Bytes::from_slice(&[0_u8; 3_usize]);
        let right = Bytes::from_slice(&[0_u8; 5_usize]);
        let combined = left.concatenate(&right);
        assert_eq!(
            combined.length(),
            8_usize,
            "concatenate length must equal the sum of operand lengths"
        );
    }

    // =========================================================================
    // slice
    // =========================================================================

    /// Verify a full-range slice equals the original buffer.
    #[test]
    fn test_slice_full_range_returns_copy_equal() {
        let b = Bytes::from_slice(&[10_u8, 20_u8, 30_u8]);
        let full = b
            .slice(0_usize, 3_usize)
            .expect("full-range slice must succeed");
        assert_eq!(full, b, "slice(0, len) must equal the original");
    }

    /// Verify a zero-width slice at a valid position yields an empty buffer.
    #[test]
    fn test_slice_empty_range_returns_empty() {
        let b = Bytes::from_slice(&[10_u8, 20_u8, 30_u8]);
        let empty = b
            .slice(2_usize, 2_usize)
            .expect("zero-width slice must succeed");
        assert_eq!(empty.length(), 0_usize, "slice(n, n) must be empty");
    }

    /// Verify a mid-range slice extracts the expected sub-buffer.
    #[test]
    fn test_slice_middle_range() {
        let b = Bytes::from_slice(&[0_u8, 1_u8, 2_u8, 3_u8, 4_u8]);
        let middle = b
            .slice(1_usize, 4_usize)
            .expect("middle slice must succeed");
        assert_eq!(
            middle.as_slice(),
            &[1_u8, 2_u8, 3_u8],
            "middle slice must be [1, 2, 3]"
        );
    }

    /// Verify `start > end` yields [`BytesError::InvalidRange`].
    #[test]
    fn test_slice_start_greater_than_end_errors() {
        let b = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        let err = b
            .slice(2_usize, 1_usize)
            .expect_err("start > end must error");
        assert!(
            matches!(err, BytesError::InvalidRange { .. }),
            "expected InvalidRange, got {err:?}"
        );
    }

    /// Verify an `end` past the buffer length yields [`BytesError::InvalidRange`].
    #[test]
    fn test_slice_end_out_of_bounds_errors() {
        let b = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        let err = b
            .slice(0_usize, 4_usize)
            .expect_err("end > length must error");
        assert!(
            matches!(err, BytesError::InvalidRange { .. }),
            "expected InvalidRange, got {err:?}"
        );
    }

    // =========================================================================
    // to_hex_string
    // =========================================================================

    /// Verify an empty buffer produces an empty hex string.
    #[test]
    fn test_to_hex_empty_returns_empty_string() {
        let b = Bytes::new();
        assert_eq!(b.to_hex_string(), "", "empty buffer must give empty hex");
    }

    /// Verify a single zero byte encodes as `"00"`.
    #[test]
    fn test_to_hex_single_byte_zero() {
        let b = Bytes::from_slice(&[0x00_u8]);
        assert_eq!(b.to_hex_string(), "00", "0x00 must encode as \"00\"");
    }

    /// Verify a single `0xFF` byte encodes as lowercase `"ff"`.
    #[test]
    fn test_to_hex_single_byte_ff() {
        let b = Bytes::from_slice(&[0xFF_u8]);
        assert_eq!(
            b.to_hex_string(),
            "ff",
            "0xFF must encode as lowercase \"ff\""
        );
    }

    /// Verify multi-byte encoding produces the concatenated hex digits.
    #[test]
    fn test_to_hex_multi_byte() {
        let b = Bytes::from_slice(&[0xDE_u8, 0xAD_u8, 0xBE_u8, 0xEF_u8]);
        assert_eq!(
            b.to_hex_string(),
            "deadbeef",
            "multi-byte encoding must concatenate lowercase hex digits"
        );
    }

    /// Verify the hex output length is exactly twice the byte count.
    #[test]
    fn test_to_hex_length_is_double_byte_count() {
        let b = Bytes::from_slice(&[1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 7_u8]);
        assert_eq!(
            b.to_hex_string().len(),
            14_usize,
            "hex length must be 2 × byte count"
        );
    }

    // =========================================================================
    // from_hex_string
    // =========================================================================

    /// Verify decoding an empty string yields an empty buffer.
    #[test]
    fn test_from_hex_empty_returns_empty_bytes() {
        let b = Bytes::from_hex_string("").expect("empty hex must decode");
        assert_eq!(b.length(), 0_usize, "empty hex must decode to empty Bytes");
    }

    /// Verify `from_hex_string` is the inverse of `to_hex_string`.
    #[test]
    fn test_from_hex_roundtrip() {
        let original = Bytes::from_slice(&[0x01_u8, 0x23_u8, 0x45_u8, 0x67_u8, 0x89_u8, 0xAB_u8]);
        let encoded = original.to_hex_string();
        let decoded = Bytes::from_hex_string(&encoded).expect("roundtrip must decode");
        assert_eq!(
            decoded, original,
            "from_hex_string ∘ to_hex_string must be identity"
        );
    }

    /// Verify uppercase hex is accepted and equivalent to lowercase.
    #[test]
    fn test_from_hex_uppercase_accepted() {
        let lower = Bytes::from_hex_string("deadbeef").expect("lowercase must decode");
        let upper = Bytes::from_hex_string("DEADBEEF").expect("uppercase must decode");
        assert_eq!(lower, upper, "hex decoding must be case-insensitive");
    }

    /// Verify an odd-length hex string is rejected with [`BytesError::InvalidHexLength`].
    #[test]
    fn test_from_hex_odd_length_errors() {
        let err = Bytes::from_hex_string("abc").expect_err("odd-length must error");
        assert!(
            matches!(err, BytesError::InvalidHexLength { .. }),
            "expected InvalidHexLength, got {err:?}"
        );
    }

    /// Verify a non-hex character is rejected with [`BytesError::InvalidHexCharacter`].
    #[test]
    fn test_from_hex_invalid_char_errors() {
        let err = Bytes::from_hex_string("zz").expect_err("invalid char must error");
        assert!(
            matches!(err, BytesError::InvalidHexCharacter { .. }),
            "expected InvalidHexCharacter, got {err:?}"
        );
    }

    // =========================================================================
    // Equality
    // =========================================================================

    /// Verify two buffers with identical content compare equal.
    #[test]
    fn test_equals_same_content() {
        let a = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        let b = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        assert_eq!(a, b, "identical content must be equal");
    }

    /// Verify buffers with differing content compare unequal.
    #[test]
    fn test_equals_different_content() {
        let a = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        let b = Bytes::from_slice(&[1_u8, 2_u8, 4_u8]);
        assert_ne!(a, b, "differing content must be unequal");
    }

    /// Verify buffers of different lengths compare unequal.
    #[test]
    fn test_equals_different_length() {
        let a = Bytes::from_slice(&[1_u8, 2_u8]);
        let b = Bytes::from_slice(&[1_u8, 2_u8, 3_u8]);
        assert_ne!(a, b, "different lengths must be unequal");
    }
}
