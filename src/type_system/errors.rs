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
        /// Optional near-match symbol suggestion when typo detection succeeds.
        suggestion: Option<String>,
        #[label("undefined symbol")]
        /// Location where the symbol was referenced
        span: SourceSpan,
    },
    /// Unknown sum-type variant was referenced in a constructor expression.
    #[error("Unknown variant '{variant_name}' on type '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::unknown_variant),
        help("Ensure the variant name exists on this sum type")
    )]
    UnknownVariant {
        /// Sum type name used as the variant owner.
        type_name: String,
        /// Missing variant name referenced by source.
        variant_name: String,
        #[label("unknown variant used here")]
        /// Source location of the unknown variant reference.
        span: SourceSpan,
    },
    /// Required constructor field was not provided.
    #[error("Missing required field '{field_name}' for '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::missing_field),
        help("Provide this field in the constructor expression")
    )]
    MissingField {
        /// Nominal type/variant requiring the field.
        type_name: String,
        /// Field missing from constructor input.
        field_name: String,
        #[label("missing field in constructor")]
        /// Source location of the constructor expression.
        span: SourceSpan,
    },
    /// Constructor provided a value that does not match the declared field type.
    #[error(
        "Field '{field_name}' type mismatch in '{type_name}': expected '{expected}', found '{found}'"
    )]
    #[diagnostic(
        code(opalescent::type_system::field_type_mismatch),
        help("Update the field value so it matches the declared field type")
    )]
    FieldTypeMismatch {
        /// Nominal type/variant owning the field.
        type_name: String,
        /// Field with incompatible value.
        field_name: String,
        /// Declared field type.
        expected: String,
        /// Inferred type of the provided value.
        found: String,
        #[label("field has incompatible type")]
        /// Source location of the mismatching field value.
        span: SourceSpan,
    },
    /// Constructor declared the same field more than once.
    #[error("Duplicate field '{field_name}' in constructor expression")]
    #[diagnostic(
        code(opalescent::type_system::duplicate_field),
        help("Remove duplicate field initializers so each field appears once")
    )]
    DuplicateField {
        /// Field name repeated within the same constructor expression.
        field_name: String,
        #[label("duplicate field specified here")]
        /// Source location of the duplicate field occurrence.
        span: SourceSpan,
    },
    /// Type declaration reuses a reserved predefined name.
    #[error("Type '{type_name}' is reserved and cannot be redeclared")]
    #[diagnostic(
        code(opalescent::type_system::reserved_type_name),
        help("Choose a different type name because '{type_name}' is predefined by the language")
    )]
    ReservedTypeName {
        /// Reserved predefined type name that user code attempted to redeclare.
        type_name: String,
        #[label("reserved type name redeclared here")]
        /// Source location of the reserved type declaration.
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
    /// Pattern cannot match the scrutinee type in a `match` expression.
    #[error("Pattern type mismatch: expected '{expected}', found '{found}'")]
    #[diagnostic(
        code(opalescent::type_system::pattern_type_mismatch),
        help("Update the match pattern so it is compatible with the scrutinee type")
    )]
    PatternTypeMismatch {
        /// Expected scrutinee type.
        expected: String,
        /// Found pattern type.
        found: String,
        #[label("pattern is incompatible with scrutinee type")]
        /// Source span of the incompatible pattern.
        span: SourceSpan,
    },
    /// Match expression is missing one or more variants for an ADT scrutinee.
    #[error("Non-exhaustive match: missing variants {missing_variants:?}")]
    #[diagnostic(
        code(opalescent::type_system::non_exhaustive_match),
        help("Add arms for every missing variant, or use `_` as a final catch-all arm")
    )]
    NonExhaustiveMatch {
        /// Fully-qualified variant names that are not covered by match arms.
        missing_variants: Vec<String>,
        #[label("match expression does not cover all variants")]
        /// Source span of the non-exhaustive match expression.
        span: SourceSpan,
    },
    /// Match arm body type differs from the expected arm result type.
    #[error("Match arm #{arm_index} has type '{found}', expected '{expected}'")]
    #[diagnostic(
        code(opalescent::type_system::match_arm_type_mismatch),
        help("Ensure all match arm bodies evaluate to compatible result types")
    )]
    MatchArmTypeMismatch {
        /// Zero-based index of the arm that mismatches.
        arm_index: usize,
        /// Expected arm result type.
        expected: String,
        /// Found arm result type.
        found: String,
        #[label("match arm result type mismatch")]
        /// Source span of the mismatching arm.
        span: SourceSpan,
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
    #[error(
        "Invalid shift count {shift_count} for {bit_width}-bit integer (must be in 0..{bit_width})"
    )]
    #[diagnostic(
        code(opalescent::type_system::invalid_shift_count),
        help("Use a non-negative shift count smaller than the left operand bit width")
    )]
    /// Shift count constant is negative or exceeds the left operand bit width.
    InvalidShiftCount {
        /// Classification for invalid shift counts (`negative` or `out of range`).
        reason: String,
        /// Original constant shift-count value from source.
        count_value: i128,
        /// Shift count value used in the human-readable error message.
        shift_count: i128,
        /// Bit width of the shifted left operand type.
        bit_width: u32,
        #[label("invalid shift count")]
        /// Span of the invalid shift-count expression.
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
        help(
            "Provide explicit generic arguments, pass arguments that constrain this generic, or consider adding type annotation"
        )
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
    /// A call to an error-producing function was made without `guard` or `propagate`.
    #[error("Call to error-producing function `{name}` must be wrapped in `guard` or `propagate`")]
    #[diagnostic(
        code(opalescent::type_system::unhandled_call_error),
        help(
            "Wrap this call in `guard ... into ... else ...` to handle errors locally, or use `propagate` in an error-declaring function"
        )
    )]
    UnhandledCallError {
        /// Name of the called function, or a placeholder for non-identifier callees.
        name: String,
        #[label("unhandled error-producing call")]
        /// The source span where the bare call occurred.
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
    /// `guard` was used but the clause cannot terminate with a handled error path.
    #[error("`guard` clause is missing a terminal error-handling expression")]
    #[diagnostic(
        code(opalescent::guard::missing_terminal),
        help(
            "End the guard clause with a terminal `return`, `propagate`, or equivalent error-handling expression"
        )
    )]
    GuardErrorClauseMissingTerminal {
        /// Source span covering the non-terminal guard clause.
        #[label("guard clause missing terminal expression")]
        clause_span: SourceSpan,
    },
    /// `propagate` was used as the final guard-handler expression when the guard requires a non-final result.
    #[error("`propagate` cannot be the final expression in this guard handler")]
    #[diagnostic(
        code(opalescent::guard::propagate_not_final),
        help(
            "Add a final handler expression after `propagate`, or restructure the guard so the clause ends with an explicit result"
        )
    )]
    GuardPropagateErrNotFinal {
        /// Source span covering the final propagate expression.
        #[label("`propagate` cannot end this guard handler")]
        propagate_span: SourceSpan,
    },
    /// `return err` is invalid in a strict guard error clause.
    #[error("`return err` is not valid in a guard error clause")]
    #[diagnostic(
        code(opalescent::guard::return_err_invalid),
        help(
            "Use `propagate err` to forward the error, or return a concrete recovery value instead"
        )
    )]
    GuardReturnErrInvalid {
        /// Source span of the invalid `return err` statement.
        #[label("`return err` is not allowed here")]
        return_span: SourceSpan,
    },
    /// Guard wrapper source expression is not a valid strict guard source.
    #[error("`guard` wrapper source expression is invalid")]
    #[diagnostic(
        code(opalescent::guard::wrapper_source_invalid),
        help(
            "Wrap a valid fallible expression or call in `guard` so the wrapper source can be type-checked"
        )
    )]
    GuardWrapperSourceInvalid {
        /// Source span of the invalid wrapped source expression.
        #[label("invalid guard wrapper source")]
        source_span: SourceSpan,
    },
    /// Shorthand guard syntax requires an explicit binding or terminal clause in strict mode.
    #[error("`guard` shorthand is required in this context")]
    #[diagnostic(
        code(opalescent::guard::shorthand_required),
        help("Provide the required shorthand guard form for this expression shape")
    )]
    GuardShorthandRequired {
        /// Source span where shorthand guard syntax is required.
        #[label("guard shorthand required here")]
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
    /// Import graph contains a dependency cycle.
    #[error("Circular module dependency detected: {cycle:?}")]
    #[diagnostic(
        code(opalescent::type_system::circular_dependency),
        help("Break the import cycle by extracting shared declarations into a third module")
    )]
    CircularDependency {
        /// Ordered cycle path where first and last module are identical.
        cycle: Vec<String>,
        #[label("dependency cycle detected here")]
        /// Span of the import site that triggered cycle validation.
        span: SourceSpan,
    },
    /// Import source path could not be resolved in the module registry.
    #[error("Unresolved import path '{path}'")]
    #[diagnostic(
        code(opalescent::type_system::unresolved_import),
        help("Check the import path and ensure the target module is registered")
    )]
    UnresolvedImport {
        /// Unresolved module path from import declaration.
        path: String,
        #[label("import cannot be resolved")]
        /// Span of the unresolved import declaration.
        span: SourceSpan,
    },
    /// Two imports introduce the same local binding name from different modules.
    #[error(
        "Import name conflict for '{name}': already imported from '{first_module}', cannot also import from '{second_module}'"
    )]
    #[diagnostic(
        code(opalescent::type_system::import_name_conflict),
        help("Use an alias on one import so each local imported name is unique")
    )]
    ImportNameConflict {
        /// Local binding name introduced by conflicting imports.
        name: String,
        /// Module that first introduced this local binding name.
        first_module: String,
        /// Module that attempted to introduce the same local binding name.
        second_module: String,
        #[label("conflicting import name")]
        /// Span of the second import that caused the conflict.
        span: SourceSpan,
    },
    /// Import attempts to read a private symbol from another module.
    #[error("Cannot access private symbol '{symbol}' from module '{module}'")]
    #[diagnostic(
        code(opalescent::type_system::private_symbol_access),
        help("Mark the symbol as public in the source module or stop importing it")
    )]
    PrivateSymbolAccess {
        /// Symbol name requested by the import.
        symbol: String,
        /// Module where the private symbol was declared.
        module: String,
        #[label("private symbol access")]
        /// Span of the import item requesting private access.
        span: SourceSpan,
    },
    /// Call to an impure function from a pure function context.
    #[error("cannot call impure function '{callee_name}' from pure function context")]
    #[diagnostic(
        code(opalescent::type_system::purity_violation),
        help(
            "pure functions cannot perform I/O or call impure functions — remove the 'pure' modifier or move the impure call outside"
        )
    )]
    PurityViolation {
        /// Name of the function being called that violates purity.
        callee_name: String,
        /// Human-readable reason for the purity violation.
        reason: String,
        #[label("{reason}")]
        /// Source span of the impure call expression.
        span: SourceSpan,
    },
    /// Public function is missing a documentation comment.
    #[error("Public function '{name}' is missing a documentation comment")]
    #[diagnostic(
        code(opalescent::type_system::missing_doc_comment),
        help("Add a ## documentation block with at least 30 characters before this function")
    )]
    MissingDocComment {
        /// Name of the public function missing documentation.
        name: String,
        #[label("missing doc comment")]
        /// Source span of the function declaration.
        span: SourceSpan,
    },
    /// Documentation comment is too short.
    #[error(
        "Documentation comment for '{name}' is too short ({found_length} characters, minimum {min_length})"
    )]
    #[diagnostic(
        code(opalescent::type_system::doc_comment_too_short),
        help("Expand the documentation to at least {min_length} characters")
    )]
    DocCommentTooShort {
        /// Name of the function with insufficient documentation.
        name: String,
        /// Actual length of the documentation comment.
        found_length: usize,
        /// Minimum required length.
        min_length: usize,
        #[label("doc comment too short")]
        /// Source span of the documentation comment.
        span: SourceSpan,
    },
    /// Entry keyword used outside the main module.
    #[error("The 'entry' keyword is only allowed in src/main.op, found in '{file_path}'")]
    #[diagnostic(
        code(opalescent::type_system::entry_not_in_main_module),
        help(
            "Move the entry function to src/main.op — only one entry point is allowed per project"
        )
    )]
    EntryNotInMainModule {
        /// File path where the entry keyword was incorrectly used.
        file_path: String,
        #[label("entry not allowed here")]
        /// Source span of the entry declaration.
        span: SourceSpan,
    },
    /// Module import path could not be resolved.
    #[error("Module '{path}' not found")]
    #[diagnostic(
        code(opalescent::type_system::module_not_found),
        help("Check the import path — expected file at '{path}.op' or '{path}.types.op'")
    )]
    ModuleNotFound {
        /// Module path that could not be resolved.
        path: String,
        #[label("module not found")]
        /// Source span of the import statement.
        span: SourceSpan,
    },
    /// Package imports are not yet supported.
    #[error("Package imports are not yet supported: '{path}'")]
    #[diagnostic(
        code(opalescent::type_system::package_import_not_supported),
        help(
            "Package imports (@scope/name) will be available once the package manager is implemented. Use local imports (./path) instead."
        )
    )]
    PackageImportNotSupported {
        /// Package import path that was attempted.
        path: String,
        #[label("package import not supported")]
        /// Source span of the package import statement.
        span: SourceSpan,
    },
    /// `type` declaration found outside a `.types.op` file.
    #[error("type declaration '{type_name}' is not allowed in '{file_path}'")]
    #[diagnostic(
        code(opalescent::type_system::type_declaration_outside_types_file),
        help(
            "Move this type to a file ending in .types.op — the language spec requires type declarations to live in .types.op files"
        )
    )]
    TypeDeclarationOutsideTypesFile {
        /// Name of the type being declared.
        type_name: String,
        /// File path where the type was incorrectly declared.
        file_path: String,
        #[label("type declaration not allowed here")]
        /// Source span of the type declaration.
        span: SourceSpan,
    },
    /// Non-type declaration found inside a `.types.op` file.
    #[error("'{decl_kind}' declaration '{decl_name}' is not allowed in types file '{file_path}'")]
    #[diagnostic(
        code(opalescent::type_system::non_type_declaration_in_types_file),
        help(
            ".types.op files may only contain type declarations — move this declaration to a regular .op file"
        )
    )]
    NonTypeDeclarationInTypesFile {
        /// Kind of declaration (e.g., "function", "variable").
        decl_kind: String,
        /// Name of the declaration.
        decl_name: String,
        /// File path where the non-type declaration was found.
        file_path: String,
        #[label("not allowed in .types.op file")]
        /// Source span of the declaration.
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
