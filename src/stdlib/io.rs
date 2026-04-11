//! Standard library I/O trait and implementations for Opalescent.
//!
//! This module wraps the lower-level `crate::runtime::io::IoHandler` with a
//! language-level API (`print`, `println`, `read_line`) that matches the Opalescent
//! built-in signatures. The trait [`StdlibIoHandler`] is injectable for test mocking
//! with no real stdin/stdout dependency.
//!
//! # Mocking
//!
//! [`MockStdlibIoHandler`] is the test-double: it captures written output in a
//! `String` buffer and returns pre-queued input lines. All stdlib tests use it
//! exclusively — no real I/O is performed during testing.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Language-level I/O abstraction used by `print`, `println`, and `read_line`.
pub trait StdlibIoHandler {
    /// Write a string to the output sink without a trailing newline.
    ///
    /// # Errors
    ///
    /// Returns an error string when the underlying write operation fails.
    fn write_str(&mut self, value: &str) -> Result<(), String>;

    /// Read one line of input from the input source, trimming the trailing newline.
    ///
    /// # Errors
    ///
    /// Returns an error string when the underlying read operation fails or no input is available.
    fn read_line_str(&mut self) -> Result<String, String>;
}

/// Test-double I/O handler for the Opalescent stdlib.
///
/// Captures all written output in an internal `String` buffer and returns
/// pre-queued input values in FIFO order. No actual stdin/stdout is accessed.
#[derive(Debug, Default)]
pub struct MockStdlibIoHandler {
    /// Buffer accumulating all written output.
    output_buffer: String,
    /// Queued input values returned in FIFO order by `read_line_str`.
    input_queue: Vec<String>,
}

impl MockStdlibIoHandler {
    /// Create an empty mock I/O handler with no queued input.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            output_buffer: String::new(),
            input_queue: Vec::new(),
        }
    }

    /// Queue a line of input to be returned by the next `read_line_str` call.
    pub fn queue_input(&mut self, line: &str) {
        self.input_queue.push(line.to_owned());
    }

    /// Drain and return the accumulated output buffer, resetting it to empty.
    pub fn take_output(&mut self) -> String {
        let captured = self.output_buffer.clone();
        self.output_buffer = String::new();
        captured
    }
}

impl StdlibIoHandler for MockStdlibIoHandler {
    fn write_str(&mut self, value: &str) -> Result<(), String> {
        self.output_buffer.push_str(value);
        Ok(())
    }

    fn read_line_str(&mut self) -> Result<String, String> {
        if self.input_queue.is_empty() {
            Err(String::from("no input queued in MockStdlibIoHandler"))
        } else {
            Ok(self.input_queue.remove(0_usize))
        }
    }
}

/// Write `value` to the output sink without appending a newline.
///
/// # Errors
///
/// Propagates write errors from the [`StdlibIoHandler`].
pub fn print(handler: &mut impl StdlibIoHandler, value: &str) -> Result<(), String> {
    handler.write_str(value)
}

/// Write `value` to the output sink followed by a newline character.
///
/// # Errors
///
/// Propagates write errors from the [`StdlibIoHandler`].
pub fn println(handler: &mut impl StdlibIoHandler, value: &str) -> Result<(), String> {
    handler.write_str(value)?;
    handler.write_str("\n")
}

/// Read a single line from the input source.
///
/// The returned string does not include the trailing newline that the handler strips.
///
/// # Errors
///
/// Propagates read errors from the [`StdlibIoHandler`].
pub fn read_line(handler: &mut impl StdlibIoHandler) -> Result<String, String> {
    handler.read_line_str()
}
