//! Generic process-control facade for the future `standard.system` surface.
//!
//! This wrapper stays separate from terminal input and only forwards to the
//! runtime process-control source state machine.

use crate::runtime::process_control::{
    ProcessControlAcknowledgementError, ProcessControlError, ProcessControlPollResult,
    ProcessControlResumeError, ProcessControlSource, ProcessControlUnavailableError,
};
use crate::runtime::wait::SystemReadinessSource;

/// Create a new process-control source.
///
/// # Errors
///
/// Returns [`ProcessControlUnavailableError::UnsupportedHost`] on hosts that do
/// not support this source contract.
pub fn process_control_source_new() -> Result<ProcessControlSource, ProcessControlUnavailableError>
{
    ProcessControlSource::new()
}

/// Return the source's stable readiness identity.
#[must_use]
pub fn process_control_readiness_source(source: &ProcessControlSource) -> SystemReadinessSource {
    source.readiness_source()
}

/// Poll the source for the next queued notification.
///
/// # Errors
///
/// Returns [`ProcessControlError`] when the host cannot be observed or a new
/// suspend generation would overflow.
pub fn process_control_poll(
    source: &mut ProcessControlSource,
) -> Result<ProcessControlPollResult, ProcessControlError> {
    source.poll()
}

/// Acknowledge a suspended generation.
///
/// # Errors
///
/// Returns [`ProcessControlAcknowledgementError`] for wrong/stale generations or
/// host suspension failure.
pub fn process_control_acknowledge_suspend(
    source: &mut ProcessControlSource,
    generation: u64,
) -> Result<(), ProcessControlAcknowledgementError> {
    source.acknowledge_suspend(generation)
}

/// Resume application work for a continued generation.
///
/// # Errors
///
/// Returns [`ProcessControlResumeError`] for wrong/stale generations or host
/// application-resume failure.
pub fn process_control_resume_application(
    source: &mut ProcessControlSource,
    generation: u64,
) -> Result<(), ProcessControlResumeError> {
    source.resume_application(generation)
}
