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

#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
/// Environment flag that turns the helper test into a subprocess probe.
const PROCESS_CONTROL_HELPER_ENV: &str = "OPAL_PROCESS_CONTROL_HELPER";
#[cfg(unix)]
/// Leaf unit-test filter used when spawning the helper subprocess.
const PROCESS_CONTROL_HELPER_TEST_NAME: &str = "production_posix_backend_child_helper";

/// Poll until a notification is returned.
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
        ProcessControlPollResult::Idle => unreachable!("queued notification should return first"),
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

#[cfg(unix)]
#[test]
fn production_posix_backend_installation_is_process_global_and_released_on_drop() {
    let _guard = super::production_backend_test_guard();
    let first_source = ProcessControlSource::new().expect("posix backend should install once");
    assert!(
        matches!(
            ProcessControlSource::new(),
            Err(ProcessControlUnavailableError::UnsupportedHost)
        ),
        "a second live source must fail while the process-global handlers are owned"
    );
    drop(first_source);
    let second_source = ProcessControlSource::new().expect("backend should reinstall after drop");
    drop(second_source);
}

#[cfg(unix)]
#[test]
fn production_posix_backend_suspend_and_continue_round_trip_in_child_process() {
    let _guard = super::production_backend_test_guard();
    let current_executable = env::current_exe().expect("current test executable should exist");
    let mut child = Command::new(current_executable)
        .arg(PROCESS_CONTROL_HELPER_TEST_NAME)
        .arg("--nocapture")
        .arg("--quiet")
        .env(PROCESS_CONTROL_HELPER_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("helper subprocess should start");

    let child_stdout = child.stdout.take().expect("helper stdout should be piped");
    let mut stdout_reader = BufReader::new(child_stdout);
    let mut child_stdin = child.stdin.take().expect("helper stdin should be piped");

    assert_eq!(
        read_helper_line(&mut stdout_reader),
        "READY",
        "helper should report backend initialization before sending signals"
    );
    assert_eq!(
        read_helper_line(&mut stdout_reader),
        "SUSPEND 1",
        "helper should observe the caught suspend request"
    );

    child_stdin
        .write_all(b"ACK\n")
        .expect("ack command should reach helper");
    child_stdin.flush().expect("ack command should flush");
    wait_for_child_stop(child.id());

    send_signal(child.id(), libc::SIGCONT);
    assert_eq!(
        read_helper_line(&mut stdout_reader),
        "ACKED 1",
        "helper should finish acknowledgement only after host continuation"
    );
    assert_eq!(
        read_helper_line(&mut stdout_reader),
        "CONTINUED 1",
        "helper should observe continuation after the host resumes it"
    );

    child_stdin
        .write_all(b"RESUME\n")
        .expect("resume command should reach helper");
    child_stdin.flush().expect("resume command should flush");
    assert_eq!(
        read_helper_line(&mut stdout_reader),
        "RESUMED 1",
        "application resume should remain an explicit action"
    );

    let exit_status = child.wait().expect("helper should exit cleanly");
    assert!(
        exit_status.success(),
        "helper subprocess should exit successfully"
    );
}

#[cfg(unix)]
#[test]
fn production_posix_backend_child_helper() {
    if env::var_os(PROCESS_CONTROL_HELPER_ENV).is_none() {
        return;
    }

    let _guard = super::production_backend_test_guard();
    let mut source = ProcessControlSource::new().expect("posix backend should install in helper");
    println!("READY");
    std::io::stdout()
        .flush()
        .expect("helper stdout should flush");

    send_signal(std::process::id(), libc::SIGTSTP);
    let suspend_generation = wait_for_specific_notification(
        &mut source,
        ProcessControlNotification::SuspendRequested(1),
        "suspend request",
    );
    println!("SUSPEND {suspend_generation}");
    std::io::stdout()
        .flush()
        .expect("helper stdout should flush");

    let mut command = String::new();
    std::io::stdin()
        .read_line(&mut command)
        .expect("helper should read ack command");
    assert_eq!(
        command.trim_end(),
        "ACK",
        "helper should receive ACK command"
    );
    source
        .acknowledge_suspend(suspend_generation)
        .expect("acknowledgement should stop and later resume the helper");
    println!("ACKED {suspend_generation}");
    std::io::stdout()
        .flush()
        .expect("helper stdout should flush");

    let continued_generation = wait_for_specific_notification(
        &mut source,
        ProcessControlNotification::Continued(suspend_generation),
        "continued notification",
    );
    println!("CONTINUED {continued_generation}");
    std::io::stdout()
        .flush()
        .expect("helper stdout should flush");

    command.clear();
    std::io::stdin()
        .read_line(&mut command)
        .expect("helper should read resume command");
    assert_eq!(
        command.trim_end(),
        "RESUME",
        "helper should receive RESUME command"
    );
    source
        .resume_application(continued_generation)
        .expect("application resume should consume the continued host observation");
    println!("RESUMED {continued_generation}");
    std::io::stdout()
        .flush()
        .expect("helper stdout should flush");
}

#[cfg(unix)]
/// Read the next semantic helper output line, skipping libtest framing noise.
fn read_helper_line(reader: &mut BufReader<std::process::ChildStdout>) -> String {
    loop {
        let mut line = String::new();
        let bytes_read = reader
            .read_line(&mut line)
            .expect("helper output should stay readable");
        assert!(bytes_read > 0, "helper should emit another output line");
        let trimmed_line = line.trim_end();
        let first_word = trimmed_line.split_whitespace().next();
        if matches!(
            first_word,
            Some("READY" | "SUSPEND" | "ACKED" | "CONTINUED" | "RESUMED")
        ) {
            return trimmed_line.to_owned();
        }
    }
}

#[cfg(unix)]
/// Wait until the helper subprocess stops after acknowledgement.
fn wait_for_child_stop(child_process_id: u32) {
    let raw_child_process_id = i32::try_from(child_process_id)
        .expect("child process id should fit into libc waitpid parameters");
    let mut wait_status = 0_i32;
    // SAFETY: `raw_child_process_id` names a live subprocess created by this test,
    // and `wait_status` points to valid writable memory for `waitpid`.
    let wait_result =
        unsafe { libc::waitpid(raw_child_process_id, &mut wait_status, libc::WUNTRACED) };
    assert_eq!(
        wait_result, raw_child_process_id,
        "waitpid should report the helper subprocess stop"
    );
    assert!(
        libc::WIFSTOPPED(wait_status),
        "helper subprocess should enter the stopped state after acknowledgement"
    );
}

#[cfg(unix)]
/// Send `signal` to `process_id` and require success.
fn send_signal(process_id: u32, signal: libc::c_int) {
    let raw_process_id =
        i32::try_from(process_id).expect("process id should fit into libc kill parameters");
    // SAFETY: `raw_process_id` is a live process id created by this test.
    let send_result = unsafe { libc::kill(raw_process_id, signal) };
    assert_eq!(send_result, 0_i32, "signal delivery should succeed");
}

#[cfg(unix)]
/// Wait for a specific notification without hanging the helper forever.
fn wait_for_specific_notification(
    source: &mut ProcessControlSource,
    expected: ProcessControlNotification,
    context: &str,
) -> u64 {
    for _ in 0..100_u32 {
        match source.poll().expect("poll should succeed while waiting") {
            ProcessControlPollResult::Notification { notification } => {
                if notification == expected {
                    return match notification {
                        ProcessControlNotification::SuspendRequested(generation)
                        | ProcessControlNotification::Continued(generation) => generation,
                    };
                }
            }
            ProcessControlPollResult::Idle => thread::sleep(Duration::from_millis(10)),
        }
    }
    panic!("timed out waiting for {context}");
}
