extern crate alloc;

use crate::runtime::errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
use crate::runtime::memory::{OpalString, RuntimeAllocator};
use crate::runtime::terminal_coordinator::{
    TerminalOperation, diagnostic_lane_allows, sanitize_diagnostic, validate_global_operation,
};
use alloc::string::String;

/// Runtime I/O abstraction used for host I/O and test mocking.
pub trait IoHandler {
    /// Write a runtime string slice to output sink.
    ///
    /// # Errors
    ///
    /// Returns runtime errors when output operations fail.
    fn write(&mut self, value: &str) -> RuntimeResult<()>;

    /// Read one line of input from input source.
    ///
    /// # Errors
    ///
    /// Returns runtime errors when input operations fail.
    fn read(&mut self) -> RuntimeResult<String>;
}

/// Default host-backed I/O implementation using stdin/stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultIoHandler;

impl IoHandler for DefaultIoHandler {
    fn write(&mut self, value: &str) -> RuntimeResult<()> {
        use std::io::Write;

        let mut stdout = std::io::stdout();
        writeln!(&mut stdout, "{value}").into_runtime_error(3_001, "failed to write to stdout")
    }

    fn read(&mut self) -> RuntimeResult<String> {
        let mut buffer = String::new();
        std::io::stdin()
            .read_line(&mut buffer)
            .into_runtime_error(3_002, "failed to read from stdin")?;

        while buffer.ends_with('\n') || buffer.ends_with('\r') {
            buffer.pop();
        }

        Ok(buffer)
    }
}

/// Runtime `print` built-in implementation.
///
/// # Errors
///
/// Returns output errors from the configured [`IoHandler`].
pub fn print(io_handler: &mut impl IoHandler, value: &OpalString) -> RuntimeResult<()> {
    if !diagnostic_lane_allows() {
        return Ok(());
    }
    io_handler.write(&sanitize_diagnostic(value.as_str()))
}

/// Runtime `take_input` built-in implementation.
///
/// # Errors
///
/// Returns input errors from [`IoHandler`] or allocation errors from [`RuntimeAllocator`].
pub fn take_input<Allocator>(
    io_handler: &mut impl IoHandler,
    allocator: &Allocator,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    validate_global_operation(TerminalOperation::TakeInput).map_err(|rejection| {
        RuntimeError::user_error(
            3_003,
            format!("TerminalCoordinatorUnavailable: {rejection:?}"),
        )
    })?;
    let input = io_handler.read()?;
    allocator.allocate_string(&input)
}

#[cfg(test)]
mod tests {
    #![allow(
        dead_code,
        reason = "test I/O handler only implements the exercised methods"
    )]

    use super::*;
    use crate::runtime::memory::DefaultRuntimeAllocator;
    use crate::runtime::terminal_coordinator::{TerminalCoordinatorState, set_state_for_tests};
    use alloc::vec::Vec;

    /// Input handler that records whether a read was attempted.
    #[derive(Debug)]
    struct CountingIoHandler {
        /// Value returned by the next read.
        value: String,
        /// Number of reads attempted.
        reads: usize,
        /// Values written to the output sink.
        writes: Vec<String>,
    }

    impl IoHandler for CountingIoHandler {
        fn write(&mut self, value: &str) -> RuntimeResult<()> {
            self.writes.push(value.to_owned());
            Ok(())
        }

        fn read(&mut self) -> RuntimeResult<String> {
            let Some(reads) = self.reads.checked_add(1) else {
                return Err(RuntimeError::user_error(9_999, "test read count overflow"));
            };
            self.reads = reads;
            Ok(self.value.clone())
        }
    }

    #[test]
    fn diagnostic_print_drops_without_writing_in_opening_state() {
        let _state_guard = set_state_for_tests(TerminalCoordinatorState::Opening);
        let mut io_handler = CountingIoHandler {
            value: String::new(),
            reads: 0,
            writes: Vec::new(),
        };

        print(&mut io_handler, &OpalString::new(String::from("hidden")))
            .expect("dropped diagnostics should succeed");

        assert!(io_handler.writes.is_empty());
    }

    #[test]
    fn diagnostic_print_escapes_controls_and_bounds_output() {
        let _state_guard = set_state_for_tests(TerminalCoordinatorState::Active);
        let mut io_handler = CountingIoHandler {
            value: String::new(),
            reads: 0,
            writes: Vec::new(),
        };
        let value = OpalString::new(format!("start\u{1B}\u{202E}{}", "x".repeat(300)));

        print(&mut io_handler, &value).expect("allowed diagnostics should write");

        assert_eq!(io_handler.writes.len(), 1);
        assert!(io_handler.writes[0].contains("\\x1B"));
        assert!(io_handler.writes[0].contains("\\u{202E}"));
        assert!(io_handler.writes[0].ends_with("...[truncated]"));
    }

    #[test]
    fn take_input_rejects_before_read_when_coordinator_is_not_free() {
        let _state_guard = set_state_for_tests(TerminalCoordinatorState::Active);
        let mut io_handler = CountingIoHandler {
            value: String::from("unconsumed"),
            reads: 0,
            writes: Vec::new(),
        };

        let result = take_input(&mut io_handler, &DefaultRuntimeAllocator);

        assert!(result.is_err(), "non-free take_input must be rejected");
        assert_eq!(io_handler.reads, 0, "rejection must precede read");
        let error = result.expect_err("rejection should return a runtime error");
        assert!(error.message().contains("TerminalCoordinatorUnavailable"));
        assert!(error.message().contains("Active"));
        assert!(error.message().contains("TakeInput"));
    }

    #[test]
    fn take_input_reads_and_allocates_when_coordinator_is_free() {
        let _state_guard = set_state_for_tests(TerminalCoordinatorState::Free);
        let mut io_handler = CountingIoHandler {
            value: String::from("accepted"),
            reads: 0,
            writes: Vec::new(),
        };

        let result = take_input(&mut io_handler, &DefaultRuntimeAllocator)
            .expect("free take_input should preserve normal behavior");

        assert_eq!(io_handler.reads, 1);
        assert_eq!(result.as_str(), "accepted");
    }
}
