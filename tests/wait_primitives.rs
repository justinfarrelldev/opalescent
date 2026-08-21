#[cfg(test)]
mod tests {
    use opalescent::runtime::{
        CancellationSource, MonotonicTimer, SystemReadyWakeStatus, SystemWaitSet, SystemWaitWake,
        monotonic_clock_now,
    };

    #[test]
    fn cancellation_only_wait_returns_cancelled_without_source_or_generation() {
        let mut cancellation = CancellationSource::new();
        let token = cancellation.token();
        cancellation.request();
        let mut wait_set = SystemWaitSet::new();

        let wake = wait_set
            .wait_sync(&token)
            .expect("cancelled token should wake an empty wait set");

        assert_eq!(wake, SystemWaitWake::Cancelled);
        assert_eq!(wake.ready_source(), None);
        assert_eq!(wake.ready_generation(), None);
    }

    #[test]
    fn monotonic_timer_ready_wake_uses_public_source_identity() {
        let mut timer = MonotonicTimer::new();
        let source = timer.readiness_source();
        let mut wait_set = SystemWaitSet::new();
        let _registration = wait_set
            .register(&source)
            .expect("timer readiness registration should succeed");
        let cancellation = CancellationSource::new();
        let token = cancellation.token();

        let generation = timer
            .arm(monotonic_clock_now())
            .expect("arming at now should succeed");
        let wake = wait_set
            .wait_sync(&token)
            .expect("timer ready wake should be observable");

        assert_eq!(wake.ready_generation(), Some(generation));
        assert_eq!(
            wake.readiness_against(&source),
            SystemReadyWakeStatus::Current
        );
    }
}
