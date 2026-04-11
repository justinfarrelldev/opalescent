extern crate alloc;

use crate::runtime::errors::{RuntimeResult, RuntimeResultExt};
use crate::runtime::memory::{OpalString, RuntimeAllocator};
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
    io_handler.write(value.as_str())
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
    let input = io_handler.read()?;
    allocator.allocate_string(&input)
}
