//! Dedicated `Bytes` type for the Opalescent standard library.
//!
//! This module is the Rust-side implementation of the `Bytes` type proposal from
//! `stdlib-proposals/byte-buffer-type/dedicated-bytes-type/`. It provides a
//! struct-backed byte buffer that wraps an owned `Vec<u8>` with a dedicated,
//! semantically clear API for binary data — concatenation, slicing, and hex
//! encoding/decoding.
//!
//! # Rationale
//!
//! Opalescent distinguishes generic `uint8[]` arrays from binary data buffers
//! through this dedicated type. This yields:
//!
//! - **Semantic clarity**: function signatures that take `Bytes` document intent.
//! - **Type safety**: a binary buffer cannot be confused with a general array of
//!   small integers at the type level.
//! - **Extensibility**: the struct can grow capacity / encoding-hint metadata
//!   without breaking any public function signature.
//!
//! # Error Model
//!
//! All fallible operations return `Result<_, BytesError>`. This matches
//! Opalescent's fail-fast-on-primitives language goal: no panics, no silent
//! truncation, no implicit wrapping. Callers must handle every error path.
//!
//! # Immutability
//!
//! `Bytes` is immutable by default, in line with Opalescent's language design.
//! Derived buffers (`concatenate`, `slice`, `from_hex_string`) are always fresh
//! owned values.  No in-place mutation is exposed.
//!
//! # `no_std` compatibility
//!
//! This module depends only on `alloc` and `core`. It is safe to link into
//! embedded targets, LLVM-generated runtime libraries, and hot-reloadable
//! modules.
//!
//! # Example
//!
//! ```
//! use opalescent::stdlib::bytes::Bytes;
//!
//! let header = Bytes::from_slice(&[0xDE_u8, 0xAD_u8]);
//! let body = Bytes::from_slice(&[0xBE_u8, 0xEF_u8]);
//! let packet = header.concatenate(&body);
//! assert_eq!(packet.to_hex_string(), "deadbeef");
//! ```

extern crate alloc;

use alloc::vec::Vec;

#[cfg(test)]
mod tests;

/// Number of bits in a hex nibble — used for byte↔hex conversions.
const NIBBLE_BITS: u8 = 4_u8;

/// Mask isolating the low nibble of a byte.
const NIBBLE_MASK: u8 = 0x0F_u8;

/// Error type for fallible [`Bytes`] operations.
///
/// Every variant carries enough context for the caller to produce a precise
/// diagnostic — we never throw away information when reporting a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytesError {
    /// The requested index was at or past the end of the buffer.
    IndexOutOfBounds {
        /// The index that was requested.
        index: usize,
        /// The current length of the buffer.
        length: usize,
    },
    /// The requested `[start, end)` range was invalid — either `start > end`
    /// or `end` exceeded the buffer length.
    InvalidRange {
        /// The start of the invalid range.
        start: usize,
        /// The end of the invalid range.
        end: usize,
        /// The current length of the buffer.
        length: usize,
    },
    /// The hex string had an odd number of characters and cannot form whole bytes.
    InvalidHexLength {
        /// The length of the offending hex string in characters.
        length: usize,
    },
    /// The hex string contained a non-hex character.
    InvalidHexCharacter {
        /// The character that failed to parse as a hex digit.
        character: char,
        /// The character position (0-based) within the hex string.
        position: usize,
    },
}

/// A fixed-length, immutable buffer of binary data.
///
/// `Bytes` is the dedicated type for binary payloads in the Opalescent standard
/// library. It wraps an owned `Vec<u8>` but exposes only non-mutating, fail-fast
/// operations. Derived buffers (concatenate, slice, hex decode) are always new
/// owned values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bytes {
    /// The owned byte storage. Kept private so future layout changes (capacity,
    /// encoding hints, reference-counted sharing) are non-breaking.
    data: Vec<u8>,
}

impl Bytes {
    /// Create a new, empty [`Bytes`] buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a [`Bytes`] buffer by copying from a byte slice.
    #[must_use]
    pub fn from_slice(source: &[u8]) -> Self {
        Self {
            data: source.to_vec(),
        }
    }

    /// Create a [`Bytes`] buffer by taking ownership of a `Vec<u8>`.
    #[must_use]
    pub const fn from_vec(source: Vec<u8>) -> Self {
        Self { data: source }
    }

    /// Return the number of bytes stored in this buffer.
    #[must_use]
    pub fn length(&self) -> usize {
        self.data.len()
    }

    /// Return the byte at `index`, or `None` if `index >= self.length()`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<u8> {
        self.data.get(index).copied()
    }

    /// Return an immutable slice view over the stored bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Concatenate `self` with `other`, returning a fresh buffer.
    ///
    /// The resulting length is exactly `self.length() + other.length()`.
    #[must_use]
    pub fn concatenate(&self, other: &Self) -> Self {
        let mut combined = Vec::with_capacity(self.data.len().saturating_add(other.data.len()));
        combined.extend_from_slice(&self.data);
        combined.extend_from_slice(&other.data);
        Self { data: combined }
    }

    /// Return a fresh [`Bytes`] buffer containing the half-open range
    /// `[start, end)` of `self`.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError::InvalidRange`] when `start > end` or when `end`
    /// exceeds the current length.
    pub fn slice(&self, start: usize, end: usize) -> Result<Self, BytesError> {
        if start > end || end > self.data.len() {
            return Err(BytesError::InvalidRange {
                start,
                end,
                length: self.data.len(),
            });
        }
        Ok(Self {
            data: self.data[start..end].to_vec(),
        })
    }

    /// Encode the buffer as a lowercase hexadecimal string.
    ///
    /// The output always has length `2 × self.length()`.
    #[must_use]
    pub fn to_hex_string(&self) -> alloc::string::String {
        let mut out = alloc::string::String::with_capacity(self.data.len().saturating_mul(2_usize));
        for &byte in &self.data {
            out.push(nibble_to_hex_char(byte >> NIBBLE_BITS));
            out.push(nibble_to_hex_char(byte & NIBBLE_MASK));
        }
        out
    }

    /// Decode a hexadecimal string (either case) into a fresh [`Bytes`] buffer.
    ///
    /// Accepts both uppercase (`A-F`) and lowercase (`a-f`) digits; the
    /// decoding is case-insensitive.
    ///
    /// # Errors
    ///
    /// - [`BytesError::InvalidHexLength`] — the input length is odd.
    /// - [`BytesError::InvalidHexCharacter`] — a character is not a hex digit.
    ///   The error carries the offending character and its 0-based position.
    pub fn from_hex_string(hex: &str) -> Result<Self, BytesError> {
        let bytes = hex.as_bytes();
        // Odd-length detection via a low-bit mask: equivalent to `len % 2 != 0`
        // but expressed as a bitwise AND to satisfy the arithmetic-side-effects
        // and integer-division lints.
        if (bytes.len() & 1_usize) != 0_usize {
            return Err(BytesError::InvalidHexLength {
                length: bytes.len(),
            });
        }
        // `chunks_exact(2)` guarantees each chunk is exactly two bytes, so we can
        // index `chunk[0]` and `chunk[1]` without risk of panic. The explicit
        // `enumerate` gives us the character position for diagnostic reporting.
        // Pre-size the output: two hex chars per byte, so capacity = len >> 1.
        // Using a right-shift avoids the lint against integer division while
        // producing identical results for the even-length case enforced above.
        let mut out = Vec::with_capacity(bytes.len() >> 1_usize);
        for (pair_index, chunk) in bytes.chunks_exact(2_usize).enumerate() {
            let base_position = pair_index.saturating_mul(2_usize);
            let hi = hex_char_to_nibble(chunk[0_usize], base_position)?;
            let lo = hex_char_to_nibble(chunk[1_usize], base_position.saturating_add(1_usize))?;
            out.push((hi << NIBBLE_BITS) | lo);
        }
        Ok(Self { data: out })
    }
}

/// Convert a nibble (low 4 bits) to its lowercase hex character.
///
/// Inputs outside `0..=15` are masked to the low nibble — this helper is
/// internal and is only called with already-masked values.
fn nibble_to_hex_char(nibble: u8) -> char {
    let n = nibble & NIBBLE_MASK;
    let ascii = if n < 10_u8 {
        b'0'.saturating_add(n)
    } else {
        b'a'.saturating_add(n.saturating_sub(10_u8))
    };
    char::from(ascii)
}

/// Convert a single hex ASCII byte to its 0–15 nibble value.
///
/// # Errors
///
/// Returns [`BytesError::InvalidHexCharacter`] when `byte` is not in
/// `[0-9A-Fa-f]`. The `position` parameter is threaded through to give callers
/// precise diagnostic location information.
fn hex_char_to_nibble(byte: u8, position: usize) -> Result<u8, BytesError> {
    match byte {
        b'0'..=b'9' => Ok(byte.saturating_sub(b'0')),
        b'a'..=b'f' => Ok(byte.saturating_sub(b'a').saturating_add(10_u8)),
        b'A'..=b'F' => Ok(byte.saturating_sub(b'A').saturating_add(10_u8)),
        _ => Err(BytesError::InvalidHexCharacter {
            character: char::from(byte),
            position,
        }),
    }
}
