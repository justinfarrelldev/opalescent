#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::process::{ChildStdout, Command, Stdio};

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(unix)]
const PROCESS_CONTROL_HELPER_SOURCE: &str = r#"
#include "runtime/opal_portability.h"
#include "runtime/opal_runtime.h"
#include <inttypes.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum {
    HELPER_PROCESS_CONTROL_NOTIFICATION_SUSPEND_REQUESTED = 0,
    HELPER_PROCESS_CONTROL_NOTIFICATION_CONTINUED = 1,
};

enum {
    HELPER_PROCESS_CONTROL_POLL_NOTIFICATION = 0,
    HELPER_PROCESS_CONTROL_POLL_IDLE = 1,
};

typedef struct {
    int64_t tag;
    uint8_t payload[64];
} HelperProcessControlNotificationValue;

typedef struct {
    int64_t tag;
    uint8_t payload[64];
} HelperProcessControlPollResultValue;

static uint64_t helper_notification_generation(
    const HelperProcessControlNotificationValue* notification
) {
    uint64_t generation = 0;
    memcpy(&generation, notification->payload, sizeof(generation));
    return generation;
}

static void helper_free_poll_result(HelperProcessControlPollResultValue* result) {
    if (result != NULL && result->tag == HELPER_PROCESS_CONTROL_POLL_NOTIFICATION) {
        HelperProcessControlNotificationValue* notification = NULL;
        memcpy(&notification, result->payload, sizeof(notification));
        free(notification);
    }
    free(result);
}

static uint64_t wait_for_notification(void* source, int64_t expected_tag, const char* context) {
    for (int attempt = 0; attempt < 100; ++attempt) {
        FsHandleResult poll = process_control_poll(source);
        if (poll.error != NULL) {
            fprintf(stderr, "poll for %s failed: %s\n", context, poll.error);
            exit(1);
        }

        HelperProcessControlPollResultValue* result =
            (HelperProcessControlPollResultValue*)poll.value;
        if (result == NULL) {
            fprintf(stderr, "poll for %s returned NULL\n", context);
            exit(1);
        }

        if (result->tag == HELPER_PROCESS_CONTROL_POLL_IDLE) {
            helper_free_poll_result(result);
            opal_sleep_ms(10);
            continue;
        }

        if (result->tag != HELPER_PROCESS_CONTROL_POLL_NOTIFICATION) {
            fprintf(stderr, "poll for %s returned unknown tag=%" PRId64 "\n", context, result->tag);
            helper_free_poll_result(result);
            exit(1);
        }

        HelperProcessControlNotificationValue* notification = NULL;
        memcpy(&notification, result->payload, sizeof(notification));
        if (notification != NULL && notification->tag == expected_tag) {
            uint64_t generation = helper_notification_generation(notification);
            helper_free_poll_result(result);
            return generation;
        }

        helper_free_poll_result(result);
    }

    fprintf(stderr, "timed out waiting for %s\n", context);
    exit(1);
}

int main(void) {
    FsHandleResult source_result = process_control_source_new();
    if (source_result.error != NULL) {
        fprintf(stderr, "process_control_source_new failed: %s\n", source_result.error);
        return 1;
    }

    void* source = source_result.value;
    printf("READY\n");
    fflush(stdout);

    if (kill(getpid(), SIGTSTP) != 0) {
        perror("kill(SIGTSTP)");
        process_control_source_drop(source);
        return 1;
    }

    uint64_t suspend_generation = wait_for_notification(
        source,
        HELPER_PROCESS_CONTROL_NOTIFICATION_SUSPEND_REQUESTED,
        "suspend request"
    );
    printf("SUSPEND %" PRIu64 "\n", suspend_generation);
    fflush(stdout);

    char command[32] = {0};
    if (fgets(command, (int)sizeof(command), stdin) == NULL) {
        fprintf(stderr, "helper should read ACK command\n");
        process_control_source_drop(source);
        return 1;
    }
    if (strncmp(command, "ACK", 3) != 0) {
        fprintf(stderr, "unexpected command before acknowledgement: %s\n", command);
        process_control_source_drop(source);
        return 1;
    }

    FsVoidResult acknowledgement = process_control_acknowledge_suspend(source, suspend_generation);
    if (acknowledgement.error != NULL) {
        fprintf(stderr, "acknowledgement failed: %s\n", acknowledgement.error);
        process_control_source_drop(source);
        return 1;
    }
    printf("ACKED %" PRIu64 "\n", suspend_generation);
    fflush(stdout);

    uint64_t continued_generation = wait_for_notification(
        source,
        HELPER_PROCESS_CONTROL_NOTIFICATION_CONTINUED,
        "continued notification"
    );
    printf("CONTINUED %" PRIu64 "\n", continued_generation);
    fflush(stdout);

    memset(command, 0, sizeof(command));
    if (fgets(command, (int)sizeof(command), stdin) == NULL) {
        fprintf(stderr, "helper should read RESUME command\n");
        process_control_source_drop(source);
        return 1;
    }
    if (strncmp(command, "RESUME", 6) != 0) {
        fprintf(stderr, "unexpected command before resume: %s\n", command);
        process_control_source_drop(source);
        return 1;
    }

    FsVoidResult resume = process_control_resume_application(source, continued_generation);
    if (resume.error != NULL) {
        fprintf(stderr, "resume failed: %s\n", resume.error);
        process_control_source_drop(source);
        return 1;
    }
    printf("RESUMED %" PRIu64 "\n", continued_generation);
    fflush(stdout);

    process_control_source_drop(source);
    return 0;
}
"#;

fn compile_and_run_core_prerequisite_smoke(temp_dir: &Path) -> Result<(), String> {
    let source_path = Path::new("test-projects/terminal-core-prerequisites/src/main.op");
    let source = "
import system_wait_set_new, system_wait_set_register, system_wait_set_remove, system_wait_set_register_owned, system_owned_wait_registration_retarget, system_owned_wait_registration_remove, system_wait_set_wait_sync, cancellation_source_new, cancellation_token, cancellation_request, monotonic_timer_new, monotonic_timer_readiness_source, monotonic_timer_arm, monotonic_timer_disarm, monotonic_timer_generation, monotonic_timer_deadline, monotonic_clock_now, process_control_source_new, process_control_readiness_source, process_control_poll from 'standard.system'
import print from standard

##
    Description: Generated runtime smoke test for prerequisite-ready terminal core APIs
##
entry main = f(): void errors AllocationFailureError, SystemWaitSetError, MonotonicTimerError, MonotonicTimerNotArmedError, ProcessControlUnavailableError, ProcessControlError =>
    let mutable wait_set = propagate system_wait_set_new()
    let mutable cancellation_source = propagate cancellation_source_new()
    let token = cancellation_token(ref cancellation_source)
    let mutable timer = propagate monotonic_timer_new()
    let deadline = monotonic_clock_now()
    let timer_source = monotonic_timer_readiness_source(ref timer)
    let registration = propagate system_wait_set_register(mutable ref wait_set, timer_source)
    propagate system_wait_set_remove(mutable ref wait_set, registration)
    let mutable owned_registration = propagate system_wait_set_register_owned(mutable ref wait_set, timer_source)
    let arm_generation = propagate monotonic_timer_arm(mutable ref timer, deadline)
    let current_generation = monotonic_timer_generation(ref timer)
    let armed_deadline = propagate monotonic_timer_deadline(ref timer)
    propagate system_owned_wait_registration_retarget(mutable ref owned_registration, timer_source)
    cancellation_request(mutable ref cancellation_source)
    let wake = propagate system_wait_set_wait_sync(mutable ref wait_set, token)
    let mutable process_source = propagate process_control_source_new()
    let process_readiness = process_control_readiness_source(ref process_source)
    let poll_result = propagate process_control_poll(mutable ref process_source)
    propagate system_owned_wait_registration_remove(mutable ref owned_registration)
    let disarm_generation = propagate monotonic_timer_disarm(mutable ref timer)
    print('CORE_READY arm={arm_generation} current={current_generation} disarm={disarm_generation}')
    print('CORE_READY_DONE')
    return void
";

    let binary_path =
        compile_program_for_tests(source_path, source, temp_dir, &TargetTriple::host()).map_err(
            |error| {
                format!(
                    "terminal core prerequisite smoke source should compile into a binary: {error}"
                )
            },
        )?;

    let run_output = run_binary_output_with_timeout(
        &binary_path,
        GENERATED_BINARY_TEST_TIMEOUT,
        "terminal core prerequisite compiled binary",
    )?;

    if !run_output.status.success() {
        return Err(format!(
            "terminal core prerequisite binary should exit successfully, status={:?}, stdout='{}', stderr='{}'",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stdout),
            String::from_utf8_lossy(&run_output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&run_output.stdout);
    if !stdout.contains("CORE_READY_DONE") {
        return Err(format!(
            "terminal core prerequisite binary should print completion marker, stdout='{}', stderr='{}'",
            stdout,
            String::from_utf8_lossy(&run_output.stderr)
        ));
    }
    if stdout.contains("runtime lowering") {
        return Err(format!(
            "terminal core prerequisite binary should not hit runtime-readiness diagnostics, stdout='{}', stderr='{}'",
            stdout,
            String::from_utf8_lossy(&run_output.stderr)
        ));
    }

    Ok(())
}

#[cfg(unix)]
fn build_process_control_helper(temp_dir: &Path) -> Result<PathBuf, String> {
    let helper_c = temp_dir.join("process_control_helper.c");
    let helper_bin = temp_dir.join("process_control_helper");

    fs::write(&helper_c, PROCESS_CONTROL_HELPER_SOURCE)
        .map_err(|error| format!("process-control helper source should be written: {error}"))?;

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_error.c")
        .arg("runtime/opal_system.c")
        .arg(&helper_c)
        .arg("-o")
        .arg(&helper_bin);
    let compile = run_command_output_with_timeout(
        &mut compile_command,
        Duration::from_secs(10),
        "process-control helper compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "process-control helper compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(helper_bin)
}

#[cfg(unix)]
fn read_helper_line(reader: &mut BufReader<ChildStdout>) -> Result<String, String> {
    let mut line = String::new();
    let bytes_read = reader
        .read_line(&mut line)
        .map_err(|error| format!("helper output should stay readable: {error}"))?;
    if bytes_read == 0 {
        return Err(String::from(
            "helper should emit another output line before the process exits",
        ));
    }
    Ok(line.trim_end().to_owned())
}

#[cfg(unix)]
fn wait_for_child_stop(child_process_id: u32) -> Result<(), String> {
    let raw_child_process_id = i32::try_from(child_process_id)
        .map_err(|error| format!("child process id should fit into waitpid parameters: {error}"))?;
    let mut wait_status = 0_i32;
    // SAFETY: `raw_child_process_id` names the live helper subprocess created by this test,
    // and `wait_status` points to valid writable storage for `waitpid`.
    let wait_result =
        unsafe { libc::waitpid(raw_child_process_id, &mut wait_status, libc::WUNTRACED) };
    if wait_result != raw_child_process_id {
        return Err(format!(
            "waitpid should report the helper subprocess stop, got {wait_result} for pid {raw_child_process_id}"
        ));
    }
    if !libc::WIFSTOPPED(wait_status) {
        return Err(String::from(
            "helper subprocess should enter the stopped state after acknowledgement",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn send_signal(process_id: u32, signal: libc::c_int) -> Result<(), String> {
    let raw_process_id = i32::try_from(process_id)
        .map_err(|error| format!("process id should fit into kill parameters: {error}"))?;
    // SAFETY: `raw_process_id` is the live helper subprocess created by this test.
    let send_result = unsafe { libc::kill(raw_process_id, signal) };
    if send_result != 0_i32 {
        return Err(format!(
            "signal delivery should succeed for pid {raw_process_id} and signal {signal}, got {send_result}"
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn run_process_control_round_trip(temp_dir: &Path) -> Result<(), String> {
    let helper_bin = build_process_control_helper(temp_dir)?;
    let mut child = Command::new(&helper_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("process-control helper subprocess should start: {error}"))?;

    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| String::from("helper stdout should be piped"))?;
    let mut stdout_reader = BufReader::new(child_stdout);
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| String::from("helper stdin should be piped"))?;

    let ready_line = read_helper_line(&mut stdout_reader)?;
    if ready_line != "READY" {
        return Err(format!(
            "helper should report backend initialization before signals, got '{ready_line}'"
        ));
    }

    let suspend_line = read_helper_line(&mut stdout_reader)?;
    if suspend_line != "SUSPEND 1" {
        return Err(format!(
            "helper should observe the caught suspend request, got '{suspend_line}'"
        ));
    }

    child_stdin
        .write_all(b"ACK\n")
        .map_err(|error| format!("ack command should reach helper: {error}"))?;
    child_stdin
        .flush()
        .map_err(|error| format!("ack command should flush: {error}"))?;
    wait_for_child_stop(child.id())?;
    send_signal(child.id(), libc::SIGCONT)?;

    let acked_line = read_helper_line(&mut stdout_reader)?;
    if acked_line != "ACKED 1" {
        return Err(format!(
            "helper should finish acknowledgement only after host continuation, got '{acked_line}'"
        ));
    }

    let continued_line = read_helper_line(&mut stdout_reader)?;
    if continued_line != "CONTINUED 1" {
        return Err(format!(
            "helper should observe continuation before explicit resume, got '{continued_line}'"
        ));
    }

    child_stdin
        .write_all(b"RESUME\n")
        .map_err(|error| format!("resume command should reach helper: {error}"))?;
    child_stdin
        .flush()
        .map_err(|error| format!("resume command should flush: {error}"))?;

    let resumed_line = read_helper_line(&mut stdout_reader)?;
    if resumed_line != "RESUMED 1" {
        return Err(format!(
            "application resume should remain explicit after continuation, got '{resumed_line}'"
        ));
    }

    let exit_status = child
        .wait()
        .map_err(|error| format!("helper subprocess should exit cleanly: {error}"))?;
    if !exit_status.success() {
        return Err(format!(
            "helper subprocess should exit successfully, status={:?}",
            exit_status.code()
        ));
    }

    Ok(())
}

#[test]
fn terminal_core_prerequisites_generated_runtime_compile_and_run() {
    let temp_dir = unique_probe_target_dir("terminal-core-prerequisites");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-core-prerequisites target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        compile_and_run_core_prerequisite_smoke(&temp_dir)?;
        #[cfg(unix)]
        {
            run_process_control_round_trip(&temp_dir)?;
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-core-prerequisites target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal core prerequisite generated runtime smoke should compile, run, and complete the helper-child suspend/continue round trip: {failure_message}"
    );
}
