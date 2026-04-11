//! Process management — trait-based for mockability.
//!
//! The [`ProcessManager`] trait abstracts over process spawning, exit-code
//! handling, and signal delivery so that tests can operate without forking
//! real OS processes.  The production implementation ([`StdProcessManager`])
//! delegates to [`std::process`]; the test double ([`MockProcessManager`])
//! records all calls in memory.

extern crate alloc;
extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

/// Exit code returned by a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ExitCode(pub i32);

impl ExitCode {
    /// The conventional success exit code (`0`).
    pub const SUCCESS: Self = Self(0);
    /// The conventional generic failure exit code (`1`).
    pub const FAILURE: Self = Self(1);

    /// Returns the raw exit-code integer.
    #[must_use]
    pub const fn code(self) -> i32 {
        self.0
    }

    /// Returns `true` when the exit code is `0`.
    #[must_use]
    pub const fn is_success(self) -> bool {
        self.0 == 0
    }
}

/// Signal number used on Unix-like platforms.
///
/// The numeric values follow POSIX (SIGTERM = 15, SIGKILL = 9, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signal(pub i32);

impl Signal {
    /// SIGTERM — request graceful shutdown.
    pub const TERM: Self = Self(15);
    /// SIGKILL — force immediate termination.
    pub const KILL: Self = Self(9);
    /// SIGINT — interrupt from terminal (Ctrl-C).
    pub const INT: Self = Self(2);
    /// SIGHUP — hang-up / reload configuration.
    pub const HUP: Self = Self(1);
}

/// Error type returned by process management operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessError {
    /// Human-readable description.
    pub message: String,
}

impl ProcessError {
    /// Creates a new [`ProcessError`] with the given message.
    #[must_use]
    pub fn new(message: &str) -> Self {
        Self {
            message: String::from(message),
        }
    }
}

/// A handle to a running child process.
pub trait ChildProcess {
    /// Returns the OS process ID of this child.
    fn pid(&self) -> u32;

    /// Waits for the child to terminate and returns its exit code.
    ///
    /// # Errors
    ///
    /// Returns a [`ProcessError`] if the wait call fails.
    fn wait(&mut self) -> Result<ExitCode, ProcessError>;

    /// Sends `signal` to the child process.
    ///
    /// On platforms where signals are not supported this is a no-op that
    /// returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns a [`ProcessError`] if the signal cannot be delivered.
    fn send_signal(&mut self, signal: Signal) -> Result<(), ProcessError>;
}

/// Spawn and manage child processes.
pub trait ProcessManager {
    /// Spawns a new process running `program` with `args`.
    ///
    /// Returns a [`ChildProcess`] handle on success.
    ///
    /// # Errors
    ///
    /// Returns a [`ProcessError`] if the process cannot be spawned.
    fn spawn(
        &mut self,
        program: &str,
        args: &[&str],
    ) -> Result<Box<dyn ChildProcess>, ProcessError>;

    /// Terminates the current process with `code`.
    ///
    /// # Safety for callers
    ///
    /// This function does not return.  Only call it when you genuinely want
    /// to exit the entire process.
    fn exit(&self, code: ExitCode) -> !;

    /// Returns the current process's ID.
    fn current_pid(&self) -> u32;

    /// Returns the current working directory as a string, or an error.
    ///
    /// # Errors
    ///
    /// Returns a [`ProcessError`] if the working directory cannot be read.
    fn current_dir(&self) -> Result<String, ProcessError>;
}

// ── Production implementation ─────────────────────────────────────────────

/// Production [`ChildProcess`] wrapping a real [`std::process::Child`].
pub struct StdChildProcess {
    /// The wrapped standard library child process handle.
    inner: std::process::Child,
}

impl ChildProcess for StdChildProcess {
    fn pid(&self) -> u32 {
        self.inner.id()
    }

    fn wait(&mut self) -> Result<ExitCode, ProcessError> {
        self.inner
            .wait()
            .map_err(|e| ProcessError::new(&e.to_string()))
            .map(|s| ExitCode(s.code().unwrap_or(1_i32)))
    }

    fn send_signal(&mut self, _signal: Signal) -> Result<(), ProcessError> {
        // Signal delivery via std is not cross-platform; this is a no-op
        // in the production shim.  Real signal delivery would require libc.
        Ok(())
    }
}

/// Production [`ProcessManager`] backed by [`std::process`].
#[derive(Debug, Clone, Copy, Default)]
pub struct StdProcessManager;

impl ProcessManager for StdProcessManager {
    fn spawn(
        &mut self,
        program: &str,
        args: &[&str],
    ) -> Result<Box<dyn ChildProcess>, ProcessError> {
        let child = std::process::Command::new(program)
            .args(args)
            .spawn()
            .map_err(|e| ProcessError::new(&e.to_string()))?;
        Ok(Box::new(StdChildProcess { inner: child }))
    }

    fn exit(&self, code: ExitCode) -> ! {
        std::process::exit(code.0)
    }

    fn current_pid(&self) -> u32 {
        std::process::id()
    }

    fn current_dir(&self) -> Result<String, ProcessError> {
        std::env::current_dir()
            .map_err(|e| ProcessError::new(&e.to_string()))
            .map(|p| p.to_string_lossy().into_owned())
    }
}

// ── Test double ───────────────────────────────────────────────────────────

/// Record of a single `spawn` call made to [`MockProcessManager`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCall {
    /// Program name passed to `spawn`.
    pub program: String,
    /// Arguments passed to `spawn`.
    pub args: Vec<String>,
}

/// In-memory [`ChildProcess`] for use in tests.
///
/// `wait` always returns the configured `exit_code`.
#[derive(Debug)]
pub struct MockChildProcess {
    /// The fake PID reported by `pid`.
    pub fake_pid: u32,
    /// Exit code that `wait` will return.
    pub exit_code: ExitCode,
    /// Signal received by `send_signal`, if any.
    pub received_signal: Option<Signal>,
}

impl MockChildProcess {
    /// Creates a new [`MockChildProcess`] with `fake_pid` and `exit_code`.
    #[must_use]
    pub const fn new(fake_pid: u32, exit_code: ExitCode) -> Self {
        Self {
            fake_pid,
            exit_code,
            received_signal: None,
        }
    }
}

impl ChildProcess for MockChildProcess {
    fn pid(&self) -> u32 {
        self.fake_pid
    }

    fn wait(&mut self) -> Result<ExitCode, ProcessError> {
        Ok(self.exit_code)
    }

    fn send_signal(&mut self, signal: Signal) -> Result<(), ProcessError> {
        self.received_signal = Some(signal);
        Ok(())
    }
}

/// In-memory [`ProcessManager`] for use in tests.
///
/// Records all `spawn` calls; `exit` panics (diverging — cannot record state).
#[derive(Debug, Default)]
pub struct MockProcessManager {
    /// All `spawn` calls recorded in order.
    pub spawn_calls: Vec<SpawnCall>,
    /// The fake PID returned by `current_pid`.
    pub fake_pid: u32,
    /// The fake working directory returned by `current_dir`.
    pub fake_dir: String,
    /// Exit code each spawned child reports when `wait` is called.
    pub child_exit_code: ExitCode,
}

impl MockProcessManager {
    /// Creates a new [`MockProcessManager`] with sensible defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spawn_calls: Vec::new(),
            fake_pid: 1234,
            fake_dir: String::from("/mock/dir"),
            child_exit_code: ExitCode::SUCCESS,
        }
    }
}

impl ProcessManager for MockProcessManager {
    fn spawn(
        &mut self,
        program: &str,
        args: &[&str],
    ) -> Result<Box<dyn ChildProcess>, ProcessError> {
        self.spawn_calls.push(SpawnCall {
            program: String::from(program),
            args: args.iter().map(|s| String::from(*s)).collect(),
        });
        Ok(Box::new(MockChildProcess::new(
            self.fake_pid,
            self.child_exit_code,
        )))
    }

    #[expect(
        clippy::panic,
        reason = "test double for diverging exit — no other way to implement -> ! without a real process exit"
    )]
    fn exit(&self, code: ExitCode) -> ! {
        panic!("MockProcessManager::exit called with code {}", code.0)
    }

    fn current_pid(&self) -> u32 {
        self.fake_pid
    }

    fn current_dir(&self) -> Result<String, ProcessError> {
        Ok(self.fake_dir.clone())
    }
}
