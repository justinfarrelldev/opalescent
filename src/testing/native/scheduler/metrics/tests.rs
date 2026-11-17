//! Callback metrics cannot claim reordered, duplicate or changed completions.
use super::{CallbackEvent, accept};

/// A minimal real-source callback event shape with an independent invocation index.
fn event(stage: &str, index: u64, status: i32, passed: bool) -> CallbackEvent {
    CallbackEvent {
        stage: stage.to_owned(),
        index,
        phase: 4,
        world: String::new(),
        file: "tests/case.test.op".to_owned(),
        offset: 10,
        length: 20,
        status,
        passed,
        duration_ms: if stage == "start" { 0 } else { 9 },
    }
}

#[test]
fn only_matching_ordered_boundaries_publish_completed_duration() {
    let mut metrics = Vec::new();
    assert!(!accept(&mut metrics, event("finish", 1, 0, true)));
    assert!(accept(&mut metrics, event("start", 1, -1, false)));
    assert!(metrics[0].duration_ms.is_none());
    assert!(!accept(&mut metrics, event("start", 2, -1, false)));
    let mut changed = event("finish", 1, 0, true);
    changed.offset = 11;
    assert!(!accept(&mut metrics, changed));
    assert!(accept(&mut metrics, event("finish", 1, 0, true)));
    assert_eq!(metrics[0].duration_ms, Some(9_u64));
    assert_eq!(metrics[0].outcome, "passed");
    assert!(!accept(&mut metrics, event("finish", 1, 0, true)));
    assert!(!accept(&mut metrics, event("start", 1, -1, false)));
    assert!(accept(&mut metrics, event("start", 2, -1, false)));
    assert!(accept(&mut metrics, event("finish", 2, 1, false)));
    assert_eq!(metrics[1].outcome, "application_error");
}
