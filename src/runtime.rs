#![expect(
    clippy::pub_use,
    reason = "Task 24 requires a runtime API surface exposed from src/runtime.rs"
)]

pub mod arrays;
pub mod errors;
pub mod io;
pub mod memory;
pub mod reporting;
pub mod stdlib;
pub mod strings;

pub use arrays::{allocate_array, array_index, array_length};
pub use errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
pub use io::{DefaultIoHandler, IoHandler, print, take_input};
pub use memory::{DefaultRuntimeAllocator, OpalArray, OpalString, RuntimeAllocator};
pub use reporting::format_runtime_error;
pub use stdlib::{
    DefaultRandomIntSource, RandomIntSource, format_interpolated_string, opal_array_slice,
    random_int32, random_int32_with_source, string_to_int32,
};
pub use strings::{string_compare, string_concat, string_equals, string_length};

#[cfg(test)]
mod tests;
