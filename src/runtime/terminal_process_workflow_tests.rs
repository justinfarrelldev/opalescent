use crate::runtime::process_control::{
    ProcessControlSource, TestProcessControlHost, TestProcessControlHostObservation,
};
use crate::runtime::terminal::{
    TerminalProcessWorkflowError, TerminalProcessWorkflowStep, TerminalSessionState,
    terminal_process_suspend_continue_sync,
};
use crate::stdlib::terminal::{terminal_session_open_sync, terminal_session_options_default};
use core::sync::atomic::AtomicUsize;

static UNSUPPORTED_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

fn process_with_host(host: &TestProcessControlHost) -> ProcessControlSource {
    static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
    ProcessControlSource::new_with_test_host(host.clone(), &ALLOCATIONS)
        .expect("test host supported")
}

#[test]
fn terminal_process_suspend_continue_workflow_order_is_exact() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    let host = TestProcessControlHost::supported();
    host.push_observation(TestProcessControlHostObservation::SuspendRequested);
    host.push_observation(TestProcessControlHostObservation::Continued);
    let mut process = process_with_host(&host);

    let steps = terminal_process_suspend_continue_sync(&mut session, &mut process)
        .expect("workflow succeeds");
    assert_eq!(
        steps,
        vec![
            TerminalProcessWorkflowStep::StopApplicationWork,
            TerminalProcessWorkflowStep::PauseTerminal,
            TerminalProcessWorkflowStep::AcknowledgeSuspend,
            TerminalProcessWorkflowStep::ObserveContinued,
            TerminalProcessWorkflowStep::ResumeTerminal,
            TerminalProcessWorkflowStep::ResumeApplication,
            TerminalProcessWorkflowStep::RedrawAndResumeWork,
        ]
    );
    assert_eq!(session.state(), TerminalSessionState::Active);
}

#[test]
fn terminal_process_workflow_rejects_out_of_order_initial_continue() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    let host = TestProcessControlHost::supported();
    host.push_observation(TestProcessControlHostObservation::Continued);
    let mut process = process_with_host(&host);

    let error = terminal_process_suspend_continue_sync(&mut session, &mut process)
        .expect_err("continued before suspend is rejected");
    assert!(matches!(
        error,
        TerminalProcessWorkflowError::ExpectedSuspend | TerminalProcessWorkflowError::Idle
    ));
    assert_eq!(session.state(), TerminalSessionState::Active);
}

#[test]
fn terminal_process_workflow_failures_keep_work_stopped_before_later_steps() {
    let options = terminal_session_options_default();
    let mut paused_session = terminal_session_open_sync(&options).expect("open succeeds");
    paused_session.force_restore_pending_for_tests();
    let pause_host = TestProcessControlHost::supported();
    pause_host.push_observation(TestProcessControlHostObservation::SuspendRequested);
    let mut pause_process = process_with_host(&pause_host);
    let pause_error =
        terminal_process_suspend_continue_sync(&mut paused_session, &mut pause_process)
            .expect_err("pause failure propagates");
    assert!(matches!(
        pause_error,
        TerminalProcessWorkflowError::Pause(_)
    ));
    assert_eq!(paused_session.state(), TerminalSessionState::RestorePending);

    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    let ack_host = TestProcessControlHost::supported();
    ack_host.push_observation(TestProcessControlHostObservation::SuspendRequested);
    ack_host.fail_next_suspend_acknowledgement();
    let mut ack_process = process_with_host(&ack_host);
    let ack_error = terminal_process_suspend_continue_sync(&mut session, &mut ack_process)
        .expect_err("ack failure propagates");
    assert!(matches!(
        ack_error,
        TerminalProcessWorkflowError::Acknowledge(_)
    ));
    assert_eq!(session.state(), TerminalSessionState::Paused);
}

#[test]
fn terminal_process_workflow_reports_unsupported_windows_source_contract() {
    let unsupported = TestProcessControlHost::unsupported();
    assert!(
        ProcessControlSource::new_with_test_host(unsupported, &UNSUPPORTED_ALLOCATIONS).is_err()
    );
}
