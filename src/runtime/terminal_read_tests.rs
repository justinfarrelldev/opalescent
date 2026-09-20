use crate::runtime::terminal::{
    TerminalCloseOutcome, TerminalInputEventKind, TerminalInputResetReason, TerminalReadEventError,
    TerminalSessionReadError, TerminalSessionState, TerminalWait,
};
use crate::runtime::wait::CancellationSource;
use crate::stdlib::terminal::{
    terminal_pause_events_at, terminal_pause_events_length, terminal_session_open_sync,
    terminal_session_options_default, terminal_session_pause_sync,
    terminal_session_read_event_sync, terminal_session_readiness_source, terminal_session_state,
};

#[test]
fn terminal_session_read_returns_one_event_then_stale_poll_timeout_with_stable_source() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    let source = terminal_session_readiness_source(&session);
    let source_again = terminal_session_readiness_source(&session);
    assert!(source.is_same_identity(&source_again));

    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusGained)
        .expect("test event enqueues");
    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusLost)
        .expect("test event enqueues");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    let first = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("first event reads");
    assert!(matches!(first.kind(), &TerminalInputEventKind::FocusGained));
    let second = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("second event reads");
    assert!(matches!(second.kind(), &TerminalInputEventKind::FocusLost));
    let drained = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("drained poll returns stale-readiness timeout event");
    assert!(matches!(drained.kind(), &TerminalInputEventKind::TimedOut));
}

#[test]
fn terminal_session_read_prioritizes_queue_then_cancellation_and_sticky_eof() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusGained)
        .expect("test event enqueues");
    session.mark_end_of_input_for_tests();
    let mut cancellation = CancellationSource::new();
    let token = cancellation.token();
    cancellation.request();

    let queued = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("queued event wins");
    assert!(matches!(
        queued.kind(),
        &TerminalInputEventKind::FocusGained
    ));
    let eof = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("sticky eof wins before cancellation");
    assert!(matches!(eof.kind(), &TerminalInputEventKind::EndOfInput));
    let eof_again = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect("sticky eof repeats");
    assert!(matches!(
        eof_again.kind(),
        &TerminalInputEventKind::EndOfInput
    ));

    let mut cancellable = terminal_session_open_sync(&options).expect("open should succeed");
    cancellable
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusLost)
        .expect("test event enqueues");
    let mut cancellation_source = CancellationSource::new();
    let cancellation_token = cancellation_source.token();
    cancellation_source.request();
    let queued_before_cancel =
        terminal_session_read_event_sync(&mut cancellable, TerminalWait::Poll, &cancellation_token)
            .expect("queued event wins before cancellation");
    assert!(matches!(
        queued_before_cancel.kind(),
        &TerminalInputEventKind::FocusLost
    ));
    let cancelled =
        terminal_session_read_event_sync(&mut cancellable, TerminalWait::Poll, &cancellation_token)
            .expect("cancellation wins after queue drains");
    assert!(matches!(
        cancelled.kind(),
        &TerminalInputEventKind::Cancelled
    ));
}

#[test]
fn terminal_session_pause_delivers_retained_events_and_boundary_then_close_discards() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusGained)
        .expect("test event enqueues");
    let result = terminal_session_pause_sync(&mut session).expect("pause succeeds");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Paused
    );
    assert_eq!(terminal_pause_events_length(&result.events), 2);
    let boundary = terminal_pause_events_at(&result.events, 1).expect("boundary present");
    assert!(matches!(
        boundary.kind(),
        &TerminalInputEventKind::InputReset {
            reason: TerminalInputResetReason::PauseBoundary
        }
    ));
    let empty = terminal_session_pause_sync(&mut session).expect("paused pause is empty");
    assert_eq!(terminal_pause_events_length(&empty.events), 0);

    session
        .resume_sync()
        .expect("resume for close-discard test");
    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusLost)
        .expect("test event enqueues");
    let close = session.close_sync().expect("close succeeds");
    assert!(matches!(
        close,
        TerminalCloseOutcome::DiscardedInput {
            discarded_events: 1,
            ..
        }
    ));
}

#[test]
fn terminal_session_read_state_rejections_and_identifier_exhaustion_are_non_mutating() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();
    session.pause_sync().expect("pause succeeds");
    let paused = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect_err("paused read is state error");
    assert_eq!(paused.state(), TerminalSessionState::Paused);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Paused
    );

    session.resume_sync().expect("resume succeeds");
    session.force_next_delivery_ordinal_for_tests(u64::MAX);
    let exhausted = session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusGained)
        .expect_err("ordinal exhaustion fails before publication");
    assert!(matches!(
        exhausted,
        TerminalSessionReadError::IdentifierExhausted { .. }
    ));
    let read_exhausted = terminal_session_read_event_sync(&mut session, TerminalWait::Poll, &token)
        .expect_err("sticky identifier exhaustion fails reads");
    assert!(matches!(
        read_exhausted,
        TerminalReadEventError::Read(TerminalSessionReadError::IdentifierExhausted { .. })
    ));
}
