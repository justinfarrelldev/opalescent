#![expect(
    clippy::pub_use,
    reason = "Task 24 requires a runtime API surface exposed from src/runtime.rs"
)]

#[path = "runtime/arrays.rs"]
pub mod arrays;
#[path = "runtime/errors.rs"]
pub mod errors;
#[path = "runtime/io.rs"]
pub mod io;
#[path = "runtime/memory.rs"]
pub mod memory;
#[path = "runtime/strings.rs"]
pub mod strings;

pub use arrays::{allocate_array, array_index, array_length};
pub use errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
pub use io::{print, take_input, DefaultIoHandler, IoHandler};
pub use memory::{DefaultRuntimeAllocator, OpalArray, OpalString, RuntimeAllocator};
pub use strings::{string_compare, string_concat, string_equals, string_length};

#[cfg(test)]
#[path = "runtime/tests.rs"]
mod tests;
