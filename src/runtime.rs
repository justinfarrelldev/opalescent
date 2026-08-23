#![expect(
    clippy::pub_use,
    reason = "Task 24 requires a runtime API surface exposed from src/runtime.rs"
)]

pub mod arrays;
pub mod errors;
pub mod io;
pub mod memory;
pub mod process_control;
pub mod reporting;
pub mod stdlib;
pub mod strings;
pub mod timer;
pub mod wait;

#[path = "runtime/terminal_coordinator.rs"]
pub(crate) mod terminal_coordinator;

pub use arrays::{allocate_array, array_index, array_length};
pub use errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
pub use io::{DefaultIoHandler, IoHandler, print, take_input};
pub use memory::{DefaultRuntimeAllocator, OpalArray, OpalString, RuntimeAllocator};
pub use process_control::{
    ProcessControlAcknowledgementError, ProcessControlError, ProcessControlNotification,
    ProcessControlPollResult, ProcessControlResumeError, ProcessControlSource,
    ProcessControlUnavailableError,
};
pub use reporting::format_runtime_error;
pub use stdlib::{
    DefaultRandomIntSource, RandomIntSource, format_interpolated_string, opal_array_slice,
    random_int32, random_int32_with_source, string_to_int32,
};
pub use strings::{
    string_compare, string_concat, string_equals, string_extract_range, string_find_index_or,
    string_find_last_index_of_text, string_index, string_is_blank, string_length,
    string_split_lines, string_take_prefix, string_take_suffix, string_trim_whitespace,
};
pub use timer::{
    MonotonicDeadline, MonotonicTimer, MonotonicTimerError, MonotonicTimerNotArmedError,
    monotonic_clock_now,
};
pub use wait::{
    CancellationSource, CancellationToken, SystemOwnedWaitRegistration, SystemReadinessSource,
    SystemReadyWakeStatus, SystemWaitRegistration, SystemWaitSet, SystemWaitSetError,
    SystemWaitWake,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod timer_tests;
#[cfg(test)]
pub(crate) mod wait_test_support;
#[cfg(test)]
mod wait_tests;
