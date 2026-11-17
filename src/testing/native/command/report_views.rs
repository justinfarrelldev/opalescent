//! Derived association summaries never multiply the canonical overall case totals.
extern crate alloc;
use super::{
    CaseRecord, CheckedNativeProject, FunctionCaseSummary, NativeCommandError, Report,
    WorldAttemptMetrics, WorldCaseRequest, WorldView,
};
use crate::testing::native::identity::CaseIdentity;
use alloc::collections::BTreeSet;
use core::fmt::Write;

/// Derive World metrics from actual callbacks, never associated process duration.
pub(super) fn populate(
    worlds: &mut [WorldView],
    records: &[CaseRecord],
    checked: &CheckedNativeProject,
) -> Result<(), NativeCommandError> {
    for world in worlds {
        world.cleanup_declared = checked
            .worlds
            .declarations
            .get(&world.identity)
            .is_some_and(|definition| definition.declaration.cleanup.is_some());
        let owner = serde_json::to_string(&world.identity).map_err(NativeCommandError::report)?;
        for record in records
            .iter()
            .filter(|record| world.cases.contains(&record.identity))
        {
            if let Some(request) = record
                .worlds
                .iter()
                .find(|request| request.identity == world.identity)
            {
                world.requests.push(WorldCaseRequest {
                    case: record.identity.clone(),
                    origins: request.origins.clone(),
                });
            }
            if let Some(ref execution) = record.execution {
                for attempt in &execution.attempts {
                    let find = |phase| {
                        attempt
                            .callbacks
                            .iter()
                            .find(|callback| callback.phase == phase && callback.world == owner)
                            .cloned()
                    };
                    world.callback_metrics.push(WorldAttemptMetrics {
                        case: record.identity.clone(),
                        attempt: attempt.attempt,
                        setup: find(2_u32),
                        cleanup: find(6_u32),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Function views retain outcomes/phases without claiming those callbacks ran inside a subject.
pub(super) fn function_cases(
    records: &[CaseRecord],
    cases: &BTreeSet<CaseIdentity>,
) -> Vec<FunctionCaseSummary> {
    records
        .iter()
        .filter(|record| cases.contains(&record.identity))
        .map(|record| {
            let execution = record.execution.as_ref();
            let outcome = execution.map_or_else(
                || {
                    if record.selected {
                        "not_run"
                    } else {
                        "skipped"
                    }
                },
                |value| value.outcome.label(),
            );
            let mut phases = BTreeSet::new();
            let mut duration = 0_u64;
            if let Some(execution) = execution {
                for attempt in &execution.attempts {
                    duration = duration.saturating_add(attempt.duration_ms);
                    if let Some(ref result) = attempt.result {
                        phases.extend(result.failures.iter().map(|failure| failure.phase));
                    } else if let Some(phase) = attempt.last_phase {
                        phases.insert(phase);
                    }
                }
            }
            FunctionCaseSummary {
                case: record.identity.clone(),
                outcome: outcome.to_owned(),
                failure_phases: phases,
                associated_test_duration_ms: duration,
            }
        })
        .collect()
}

/// Text exposes the same association views and explicitly labeled duration semantics.
pub(super) fn text(output: &mut String, report: &Report) -> Result<(), NativeCommandError> {
    for function in &report.by_function {
        writeln!(output,"function {}::{}: {} associated cases, {}ms associated test duration (not function execution time)",function.identity.0.module,function.identity.0.name,function.cases.len(),function.associated_test_duration_ms).map_err(NativeCommandError::report)?;
        for case in &function.outcomes {
            writeln!(
                output,
                "  {}: {} phases {}",
                super::display(&case.case),
                case.outcome,
                serde_json::to_string(&case.failure_phases).map_err(NativeCommandError::report)?
            )
            .map_err(NativeCommandError::report)?;
        }
    }
    for world in &report.by_world {
        writeln!(
            output,
            "world {}::{}: {} associated cases; cleanup {}",
            world.identity.0.module,
            world.identity.0.name,
            world.cases.len(),
            if world.cleanup_declared {
                "declared"
            } else {
                "omitted"
            }
        )
        .map_err(NativeCommandError::report)?;
        for request in &world.requests {
            writeln!(
                output,
                "  {} origins {}",
                super::display(&request.case),
                serde_json::to_string(&request.origins).map_err(NativeCommandError::report)?
            )
            .map_err(NativeCommandError::report)?;
        }
        for metric in &world.callback_metrics {
            for (phase, callback) in [
                ("setup", metric.setup.as_ref()),
                ("cleanup", metric.cleanup.as_ref()),
            ] {
                if let Some(callback) = callback {
                    writeln!(
                        output,
                        "  {} attempt {} {phase}: {} duration_ms={}",
                        super::display(&metric.case),
                        metric.attempt,
                        callback.outcome,
                        callback
                            .duration_ms
                            .map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
                    )
                    .map_err(NativeCommandError::report)?;
                }
            }
        }
    }
    Ok(())
}
