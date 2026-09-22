//! Warning diagnostics emitted by type checking and lint passes.
extern crate alloc;

use alloc::string::String;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Warning diagnostics produced during type checking.
///
/// Warnings represent non-fatal issues that should be surfaced to users without
/// preventing successful compilation. This mirrors [`super::TypeError`] so diagnostics
/// remain consistent across fatal and non-fatal analysis paths.
#[derive(Error, Debug, Clone, PartialEq, Eq, Diagnostic)]
pub enum Warning {
    /// Compile-time constant integer arithmetic overflows the destination type.
    #[error("Compile-time {operation} overflows type '{type_name}'; this traps in debug builds")]
    #[diagnostic(
        code(opalescent::type_system::warning::arithmetic_overflow),
        help(
            "Use checked_*, wrapping_*, or saturating_* explicit variants when overflow behavior is intentional"
        )
    )]
    ArithmeticOverflow {
        /// Arithmetic operation that overflowed (`addition`, `subtraction`, `multiplication`).
        operation: String,
        /// Destination integer type affected by overflow.
        type_name: String,
        #[label("constant expression overflows here")]
        /// Source span highlighting the overflowing constant expression.
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// Unsafe cast that may lose data or precision.
    #[error("Unsafe cast from '{from_type}' to '{to_type}' may lose data")]
    #[diagnostic(
        code(opalescent::type_system::warning::unsafe_cast),
        help(
            "This cast is narrowing and may lose data. Consider validating the value before casting or using a checked conversion API."
        )
    )]
    UnsafeCast {
        /// Source type of the cast.
        from_type: String,
        /// Target type of the cast.
        to_type: String,
        #[label("unsafe narrowing cast")]
        /// Source span highlighting where the unsafe cast was attempted.
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// `let` binding that is never read during type checking.
    #[error("Variable '{name}' is never used")]
    #[diagnostic(
        code(opalescent::type_system::unused_variable),
        help("Remove the variable or prefix it with '_' if unused intentionally")
    )]
    UnusedVariable {
        /// Name of the variable that is unused.
        name: String,
        #[label("unused variable")]
        /// Source span for the unused variable binding.
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// Placeholder warning for future unreachable-code analysis.
    #[error("Unreachable code detected")]
    #[diagnostic(
        code(opalescent::type_system::warning::unreachable_code),
        help("Remove unreachable statements or refactor control flow")
    )]
    UnreachableCode {
        #[label("unreachable code")]
        /// Source span for unreachable code.
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// A complete error leaf list that can be replaced by a named error set.
    #[error("Error declarations can be replaced with '{family_name}'")]
    #[diagnostic(
        code(opalescent::type_system::warning::replaceable_error_list),
        help("Replace `errors {replaceable_errors}` with `errors {family_name}`.")
    )]
    ReplaceableErrorList {
        family_name: String,
        replaceable_errors: String,
        #[label("complete replaceable error list")]
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// A leaf or set member is redundant because another named set covers it.
    #[error("Error declaration '{redundant_member}' is already covered by '{covering_set}'")]
    #[diagnostic(
        code(opalescent::type_system::warning::error_set_redundant_member),
        help("Remove `{redundant_member}` from this `errors` clause.")
    )]
    ErrorSetRedundantMember {
        redundant_member: String,
        covering_set: String,
        #[label("redundant error declaration")]
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// One named error set is wholly contained in another named set in the same clause.
    #[error("Error set '{narrower_set}' is already covered by '{broader_set}'")]
    #[diagnostic(
        code(opalescent::type_system::warning::error_set_overlap),
        help("Remove `{narrower_set}` unless the broader set should be narrowed.")
    )]
    ErrorSetOverlap {
        narrower_set: String,
        broader_set: String,
        #[label("overlapping error set")]
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// A named error set includes members that cannot escape the function body.
    #[error("Error set '{set_name}' includes errors that cannot escape this function")]
    #[diagnostic(
        code(opalescent::type_system::warning::error_set_unused_member),
        help("Use `errors {suggested_errors}` instead.")
    )]
    ErrorSetUnusedMembers {
        set_name: String,
        unused_members: String,
        suggested_errors: String,
        #[label("broader than this function needs")]
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
    /// Placeholder warning for future exhaustiveness analysis.
    #[error("Pattern match may be non-exhaustive")]
    #[diagnostic(
        code(opalescent::type_system::warning::non_exhaustive_match),
        help("Add missing pattern arms to handle all possible cases")
    )]
    NonExhaustiveMatch {
        #[label("non-exhaustive pattern match")]
        /// Source span for the non-exhaustive match.
        span: SourceSpan,
        /// Optional suppression annotation identifier for future warning controls.
        suppression_annotation: Option<String>,
    },
}
