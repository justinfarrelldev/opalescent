#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn terminal_core_prerequisites_generated_runtime_compile_and_run() {
    let temp_dir = unique_probe_target_dir("terminal-core-prerequisites");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-core-prerequisites target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-core-prerequisites/src/main.op");
        let source = "
import system_wait_set_new, system_wait_set_register, system_wait_set_remove, system_wait_set_register_owned, system_owned_wait_registration_retarget, system_owned_wait_registration_remove, system_wait_set_wait_sync, cancellation_source_new, cancellation_token, cancellation_request, monotonic_timer_new, monotonic_timer_readiness_source, monotonic_timer_arm, monotonic_timer_disarm, monotonic_timer_generation, monotonic_timer_deadline, monotonic_clock_now, process_control_source_new, process_control_readiness_source, process_control_poll, process_control_acknowledge_suspend, process_control_resume_application from 'standard.system'
import print from standard

##
    Description: Generated runtime smoke test for prerequisite-ready terminal core APIs
##
entry main = f(): void errors AllocationFailureError, SystemWaitSetError, MonotonicTimerError, MonotonicTimerNotArmedError, ProcessControlUnavailableError, ProcessControlError, ProcessControlAcknowledgementError, ProcessControlResumeError =>
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
    propagate process_control_acknowledge_suspend(mutable ref process_source, 0)
    propagate process_control_resume_application(mutable ref process_source, 0)
    propagate system_owned_wait_registration_remove(mutable ref owned_registration)
    let disarm_generation = propagate monotonic_timer_disarm(mutable ref timer)
    print('CORE_READY arm={arm_generation} current={current_generation} disarm={disarm_generation}')
    print('CORE_READY_DONE')
    return void
";

        let binary_path = compile_program_for_tests(
            source_path,
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal core prerequisite smoke source should compile into a binary: {error}")
        })?;

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
        "terminal core prerequisite generated runtime smoke should compile, run, and print completion markers: {failure_message}"
    );
}
