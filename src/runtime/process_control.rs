//! Process-control readiness source and notification queue.
//!
//! This module keeps process-control notifications separate from terminal input
//! events while exposing a stable readiness source for generic wait sets.

extern crate alloc;
extern crate std;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
#[cfg(test)]
use alloc::sync::Arc;
use core::fmt;
#[cfg(test)]
use std::sync::Mutex;

use crate::runtime::wait::{SourceAvailability, SystemReadinessSource};

/// Host support failure for process-control source construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlUnavailableError {
    /// The current host does not support this source contract.
    UnsupportedHost,
}

impl fmt::Display for ProcessControlUnavailableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnsupportedHost => f.write_str("ProcessControlUnavailableError.UnsupportedHost"),
        }
    }
}

impl std::error::Error for ProcessControlUnavailableError {}

/// Process-control polling failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlError {
    /// The host could not be observed for a new notification.
    HostNotificationObservationFailed,
    /// The next generation would wrap or reuse an earlier generation.
    GenerationExhausted {
        /// The last generation that remains valid.
        last_issued_generation: u64,
    },
}

impl fmt::Display for ProcessControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::HostNotificationObservationFailed => {
                f.write_str("ProcessControlError.HostNotificationObservationFailed")
            }
            Self::GenerationExhausted { .. } => {
                f.write_str("ProcessControlError.GenerationExhausted")
            }
        }
    }
}

impl std::error::Error for ProcessControlError {}

/// Errors returned while acknowledging suspend notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlAcknowledgementError {
    /// The requested generation does not match the current one.
    WrongGeneration,
    /// The requested generation is already past the acknowledgement phase.
    StaleGeneration,
    /// The host refused to accept the suspend acknowledgement.
    HostSuspendFailed,
}

impl fmt::Display for ProcessControlAcknowledgementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongGeneration => {
                f.write_str("ProcessControlAcknowledgementError.WrongGeneration")
            }
            Self::StaleGeneration => {
                f.write_str("ProcessControlAcknowledgementError.StaleGeneration")
            }
            Self::HostSuspendFailed => {
                f.write_str("ProcessControlAcknowledgementError.HostSuspendFailed")
            }
        }
    }
}

impl std::error::Error for ProcessControlAcknowledgementError {}

/// Errors returned while resuming application work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlResumeError {
    /// The requested generation does not match the current one.
    WrongGeneration,
    /// The requested generation is already past the resume phase.
    StaleGeneration,
    /// The host refused to accept the application-resume request.
    HostApplicationResumeFailed,
}

impl fmt::Display for ProcessControlResumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongGeneration => f.write_str("ProcessControlResumeError.WrongGeneration"),
            Self::StaleGeneration => f.write_str("ProcessControlResumeError.StaleGeneration"),
            Self::HostApplicationResumeFailed => {
                f.write_str("ProcessControlResumeError.HostApplicationResumeFailed")
            }
        }
    }
}

impl std::error::Error for ProcessControlResumeError {}

/// A process-control notification observed from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessControlNotification {
    /// The host requested suspension for this generation.
    SuspendRequested(u64),
    /// The host continued the matching generation.
    Continued(u64),
}

/// Result of polling a process-control source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessControlPollResult {
    /// A notification is available.
    Notification {
        /// The delivered process-control notification.
        notification: ProcessControlNotification,
    },
    /// No host indication is currently observable.
    Idle,
}

/// Observable host indications used by the source backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the placeholder POSIX backend does not synthesize host notifications outside tests yet"
    )
)]
enum ProcessControlHostObservation {
    /// The host requested suspension.
    SuspendRequested,
    /// The host reported continuation.
    Continued,
}

/// Host observation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessControlHostObservationError;

/// Host action failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the placeholder POSIX backend does not fail suspend or resume actions outside tests yet"
    )
)]
enum ProcessControlHostActionError {
    /// Host suspension failed.
    SuspendFailed,
    /// Host application resume failed.
    ResumeFailed,
}

/// Backend interface used by the source state machine.
trait ProcessControlHost: Send {
    /// Observe one nonblocking host indication, if any.
    fn observe_notification(
        &mut self,
    ) -> Result<Option<ProcessControlHostObservation>, ProcessControlHostObservationError>;

    /// Ask the host to enter suspended state for `generation`.
    fn acknowledge_suspend(&mut self, generation: u64)
    -> Result<(), ProcessControlHostActionError>;

    /// Ask the host to resume application work for `generation`.
    fn resume_application(&mut self, generation: u64) -> Result<(), ProcessControlHostActionError>;
}

/// Placeholder POSIX backend used until live host observation is wired.
#[derive(Debug, Default)]
struct NoopProcessControlHost;

impl ProcessControlHost for NoopProcessControlHost {
    fn observe_notification(
        &mut self,
    ) -> Result<Option<ProcessControlHostObservation>, ProcessControlHostObservationError> {
        Ok(None)
    }

    fn acknowledge_suspend(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        Ok(())
    }

    fn resume_application(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
/// Controllable host backend used by unit tests.
pub(crate) struct TestProcessControlHost {
    /// Shared mutable test state.
    inner: Arc<Mutex<TestProcessControlHostState>>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Synthetic host observations published by tests.
pub(crate) enum TestProcessControlHostObservation {
    /// Emit a suspend request.
    SuspendRequested,
    /// Emit a continuation.
    Continued,
}

#[cfg(test)]
#[derive(Debug, Default)]
/// Shared state for the synthetic test backend.
struct TestProcessControlHostState {
    /// Whether the backend claims host support.
    supported: bool,
    /// Observations or observation failures still to publish.
    observations:
        VecDeque<Result<TestProcessControlHostObservation, ProcessControlHostObservationError>>,
    /// Remaining suspend acknowledgements that should fail.
    suspend_failures_remaining: u64,
    /// Remaining resume requests that should fail.
    resume_failures_remaining: u64,
}

#[cfg(test)]
impl TestProcessControlHost {
    /// Create a supported test host.
    pub(crate) fn supported() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TestProcessControlHostState {
                supported: true,
                observations: VecDeque::new(),
                suspend_failures_remaining: 0,
                resume_failures_remaining: 0,
            })),
        }
    }

    /// Create an unsupported test host.
    pub(crate) fn unsupported() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TestProcessControlHostState {
                supported: false,
                observations: VecDeque::new(),
                suspend_failures_remaining: 0,
                resume_failures_remaining: 0,
            })),
        }
    }

    /// Return whether the test host advertises support.
    pub(crate) fn is_supported(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .supported
    }

    /// Queue one successful observation.
    pub(crate) fn push_observation(&self, observation: TestProcessControlHostObservation) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.observations.push_back(Ok(observation));
    }

    /// Queue one observation failure.
    pub(crate) fn push_observation_failure(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .observations
            .push_back(Err(ProcessControlHostObservationError));
    }

    /// Make the next acknowledgement fail once.
    pub(crate) fn fail_next_suspend_acknowledgement(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.suspend_failures_remaining = state.suspend_failures_remaining.saturating_add(1);
    }

    /// Make the next resume call fail once.
    pub(crate) fn fail_next_resume(&self) {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.resume_failures_remaining = state.resume_failures_remaining.saturating_add(1);
    }
}

#[cfg(test)]
impl ProcessControlHost for TestProcessControlHost {
    fn observe_notification(
        &mut self,
    ) -> Result<Option<ProcessControlHostObservation>, ProcessControlHostObservationError> {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.observations.pop_front() {
            Some(Ok(TestProcessControlHostObservation::SuspendRequested)) => {
                Ok(Some(ProcessControlHostObservation::SuspendRequested))
            }
            Some(Ok(TestProcessControlHostObservation::Continued)) => {
                Ok(Some(ProcessControlHostObservation::Continued))
            }
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    fn acknowledge_suspend(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.suspend_failures_remaining > 0 {
            let Some(remaining_failures) = state.suspend_failures_remaining.checked_sub(1) else {
                return Ok(());
            };
            state.suspend_failures_remaining = remaining_failures;
            drop(state);
            return Err(ProcessControlHostActionError::SuspendFailed);
        }
        Ok(())
    }

    fn resume_application(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.resume_failures_remaining > 0 {
            let Some(remaining_failures) = state.resume_failures_remaining.checked_sub(1) else {
                return Ok(());
            };
            state.resume_failures_remaining = remaining_failures;
            drop(state);
            return Err(ProcessControlHostActionError::ResumeFailed);
        }
        Ok(())
    }
}

/// The current state of the active process-control generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessControlGenerationState {
    /// A suspend request was published but not yet acknowledged.
    SuspendPending,
    /// Host suspension was acknowledged successfully.
    AcknowledgedHostSuspended,
    /// Continuation arrived and application resume is still pending.
    ContinuedAwaitingApplicationResume,
    /// The generation completed its full suspend/resume cycle.
    Completed,
}

/// Mutable source state.
struct ProcessControlState {
    /// The highest generation ever issued by this source.
    last_issued_generation: u64,
    /// The currently active generation, if one exists.
    current_generation: Option<u64>,
    /// Lifecycle state for the current generation.
    generation_state: Option<ProcessControlGenerationState>,
    /// FIFO queue of published notifications.
    queued_notifications: VecDeque<ProcessControlNotification>,
    /// Retained host indication that has not yet been translated.
    pending_host_observation: Option<ProcessControlHostObservation>,
}

impl ProcessControlState {
    /// Create an empty process-control state.
    const fn new() -> Self {
        Self {
            last_issued_generation: 0,
            current_generation: None,
            generation_state: None,
            queued_notifications: VecDeque::new(),
            pending_host_observation: None,
        }
    }
}

/// A separate process-control source.
pub struct ProcessControlSource {
    /// Stable readiness identity shared with wait sets.
    readiness_source: SystemReadinessSource,
    /// Mutable queue and generation state.
    state: ProcessControlState,
    /// Host-specific observation backend.
    backend: Box<dyn ProcessControlHost>,
}

impl ProcessControlSource {
    /// Create a new process-control source for the current host.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessControlUnavailableError::UnsupportedHost`] before any
    /// source identity, generation state, or queue state is allocated when the
    /// host does not support process control.
    pub fn new() -> Result<Self, ProcessControlUnavailableError> {
        #[cfg(windows)]
        {
            return Err(ProcessControlUnavailableError::UnsupportedHost);
        }

        #[cfg(not(windows))]
        {
            Ok(Self::from_backend(Box::new(NoopProcessControlHost)))
        }
    }

    /// Return a clone of the stable readiness source identity.
    #[must_use]
    pub fn readiness_source(&self) -> SystemReadinessSource {
        self.readiness_source.clone()
    }

    /// Poll for the oldest queued notification or the next observable host event.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessControlError::HostNotificationObservationFailed`] when the
    /// host cannot be observed.
    ///
    /// Returns [`ProcessControlError::GenerationExhausted`] without mutating the
    /// source when a suspend request needs a wrapped or reused generation.
    pub fn poll(&mut self) -> Result<ProcessControlPollResult, ProcessControlError> {
        if let Some(notification) = self.state.queued_notifications.pop_front() {
            self.publish_queue_readiness();
            return Ok(ProcessControlPollResult::Notification { notification });
        }

        let observation = match self.state.pending_host_observation {
            Some(observation) => observation,
            None => match self.backend.observe_notification() {
                Ok(Some(observation)) => {
                    self.state.pending_host_observation = Some(observation);
                    observation
                }
                Ok(None) => return Ok(ProcessControlPollResult::Idle),
                Err(_) => return Err(ProcessControlError::HostNotificationObservationFailed),
            },
        };

        let translated_notification = match observation {
            ProcessControlHostObservation::SuspendRequested => self.handle_suspend_observation()?,
            ProcessControlHostObservation::Continued => self.handle_continued_observation(),
        };

        self.state.pending_host_observation = None;
        let Some(translated_notification) = translated_notification else {
            return Ok(ProcessControlPollResult::Idle);
        };
        self.enqueue_notification(translated_notification);
        let Some(queued_notification) = self.state.queued_notifications.pop_front() else {
            return Ok(ProcessControlPollResult::Idle);
        };
        self.publish_queue_readiness();
        Ok(ProcessControlPollResult::Notification {
            notification: queued_notification,
        })
    }

    /// Acknowledge a pending suspend generation.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessControlAcknowledgementError`] for generation mismatch,
    /// stale state, or host suspension failure.
    pub fn acknowledge_suspend(
        &mut self,
        generation: u64,
    ) -> Result<(), ProcessControlAcknowledgementError> {
        let Some(current_generation) = self.state.current_generation else {
            return Err(ProcessControlAcknowledgementError::WrongGeneration);
        };
        if current_generation != generation {
            return Err(ProcessControlAcknowledgementError::WrongGeneration);
        }

        match self.state.generation_state {
            Some(ProcessControlGenerationState::SuspendPending) => {
                if self.backend.acknowledge_suspend(generation).is_err() {
                    return Err(ProcessControlAcknowledgementError::HostSuspendFailed);
                }
                self.state.generation_state =
                    Some(ProcessControlGenerationState::AcknowledgedHostSuspended);
                Ok(())
            }
            Some(ProcessControlGenerationState::AcknowledgedHostSuspended) => Ok(()),
            Some(
                ProcessControlGenerationState::ContinuedAwaitingApplicationResume
                | ProcessControlGenerationState::Completed,
            )
            | None => Err(ProcessControlAcknowledgementError::StaleGeneration),
        }
    }

    /// Resume application work for a continued generation.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessControlResumeError`] for generation mismatch, stale
    /// state, or host resume failure.
    pub fn resume_application(&mut self, generation: u64) -> Result<(), ProcessControlResumeError> {
        let Some(current_generation) = self.state.current_generation else {
            return Err(ProcessControlResumeError::WrongGeneration);
        };
        if current_generation != generation {
            return Err(ProcessControlResumeError::WrongGeneration);
        }

        match self.state.generation_state {
            Some(ProcessControlGenerationState::ContinuedAwaitingApplicationResume) => {
                if self.backend.resume_application(generation).is_err() {
                    return Err(ProcessControlResumeError::HostApplicationResumeFailed);
                }
                self.state.generation_state = Some(ProcessControlGenerationState::Completed);
                Ok(())
            }
            Some(ProcessControlGenerationState::Completed) => Ok(()),
            Some(
                ProcessControlGenerationState::SuspendPending
                | ProcessControlGenerationState::AcknowledgedHostSuspended,
            )
            | None => Err(ProcessControlResumeError::StaleGeneration),
        }
    }

    #[cfg(test)]
    /// Build a source with a synthetic test backend.
    pub(crate) fn new_with_test_host(
        host: TestProcessControlHost,
        allocation_probe: &std::sync::atomic::AtomicUsize,
    ) -> Result<Self, ProcessControlUnavailableError> {
        Self::from_test_host(host, allocation_probe)
    }

    #[cfg(test)]
    /// Inject a queued notification for deterministic unit tests.
    pub(crate) fn enqueue_notification_for_tests(
        &mut self,
        notification: ProcessControlNotification,
    ) {
        self.enqueue_notification(notification);
    }

    /// Build a source from a concrete backend.
    fn from_backend(backend: Box<dyn ProcessControlHost>) -> Self {
        let readiness_source = SystemReadinessSource::new();
        Self {
            readiness_source,
            state: ProcessControlState::new(),
            backend,
        }
    }

    #[cfg(test)]
    /// Build a source from a test backend while tracking successful allocation.
    fn from_test_host(
        host: TestProcessControlHost,
        allocation_probe: &std::sync::atomic::AtomicUsize,
    ) -> Result<Self, ProcessControlUnavailableError> {
        if !host.is_supported() {
            return Err(ProcessControlUnavailableError::UnsupportedHost);
        }
        let source = Self::from_backend(Box::new(host));
        allocation_probe.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(source)
    }

    /// Translate a suspend observation into queue state.
    fn handle_suspend_observation(
        &mut self,
    ) -> Result<Option<ProcessControlNotification>, ProcessControlError> {
        match self.state.generation_state {
            None | Some(ProcessControlGenerationState::Completed) => {
                let generation = self.next_generation()?;
                self.state.current_generation = Some(generation);
                self.state.generation_state = Some(ProcessControlGenerationState::SuspendPending);
                Ok(Some(ProcessControlNotification::SuspendRequested(
                    generation,
                )))
            }
            Some(ProcessControlGenerationState::SuspendPending) => {
                let Some(generation) = self.state.current_generation else {
                    return Ok(None);
                };
                Ok(Some(ProcessControlNotification::SuspendRequested(
                    generation,
                )))
            }
            Some(
                ProcessControlGenerationState::AcknowledgedHostSuspended
                | ProcessControlGenerationState::ContinuedAwaitingApplicationResume,
            ) => Ok(None),
        }
    }

    /// Translate a continuation observation into queue state.
    fn handle_continued_observation(&mut self) -> Option<ProcessControlNotification> {
        match self.state.generation_state {
            Some(ProcessControlGenerationState::AcknowledgedHostSuspended) => {
                let generation = self.state.current_generation?;
                self.state.generation_state =
                    Some(ProcessControlGenerationState::ContinuedAwaitingApplicationResume);
                Some(ProcessControlNotification::Continued(generation))
            }
            Some(
                ProcessControlGenerationState::SuspendPending
                | ProcessControlGenerationState::ContinuedAwaitingApplicationResume
                | ProcessControlGenerationState::Completed,
            )
            | None => None,
        }
    }

    /// Allocate the next nonzero process-control generation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "checked_add is not const on the current stable toolchain"
    )]
    fn next_generation(&mut self) -> Result<u64, ProcessControlError> {
        let last_issued_generation = self.state.last_issued_generation;
        let Some(next_generation) = last_issued_generation.checked_add(1) else {
            return Err(ProcessControlError::GenerationExhausted {
                last_issued_generation,
            });
        };
        self.state.last_issued_generation = next_generation;
        Ok(next_generation)
    }

    /// Publish one notification into the FIFO queue.
    fn enqueue_notification(&mut self, notification: ProcessControlNotification) {
        self.state.queued_notifications.push_back(notification);
        self.publish_queue_readiness();
    }

    /// Reflect the queue's current readiness level.
    fn publish_queue_readiness(&self) {
        let availability = if self.state.queued_notifications.is_empty() {
            SourceAvailability::Idle
        } else {
            SourceAvailability::Ready
        };
        self.readiness_source.publish_transition(availability);
    }
}

impl fmt::Debug for ProcessControlSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessControlSource")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::panic,
        clippy::too_many_lines,
        reason = "the focused unit tests use explicit panic assertions and cover the full state machine"
    )]
    use super::{
        ProcessControlAcknowledgementError, ProcessControlError, ProcessControlNotification,
        ProcessControlPollResult, ProcessControlResumeError, ProcessControlSource,
        ProcessControlUnavailableError, TestProcessControlHost, TestProcessControlHostObservation,
    };
    use std::sync::atomic::AtomicUsize;

    fn poll_notification(source: &mut ProcessControlSource) -> ProcessControlNotification {
        match source.poll().expect("poll should succeed") {
            ProcessControlPollResult::Notification { notification } => notification,
            ProcessControlPollResult::Idle => unreachable!("expected notification, found idle"),
        }
    }

    #[test]
    fn process_control_source_rejects_unsupported_host_before_allocation() {
        let host = TestProcessControlHost::unsupported();
        let allocation_probe = AtomicUsize::new(0);

        let result = ProcessControlSource::new_with_test_host(host, &allocation_probe);
        assert!(matches!(
            result,
            Err(ProcessControlUnavailableError::UnsupportedHost)
        ));
        assert_eq!(
            allocation_probe.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }

    #[test]
    fn process_control_source_exposes_stable_readiness_identity() {
        let host = TestProcessControlHost::supported();
        let allocation_probe = AtomicUsize::new(0);
        let source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");
        let readiness = source.readiness_source();
        assert!(readiness.is_same_identity(&source.readiness_source()));
        assert_eq!(
            allocation_probe.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn process_control_poll_returns_queued_notifications_before_host_observation() {
        let host = TestProcessControlHost::supported();
        host.push_observation(TestProcessControlHostObservation::SuspendRequested);
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");
        source.enqueue_notification_for_tests(ProcessControlNotification::Continued(7));

        let first = match source.poll().expect("poll should succeed") {
            ProcessControlPollResult::Notification { notification } => notification,
            ProcessControlPollResult::Idle => {
                unreachable!("queued notification should return first")
            }
        };
        assert_eq!(first, ProcessControlNotification::Continued(7));
        let second = match source.poll().expect("poll should succeed") {
            ProcessControlPollResult::Notification { notification } => notification,
            ProcessControlPollResult::Idle => unreachable!("host observation should return second"),
        };
        assert_eq!(second, ProcessControlNotification::SuspendRequested(1));
    }

    #[test]
    fn process_control_poll_returns_idle_without_mutation_when_no_host_indication_is_available() {
        let host = TestProcessControlHost::supported();
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");
        let readiness = source.readiness_source();
        let original_generation = readiness.generation();

        assert_eq!(
            source.poll().expect("poll should succeed"),
            ProcessControlPollResult::Idle
        );
        assert_eq!(
            source.poll().expect("poll should succeed"),
            ProcessControlPollResult::Idle
        );
        assert_eq!(
            readiness.generation(),
            original_generation,
            "idle polls must not publish readiness transitions"
        );
    }

    #[test]
    fn process_control_poll_reports_host_observation_failure_without_mutation() {
        let host = TestProcessControlHost::supported();
        host.push_observation_failure();
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");
        let readiness = source.readiness_source();
        let original_generation = readiness.generation();

        assert_eq!(
            source.poll(),
            Err(ProcessControlError::HostNotificationObservationFailed)
        );
        assert_eq!(
            readiness.generation(),
            original_generation,
            "observation failure must not publish readiness transitions"
        );
        assert_eq!(
            source.poll().expect("poll should succeed"),
            ProcessControlPollResult::Idle
        );
    }

    #[test]
    fn process_control_generation_exhaustion_preserves_pending_host_indication() {
        let host = TestProcessControlHost::supported();
        host.push_observation(TestProcessControlHostObservation::SuspendRequested);
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");
        source.state.last_issued_generation = u64::MAX;
        let first = source.poll();
        assert_eq!(
            first,
            Err(ProcessControlError::GenerationExhausted {
                last_issued_generation: u64::MAX
            })
        );
        let second = source.poll();
        assert_eq!(
            second,
            Err(ProcessControlError::GenerationExhausted {
                last_issued_generation: u64::MAX
            })
        );
    }

    #[test]
    fn process_control_suspend_acknowledge_and_resume_state_machine() {
        let host = TestProcessControlHost::supported();
        host.push_observation(TestProcessControlHostObservation::SuspendRequested);
        host.push_observation(TestProcessControlHostObservation::Continued);
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");

        let suspend_notification = poll_notification(&mut source);
        assert_eq!(
            suspend_notification,
            ProcessControlNotification::SuspendRequested(1)
        );
        assert_eq!(source.acknowledge_suspend(1), Ok(()));
        assert_eq!(source.acknowledge_suspend(1), Ok(()));
        assert_eq!(
            source.acknowledge_suspend(2),
            Err(ProcessControlAcknowledgementError::WrongGeneration)
        );

        let continued_notification = poll_notification(&mut source);
        assert_eq!(
            continued_notification,
            ProcessControlNotification::Continued(1)
        );
        assert_eq!(source.resume_application(1), Ok(()));
        assert_eq!(source.resume_application(1), Ok(()));
        assert_eq!(
            source.resume_application(2),
            Err(ProcessControlResumeError::WrongGeneration)
        );
    }

    #[test]
    fn process_control_suspend_and_resume_failures_are_retryable() {
        let host = TestProcessControlHost::supported();
        host.push_observation(TestProcessControlHostObservation::SuspendRequested);
        host.fail_next_suspend_acknowledgement();
        host.fail_next_resume();
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host.clone(), &allocation_probe)
            .expect("supported host should build");

        let suspend_notification = poll_notification(&mut source);
        assert_eq!(
            suspend_notification,
            ProcessControlNotification::SuspendRequested(1),
            "the first notification must be the initial suspend request"
        );
        assert_eq!(
            source.acknowledge_suspend(1),
            Err(ProcessControlAcknowledgementError::HostSuspendFailed)
        );
        assert_eq!(source.acknowledge_suspend(1), Ok(()));

        host.push_observation(TestProcessControlHostObservation::Continued);
        let continued_notification = poll_notification(&mut source);
        assert_eq!(
            continued_notification,
            ProcessControlNotification::Continued(1),
            "the retry path must still deliver the continued notification"
        );
        assert_eq!(
            source.resume_application(1),
            Err(ProcessControlResumeError::HostApplicationResumeFailed)
        );
        assert_eq!(source.resume_application(1), Ok(()));
    }

    #[test]
    fn process_control_stale_generation_errors_are_distinct_from_wrong_generation_errors() {
        let host = TestProcessControlHost::supported();
        host.push_observation(TestProcessControlHostObservation::SuspendRequested);
        let allocation_probe = AtomicUsize::new(0);
        let mut source = ProcessControlSource::new_with_test_host(host, &allocation_probe)
            .expect("supported host should build");

        let suspend_notification = poll_notification(&mut source);
        assert_eq!(
            suspend_notification,
            ProcessControlNotification::SuspendRequested(1),
            "the first notification must be the initial suspend request"
        );
        assert_eq!(
            source.acknowledge_suspend(2),
            Err(ProcessControlAcknowledgementError::WrongGeneration)
        );
        assert_eq!(
            source.resume_application(2),
            Err(ProcessControlResumeError::WrongGeneration)
        );
        assert_eq!(
            source.resume_application(1),
            Err(ProcessControlResumeError::StaleGeneration)
        );
        assert_eq!(source.acknowledge_suspend(1), Ok(()));
        assert_eq!(
            source.resume_application(1),
            Err(ProcessControlResumeError::StaleGeneration)
        );
    }
}
