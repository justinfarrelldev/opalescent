use super::TypeError;
use crate::token::Span;
use miette::SourceSpan;

impl TypeError {
    /// Convert AST Span to miette `SourceSpan`
    ///
    /// This utility method provides consistent conversion from the compiler's internal
    /// [`Span`] type to miette's [`SourceSpan`] for error reporting.
    pub fn span_from_span(span: Span) -> SourceSpan {
        let start: usize = span.start.offset;
        let len = span.end.offset.saturating_sub(span.start.offset);
        SourceSpan::new(start.into(), len)
    }

    /// Create a default/unknown source span for errors without location information
    ///
    /// Used as a temporary measure for code that doesn't yet track source locations.
    /// All code should eventually be updated to provide actual spans.
    pub fn unknown_span() -> SourceSpan {
        SourceSpan::new(0.into(), 0)
    }
}
