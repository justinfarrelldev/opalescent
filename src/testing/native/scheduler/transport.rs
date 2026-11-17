//! Dedicated worker transport; source stdout/stderr are never parsed as protocol.
use super::{
    DiagnosticAttachment, DoubleBoundaryEvent, WorkerResult,
    metrics::{self, CallbackMetric},
};
use serde::Deserialize;
use std::path::Path;

/// Complete events observed before termination, even when no result could be emitted.
#[derive(Default)]
pub(super) struct Progress {
    /// Last flushed lifecycle phase.
    pub phase: Option<u32>,
    /// Terminal result, validated against the selected attempt separately.
    pub result: Option<WorkerResult>,
    /// Transport/initialization infrastructure error.
    pub message: Option<String>,
    /// Validated callback boundaries.
    pub callbacks: Vec<CallbackMetric>,
    /// Owned flushed diagnostics independent of terminal result availability.
    pub attachments: Vec<DiagnosticAttachment>,
    /// Canonical actual activation/restore observations.
    pub double_events: Vec<DoubleBoundaryEvent>,
}

/// Exact finite protocol event vocabulary, independent of application payloads.
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Event {
    /// Monotonic lifecycle progress.
    Phase {
        /// Versioned lifecycle phase.
        phase: u32,
    },
    /// Observed source callback entry or completion.
    Callback(metrics::CallbackEvent),
    /// Successful canonical mock activation/restoration.
    Double {
        /// Serialized canonical declaration.
        identity: String,
        /// Serialized canonical import edge.
        edge: String,
        /// Active versus restored state.
        active: bool,
    },
    /// Copied user diagnostic data.
    Attachment(DiagnosticAttachment),
    /// Complete lifecycle result.
    Result(WorkerResult),
    /// Worker selection/initialization failure.
    Infrastructure {
        /// Owned failure explanation.
        message: String,
    },
}

/// A partial final line cannot claim a completed callback or completed lifecycle.
pub(super) fn read(path: &Path) -> Progress {
    let mut progress = Progress::default();
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            progress.message = Some(error.to_string());
            return progress;
        }
    };
    for line in text
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        match serde_json::from_str::<Event>(line) {
            Ok(Event::Phase { phase }) if (1..=7).contains(&phase) && progress.result.is_none() => {
                progress.phase = Some(phase);
            }
            Ok(Event::Callback(event)) if progress.result.is_none() => {
                if !metrics::accept(&mut progress.callbacks, event) {
                    progress.message = Some("invalid native callback boundary event".to_owned());
                }
            }
            Ok(Event::Attachment(attachment))
                if progress.result.is_none()
                    && !attachment.name.is_empty()
                    && attachment.mime == "text/plain; charset=utf-8" =>
            {
                progress.attachments.push(attachment);
            }
            Ok(Event::Double {
                identity,
                edge,
                active,
            }) if progress.result.is_none() => {
                let parsed = serde_json::from_str(&identity).and_then(|identity| {
                    serde_json::from_str(&edge).map(|edge| DoubleBoundaryEvent {
                        identity,
                        edge,
                        active,
                    })
                });
                match parsed {
                    Ok(event) => progress.double_events.push(event),
                    Err(error) => {
                        progress.message = Some(format!("invalid canonical double event: {error}"));
                    }
                }
            }
            Ok(Event::Result(result)) if progress.result.is_none() => {
                progress.result = Some(result);
            }
            Ok(Event::Infrastructure { message }) => progress.message = Some(message),
            Ok(_) => progress.message = Some("invalid or duplicate native worker event".to_owned()),
            Err(error) => {
                progress.message = Some(format!("malformed native worker event: {error}"));
            }
        }
    }
    if progress.result.is_some()
        && progress
            .callbacks
            .iter()
            .any(|callback| callback.outcome == "running")
    {
        progress.message = Some("worker claimed completion with an unfinished callback".to_owned());
    }
    progress
}
