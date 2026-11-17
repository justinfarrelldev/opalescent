//! Behavior/function/World views over one canonical case record collection.
extern crate alloc;
#[path = "report_views.rs"]
mod views;
use super::{NativeCommandError, options::Reporter};
use crate::testing::native::cases::WorldUseOrigin;
use crate::testing::native::checking::CheckedNativeProject;
use crate::testing::native::identity::{CaseIdentity, FunctionIdentity, WorldIdentity};
use crate::testing::native::scheduler::{CaseExecution, CaseOutcome, SchedulerReport};
use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt::Write;
use serde::Serialize;

/// Canonical World membership preserves direct/hook/activation/transitive origins.
#[derive(Serialize)]
struct WorldRequirement {
    /// Canonical provider, not a source alias.
    identity: WorldIdentity,
    /// Distinct request origins, without creating additional instances.
    origins: Vec<WorldUseOrigin>,
}

/// One behavior record, independent of its several reporting associations.
#[derive(Serialize)]
struct CaseRecord {
    /// Stable deterministic exact-selection encoding.
    id: String,
    /// Full structured source identity.
    identity: CaseIdentity,
    /// Display prose, never executable instructions.
    display: String,
    /// Declared/inherited test category.
    category: String,
    /// Retained paragraph documentation.
    documentation: Vec<String>,
    /// Complete scheduler-only exclusion keys, not external service locks.
    exclusion_keys: Vec<String>,
    /// Canonical function associations, not execution coverage.
    subjects: Vec<FunctionIdentity>,
    /// Actual lifecycle membership and provenance.
    worlds: Vec<WorldRequirement>,
    /// Immutable double declarations in this artifact; actual activation is attempt data.
    declared_doubles: Vec<DoubleDeclarationRecord>,
    /// Whether source selection includes this case.
    selected: bool,
    /// Executed attempts, absent for listing or selection skips.
    execution: Option<CaseExecution>,
}

/// Canonical static double boundary, independent from whether a case activates it.
#[derive(Serialize)]
struct DoubleDeclarationRecord {
    /// Canonical declaration provider.
    identity: crate::testing::native::identity::DoubleIdentity,
    /// Canonical source import edge.
    edge: crate::testing::native::identity::ResolvedImportEdge,
}

/// Overall case totals never sum function/World association totals.
#[derive(Default, Serialize)]
struct Totals {
    /// Clean first-attempt passes.
    passed: usize,
    /// Success after retained failed attempts.
    flaky: usize,
    /// Failed selected cases.
    failed: usize,
    /// Cases excluded by source selection.
    skipped: usize,
    /// Selected cases displayed without execution.
    listed: usize,
}

/// A function view contains associated case durations, never function timing.
#[derive(Serialize)]
struct FunctionView {
    /// Genuine canonical callable identity.
    identity: FunctionIdentity,
    /// Distinct associated case identities.
    cases: BTreeSet<CaseIdentity>,
    /// Associated test process durations, not function execution time.
    associated_test_duration_ms: u64,
    /// Canonical case outcomes and failure phases under this association.
    outcomes: Vec<FunctionCaseSummary>,
}

/// Function association details reference cases and preserve failure attribution.
#[derive(Serialize)]
struct FunctionCaseSummary {
    /// Full independent case identity.
    case: CaseIdentity,
    /// Observed whole-case outcome, not a claim of function coverage.
    outcome: String,
    /// Actual failed/last-known lifecycle phases.
    failure_phases: BTreeSet<u32>,
    /// Associated process lifetimes, never function execution time.
    associated_test_duration_ms: u64,
}

/// A World view preserves lifecycle associations; success is not leak protection.
#[derive(Serialize)]
struct WorldView {
    /// Genuine canonical provider identity.
    identity: WorldIdentity,
    /// Distinct lifecycle-associated cases.
    cases: BTreeSet<CaseIdentity>,
    /// Original World documentation, not a claim about released resources.
    documentation: Option<String>,
    /// Whether user cleanup exists; omission is not a cleanup failure.
    cleanup_declared: bool,
    /// Per-case direct, hook, activation and transitive request provenance.
    requests: Vec<WorldCaseRequest>,
    /// Actual observed setup/cleanup outcomes and timings for each associated attempt.
    callback_metrics: Vec<WorldAttemptMetrics>,
}

/// Request provenance is a view over the canonical case's effective World graph.
#[derive(Serialize)]
struct WorldCaseRequest {
    /// Canonical independently selectable case.
    case: CaseIdentity,
    /// Effective request origins retained during static resolution.
    origins: Vec<WorldUseOrigin>,
}

/// Derived World view references canonical cases rather than adding to overall totals.
#[derive(Serialize)]
struct WorldAttemptMetrics {
    /// Structured canonical associated case identity.
    case: CaseIdentity,
    /// Independent fresh-process attempt.
    attempt: u64,
    /// Absent when setup was not entered.
    setup: Option<crate::testing::native::scheduler::metrics::CallbackMetric>,
    /// Absent for omitted/unentered cleanup; declaration flag distinguishes them.
    cleanup: Option<crate::testing::native::scheduler::metrics::CallbackMetric>,
}

/// One source of truth for all supported report formats.
#[derive(Serialize)]
struct Report {
    /// Machine-readable native report schema.
    schema_version: u32,
    /// Association metadata is not execution coverage.
    coverage: &'static str,
    /// Whether callbacks intentionally did not run.
    list_only: bool,
    /// Canonical deduplicated records.
    cases: Vec<CaseRecord>,
    /// Independent case totals.
    totals: Totals,
    /// Honest canonical function associations.
    by_function: Vec<FunctionView>,
    /// Canonical World memberships.
    by_world: Vec<WorldView>,
}

/// Render metadata and retained attempts without re-running any source operation.
pub(super) fn render(
    checked: &CheckedNativeProject,
    selected: &BTreeSet<CaseIdentity>,
    executions: Option<&SchedulerReport>,
    list: bool,
    reporter: Reporter,
) -> Result<String, NativeCommandError> {
    let mut records = Vec::new();
    let mut totals = Totals::default();
    for case in &checked.suite.cases {
        let body = &checked.suite.bodies[&case.body_identity];
        let selected = selected.contains(&case.identity);
        let execution = executions
            .and_then(|report| {
                report
                    .cases
                    .iter()
                    .find(|record| record.identity == case.identity)
            })
            .cloned();
        if !selected {
            totals.skipped = totals.skipped.saturating_add(1);
        } else if list {
            totals.listed = totals.listed.saturating_add(1);
        } else if let Some(ref execution) = execution {
            match execution.outcome {
                CaseOutcome::Passed => totals.passed = totals.passed.saturating_add(1),
                CaseOutcome::FlakyPassed => totals.flaky = totals.flaky.saturating_add(1),
                _ => totals.failed = totals.failed.saturating_add(1),
            }
        }
        let mut documentation: Vec<_> = body
            .documentation
            .iter()
            .map(|doc| doc.raw.clone())
            .collect();
        if let Some(doc) = case
            .example
            .as_ref()
            .and_then(|example| example.documentation.as_ref())
        {
            documentation.push(doc.raw.clone());
        }
        records.push(CaseRecord {
            id: serde_json::to_string(&case.identity).map_err(NativeCommandError::report)?,
            identity: case.identity.clone(),
            display: display(&case.identity),
            category: kind(body.kind).to_owned(),
            documentation,
            exclusion_keys: body.exclusion_keys.clone(),
            subjects: body.subjects.clone(),
            worlds: body
                .world_origins
                .iter()
                .map(|(identity, origins)| WorldRequirement {
                    identity: identity.clone(),
                    origins: origins.iter().cloned().collect(),
                })
                .collect(),
            declared_doubles: checked
                .doubles
                .declarations
                .values()
                .map(|declaration| DoubleDeclarationRecord {
                    identity: declaration.identity.clone(),
                    edge: declaration.edge.clone(),
                })
                .collect(),
            selected,
            execution,
        });
    }
    let (by_function, mut by_world) = associations(&records);
    for world in &mut by_world {
        world.documentation = checked
            .worlds
            .declarations
            .get(&world.identity)
            .and_then(|definition| definition.declaration.documentation.as_ref())
            .map(|doc| doc.raw.clone());
    }
    views::populate(&mut by_world, &records, checked)?;
    let report = Report {
        schema_version: 1,
        coverage: "not_collected",
        list_only: list,
        cases: records,
        totals,
        by_function,
        by_world,
    };
    match reporter {
        Reporter::Json => serde_json::to_string_pretty(&report).map_err(NativeCommandError::report),
        Reporter::Text => text(&report),
        Reporter::Junit => junit(&report),
    }
}

/// Category spellings are the language's finite vocabulary.
pub(super) const fn kind(kind: crate::ast::testing::TestKind) -> &'static str {
    match kind {
        crate::ast::testing::TestKind::Unit => "unit",
        crate::ast::testing::TestKind::Integration => "integration",
        crate::ast::testing::TestKind::EndToEnd => "end_to_end",
    }
}

/// Human-readable grouping does not participate in scheduler locks.
pub(super) fn display(identity: &CaseIdentity) -> String {
    let mut parts = identity.scenarios.clone();
    parts.push(identity.test.clone());
    if let Some(ref example) = identity.example {
        parts.push(example.clone());
    }
    format!("{}: {}", identity.module, parts.join(" / "))
}

/// Views reference the same distinct case objects; durations have honest names.
fn associations(records: &[CaseRecord]) -> (Vec<FunctionView>, Vec<WorldView>) {
    let mut functions: BTreeMap<FunctionIdentity, (BTreeSet<CaseIdentity>, u64)> = BTreeMap::new();
    let mut worlds: BTreeMap<WorldIdentity, BTreeSet<CaseIdentity>> = BTreeMap::new();
    for case in records {
        let duration = case.execution.as_ref().map_or(0, |execution| {
            execution.attempts.iter().fold(0_u64, |sum, attempt| {
                sum.saturating_add(attempt.duration_ms)
            })
        });
        for subject in &case.subjects {
            let entry = functions.entry(subject.clone()).or_default();
            if entry.0.insert(case.identity.clone()) {
                entry.1 = entry.1.saturating_add(duration);
            }
        }
        for world in &case.worlds {
            worlds
                .entry(world.identity.clone())
                .or_default()
                .insert(case.identity.clone());
        }
    }
    (
        functions
            .into_iter()
            .map(|(identity, (cases, duration))| FunctionView {
                identity,
                outcomes: views::function_cases(records, &cases),
                cases,
                associated_test_duration_ms: duration,
            })
            .collect(),
        worlds
            .into_iter()
            .map(|(identity, cases)| WorldView {
                identity,
                cases,
                documentation: None,
                cleanup_declared: false,
                requests: Vec::new(),
                callback_metrics: Vec::new(),
            })
            .collect(),
    )
}

/// Listing includes exact stable selection IDs; execution includes failure phases.
fn text(report: &Report) -> Result<String, NativeCommandError> {
    let mut output = String::new();
    for case in &report.cases {
        if report.list_only && !case.selected {
            continue;
        }
        let status = case.execution.as_ref().map_or_else(
            || {
                if case.selected {
                    "listed".to_owned()
                } else {
                    "skipped".to_owned()
                }
            },
            |execution| execution.outcome.label().to_owned(),
        );
        writeln!(output, "{status}: {}", case.display).map_err(NativeCommandError::report)?;
        if report.list_only {
            writeln!(output, "  case: {}", case.id).map_err(NativeCommandError::report)?;
        }
        for doc in &case.documentation {
            writeln!(output, "  {doc}").map_err(NativeCommandError::report)?;
        }
        if let Some(ref execution) = case.execution {
            for attempt in &execution.attempts {
                views::attempt(&mut output, attempt)?;
                if let Some(ref result) = attempt.result {
                    for failure in &result.failures {
                        writeln!(
                            output,
                            "  attempt {} phase {} {}:{}: {}",
                            attempt.attempt,
                            failure.phase,
                            failure.file,
                            failure.offset,
                            failure.message
                        )
                        .map_err(NativeCommandError::report)?;
                    }
                }
                if attempt.result.is_none() {
                    if let Some(phase) = attempt.last_phase {
                        writeln!(
                            output,
                            "  attempt {} last known phase {}; lifecycle completion unavailable",
                            attempt.attempt, phase
                        )
                        .map_err(NativeCommandError::report)?;
                    }
                }
                if let Some(ref message) = attempt.infrastructure_message {
                    writeln!(output, "  infrastructure: {message}")
                        .map_err(NativeCommandError::report)?;
                }
            }
        }
    }
    writeln!(
        output,
        "{} passed, {} flaky, {} failed, {} skipped, {} listed; coverage not collected",
        report.totals.passed,
        report.totals.flaky,
        report.totals.failed,
        report.totals.skipped,
        report.totals.listed
    )
    .map_err(NativeCommandError::report)?;
    views::text(&mut output, report)?;
    Ok(output)
}

/// `JUnit` uses one testcase per independent identity and retains attempt detail.
fn junit(report: &Report) -> Result<String, NativeCommandError> {
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"Opalescent native tests\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
        report.cases.len(),
        report.totals.failed,
        report.totals.skipped.saturating_add(report.totals.listed)
    );
    for case in &report.cases {
        writeln!(
            output,
            "<testcase classname=\"{}\" name=\"{}\">",
            xml(&case.identity.module),
            xml(&case.display)
        )
        .map_err(NativeCommandError::report)?;
        if let Some(ref execution) = case.execution {
            if !execution.outcome.is_success() {
                let details = serde_json::to_string(&execution.attempts)
                    .map_err(NativeCommandError::report)?;
                writeln!(
                    output,
                    "<failure type=\"{}\" message=\"native test failed\">{}</failure>",
                    execution.outcome.label(),
                    xml(&details)
                )
                .map_err(NativeCommandError::report)?;
            }
            if execution.outcome == CaseOutcome::FlakyPassed {
                writeln!(
                    output,
                    "<system-out>{}</system-out>",
                    xml(&serde_json::to_string(&execution.attempts)
                        .map_err(NativeCommandError::report)?)
                )
                .map_err(NativeCommandError::report)?;
            }
        } else {
            output.push_str("<skipped/>\n");
        }
        output.push_str("</testcase>\n");
    }
    output.push_str("</testsuite>\n");
    Ok(output)
}

/// Replace XML-forbidden controls and escape all text/attribute delimiters.
pub(super) fn xml(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            '\n' => output.push_str("&#10;"),
            '\r' => output.push_str("&#13;"),
            '\t' => output.push_str("&#9;"),
            character if character.is_control() || matches!(character, '\u{fffe}' | '\u{ffff}') => {
                output.push('\u{fffd}');
            }
            character => output.push(character),
        }
    }
    output
}
