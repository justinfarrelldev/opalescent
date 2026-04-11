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

    /// Program contains no entry point declaration.
    #[error("Program is missing an `entry` function")]
    #[diagnostic(
        code(opalescent::type_system::missing_entry_point),
        help("Define exactly one `entry` function to mark the program entry point")
    )]
    MissingEntryPoint {
        /// Source span for whole-program entry validation failure.
        #[label("no entry function declared")]
        /// Label span used in diagnostics.
        span: SourceSpan,
    },

    /// Program declares more than one entry point.
    #[error("Program declares multiple `entry` functions ({count} found)")]
    #[diagnostic(
        code(opalescent::type_system::duplicate_entry_point),
        help("Keep exactly one `entry` function and remove the extra entry annotations")
    )]
    DuplicateEntryPoint {
        /// Number of entry declarations discovered.
        count: usize,
        /// Source span pointing at one duplicate entry declaration.
        #[label("duplicate entry function declared here")]
        /// Label span used in diagnostics.
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

    /// Assignment attempted on an immutable `let` binding.
    #[error("Cannot assign to immutable variable '{name}'")]
    #[diagnostic(
        code(opalescent::type_system::immutable_assignment),
        help("Declare the binding as `let mutable {name}` if reassignment is intended")
    )]
    ImmutableAssignment {
        /// Name of the immutable variable receiving assignment.
        name: String,
        #[label("immutable variable assigned here")]
        /// Source span of the assignment expression.
        assignment_span: SourceSpan,
        #[label("variable declared immutable here")]
        /// Optional source span of the original immutable declaration.
        declaration_span: Option<SourceSpan>,
    },

    /// Compile-time detected division or modulo by a constant zero divisor.
    #[error("Compile-time {operation} by zero is not allowed")]
    #[diagnostic(
        code(opalescent::type_system::division_by_zero),
        help("Use a non-zero divisor or guard divisor values before this expression")
    )]
    DivisionByZero {
        /// Operation name (`division` or `modulo`) that used a zero divisor.
        operation: String,
        #[label("zero divisor")]
        /// Source span highlighting the zero-valued divisor expression.
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

    /// Generic call-site inference failed for a declared type parameter.
    #[error("Cannot infer generic type parameter '{param_name}' at this call site")]
    #[diagnostic(
        code(opalescent::type_system::cannot_infer_generic_type),
        help("Provide explicit generic arguments or pass arguments that constrain this generic")
    )]
    CannotInferGenericType {
        /// Name of the generic parameter that could not be inferred.
        param_name: String,
        /// Source span of the call expression causing inference failure.
        #[label("generic type cannot be inferred here")]
        /// Label span used in diagnostics.
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

    /// Invalid cast between types
    #[error("Invalid cast from '{from_type}' to '{to_type}'")]
    #[diagnostic(
        code(opalescent::type_system::invalid_cast),
        help(
            "These types cannot be converted. Consider using a different conversion method or intermediate type"
        )
    )]
    InvalidCast {
        /// Source type of the cast
        from_type: String,
        /// Target type of the cast
        to_type: String,
        #[label("cannot cast from '{from_type}' to '{to_type}'")]
        /// Source span highlighting where the cast was attempted
        span: SourceSpan,
    },

    /// An error type was used in a function signature but not declared in the current scope.
    #[error("Undeclared error type '{name}'")]
    #[diagnostic(
        code(opalescent::type_system::undeclared_error_type),
        help("Ensure the error type '{name}' is defined and visible in the current scope.")
    )]
    UndeclaredErrorType {
        /// The name of the undeclared error type.
        name: String,
        #[label("undeclared error type")]
        /// The source span where the undeclared error type was referenced.
        span: SourceSpan,
    },

    /// `propagate` was used outside a function that declares error types.
    #[error("`propagate` used outside a function that declares errors")]
    #[diagnostic(
        code(opalescent::type_system::propagate_outside_error_function),
        help(
            "To use `propagate`, the enclosing function must declare at least one error type in its signature, like `f(): T errors E => ...`"
        )
    )]
    PropagateOutsideErrorFunction {
        #[label("`propagate` used here")]
        /// The source span where `propagate` was used.
        span: SourceSpan,
    },

    /// The error types of a propagated call are not a subset of the enclosing function's declared error types.
    #[error("Propagated error types are not compatible with the function's declared errors")]
    #[diagnostic(
        code(opalescent::type_system::propagate_error_mismatch),
        help(
            "The errors from the called function must be a subset of the errors declared by the current function."
        )
    )]
    PropagateErrorMismatch {
        /// The error types declared by the current function.
        expected: String,
        /// The error types returned by the propagated function call.
        found: String,
        #[label("this function declares errors: {expected}")]
        /// The source span of the current function's error declaration.
        span: SourceSpan,
        #[label("this call can produce errors: {found}")]
        /// The source span of the call being propagated.
        callee_span: SourceSpan,
    },

    /// `propagate` was used on a call that does not declare any error types.
    #[error("`propagate` used on a call that cannot produce errors")]
    #[diagnostic(
        code(opalescent::type_system::propagate_on_non_error_expression),
        help(
            "Remove `propagate` or update the callee to declare the errors it can emit in its signature"
        )
    )]
    PropagateOnNonErrorExpression {
        #[label("`propagate` used here")]
        /// The span covering the `propagate` expression.
        span: SourceSpan,
    },

    /// A `guard` expression was used on an expression that does not return errors.
    #[error("`guard` used on an expression that cannot produce errors")]
    #[diagnostic(
        code(opalescent::type_system::guard_on_non_error_expression),
        help(
            "`guard` is only valid on function calls or expressions that can result in an error."
        )
    )]
    GuardOnNonErrorExpression {
        #[label("`guard` used here on a non-erroring expression")]
        /// The source span of the `guard` expression.
        span: SourceSpan,
    },

    /// The type of the binding in a `guard` expression does not match the success type of the guarded expression.
    #[error("Guard binding type mismatch: expected '{expected}', found '{found}'")]
    #[diagnostic(
        code(opalescent::type_system::guard_binding_type_mismatch),
        help(
            "The type of the variable in `guard ... into var` must match the success type of the expression."
        )
    )]
    GuardBindingTypeMismatch {
        /// The expected success type.
        expected: String,
        /// The actual type of the binding.
        found: String,
        #[label("type mismatch in `guard` binding")]
        /// The source span of the binding.
        span: SourceSpan,
    },

    /// The `else` branch of a `guard` expression does not handle the possible error types.
    #[error(
        "Guard `else` branch is incompatible with error types: expected '{expected}', found '{found}'"
    )]
    #[diagnostic(
        code(opalescent::type_system::guard_else_incompatible_error),
        help(
            "The `else` branch of a `guard` must be able to handle all possible errors from the guarded expression."
        )
    )]
    GuardElseIncompatibleError {
        /// The expected error types.
        expected: String,
        /// The type handled by the `else` branch.
        found: String,
        #[label("incompatible `else` branch here")]
        /// The source span of the `else` branch.
        span: SourceSpan,
    },

    /// Nested guard or propagate introduces error types that differ from the surrounding guard.
    #[error(
        "Guard handler introduces incompatible error types: expected '{expected}', found '{found}'"
    )]
    #[diagnostic(
        code(opalescent::type_system::guard_chained_error_mismatch),
        help(
            "Guard handlers must not introduce new error types. Ensure nested guard/propagate expressions use the same error set as the surrounding guard."
        )
    )]
    GuardChainedErrorMismatch {
        /// Error types managed by the surrounding guard.
        expected: String,
        /// Error types introduced by the nested expression.
        found: String,
        #[label("incompatible error types introduced here")]
        /// Span for the nested guard/propagate expression.
        span: SourceSpan,
    },

    /// Return statements in the same function use incompatible label shapes.
    #[error("Return label mismatch: expected '{expected}', found '{found}'")]
    #[diagnostic(
        code(opalescent::type_system::return_label_mismatch),
        help(
            "Use a consistent labeled or unlabeled return shape across all returns in this function, and keep label order stable."
        )
    )]
    ReturnLabelMismatch {
        /// Expected label shape for returns in the current function/lambda.
        expected: String,
        /// Label shape found on the mismatching return statement.
        found: String,
        #[label("return labels do not match expected shape")]
        /// Source span of the mismatching return statement.
        span: SourceSpan,
    },

    #[error("If expression used for a value must include an else branch")]
    #[diagnostic(
        code(opalescent::type_system::missing_else_branch),
        help(
            "Add an `else` branch that returns '{expected_type}' so the expression is exhaustive"
        )
    )]
    /// If expression without else used in a non-unit value context.
    MissingElseBranch {
        /// Non-unit type required by the surrounding value position.
        expected_type: String,
        #[label("if expression without else used in non-unit context")]
        /// Span of the non-exhaustive if expression.
        span: SourceSpan,
    },
}

/// Warning diagnostics produced during type checking.
///
/// Warnings represent non-fatal issues that should be surfaced to users without
/// preventing successful compilation. This mirrors [`TypeError`] so diagnostics
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

impl Warning {
    /// Return the suppression annotation attached to this warning, if present.
    pub fn suppression_annotation(&self) -> Option<&str> {
        let suppression_annotation = match *self {
            Self::ArithmeticOverflow {
                ref suppression_annotation,
                ..
            }
            | Self::UnsafeCast {
                ref suppression_annotation,
                ..
            }
            | Self::UnusedVariable {
                ref suppression_annotation,
                ..
            }
            | Self::UnreachableCode {
                ref suppression_annotation,
                ..
            }
            | Self::NonExhaustiveMatch {
                ref suppression_annotation,
                ..
            } => suppression_annotation,
        };

        suppression_annotation.as_deref()
    }
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
