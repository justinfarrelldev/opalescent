#[cfg(test)]
mod tests {
    use opalescent::runtime::{CancellationSource, SystemWaitSet, SystemWaitWake};

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
}
