//! Type checking error types and error handling utilities

extern crate alloc;

use crate::token::Span;
use alloc::string::String;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Type checking errors that can occur during type analysis
#[derive(Error, Debug, Clone, PartialEq, Eq, Diagnostic)]
pub enum TypeError {
    /// Type was not found in the current scope
    #[error("Type '{type_name}' not found")]
    #[diagnostic(
        code(opalescent::type_system::type_not_found),
        help("Check that the type is defined or imported in this scope")
    )]
    TypeNotFound {
        /// Name of the type that was not found
        type_name: String,
        #[label("undefined type")]
        /// Source span highlighting where the type was referenced
        span: SourceSpan,
    },

    /// Symbol (variable/function) was not found in the current scope
    #[error("Symbol '{name}' not found in this scope")]
    #[diagnostic(
        code(opalescent::type_system::symbol_not_found),
        help("Ensure the symbol is declared before use or imported from the correct module")
    )]
    SymbolNotFound {
        /// Name of the missing symbol
        name: String,
        #[label("undefined symbol")]
        /// Location where the symbol was referenced
        span: SourceSpan,
    },

    /// Types do not match in an expression
    #[error("Type mismatch: expected '{expected}', found '{found}'")]
    #[diagnostic(
        code(opalescent::type_system::type_mismatch),
        help(
            "Consider using an explicit cast if this conversion is intentional, or change one of the types to match"
        )
    )]
    TypeMismatch {
        /// Expected type name
        expected: String,
        /// Actually found type name
        found: String,
        #[label("type '{found}' found here")]
        /// Source span highlighting where the incompatible type was found
        found_span: SourceSpan,
        #[label("type '{expected}' expected here")]
        /// Source span highlighting where the expected type was required
        expected_span: Option<SourceSpan>,
    },

    /// Invalid type operation
    #[error("Invalid operation '{operation}' for type '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::invalid_operation),
        help(
            "This operation is not supported for this type. Check the language reference for valid operations"
        )
    )]
    InvalidOperation {
        /// Operation that was attempted
        operation: String,
        /// Name of the type the operation was attempted on
        type_name: String,
        #[label("invalid operation here")]
        /// Source span highlighting where the invalid operation was attempted
        span: SourceSpan,
    },

    /// Generic type parameter not found
    #[error("Generic type parameter '{param_name}' not found")]
    #[diagnostic(
        code(opalescent::type_system::generic_parameter_not_found),
        help("Check that the generic parameter is declared in the type or function signature")
    )]
    GenericParameterNotFound {
        /// Name of the generic parameter that was not found
        param_name: String,
        #[label("undefined generic parameter")]
        /// Source span highlighting where the parameter was referenced
        span: SourceSpan,
    },

    /// Unification failed between two types
    #[error("Cannot unify types '{left}' and '{right}'")]
    #[diagnostic(
        code(opalescent::type_system::unification_failed),
        help(
            "These types are incompatible. Consider using an explicit cast or changing one of the types"
        )
    )]
    UnificationFailed {
        /// Left type in the unification
        left: String,
        /// Right type in the unification
        right: String,
        #[label("type '{left}' found here")]
        /// Source span highlighting the left type location
        left_span: SourceSpan,
        #[label("type '{right}' expected here")]
        /// Source span highlighting the right type location
        right_span: SourceSpan,
    },

    /// Occurs check failed (infinite type)
    #[error("Occurs check failed: type variable '{var_name}' occurs in '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::occurs_check_failed),
        help(
            "This would create an infinite type. Check for recursive type definitions or incorrect type constraints"
        )
    )]
    OccursCheckFailed {
        /// Name of the type variable
        var_name: String,
        /// Name of the type it occurs in
        type_name: String,
        #[label("infinite type would be created here")]
        /// Source span highlighting where the occurs check failed
        span: SourceSpan,
    },

    /// Constraint solving failed
    #[error("Constraint solving failed: {reason}")]
    #[diagnostic(
        code(opalescent::type_system::constraint_solving_failed),
        help(
            "The type constraints for this expression could not be satisfied. Review the types involved"
        )
    )]
    ConstraintSolvingFailed {
        /// Reason for the failure
        reason: String,
        #[label("constraint violation")]
        /// Source span highlighting where the constraint failed
        span: SourceSpan,
    },

    /// Type variable ID overflow occurred
    #[error("Type variable ID overflow - too many type variables generated")]
    #[diagnostic(
        code(opalescent::type_system::type_variable_overflow),
        help(
            "This is an internal compiler error. The program has generated too many type variables"
        )
    )]
    TypeVariableOverflow {
        #[label("overflow occurred during type inference here")]
        /// Source span highlighting where the overflow occurred
        span: SourceSpan,
    },

    /// Feature not yet implemented
    #[error("Feature not yet implemented: {feature}")]
    #[diagnostic(
        code(opalescent::type_system::not_implemented),
        help(
            "This feature is planned but not yet available. Check the project roadmap for implementation status"
        )
    )]
    NotImplementedYet {
        /// Description of the feature not yet implemented
        feature: String,
        #[label("unimplemented feature used here")]
        /// Source span highlighting where the unimplemented feature was used
        span: SourceSpan,
    },

    /// Function arity mismatch (wrong number of arguments)
    #[error("Function arity mismatch: expected {expected} argument(s), found {found}")]
    #[diagnostic(
        code(opalescent::type_system::arity_mismatch),
        help("Ensure the number of arguments matches the function signature")
    )]
    ArityMismatch {
        /// Expected number of arguments
        expected: usize,
        /// Actual number of arguments provided
        found: usize,
        #[label("wrong number of arguments")]
        /// Source span highlighting where the call occurred
        span: SourceSpan,
    },

    /// Type is not callable
    #[error("Type '{type_name}' is not callable")]
    #[diagnostic(
        code(opalescent::type_system::not_callable),
        help(
            "Only function types can be called. Check that this is a function or function-typed variable"
        )
    )]
    NotCallable {
        /// Name of the non-callable type
        type_name: String,
        #[label("not a function")]
        /// Source span highlighting where the call was attempted
        span: SourceSpan,
    },
}

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
