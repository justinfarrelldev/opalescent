//! Terminal/process-control suspend-continue workflow helper.

extern crate alloc;

use super::{TerminalPauseError, TerminalSession, TerminalSessionStateError};
use crate::runtime::process_control::{
    ProcessControlAcknowledgementError, ProcessControlError, ProcessControlNotification,
    ProcessControlPollResult, ProcessControlResumeError, ProcessControlSource,
};
use alloc::vec::Vec;

/// Observable workflow step for deterministic tests and examples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalProcessWorkflowStep {
    /// Application ordinary work was stopped.
    StopApplicationWork,
    /// Terminal pause delivered events through `PauseBoundary`.
    PauseTerminal,
    /// Host suspend was acknowledged.
    AcknowledgeSuspend,
    /// A stale or out-of-order process notification was ignored.
    IgnoreProcessNotification,
    /// Matching continuation was observed.
    ObserveContinued,
    /// Terminal was resumed before application work.
    ResumeTerminal,
    /// Process-control source was told application work resumed.
    ResumeApplication,
    /// Redraw/work may resume.
    RedrawAndResumeWork,
}

/// Structured workflow failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalProcessWorkflowError {
    /// No process notification is ready.
    Idle,
    /// Process-control polling failed.
    ProcessPoll(ProcessControlError),
    /// First ready notification was not a suspend request.
    ExpectedSuspend,
    /// Terminal pause failed before acknowledgement.
    Pause(TerminalPauseError),
    /// Host suspend acknowledgement failed.
    Acknowledge(ProcessControlAcknowledgementError),
    /// Terminal resume failed; application remains stopped.
    ResumeTerminal(TerminalSessionStateError),
    /// Application resume failed after terminal resume.
    ResumeApplication(ProcessControlResumeError),
}

/// Execute one exact suspend/continue workflow.
///
/// # Errors
///
/// Returns a structured workflow error and stops before later side effects for
/// fail-closed paths.
pub fn terminal_process_suspend_continue_sync(
    session: &mut TerminalSession,
    process: &mut ProcessControlSource,
) -> Result<Vec<TerminalProcessWorkflowStep>, TerminalProcessWorkflowError> {
    let mut steps = Vec::new();
    let notification = match process
        .poll()
        .map_err(TerminalProcessWorkflowError::ProcessPoll)?
    {
        ProcessControlPollResult::Notification { notification } => notification,
        ProcessControlPollResult::Idle => return Err(TerminalProcessWorkflowError::Idle),
    };
    let ProcessControlNotification::SuspendRequested(generation) = notification else {
        return Err(TerminalProcessWorkflowError::ExpectedSuspend);
    };

    steps.push(TerminalProcessWorkflowStep::StopApplicationWork);
    session
        .pause_sync()
        .map_err(TerminalProcessWorkflowError::Pause)?;
    steps.push(TerminalProcessWorkflowStep::PauseTerminal);
    process
        .acknowledge_suspend(generation)
        .map_err(TerminalProcessWorkflowError::Acknowledge)?;
    steps.push(TerminalProcessWorkflowStep::AcknowledgeSuspend);

    loop {
        match process
            .poll()
            .map_err(TerminalProcessWorkflowError::ProcessPoll)?
        {
            ProcessControlPollResult::Notification {
                notification: ProcessControlNotification::Continued(candidate),
            } if candidate == generation => {
                steps.push(TerminalProcessWorkflowStep::ObserveContinued);
                break;
            }
            ProcessControlPollResult::Idle | ProcessControlPollResult::Notification { .. } => {
                steps.push(TerminalProcessWorkflowStep::IgnoreProcessNotification);
            }
        }
    }

    session
        .resume_sync()
        .map_err(TerminalProcessWorkflowError::ResumeTerminal)?;
    steps.push(TerminalProcessWorkflowStep::ResumeTerminal);
    process
        .resume_application(generation)
        .map_err(TerminalProcessWorkflowError::ResumeApplication)?;
    steps.push(TerminalProcessWorkflowStep::ResumeApplication);
    steps.push(TerminalProcessWorkflowStep::RedrawAndResumeWork);
    Ok(steps)
}
