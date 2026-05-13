//! Shared bounded subprocess execution with timeout-aware diagnostics.
//!
//! This module provides an opt-in bounded execution policy that captures child
//! process output, drains pipes concurrently, and terminates Unix process groups
//! on timeout so tests fail with diagnostics instead of hanging forever.

use core::fmt;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

/// Poll interval used while waiting for bounded child processes.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Execution policy for subprocesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPolicy {
    /// Wait indefinitely for the child process to finish.
    Unbounded,
    /// Enforce a timeout and perform cleanup on timeout.
    Bounded {
        /// Maximum time to allow the command to run before cleanup starts.
        timeout: Duration,
        /// Grace window between TERM and KILL for Unix process-group cleanup.
        grace: Duration,
        /// Whether to terminate the entire child process group on timeout.
        kill_group: bool,
    },
}

impl RunPolicy {
    /// Returns the configured timeout for bounded policies.
    #[must_use]
    pub const fn timeout(self) -> Option<Duration> {
        match self {
            Self::Unbounded => None,
            Self::Bounded { timeout, .. } => Some(timeout),
        }
    }

    /// Returns the configured grace window for bounded policies.
    #[must_use]
    pub const fn grace(self) -> Option<Duration> {
        match self {
            Self::Unbounded => None,
            Self::Bounded { grace, .. } => Some(grace),
        }
    }

    /// Returns whether timeout cleanup should target the full process group.
    #[must_use]
    pub const fn kill_group(self) -> bool {
        match self {
            Self::Unbounded => false,
            Self::Bounded { kill_group, .. } => kill_group,
        }
    }
}

/// Exit details normalized across platforms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitDetails {
    /// Numeric exit code when the platform provides one.
    pub code: Option<i32>,
    /// Signal number when the process terminated because of a signal.
    pub signal: Option<i32>,
    /// Whether the process reported a successful exit status.
    pub success: bool,
}

/// Successful subprocess execution result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutput {
    /// Human-readable context supplied by the caller.
    pub context: String,
    /// Program followed by argv elements, rendered lossily for diagnostics.
    pub argv: Vec<String>,
    /// Working directory used for the command.
    pub cwd: Option<PathBuf>,
    /// Total elapsed runtime.
    pub duration: Duration,
    /// Exit details captured from the child process.
    pub exit: ExitDetails,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
}

/// Structured subprocess execution failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunFailure {
    /// Human-readable context supplied by the caller.
    pub context: String,
    /// Program followed by argv elements, rendered lossily for diagnostics.
    pub argv: Vec<String>,
    /// Working directory used for the command.
    pub cwd: Option<PathBuf>,
    /// Total elapsed runtime.
    pub duration: Duration,
    /// Timeout that governed the run, if any.
    pub timeout: Option<Duration>,
    /// Grace window used during timeout cleanup, if any.
    pub grace: Option<Duration>,
    /// Whether timeout cleanup targeted the full process group.
    pub kill_group: bool,
    /// Whether this failure was triggered by a timeout.
    pub timed_out: bool,
    /// Exit details when the child produced a status.
    pub exit: Option<ExitDetails>,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
}

impl fmt::Display for RunFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let argv = if self.argv.is_empty() {
            String::from("<empty command>")
        } else {
            self.argv.join(" ")
        };
        let cwd = self.cwd.as_ref().map_or_else(
            || String::from("<unknown cwd>"),
            |path| path.display().to_string(),
        );
        let exit = self.exit.as_ref().map_or_else(
            || String::from("<no exit status>"),
            |exit| {
                format!(
                    "code={}, signal={}",
                    format_optional_i32(exit.code),
                    format_optional_i32(exit.signal)
                )
            },
        );

        write!(
            f,
            "{} failed\ncommand: {}\ncwd: {}\nduration: {}\ntimeout: {}\ngrace: {}\nkill_group: {}\ntimed_out: {}\nexit: {}\nstdout:\n{}\nstderr:\n{}",
            self.context,
            argv,
            cwd,
            format_duration(self.duration),
            format_optional_duration(self.timeout),
            format_optional_duration(self.grace),
            self.kill_group,
            self.timed_out,
            exit,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Errors produced while running a subprocess.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The child process could not be spawned.
    #[error("failed to spawn {context}: {source}")]
    Spawn {
        /// Human-readable context supplied by the caller.
        context: String,
        /// Program followed by argv elements, rendered lossily for diagnostics.
        argv: Vec<String>,
        /// Working directory used for the command.
        cwd: Option<PathBuf>,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// I/O failed while draining pipes or writing stdin.
    #[error("failed during {stage} for {context}: {source}")]
    Io {
        /// Human-readable context supplied by the caller.
        context: String,
        /// Stage that failed.
        stage: &'static str,
        /// Program followed by argv elements, rendered lossily for diagnostics.
        argv: Vec<String>,
        /// Working directory used for the command.
        cwd: Option<PathBuf>,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The process exited unsuccessfully or timed out.
    #[error("{0}")]
    Execution(Box<RunFailure>),
}

impl RunError {
    /// Returns the structured failure payload for execution failures.
    #[must_use]
    pub const fn failure(&self) -> Option<&RunFailure> {
        match *self {
            Self::Execution(ref failure) => Some(failure),
            Self::Spawn { .. } | Self::Io { .. } => None,
        }
    }
}

/// Immutable command metadata reused across subprocess diagnostics.
#[derive(Clone, Debug)]
struct CommandMetadata {
    /// Human-readable context supplied by the caller.
    context: String,
    /// Program followed by argv elements, rendered lossily for diagnostics.
    argv: Vec<String>,
    /// Working directory used for the command.
    cwd: Option<PathBuf>,
}

impl CommandMetadata {
    /// Capture program, argv, and cwd details from a command before spawning it.
    fn from_command(command: &Command, context: String) -> Self {
        let mut argv = Vec::new();
        argv.push(command.get_program().to_string_lossy().into_owned());
        argv.extend(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned()),
        );

        let cwd = command
            .get_current_dir()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Self { context, argv, cwd }
    }
}

/// Format a duration in seconds for human-readable diagnostics.
fn format_duration(duration: Duration) -> String {
    format!("{:.3}s", duration.as_secs_f64())
}

/// Format an optional duration for human-readable diagnostics.
fn format_optional_duration(duration: Option<Duration>) -> String {
    duration.map_or_else(|| String::from("<none>"), format_duration)
}

/// Format an optional integer field for human-readable diagnostics.
fn format_optional_i32(value: Option<i32>) -> String {
    value.map_or_else(|| String::from("<none>"), |value| value.to_string())
}

/// Run a command, capturing stdout/stderr and failing on non-zero exit status.
pub fn run_command(
    command: &mut Command,
    policy: RunPolicy,
    context: impl Into<String>,
) -> Result<RunOutput, RunError> {
    run_command_internal(command, None, policy, context.into())
}

/// Run a command with explicit stdin bytes.
pub fn run_command_with_input(
    command: &mut Command,
    input: Vec<u8>,
    policy: RunPolicy,
    context: impl Into<String>,
) -> Result<RunOutput, RunError> {
    run_command_internal(command, Some(input), policy, context.into())
}

/// Wait for a child process with a timeout and collect its output synchronously.
fn wait_for_child_output_internal(
    child: &mut Child,
    timeout: Duration,
    command_context: String,
) -> Result<Output, RunError> {
    let metadata = CommandMetadata {
        context: command_context,
        argv: Vec::new(),
        cwd: None,
    };
    let policy = RunPolicy::Bounded {
        timeout,
        grace: Duration::from_millis(50),
        kill_group: false,
    };
    let start = Instant::now();
    let CommandMetadata { context, argv, cwd } = metadata;

    let (status, timed_out) =
        wait_for_exit(child, policy, start).map_err(|source| RunError::Io {
            context: context.clone(),
            stage: "waiting for child exit",
            argv: argv.clone(),
            cwd: cwd.clone(),
            source,
        })?;

    let stdout = if let Some(mut pipe) = child.stdout.take() {
        let mut buffer = Vec::new();
        pipe.read_to_end(&mut buffer)
            .map_err(|source| RunError::Io {
                context: context.clone(),
                stage: "reading stdout",
                argv: argv.clone(),
                cwd: cwd.clone(),
                source,
            })?;
        buffer
    } else {
        Vec::new()
    };

    let stderr = if let Some(mut pipe) = child.stderr.take() {
        let mut buffer = Vec::new();
        pipe.read_to_end(&mut buffer)
            .map_err(|source| RunError::Io {
                context: context.clone(),
                stage: "reading stderr",
                argv: argv.clone(),
                cwd: cwd.clone(),
                source,
            })?;
        buffer
    } else {
        Vec::new()
    };

    let output = Output {
        status,
        stdout,
        stderr,
    };
    if timed_out {
        return Err(RunError::Execution(Box::new(RunFailure {
            context,
            argv,
            cwd,
            duration: start.elapsed(),
            timeout: Some(timeout),
            grace: Some(Duration::from_millis(50)),
            kill_group: false,
            timed_out,
            exit: Some(exit_details(output.status)),
            stdout: output.stdout,
            stderr: output.stderr,
        })));
    }

    Ok(output)
}

/// Wait for a spawned child process within a bounded policy and collect output.
pub fn wait_for_child_output_with_timeout(
    mut child: Child,
    timeout: Duration,
    context: impl Into<String>,
) -> Result<Output, RunError> {
    wait_for_child_output_internal(&mut child, timeout, context.into())
}

/// Spawn, monitor, and collect output for a command under the provided policy.
fn run_command_internal(
    command: &mut Command,
    input: Option<Vec<u8>>,
    policy: RunPolicy,
    context: String,
) -> Result<RunOutput, RunError> {
    let metadata = CommandMetadata::from_command(command, context);
    configure_command(command, input.is_some(), policy);

    let start = Instant::now();
    let mut child = command.spawn().map_err(|source| RunError::Spawn {
        context: metadata.context.clone(),
        argv: metadata.argv.clone(),
        cwd: metadata.cwd.clone(),
        source,
    })?;

    let stdout_handle = spawn_reader_thread(child.stdout.take());
    let stderr_handle = spawn_reader_thread(child.stderr.take());
    let stdin_handle = spawn_stdin_writer(child.stdin.take(), input);

    let (status, timed_out) =
        wait_for_exit(&mut child, policy, start).map_err(|source| RunError::Io {
            context: metadata.context.clone(),
            stage: "waiting for child exit",
            argv: metadata.argv.clone(),
            cwd: metadata.cwd.clone(),
            source,
        })?;

    let stdout = join_reader_thread(stdout_handle, &metadata, "reading stdout")?;
    let stderr = join_reader_thread(stderr_handle, &metadata, "reading stderr")?;
    join_stdin_writer(stdin_handle, &metadata)?;

    let duration = start.elapsed();
    let exit = exit_details(status);
    if timed_out || !exit.success {
        return Err(RunError::Execution(Box::new(RunFailure {
            context: metadata.context,
            argv: metadata.argv,
            cwd: metadata.cwd,
            duration,
            timeout: policy.timeout(),
            grace: policy.grace(),
            kill_group: policy.kill_group(),
            timed_out,
            exit: Some(exit),
            stdout,
            stderr,
        })));
    }

    Ok(RunOutput {
        context: metadata.context,
        argv: metadata.argv,
        cwd: metadata.cwd,
        duration,
        exit,
        stdout,
        stderr,
    })
}

/// Configure stdio pipes and Unix process-group behavior before spawning a command.
fn configure_command(command: &mut Command, has_input: bool, policy: RunPolicy) {
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if has_input {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    #[cfg(unix)]
    if matches!(
        policy,
        RunPolicy::Bounded {
            kill_group: true,
            ..
        }
    ) {
        command.process_group(0);
    }
}

/// Spawn a background reader that drains a child stdout or stderr pipe.
fn spawn_reader_thread<T>(pipe: Option<T>) -> Option<JoinHandle<io::Result<Vec<u8>>>>
where
    T: Read + Send + 'static,
{
    pipe.map(|mut pipe| {
        thread::spawn(move || {
            let mut buffer = Vec::new();
            pipe.read_to_end(&mut buffer)?;
            Ok(buffer)
        })
    })
}

/// Spawn a background writer that feeds buffered stdin into a child process.
fn spawn_stdin_writer(
    pipe: Option<std::process::ChildStdin>,
    input: Option<Vec<u8>>,
) -> Option<JoinHandle<io::Result<()>>> {
    match (pipe, input) {
        (Some(mut stdin), Some(input)) => Some(thread::spawn(move || {
            stdin.write_all(&input)?;
            stdin.flush()?;
            Ok(())
        })),
        _ => None,
    }
}

/// Join a reader thread and normalize its panic or I/O failure into `RunError`.
fn join_reader_thread(
    handle: Option<JoinHandle<io::Result<Vec<u8>>>>,
    metadata: &CommandMetadata,
    stage: &'static str,
) -> Result<Vec<u8>, RunError> {
    match handle {
        Some(handle) => handle.join().map_err(|_panic| RunError::Io {
            context: metadata.context.clone(),
            stage,
            argv: metadata.argv.clone(),
            cwd: metadata.cwd.clone(),
            source: io::Error::other("reader thread panicked"),
        })?,
        None => Ok(Vec::new()),
    }
    .map_err(|source| RunError::Io {
        context: metadata.context.clone(),
        stage,
        argv: metadata.argv.clone(),
        cwd: metadata.cwd.clone(),
        source,
    })
}

/// Join the stdin writer thread and normalize its panic or I/O failure.
fn join_stdin_writer(
    handle: Option<JoinHandle<io::Result<()>>>,
    metadata: &CommandMetadata,
) -> Result<(), RunError> {
    match handle {
        Some(handle) => handle.join().map_err(|_panic| RunError::Io {
            context: metadata.context.clone(),
            stage: "writing stdin",
            argv: metadata.argv.clone(),
            cwd: metadata.cwd.clone(),
            source: io::Error::other("stdin writer thread panicked"),
        })?,
        None => Ok(()),
    }
    .map_err(|source| RunError::Io {
        context: metadata.context.clone(),
        stage: "writing stdin",
        argv: metadata.argv.clone(),
        cwd: metadata.cwd.clone(),
        source,
    })
}

/// Wait for a child to exit and perform timeout cleanup when required.
fn wait_for_exit(
    child: &mut Child,
    policy: RunPolicy,
    start: Instant,
) -> io::Result<(ExitStatus, bool)> {
    match policy {
        RunPolicy::Unbounded => child.wait().map(|status| (status, false)),
        RunPolicy::Bounded {
            timeout,
            grace,
            kill_group,
        } => loop {
            if let Some(status) = child.try_wait()? {
                return Ok((status, false));
            }

            if start.elapsed() >= timeout {
                let status = terminate_timed_out_child(child, grace, kill_group)?;
                return Ok((status, true));
            }

            thread::sleep(POLL_INTERVAL);
        },
    }
}

/// Terminate a timed-out child, optionally targeting its whole process group.
fn terminate_timed_out_child(
    child: &mut Child,
    grace: Duration,
    kill_group: bool,
) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    if kill_group {
        return terminate_process_group(child, grace);
    }

    let _: Duration = grace;
    child.kill()?;
    child.wait()
}

/// Terminate a Unix process group with TERM followed by KILL after the grace period.
#[cfg(unix)]
fn terminate_process_group(child: &mut Child, grace: Duration) -> io::Result<ExitStatus> {
    let pgid = i32::try_from(child.id())
        .map_err(|error| io::Error::other(format!("child pid does not fit in i32: {error}")))?;
    send_signal_to_group(pgid, libc::SIGTERM)?;

    if let Some(status) = wait_for_exit_with_deadline(child, grace)? {
        return Ok(status);
    }

    send_signal_to_group(pgid, libc::SIGKILL)?;
    child.wait()
}

/// Poll for child exit until the provided deadline elapses.
#[cfg(unix)]
fn wait_for_exit_with_deadline(
    child: &mut Child,
    timeout: Duration,
) -> io::Result<Option<ExitStatus>> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }

        if start.elapsed() >= timeout {
            return Ok(None);
        }

        thread::sleep(POLL_INTERVAL);
    }
}

/// Send a signal to the Unix process group identified by `pgid`.
#[cfg(unix)]
fn send_signal_to_group(pgid: i32, signal: libc::c_int) -> io::Result<()> {
    let process_group = pgid
        .checked_neg()
        .ok_or_else(|| io::Error::other("process group id overflow"))?;
    // SAFETY: `process_group` is a valid negative process-group id derived from a child pid,
    // and `signal` is forwarded directly to libc without aliasing or lifetime concerns.
    let result = unsafe { libc::kill(process_group, signal) };
    if result == 0_i32 {
        return Ok(());
    }

    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }

    Err(error)
}

/// Normalize a platform exit status into cross-platform exit details.
fn exit_details(status: ExitStatus) -> ExitDetails {
    ExitDetails {
        code: status.code(),
        signal: exit_signal(status),
        success: status.success(),
    }
}

/// Extract the terminating Unix signal from an exit status when available.
#[cfg(unix)]
fn exit_signal(status: ExitStatus) -> Option<i32> {
    status.signal()
}

/// Stub signal extraction for non-Unix platforms.
#[cfg(not(unix))]
fn exit_signal(_status: ExitStatus) -> Option<i32> {
    None
}

#[cfg(test)]
mod tests {
    use super::{RunError, RunPolicy, run_command};
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    use tempfile::tempdir;

    fn bounded_policy(timeout: Duration) -> RunPolicy {
        RunPolicy::Bounded {
            timeout,
            grace: Duration::from_secs(2),
            kill_group: true,
        }
    }

    #[cfg(unix)]
    fn shell_command(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    #[cfg(unix)]
    #[test]
    fn bounded_proc_timeout_returns_quickly() {
        let start = Instant::now();
        let error = run_command(
            &mut shell_command("sleep 10"),
            bounded_policy(Duration::from_millis(250)),
            "timeout smoke test",
        )
        .expect_err("sleep should time out");

        let elapsed = start.elapsed();
        let failure = error
            .failure()
            .expect("timeout should yield execution failure");
        assert!(failure.timed_out, "timeout should be reported explicitly");
        assert!(
            elapsed < Duration::from_secs(5),
            "timeout path should finish quickly, took {elapsed:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_proc_captures_stdout_and_stderr_on_timeout() {
        let script = "i=0; while [ $i -lt 4096 ]; do printf '0123456789abcdef0123456789abcdef'; printf 'fedcba9876543210fedcba9876543210' >&2; i=$((i+1)); done; sleep 10";
        let error = run_command(
            &mut shell_command(script),
            bounded_policy(Duration::from_millis(400)),
            "chatty timeout test",
        )
        .expect_err("chatty process should time out");

        let failure = error
            .failure()
            .expect("timeout should yield execution failure");
        assert!(failure.timed_out, "timeout should be reported explicitly");
        assert_eq!(
            failure.stdout.len(),
            0x0002_0000,
            "stdout should be drained without deadlock"
        );
        assert_eq!(
            failure.stderr.len(),
            0x0002_0000,
            "stderr should be drained without deadlock"
        );
        assert!(
            String::from_utf8_lossy(&failure.stdout).starts_with("0123456789abcdef"),
            "stdout should preserve the emitted prefix"
        );
        assert!(
            String::from_utf8_lossy(&failure.stderr).starts_with("fedcba9876543210"),
            "stderr should preserve the emitted prefix"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_proc_kills_process_group_and_leaves_no_sleep_orphan() {
        let temp = tempdir().expect("tempdir should be created");
        let pid_file = temp.path().join("grandchild.pid");
        let script = format!(
            "sleep 30 & child=$!; printf '%s' \"$child\" > \"{}\"; wait",
            pid_file.display()
        );

        let error = run_command(
            &mut shell_command(&script),
            bounded_policy(Duration::from_millis(300)),
            "process group cleanup test",
        )
        .expect_err("shell waiting on sleep should time out");

        let failure = error
            .failure()
            .expect("timeout should yield execution failure");
        assert!(failure.timed_out, "cleanup test should time out");
        let grandchild_pid = std::fs::read_to_string(&pid_file)
            .expect("child pid file should be written before timeout")
            .trim()
            .parse::<i32>()
            .expect("child pid should parse");

        assert!(
            wait_for_process_exit(grandchild_pid, Duration::from_secs(2)),
            "sleep grandchild pid {grandchild_pid} should be gone after process-group cleanup"
        );
    }

    #[cfg(unix)]
    fn wait_for_process_exit(pid: i32, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            if !process_exists(pid) {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    #[cfg(unix)]
    fn process_exists(pid: i32) -> bool {
        if !Path::new(&format!("/proc/{pid}")).exists() {
            return false;
        }

        // SAFETY: `pid` is only used as an observed process identifier for signal 0 probing.
        let result = unsafe { libc::kill(pid, 0) };
        if result == 0_i32 {
            return true;
        }

        let error = std::io::Error::last_os_error();
        error.raw_os_error() != Some(libc::ESRCH)
    }

    #[test]
    fn run_error_exposes_execution_failure_payload() {
        let error = RunError::Execution(Box::new(super::RunFailure {
            context: String::from("context"),
            argv: vec![String::from("cmd")],
            cwd: None,
            duration: Duration::from_millis(1),
            timeout: Some(Duration::from_secs(1)),
            grace: Some(Duration::from_secs(2)),
            kill_group: true,
            timed_out: true,
            exit: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }));

        assert!(
            error.failure().is_some(),
            "execution failures should expose structured diagnostics"
        );
    }
}
