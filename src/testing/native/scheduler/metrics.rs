//! Validated real callback boundary records, independent of subject associations.
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod tests;

/// One source callback invocation, including incomplete workers' last entered callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackMetric {
    /// Invocation ordinal, independent within each fresh attempt.
    pub index: u64,
    /// Actual versioned lifecycle phase.
    pub phase: u32,
    /// Serialized canonical World identity, empty for hooks/body/examples.
    pub world: String,
    /// Original callback source file.
    pub file: String,
    /// Original source byte offset.
    pub offset: u64,
    /// Original source extent.
    pub length: u64,
    /// Boundary outcome, never inferred from resource existence.
    pub outcome: String,
    /// Actual monotonic callback duration; absent when no completion was observed.
    pub duration_ms: Option<u64>,
}

/// Exact worker event wire shape, checked before inclusion in reports.
#[derive(Deserialize)]
pub(super) struct CallbackEvent {
    /// Start or finish boundary.
    stage: String,
    /// Attempt-local invocation ordinal.
    index: u64,
    /// Actual lifecycle phase.
    phase: u32,
    /// Canonical owner identity.
    world: String,
    /// Source file.
    file: String,
    /// Source byte offset.
    offset: u64,
    /// Source extent length.
    length: u64,
    /// Independent callback status, not an application value.
    status: i32,
    /// Whether the boundary completed without a retained failure.
    passed: bool,
    /// Monotonic elapsed milliseconds.
    duration_ms: u64,
}

/// Reject duplicates, reordered completions or changed source/owner identities.
pub(super) fn accept(metrics: &mut Vec<CallbackMetric>, event: CallbackEvent) -> bool {
    if !(1..=6).contains(&event.phase) {
        return false;
    }
    if event.stage == "start" {
        if Some(event.index)
            != metrics
                .last()
                .map_or(Some(1), |last| last.index.checked_add(1))
            || metrics.last().is_some_and(|last| last.outcome == "running")
            || event.status != -1_i32
            || event.passed
            || event.duration_ms != 0
        {
            return false;
        }
        metrics.push(CallbackMetric {
            index: event.index,
            phase: event.phase,
            world: event.world,
            file: event.file,
            offset: event.offset,
            length: event.length,
            outcome: "running".to_owned(),
            duration_ms: None,
        });
        return true;
    }
    let Some(last) = metrics.last_mut() else {
        return false;
    };
    if event.stage != "finish"
        || last.outcome != "running"
        || last.index != event.index
        || last.phase != event.phase
        || last.world != event.world
        || last.file != event.file
        || last.offset != event.offset
        || last.length != event.length
        || !(0_i32..=3_i32).contains(&event.status)
        || (event.passed && event.status != 0_i32)
    {
        return false;
    }
    match (event.status, event.passed) {
        (0_i32, true) => "passed",
        (1_i32, _) => "application_error",
        (2_i32, _) => "aborted",
        _ => "infrastructure_failed",
    }
    .clone_into(&mut last.outcome);
    last.duration_ms = Some(event.duration_ms);
    true
}
