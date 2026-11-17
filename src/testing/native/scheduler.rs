//! Bounded fresh-process case scheduling with scheduler-only exclusion keys.
extern crate alloc;
mod processes;
use super::artifact::NativeTestArtifact;
use super::identity::CaseIdentity;
use alloc::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// One selected independent case and its complete exclusion key set.
#[derive(Debug, Clone)]
pub struct ScheduledCase {
    /// Canonical complete identity, including an optional named example.
    pub identity: CaseIdentity,
    /// Invocation-local scheduling keys, not external service locks.
    pub exclusion_keys: Vec<String>,
}

/// Coordinator controls; setup/hook/body/cleanup ordering stays in the worker.
#[derive(Debug, Clone)]
pub struct NativeRunOptions {
    /// Maximum number of concurrently active native worker processes.
    pub jobs: usize,
    /// Additional fresh attempts after an unsuccessful attempt; default zero.
    pub retries: u32,
    /// Hard monotonic deadline for each worker process.
    pub timeout: Duration,
    /// Initial source working directory; this is not a filesystem sandbox.
    pub working_directory: PathBuf,
    /// Normal production artifact explicitly built by this coordinator, never a worker.
    pub application_executable: Option<PathBuf>,
    /// Private coordinator build/result product directory.
    pub output_directory: PathBuf,
}

impl Default for NativeRunOptions {
    fn default() -> Self {
        Self {
            jobs: std::thread::available_parallelism().map_or(1, core::num::NonZeroUsize::get),
            retries: 0,
            timeout: Duration::from_secs(30),
            working_directory: PathBuf::from("."),
            application_executable: None,
            output_directory: PathBuf::from("target/native-testing/results"),
        }
    }
}

/// Honest case/attempt distinctions, independent of function associations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseOutcome {
    /// First-attempt successful lifecycle, including declared cleanup.
    Passed,
    /// A later attempt passed; earlier failed attempts remain present.
    FlakyPassed,
    /// Assertion, unexpected call or verification failure in the body.
    AssertionFailed,
    /// Unhandled ordinary application error in the body.
    ApplicationError,
    /// Typed example construction failed before setup.
    ExampleFailed,
    /// World setup failed, not a defect attributed to a declared subject.
    WorldSetupFailed,
    /// A before-hook failed and applicable exit boundaries still ran.
    BeforeHookFailed,
    /// An after-hook failed.
    AfterHookFailed,
    /// Explicit cleanup failed, regardless of body success.
    CleanupFailed,
    /// Worker was forcibly terminated; no cleanup completion is inferred.
    TimedOut,
    /// Worker terminated without a valid completed result.
    WorkerCrashed,
    /// Compiler/protocol/transport/process infrastructure failed.
    InfrastructureFailed,
}

impl CaseOutcome {
    /// Stable report vocabulary, independent of Rust debug formatting.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::FlakyPassed => "flaky_passed",
            Self::AssertionFailed => "assertion_failed",
            Self::ApplicationError => "application_error",
            Self::ExampleFailed => "example_failed",
            Self::WorldSetupFailed => "world_setup_failed",
            Self::BeforeHookFailed => "before_hook_failed",
            Self::AfterHookFailed => "after_hook_failed",
            Self::CleanupFailed => "cleanup_failed",
            Self::TimedOut => "timed_out",
            Self::WorkerCrashed => "worker_crashed",
            Self::InfrastructureFailed => "infrastructure_failed",
        }
    }

    /// Whether default CLI exit policy considers the case successful.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Passed | Self::FlakyPassed)
    }
}

/// Immutable diagnostic retained by one worker attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerFailure {
    /// Internal failure identity, not an application error.
    pub id: u64,
    /// First failure remains primary.
    pub primary_failure_id: u64,
    /// Originating failure when this diagnostic is secondary.
    pub parent_failure_id: u64,
    /// Runtime failure kind from the versioned native ABI.
    pub kind: u32,
    /// Actual lifecycle phase, not subject association.
    pub phase: u32,
    /// Owned captured diagnostic text.
    pub message: String,
    /// Defining source file.
    pub file: String,
    /// Original byte offset.
    pub offset: u64,
    /// Original extent length.
    pub length: u64,
}

/// Scalar worker completion status, not an aggregate application return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    /// Complete attempt succeeded.
    Passed,
    /// At least one callback failed.
    Failed,
}

/// Terminal result emitted only after normal lifecycle processing ends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// Complete serialized structured identity.
    pub case_id: String,
    /// Independent attempt number.
    pub attempt: u64,
    /// Whole-attempt status, including cleanup.
    pub status: WorkerStatus,
    /// Original primary diagnostic identity, or zero.
    pub primary_failure_id: u64,
    /// Whether body execution actually began.
    pub body_started: bool,
    /// Whether body execution completed successfully.
    pub body_passed: bool,
    /// Successfully initialized World count.
    pub initialized_worlds: usize,
    /// Entered scope count.
    pub entered_scopes: usize,
    /// All retained primary and secondary diagnostics.
    pub failures: Vec<WorkerFailure>,
}

/// A fresh worker process and its retained outcome/transport artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptExecution {
    /// Separate attempt identity beneath the complete case identity.
    pub attempt: u64,
    /// Distinct worker process, absent only if launch failed.
    pub process_id: Option<u32>,
    /// Whole-attempt distinction.
    pub outcome: CaseOutcome,
    /// Actual process lifetime as observed by the coordinator.
    pub duration_ms: u64,
    /// Queueing time, not function execution duration.
    pub scheduler_wait_ms: u64,
    /// Time blocked specifically by invocation-local exclusion keys.
    pub exclusion_wait_ms: u64,
    /// Last synchronously published phase, even without a terminal result.
    pub last_phase: Option<u32>,
    /// Present only for validated complete worker transport.
    pub result: Option<WorkerResult>,
    /// Protocol/launch diagnostics, never relabeled as an assertion.
    pub infrastructure_message: Option<String>,
    /// Captured application stdout, separate from result transport.
    pub stdout_path: PathBuf,
    /// Captured application stderr.
    pub stderr_path: PathBuf,
}

/// Overall totals deduplicate cases, even when several subjects are associated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseExecution {
    /// Complete stable identity.
    pub identity: CaseIdentity,
    /// Final outcome retains flaky distinction.
    pub outcome: CaseOutcome,
    /// Every fresh attempt, including failed attempts before a pass.
    pub attempts: Vec<AttemptExecution>,
}

/// Independent scheduling result in input case order, not completion order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerReport {
    /// One record per selected case.
    pub cases: Vec<CaseExecution>,
    /// Observed bounded coordinator worker occupancy, useful for instrumentation.
    pub peak_workers: usize,
}

/// Run independent cases; World, file, scenario and subject membership are not locks.
///
/// # Errors
/// Rejects invalid options/identities and coordinator I/O failures. Attempt-level
/// failures remain structured case results and do not suppress other cases.
pub fn run_cases(
    artifact: &NativeTestArtifact,
    cases: &[ScheduledCase],
    options: &NativeRunOptions,
) -> Result<SchedulerReport, std::io::Error> {
    if options.jobs == 0 || options.timeout.is_zero() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "jobs and timeout must be positive",
        ));
    }
    let mut identities = BTreeSet::new();
    for case in cases {
        if !artifact.cases.contains(&case.identity) || !identities.insert(&case.identity) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "unknown or duplicate selected native case",
            ));
        }
    }
    processes::run(artifact, cases, options)
}
