//! Return-shape tracking helpers for function and lambda bodies.

extern crate alloc;

use super::{ReturnLabelMode, TypeChecker};
use crate::token::Span;
use crate::type_system::errors::TypeError;
use alloc::{collections::BTreeSet, format, string::String};

impl TypeChecker {
    /// Start return-shape tracking for a function or lambda body.
    pub(super) fn begin_return_context(&mut self) {
        self.context
            .return_label_modes
            .push(ReturnLabelMode::Unknown);
    }

    /// Finish return-shape tracking for a function or lambda body.
    pub(super) fn end_return_context(&mut self) {
        let popped = self.context.return_label_modes.pop();
        debug_assert!(
            popped.is_some(),
            "return label mode stack underflow when exiting return context"
        );
    }

    /// Validate the current return statement label shape against active function context.
    pub(super) fn ensure_return_label_mode(
        &mut self,
        labels: &[String],
        span: Span,
    ) -> Result<(), TypeError> {
        let Some(mode) = self.context.return_label_modes.last_mut() else {
            return Ok(());
        };

        if labels.is_empty() {
            match *mode {
                ReturnLabelMode::Unknown => {
                    *mode = ReturnLabelMode::Unlabeled;
                    Ok(())
                }
                ReturnLabelMode::Unlabeled => Ok(()),
                ReturnLabelMode::Labeled(ref expected_labels) => {
                    Err(TypeError::ReturnLabelMismatch {
                        expected: Self::render_return_labels(expected_labels.as_slice()),
                        found: "unlabeled return".to_owned(),
                        span: TypeError::span_from_span(span),
                    })
                }
            }
        } else {
            let mut seen = BTreeSet::new();
            for label in labels {
                if !seen.insert(label.clone()) {
                    return Err(TypeError::ReturnLabelMismatch {
                        expected: "unique labels".to_owned(),
                        found: format!("duplicate label '{label}'"),
                        span: TypeError::span_from_span(span),
                    });
                }
            }

            match *mode {
                ReturnLabelMode::Unknown => {
                    *mode = ReturnLabelMode::Labeled(labels.to_vec());
                    Ok(())
                }
                ReturnLabelMode::Unlabeled => Err(TypeError::ReturnLabelMismatch {
                    expected: "unlabeled return".to_owned(),
                    found: Self::render_return_labels(labels),
                    span: TypeError::span_from_span(span),
                }),
                ReturnLabelMode::Labeled(ref expected_labels) => {
                    if expected_labels.as_slice() == labels {
                        Ok(())
                    } else {
                        Err(TypeError::ReturnLabelMismatch {
                            expected: Self::render_return_labels(expected_labels.as_slice()),
                            found: Self::render_return_labels(labels),
                            span: TypeError::span_from_span(span),
                        })
                    }
                }
            }
        }
    }

    /// Render ordered return labels for diagnostics.
    fn render_return_labels(labels: &[String]) -> String {
        if labels.is_empty() {
            return "unlabeled return".to_owned();
        }

        let mut rendered = String::new();
        for (index, label) in labels.iter().enumerate() {
            if index > 0 {
                rendered.push_str(", ");
            }
            rendered.push_str(label);
        }
        rendered
    }
}
