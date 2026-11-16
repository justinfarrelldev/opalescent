//! Diagnostics shared by affine-owner and second-class reference validation.
use super::{BorrowKind, OwnerBinding, Span, TypeChecker, TypeError};

impl TypeChecker {
    /// Build a precise use-after-move diagnostic.
    pub(super) fn affine_use_after_move_error(
        name: &str,
        owner: &OwnerBinding,
        span: Span,
    ) -> TypeError {
        let moved_at = owner.moved_at.unwrap_or(owner.source_location);
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "affine resource '{name}' of type '{}' was already moved at byte {} and cannot be used again",
                owner.type_name, moved_at.start.offset,
            ),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a precise affine escape diagnostic.
    pub(super) fn affine_escape_error(
        name: &str,
        type_name: &str,
        context: &str,
        span: Span,
    ) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!("affine resource '{name}' of type '{type_name}' cannot {context}"),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a precise borrow escape diagnostic.
    pub(super) fn borrow_escape_error(name: &str, context: &str, span: Span) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!("second-class borrow '{name}' cannot {context}"),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a diagnostic for a missing call-site borrow argument.
    pub(super) fn borrow_required_error(expected: BorrowKind, span: Span) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "parameter requires a '{}' call-site borrow argument",
                borrow_kind_label(expected)
            ),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a diagnostic for incompatible borrow modes.
    pub(super) fn borrow_mode_mismatch_error(
        expected: BorrowKind,
        found: BorrowKind,
        span: Span,
    ) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "borrow argument mismatch: expected '{}', found '{}'",
                borrow_kind_label(expected),
                borrow_kind_label(found)
            ),
            span: TypeError::span_from_span(span),
        }
    }
}

/// Human-readable borrow kind for diagnostics.
const fn borrow_kind_label(kind: BorrowKind) -> &'static str {
    match kind {
        BorrowKind::Owned => "owned",
        BorrowKind::Ref => "ref",
        BorrowKind::MutableRef => "mutable ref",
    }
}
