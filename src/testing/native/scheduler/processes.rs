//! Process ownership, complete-key admission and validated worker transport.
extern crate alloc;
use super::{
    AttemptExecution, CaseExecution, CaseOutcome, NativeRunOptions, NativeTestArtifact,
    ScheduledCase, SchedulerReport, WorkerResult, WorkerStatus,
};
use alloc::collections::{BTreeSet, VecDeque};
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// One queued attempt, never concurrently retried with itself.
struct Pending {
    /// Stable source case position, not a public case identity.
    index: usize,
    /// Independent attempt number.
    attempt: u64,
    /// Capacity and scheduling wait starts when queued.
    queued: Instant,
    /// Start of current complete-key blocking interval.
    blocked_since: Option<Instant>,
    /// Completed exclusion-key blocking intervals.
    exclusion_wait: Duration,
}

/// Exactly one coordinator-owned fresh worker process.
struct Running {
    /// Queue identity and waiting metrics.
    pending: Pending,
    /// Native process; no mutable worker state is reused.
    child: Child,
    /// Process lifetime starts after successful spawn.
    started: Instant,
    /// Dedicated result channel.
    transport: PathBuf,
    /// Attempt-local ordinary stdout capture.
    stdout: PathBuf,
    /// Attempt-local ordinary stderr capture.
    stderr: PathBuf,
    /// Frozen admission wait, separate from execution.
    scheduler_wait: Duration,
}

/// Complete invocation-local keys, not World/subject/resource tracking.
struct Coordinator<'source> {
    /// Selected independent case metadata.
    cases: &'source [ScheduledCase],
    /// Explicit worker/retry/deadline controls.
    options: &'source NativeRunOptions,
    /// Absolute reusable artifact path.
    executable: PathBuf,
    /// Absolute private coordinator output directory.
    output: PathBuf,
    /// Pending fresh attempts.
    pending: VecDeque<Pending>,
    /// Bounded coordinator-owned processes.
    running: Vec<Running>,
    /// Complete scheduler-only key sets held through worker completion/termination.
    keys: BTreeSet<String>,
    /// Retained attempts, including failures before retries.
    attempts: Vec<Vec<AttemptExecution>>,
    /// Observed coordinator worker occupancy.
    peak: usize,
}

/// Parse tagged worker events independently from captured user output.
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Event {
    /// Synchronously flushed progress survives hard termination.
    Phase {
        /// Versioned native lifecycle phase number.
        phase: u32,
    },
    /// Real source callback entry/completion, streamed before the terminal result.
    Callback(super::metrics::CallbackEvent),
    /// Complete normal lifecycle result.
    Result(WorkerResult),
    /// Worker initialization/selection/transport infrastructure error.
    Infrastructure {
        /// Worker initialization or protocol failure description.
        message: String,
    },
}

/// Schedule eligible independent cases, polling only bounded live child processes.
pub(super) fn run(
    artifact: &NativeTestArtifact,
    cases: &[ScheduledCase],
    options: &NativeRunOptions,
) -> Result<SchedulerReport, std::io::Error> {
    std::fs::create_dir_all(&options.output_directory)?;
    let now = Instant::now();
    let mut coordinator = Coordinator {
        cases,
        options,
        executable: std::fs::canonicalize(&artifact.executable)?,
        output: std::fs::canonicalize(&options.output_directory)?,
        pending: (0..cases.len())
            .map(|index| Pending {
                index,
                attempt: 1,
                queued: now,
                blocked_since: None,
                exclusion_wait: Duration::ZERO,
            })
            .collect(),
        running: Vec::new(),
        keys: BTreeSet::new(),
        attempts: vec![Vec::new(); cases.len()],
        peak: 0,
    };
    while !coordinator.pending.is_empty() || !coordinator.running.is_empty() {
        coordinator.admit();
        coordinator.poll();
        if !coordinator.running.is_empty() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let records = coordinator
        .attempts
        .into_iter()
        .zip(cases)
        .map(|(attempts, case)| {
            let final_outcome = attempts
                .last()
                .map_or(CaseOutcome::InfrastructureFailed, |attempt| attempt.outcome);
            let outcome = if final_outcome.is_success() && attempts.len() > 1 {
                CaseOutcome::FlakyPassed
            } else {
                final_outcome
            };
            CaseExecution {
                identity: case.identity.clone(),
                outcome,
                attempts,
            }
        })
        .collect();
    Ok(SchedulerReport {
        cases: records,
        peak_workers: coordinator.peak,
    })
}

impl Coordinator<'_> {
    /// Acquire a complete key set before spawning any setup/example code.
    fn admit(&mut self) {
        let available = self.pending.len();
        for _ in 0..available {
            if self.running.len() >= self.options.jobs {
                break;
            }
            let Some(mut pending) = self.pending.pop_front() else {
                break;
            };
            let requested = &self.cases[pending.index].exclusion_keys;
            if requested.iter().any(|key| self.keys.contains(key)) {
                if pending.blocked_since.is_none() {
                    pending.blocked_since = Some(Instant::now());
                }
                self.pending.push_back(pending);
                continue;
            }
            if let Some(start) = pending.blocked_since.take() {
                pending.exclusion_wait = pending.exclusion_wait.saturating_add(start.elapsed());
            }
            match self.spawn(pending) {
                Ok(running) => {
                    self.keys.extend(requested.iter().cloned());
                    self.running.push(running);
                    self.peak = self.peak.max(self.running.len());
                }
                Err((pending, message)) => {
                    let record = AttemptExecution {
                        attempt: pending.attempt,
                        process_id: None,
                        outcome: CaseOutcome::InfrastructureFailed,
                        duration_ms: 0,
                        scheduler_wait_ms: millis(pending.queued.elapsed()),
                        exclusion_wait_ms: millis(pending.exclusion_wait),
                        last_phase: None,
                        callbacks: Vec::new(),
                        result: None,
                        infrastructure_message: Some(message),
                        stdout_path: PathBuf::new(),
                        stderr_path: PathBuf::new(),
                    };
                    self.retain(pending.index, record);
                }
            }
        }
    }

    /// Files cannot deadlock a worker on unread stdout/stderr pipes.
    fn spawn(&self, pending: Pending) -> Result<Running, (Pending, String)> {
        let prefix = format!("case-{}-attempt-{}", pending.index, pending.attempt);
        let transport = self.output.join(format!("{prefix}.jsonl"));
        let stdout = self.output.join(format!("{prefix}.stdout"));
        let stderr = self.output.join(format!("{prefix}.stderr"));
        let launch = || -> Result<Child, std::io::Error> {
            if transport.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "native attempt result already exists",
                ));
            }
            let identity = serde_json::to_string(&self.cases[pending.index].identity)
                .map_err(std::io::Error::other)?;
            let mut command = Command::new(&self.executable);
            if let Some(ref executable) = self.options.application_executable {
                command.env("OPAL_TEST_APPLICATION_EXECUTABLE", executable);
            } else {
                command.env_remove("OPAL_TEST_APPLICATION_EXECUTABLE");
            }
            command
                .arg(identity)
                .arg(pending.attempt.to_string())
                .arg(&transport)
                .current_dir(&self.options.working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::from(create_capture(&stdout)?))
                .stderr(Stdio::from(create_capture(&stderr)?))
                .spawn()
        };
        match launch() {
            Ok(child) => {
                let scheduler_wait = pending.queued.elapsed();
                Ok(Running {
                    pending,
                    child,
                    started: Instant::now(),
                    transport,
                    stdout,
                    stderr,
                    scheduler_wait,
                })
            }
            Err(error) => Err((pending, error.to_string())),
        }
    }

    /// A hard timeout kills only the framework worker, not undiscovered user resources.
    fn poll(&mut self) {
        let mut index = 0;
        while index < self.running.len() {
            let polled = self.running[index].child.try_wait();
            let timed_out = matches!(polled, Ok(None))
                && self.running[index].started.elapsed() >= self.options.timeout;
            if matches!(polled, Ok(None)) && !timed_out {
                index = index.saturating_add(1);
                continue;
            }
            let mut running = self.running.swap_remove(index);
            let status = match polled {
                Ok(Some(status)) => Ok(status),
                Ok(None) => {
                    drop(running.child.kill());
                    running.child.wait()
                }
                Err(error) => {
                    drop(running.child.kill());
                    drop(running.child.wait());
                    Err(error)
                }
            };
            let record = finish(
                &running,
                status,
                timed_out,
                &self.cases[running.pending.index],
            );
            for key in &self.cases[running.pending.index].exclusion_keys {
                self.keys.remove(key);
            }
            self.retain(running.pending.index, record);
        }
    }

    /// Retry retains earlier outcomes and queues a new process with fresh source state.
    fn retain(&mut self, index: usize, record: AttemptExecution) {
        let retry =
            !record.outcome.is_success() && record.attempt <= u64::from(self.options.retries);
        let next = record.attempt.saturating_add(1);
        self.attempts[index].push(record);
        if retry {
            self.pending.push_back(Pending {
                index,
                attempt: next,
                queued: Instant::now(),
                blocked_since: None,
                exclusion_wait: Duration::ZERO,
            });
        }
    }
}

/// Coordinator files are build/result products, not registrations of test resources.
fn create_capture(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Millisecond reporting is bounded without wrapping long-running durations.
fn millis(value: Duration) -> u64 {
    u64::try_from(value.as_millis()).unwrap_or(u64::MAX)
}

/// Preserve progress even when a worker crashes or is terminated mid-record.
fn finish(
    running: &Running,
    status: Result<ExitStatus, std::io::Error>,
    timed_out: bool,
    case: &ScheduledCase,
) -> AttemptExecution {
    let (last_phase, mut result, mut message, mut callbacks) = transport(&running.transport);
    let outcome = if timed_out {
        CaseOutcome::TimedOut
    } else {
        match status {
            Err(error) => {
                message = Some(error.to_string());
                CaseOutcome::InfrastructureFailed
            }
            Ok(status) if status.code().is_none() => CaseOutcome::WorkerCrashed,
            Ok(status) => validate_result(
                case,
                running.pending.attempt,
                status,
                result.as_ref(),
                &mut message,
            ),
        }
    };
    // Incomplete or rejected transport cannot assert callback completion.
    if timed_out || outcome == CaseOutcome::WorkerCrashed || message.is_some() {
        result = None;
        for callback in &mut callbacks {
            if callback.outcome == "running" {
                "interrupted".clone_into(&mut callback.outcome);
            }
        }
    }
    AttemptExecution {
        attempt: running.pending.attempt,
        process_id: Some(running.child.id()),
        outcome,
        duration_ms: millis(running.started.elapsed()),
        scheduler_wait_ms: millis(running.scheduler_wait),
        exclusion_wait_ms: millis(running.pending.exclusion_wait),
        last_phase,
        callbacks,
        result,
        infrastructure_message: message,
        stdout_path: running.stdout.clone(),
        stderr_path: running.stderr.clone(),
    }
}

/// Completed records must match the exact selected case, attempt and native exit status.
fn validate_result(
    case: &ScheduledCase,
    attempt: u64,
    status: ExitStatus,
    result: Option<&WorkerResult>,
    message: &mut Option<String>,
) -> CaseOutcome {
    let Some(result) = result else {
        if status.code() == Some(2_i32) || message.is_some() {
            return CaseOutcome::InfrastructureFailed;
        }
        return CaseOutcome::WorkerCrashed;
    };
    let expected = serde_json::to_string(&case.identity).ok();
    let valid = expected.as_deref() == Some(&result.case_id)
        && result.attempt == attempt
        && status.code() == Some(i32::from(result.status != WorkerStatus::Passed))
        && match result.status {
            WorkerStatus::Passed => {
                result.primary_failure_id == 0
                    && result.failures.is_empty()
                    && result.body_started
                    && result.body_passed
            }
            WorkerStatus::Failed => {
                result.primary_failure_id != 0
                    && result
                        .failures
                        .iter()
                        .any(|failure| failure.id == result.primary_failure_id)
            }
        };
    if !valid || !diagnostics_valid(result) || message.is_some() {
        *message = Some(
            "worker result identity/status/diagnostics do not match the selected attempt"
                .to_owned(),
        );
        return CaseOutcome::InfrastructureFailed;
    }
    if result.status == WorkerStatus::Passed {
        return CaseOutcome::Passed;
    }
    let Some(primary) = result
        .failures
        .iter()
        .find(|failure| failure.id == result.primary_failure_id)
    else {
        return CaseOutcome::InfrastructureFailed;
    };
    if primary.kind == 5 {
        return CaseOutcome::InfrastructureFailed;
    }
    match primary.phase {
        1 => CaseOutcome::ExampleFailed,
        2 => CaseOutcome::WorldSetupFailed,
        3 => CaseOutcome::BeforeHookFailed,
        5 => CaseOutcome::AfterHookFailed,
        6 => CaseOutcome::CleanupFailed,
        4 if primary.kind == 4 => CaseOutcome::ApplicationError,
        4 => CaseOutcome::AssertionFailed,
        _ => CaseOutcome::InfrastructureFailed,
    }
}

/// Validate retained immutable failure identities and the versioned native enums.
fn diagnostics_valid(result: &WorkerResult) -> bool {
    let mut identities = BTreeSet::new();
    result.failures.iter().all(|failure| {
        failure.id != 0
            && (1..=5).contains(&failure.kind)
            && (1..=7).contains(&failure.phase)
            && failure.primary_failure_id == result.primary_failure_id
            && (failure.parent_failure_id == 0 || identities.contains(&failure.parent_failure_id))
            && identities.insert(failure.id)
    })
}

/// A partial final line is not a complete event and cannot claim cleanup completion.
fn transport(
    path: &Path,
) -> (
    Option<u32>,
    Option<WorkerResult>,
    Option<String>,
    Vec<super::metrics::CallbackMetric>,
) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return (None, None, Some(error.to_string()), Vec::new()),
    };
    let mut phase = None;
    let mut callbacks = Vec::new();
    let mut result = None;
    let mut message = None;
    for line in text
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        match serde_json::from_str::<Event>(line) {
            Ok(Event::Phase { phase: current })
                if (1..=7).contains(&current) && result.is_none() =>
            {
                phase = Some(current);
            }
            Ok(Event::Callback(current)) if result.is_none() => {
                if !super::metrics::accept(&mut callbacks, current) {
                    message = Some("invalid native callback boundary event".to_owned());
                }
            }
            Ok(Event::Result(current)) if result.is_none() => result = Some(current),
            Ok(Event::Infrastructure { message: current }) => message = Some(current),
            Ok(_) => message = Some("invalid or duplicate native worker event".to_owned()),
            Err(error) => message = Some(format!("malformed native worker event: {error}")),
        }
    }
    if result.is_some()
        && callbacks
            .iter()
            .any(|callback| callback.outcome == "running")
    {
        message = Some("worker claimed completion with an unfinished callback".to_owned());
    }
    (phase, result, message, callbacks)
}
