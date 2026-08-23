//! POSIX process-control backend.
//!
//! This module owns the process-global signal handlers used to translate
//! catchable job-control requests into runtime observations without doing
//! allocation or locking inside the handlers themselves.

extern crate alloc;
extern crate std;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use core::ptr;
use std::io;
use std::os::fd::RawFd;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread::{self, JoinHandle};

use super::{
    ProcessControlHost, ProcessControlHostActionError, ProcessControlHostObservation,
    ProcessControlHostObservationError, ProcessControlUnavailableError,
};
use crate::runtime::wait::{SourceAvailability, SystemReadinessSource};

/// Synthetic byte written to the signal pipe for `SIGTSTP`.
const SIGNAL_CODE_SUSPEND_REQUESTED: u8 = 1;
/// Synthetic byte written to the signal pipe for `SIGCONT`.
const SIGNAL_CODE_CONTINUED: u8 = 2;
/// Sentinel for an unconfigured pipe descriptor.
const INVALID_PIPE_FD: RawFd = -1;

/// Process-global ownership bit for the installed signal handlers.
static PROCESS_CONTROL_BACKEND_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Write end used by the async-signal-safe handlers.
static PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(INVALID_PIPE_FD);

/// Shared state populated by the helper thread and consumed by polling.
#[derive(Debug, Default)]
struct PosixProcessControlHostState {
    /// FIFO of host indications observed by the helper thread.
    pending_observations: VecDeque<ProcessControlHostObservation>,
    /// Monotonic counter of delivered `SIGCONT` observations.
    continued_epoch: u64,
    /// Last continuation epoch consumed by `resume_application`.
    consumed_continued_epoch: u64,
}

/// Real POSIX backend used on supported Unix hosts.
pub(super) struct PosixProcessControlHost {
    /// Shared pending-observation state.
    state: Arc<Mutex<PosixProcessControlHostState>>,
    /// Read end consumed by the helper thread.
    read_fd: RawFd,
    /// Write end used by the signal handlers.
    write_fd: RawFd,
    /// Previous `SIGTSTP` action restored on drop.
    previous_suspend_action: libc::sigaction,
    /// Previous `SIGCONT` action restored on drop.
    previous_continue_action: libc::sigaction,
    /// Helper thread that bridges signal-pipe bytes into runtime readiness.
    helper_thread: Option<JoinHandle<()>>,
}

impl PosixProcessControlHost {
    /// Install the signal bridge and start the helper thread.
    pub(super) fn new(
        readiness_source: SystemReadinessSource,
    ) -> Result<Self, ProcessControlUnavailableError> {
        if PROCESS_CONTROL_BACKEND_ACTIVE.swap(true, Ordering::AcqRel) {
            return Err(ProcessControlUnavailableError::UnsupportedHost);
        }

        let (read_fd, write_fd) = create_signal_pipe().map_err(|_pipe_error| {
            PROCESS_CONTROL_BACKEND_ACTIVE.store(false, Ordering::Release);
            ProcessControlUnavailableError::UnsupportedHost
        })?;

        let state = Arc::new(Mutex::new(PosixProcessControlHostState::default()));
        let mut previous_suspend_action = zeroed_sigaction();
        let mut previous_continue_action = zeroed_sigaction();

        if install_signal_handler(libc::SIGTSTP, &mut previous_suspend_action).is_err() {
            close_fd(write_fd);
            close_fd(read_fd);
            PROCESS_CONTROL_BACKEND_ACTIVE.store(false, Ordering::Release);
            return Err(ProcessControlUnavailableError::UnsupportedHost);
        }

        if install_signal_handler(libc::SIGCONT, &mut previous_continue_action).is_err() {
            drop(restore_signal_handler(
                libc::SIGTSTP,
                &previous_suspend_action,
            ));
            close_fd(write_fd);
            close_fd(read_fd);
            PROCESS_CONTROL_BACKEND_ACTIVE.store(false, Ordering::Release);
            return Err(ProcessControlUnavailableError::UnsupportedHost);
        }

        PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD.store(write_fd, Ordering::Release);
        let helper_state = Arc::clone(&state);
        let helper_thread = thread::Builder::new()
            .name("opal-process-control".to_owned())
            .spawn(move || signal_forwarder_loop(read_fd, helper_state, readiness_source))
            .map_err(|_thread_error| {
                PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD.store(INVALID_PIPE_FD, Ordering::Release);
                drop(restore_signal_handler(
                    libc::SIGCONT,
                    &previous_continue_action,
                ));
                drop(restore_signal_handler(
                    libc::SIGTSTP,
                    &previous_suspend_action,
                ));
                close_fd(write_fd);
                close_fd(read_fd);
                PROCESS_CONTROL_BACKEND_ACTIVE.store(false, Ordering::Release);
                ProcessControlUnavailableError::UnsupportedHost
            })?;

        Ok(Self {
            state,
            read_fd,
            write_fd,
            previous_suspend_action,
            previous_continue_action,
            helper_thread: Some(helper_thread),
        })
    }
}

impl ProcessControlHost for PosixProcessControlHost {
    fn observe_notification(
        &mut self,
    ) -> Result<Option<ProcessControlHostObservation>, ProcessControlHostObservationError> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pending_observations
            .pop_front())
    }

    fn has_pending_observation(&self) -> bool {
        !self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pending_observations
            .is_empty()
    }

    fn acknowledge_suspend(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        // SAFETY: `getpid` reads the current process id without borrowing Rust data.
        let process_id = unsafe { libc::getpid() };
        // SAFETY: `process_id` names the current process and `SIGSTOP` is forwarded
        // directly to the host kernel to enter the stopped state.
        let result = unsafe { libc::kill(process_id, libc::SIGSTOP) };
        if result == 0_i32 {
            return Ok(());
        }
        Err(ProcessControlHostActionError::SuspendFailed)
    }

    fn resume_application(
        &mut self,
        _generation: u64,
    ) -> Result<(), ProcessControlHostActionError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.continued_epoch == state.consumed_continued_epoch {
            return Err(ProcessControlHostActionError::ResumeFailed);
        }
        state.consumed_continued_epoch = state.continued_epoch;
        drop(state);
        Ok(())
    }
}

impl Drop for PosixProcessControlHost {
    fn drop(&mut self) {
        drop(restore_signal_handler(
            libc::SIGCONT,
            &self.previous_continue_action,
        ));
        drop(restore_signal_handler(
            libc::SIGTSTP,
            &self.previous_suspend_action,
        ));
        PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD.store(INVALID_PIPE_FD, Ordering::Release);
        close_fd(self.write_fd);
        if let Some(helper_thread) = self.helper_thread.take() {
            drop(helper_thread.join());
        }
        close_fd(self.read_fd);
        PROCESS_CONTROL_BACKEND_ACTIVE.store(false, Ordering::Release);
    }
}

/// Async-signal-safe handler that forwards signal codes into the pipe.
extern "C" fn process_control_signal_handler(signal: libc::c_int) {
    let write_fd = PROCESS_CONTROL_SIGNAL_PIPE_WRITE_FD.load(Ordering::Relaxed);
    if write_fd == INVALID_PIPE_FD {
        return;
    }

    let signal_code = match signal {
        libc::SIGTSTP => SIGNAL_CODE_SUSPEND_REQUESTED,
        libc::SIGCONT => SIGNAL_CODE_CONTINUED,
        _ => return,
    };

    let buffer = [signal_code];
    // SAFETY: `write_fd` is a process-global pipe descriptor prepared during
    // backend installation, and writing one byte is async-signal-safe.
    let _write_result: isize = unsafe { libc::write(write_fd, buffer.as_ptr().cast(), 1) };
}

/// Convert the signal handler function into the libc handler representation.
#[expect(
    clippy::as_conversions,
    clippy::fn_to_numeric_cast_any,
    reason = "libc exposes sa_sigaction as an untyped handler slot that must be filled with a C function pointer cast"
)]
fn signal_handler_pointer() -> libc::sighandler_t {
    process_control_signal_handler as libc::sighandler_t
}

/// Drain the signal pipe on a helper thread and publish runtime readiness.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the helper thread owns the shared state and readiness clone for its full lifetime"
)]
fn signal_forwarder_loop(
    read_fd: RawFd,
    state: Arc<Mutex<PosixProcessControlHostState>>,
    readiness_source: SystemReadinessSource,
) {
    loop {
        let mut signal_code = 0_u8;
        // SAFETY: `read_fd` is owned by this backend and the one-byte buffer is valid.
        let result = unsafe { libc::read(read_fd, ptr::from_mut(&mut signal_code).cast(), 1) };
        if result == 0_isize {
            return;
        }
        if result < 0_isize {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return;
        }

        let observation = match signal_code {
            SIGNAL_CODE_SUSPEND_REQUESTED => ProcessControlHostObservation::SuspendRequested,
            SIGNAL_CODE_CONTINUED => ProcessControlHostObservation::Continued,
            _ => continue,
        };

        let mut state = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if observation == ProcessControlHostObservation::Continued {
            state.continued_epoch = state.continued_epoch.saturating_add(1);
        }
        state.pending_observations.push_back(observation);
        drop(state);
        let _published_generation = readiness_source.publish_transition(SourceAvailability::Ready);
    }
}

/// Create the pipe used to bridge signal handlers into safe Rust code.
fn create_signal_pipe() -> io::Result<(RawFd, RawFd)> {
    let mut pipe_fds = [0_i32; 2];
    // SAFETY: `pipe_fds` points to two writable integers for `libc::pipe`.
    let create_result = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    if create_result != 0_i32 {
        return Err(io::Error::last_os_error());
    }

    configure_close_on_exec(pipe_fds[0])?;
    configure_close_on_exec(pipe_fds[1])?;
    configure_nonblocking(pipe_fds[1])?;
    Ok(<(RawFd, RawFd)>::from(pipe_fds))
}

/// Enable close-on-exec on `fd`.
fn configure_close_on_exec(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is an open descriptor created by `pipe` for this backend.
    let current_flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if current_flags < 0_i32 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is valid and the new flag set only toggles `FD_CLOEXEC`.
    let set_result = unsafe { libc::fcntl(fd, libc::F_SETFD, current_flags | libc::FD_CLOEXEC) };
    if set_result == 0_i32 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Enable nonblocking writes on `fd`.
fn configure_nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fd` is an open descriptor created by `pipe` for this backend.
    let current_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if current_flags < 0_i32 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is valid and the new flag set only adds `O_NONBLOCK`.
    let set_result = unsafe { libc::fcntl(fd, libc::F_SETFL, current_flags | libc::O_NONBLOCK) };
    if set_result == 0_i32 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Install the shared signal handler for `signal`.
fn install_signal_handler(
    signal: libc::c_int,
    previous_action: &mut libc::sigaction,
) -> io::Result<()> {
    let mut action = zeroed_sigaction();
    action.sa_flags = libc::SA_RESTART;
    action.sa_sigaction = signal_handler_pointer();
    // SAFETY: `sa_mask` is a valid writable sigset inside `action`.
    let empty_mask_result = unsafe { libc::sigemptyset(&mut action.sa_mask) };
    if empty_mask_result != 0_i32 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `action` and `previous_action` point to valid initialized storage.
    let install_result = unsafe { libc::sigaction(signal, &action, previous_action) };
    if install_result == 0_i32 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Restore the prior handler for `signal`.
fn restore_signal_handler(
    signal: libc::c_int,
    previous_action: &libc::sigaction,
) -> io::Result<()> {
    // SAFETY: `previous_action` came from a successful earlier `sigaction` call.
    let restore_result = unsafe { libc::sigaction(signal, previous_action, ptr::null_mut()) };
    if restore_result == 0_i32 {
        return Ok(());
    }
    Err(io::Error::last_os_error())
}

/// Close `fd`, ignoring already-closed cleanup failures.
fn close_fd(fd: RawFd) {
    if fd == INVALID_PIPE_FD {
        return;
    }
    // SAFETY: closing an owned raw descriptor releases this backend resource.
    let _close_result: libc::c_int = unsafe { libc::close(fd) };
}

/// Create a zero-initialized `sigaction` suitable for immediate population.
const fn zeroed_sigaction() -> libc::sigaction {
    let action = MaybeUninit::<libc::sigaction>::zeroed();
    // SAFETY: `sigaction` is a plain C struct and zero is a valid baseline before
    // we populate the handler pointer, flags, and signal mask.
    unsafe { action.assume_init() }
}
