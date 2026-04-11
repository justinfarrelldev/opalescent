//! Type checker implementation for the Opalescent type system

#![allow(
    clippy::multiple_inherent_impl,
    reason = "TypeChecker impl blocks intentionally split across submodules (checker/*.rs) for code organization - each submodule handles a specific aspect of type checking"
)]

extern crate alloc;

use super::constraints::TypeConstraint;
use super::environment::TypeEnvironment;
use super::errors::{TypeError, Warning};
use super::substitution::Substitution;
use super::symbol_table::{SymbolInfo, SymbolTable, SymbolType, Visibility};
use super::types::{CoreType, TypeVar};
use crate::ast::Type;
use crate::token::Span;
use crate::type_system::arithmetic::ArithmeticMode;
use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
use hot_reload::FunctionHotReloadMetadata;

// Sub-modules
mod call_resolution;
mod control_flow;
mod declarations;
mod expr_collections;
mod expressions;
mod helpers;
mod hot_reload;
/// Pattern-matching typing and exhaustiveness checks.
mod patterns;
mod returns;
mod statements;
mod unification;

/// Labeling mode tracked for return statements within a function/lambda body.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReturnLabelMode {
    /// No return statement has been analyzed yet for the current body.
    Unknown,
    /// Return statements are unlabeled.
    Unlabeled,
    /// Return statements use a fixed ordered label set.
    Labeled(Vec<String>),
}

/// Core type checker responsible for validating and inferring types
/// throughout the Opalescent type system
pub struct TypeChecker {
    /// Current type environment
    environment: TypeEnvironment,
    /// Counter for generating fresh type variables
    next_var_id: usize,
    /// Symbol table for tracking symbols in scope (Phase 2 and Phase 6)
    symbol_table: SymbolTable,
    /// Collected type constraints for inference (Phase 2)
    constraints: Vec<TypeConstraint>,
    /// Nesting depth of guard `else` handlers currently being type checked
    guard_else_depth: usize,
    /// Stack tracking the error types handled by active guard else branches
    guard_error_stack: Vec<Vec<CoreType>>,
    /// Stack tracking return label mode for active function/lambda bodies.
    return_label_modes: Vec<ReturnLabelMode>,
    /// Collected non-fatal warnings produced while type checking.
    warnings: Vec<Warning>,
    /// Cached function signatures for hot-reload compatibility checks.
    function_hot_reload_metadata: BTreeMap<String, FunctionHotReloadMetadata>,
    /// Per-expression arithmetic overflow semantics for later code generation.
    arithmetic_modes: BTreeMap<usize, ArithmeticMode>,
    /// Per-expression integer constant folding results for compile-time analysis.
    constant_integer_values: BTreeMap<usize, i128>,
    /// Sum-type variant registry used for match exhaustiveness checks.
    adt_variants: BTreeMap<String, Vec<String>>,
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        let mut checker = Self {
            environment: TypeEnvironment::new(),
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
            guard_else_depth: 0,
            guard_error_stack: Vec::new(),
            return_label_modes: Vec::new(),
            warnings: Vec::new(),
            function_hot_reload_metadata: BTreeMap::new(),
            arithmetic_modes: BTreeMap::new(),
            constant_integer_values: BTreeMap::new(),
            adt_variants: BTreeMap::new(),
        };
        checker.register_standard_builtins();
        checker
    }

    /// Create a type checker with a specific environment
    pub fn with_environment(environment: TypeEnvironment) -> Self {
        let mut checker = Self {
            environment,
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
            guard_else_depth: 0,
            guard_error_stack: Vec::new(),
            return_label_modes: Vec::new(),
            warnings: Vec::new(),
            function_hot_reload_metadata: BTreeMap::new(),
            arithmetic_modes: BTreeMap::new(),
            constant_integer_values: BTreeMap::new(),
            adt_variants: BTreeMap::new(),
        };
        checker.register_standard_builtins();
        checker
    }

    /// Register all phase-2 standard-library built-in signatures.
    fn register_standard_builtins(&mut self) {
        let generic_print_param = CoreType::Variable(TypeVar::new(0, "T".to_owned()));

        let print_signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![generic_print_param],
            return_types: vec![CoreType::Unit],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin("print".to_owned(), print_signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: "print".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: print_signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });

        let take_input_signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::String],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin("take_input".to_owned(), take_input_signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: "take_input".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: take_input_signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });

        let string_to_int32_signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![CoreType::String],
            return_types: vec![CoreType::Int32],
            error_types: vec![CoreType::Generic {
                name: "ParseError".to_owned(),
                type_args: Vec::new(),
            }],
        };
        self.environment.register_builtin(
            "string_to_int32".to_owned(),
            string_to_int32_signature.clone(),
        );
        self.symbol_table.register(SymbolInfo {
            name: "string_to_int32".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: string_to_int32_signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });

        self.environment.register_type(
            "Option".to_owned(),
            CoreType::Generic {
                name: "Option".to_owned(),
                type_args: Vec::new(),
            },
        );

        let random_int32_signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![CoreType::Int32, CoreType::Int32],
            return_types: vec![CoreType::Int32],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin("random_int32".to_owned(), random_int32_signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: "random_int32".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: random_int32_signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });

        self.register_integer_intrinsics_for_type("int8", &CoreType::Int8);
        self.register_integer_intrinsics_for_type("int16", &CoreType::Int16);
        self.register_integer_intrinsics_for_type("int32", &CoreType::Int32);
        self.register_integer_intrinsics_for_type("int64", &CoreType::Int64);
        self.register_integer_intrinsics_for_type("uint8", &CoreType::UInt8);
        self.register_integer_intrinsics_for_type("uint16", &CoreType::UInt16);
        self.register_integer_intrinsics_for_type("uint32", &CoreType::UInt32);
        self.register_integer_intrinsics_for_type("uint64", &CoreType::UInt64);
    }

    /// Register arithmetic intrinsics for a concrete integer type name.
    fn register_integer_intrinsics_for_type(&mut self, type_name: &str, integer_type: &CoreType) {
        self.register_integer_checked_intrinsic(type_name, "checked_add", integer_type);
        self.register_integer_checked_intrinsic(type_name, "checked_sub", integer_type);
        self.register_integer_checked_intrinsic(type_name, "checked_mul", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "wrapping_add", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "wrapping_sub", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "wrapping_mul", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "saturating_add", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "saturating_sub", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "saturating_mul", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "bshl_masked", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "bshr_masked", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "masked_bshl", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "masked_bshr", integer_type);
        self.register_integer_same_type_intrinsic(type_name, "masked_bushr", integer_type);
    }

    /// Register a checked arithmetic intrinsic that returns `Option<T>`.
    fn register_integer_checked_intrinsic(
        &mut self,
        type_name: &str,
        method_name: &str,
        integer_type: &CoreType,
    ) {
        let qualified_name = format!("{type_name}.{method_name}");
        let signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![integer_type.clone()],
            return_types: vec![CoreType::Generic {
                name: "Option".to_owned(),
                type_args: vec![integer_type.clone()],
            }],
            error_types: Vec::new(),
        };

        self.environment
            .register_builtin(qualified_name.clone(), signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: qualified_name,
            symbol_type: SymbolType::Function,
            core_type: signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }

    /// Register a same-type arithmetic intrinsic that returns `T`.
    fn register_integer_same_type_intrinsic(
        &mut self,
        type_name: &str,
        method_name: &str,
        integer_type: &CoreType,
    ) {
        let qualified_name = format!("{type_name}.{method_name}");
        let signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![integer_type.clone()],
            return_types: vec![integer_type.clone()],
            error_types: Vec::new(),
        };

        self.environment
            .register_builtin(qualified_name.clone(), signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: qualified_name,
            symbol_type: SymbolType::Function,
            core_type: signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }

    /// Get a reference to the current environment
    pub const fn environment(&self) -> &TypeEnvironment {
        &self.environment
    }

    /// Get a mutable reference to the current environment
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Cannot have const fn with mutable reference"
    )]
    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.environment
    }

    /// Get a reference to the symbol table
    pub const fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub const fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Get all warnings collected so far.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec deref coercion to slice is not allowed in const fn"
    )]
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// Clear all collected warnings.
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    /// Push a warning into the checker warning collection.
    pub fn push_warning(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Record arithmetic overflow semantics metadata for a typed expression.
    pub fn record_arithmetic_mode(&mut self, expr_id: usize, mode: ArithmeticMode) {
        self.arithmetic_modes.insert(expr_id, mode);
    }

    /// Query arithmetic overflow semantics metadata for an expression id.
    pub fn arithmetic_mode_for_expr(&self, expr_id: usize) -> Option<ArithmeticMode> {
        self.arithmetic_modes.get(&expr_id).copied()
    }

    /// Store folded integer constant metadata for an expression id.
    pub fn record_constant_integer_value(&mut self, expr_id: usize, value: i128) {
        self.constant_integer_values.insert(expr_id, value);
    }

    /// Query folded integer constant metadata for an expression id.
    pub fn constant_integer_for_expr(&self, expr_id: usize) -> Option<i128> {
        self.constant_integer_values.get(&expr_id).copied()
    }

    /// Clear folded integer constant metadata for one expression id.
    pub fn clear_constant_integer_value(&mut self, expr_id: usize) {
        self.constant_integer_values.remove(&expr_id);
    }

    /// Clear all per-expression arithmetic metadata caches.
    pub fn clear_expression_metadata(&mut self) {
        self.arithmetic_modes.clear();
        self.constant_integer_values.clear();
    }

    /// Register a symbol for ABI signature generation (Phase 6)
    pub fn register_symbol(&mut self, symbol: SymbolInfo) {
        self.symbol_table.register(symbol);
    }

    /// Add a type constraint for inference (Phase 2)
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    /// Get all collected constraints
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec deref coercion to slice is not allowed in const fn"
    )]
    pub fn constraints(&self) -> &[TypeConstraint] {
        &self.constraints
    }

    /// Clear all collected constraints
    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }

    /// Solve all collected constraints (Phase 2 - not yet implemented)
    ///
    /// This will be the main entry point for constraint-based type inference.
    /// It should unify all constraints and return a substitution that satisfies them all.
    ///
    /// # Span Integration Status
    ///
    /// This method attempts to provide precise source spans for all diagnostics. However,
    /// some constraints may have `None` spans when:
    /// - The constraint was generated by the compiler (not directly from user code)
    /// - The constraint originated from a synthesized type during inference
    /// - The constraint involves implicit conversions or compiler-generated code
    ///
    /// In these cases, `unknown_span()` is used as a fallback. The goal is to eliminate
    /// all `unknown_span()` usage by ensuring all constraints capture their originating
    /// source locations during AST traversal.
    ///
    /// TODO(Phase 2): Track and eliminate all remaining `unknown_span()` usage. Consider
    /// adding debug assertions in test builds to detect constraints with missing spans.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if constraints cannot be satisfied.
    pub fn solve_constraints(&mut self) -> Result<Substitution, TypeError> {
        let pending_constraints = core::mem::take(&mut self.constraints);
        let mut substitution = Substitution::empty();

        for constraint in pending_constraints {
            match constraint {
                TypeConstraint::Equality {
                    left,
                    right,
                    left_span,
                    right_span,
                } => {
                    let left_applied = substitution.apply(&left);
                    let right_applied = substitution.apply(&right);
                    let constraint_subst =
                        self.unify(&left_applied, &right_applied, left_span, right_span)?;
                    substitution = substitution.compose(&constraint_subst);
                }
                TypeConstraint::HasField {
                    owner_span,
                    field_span,
                    ..
                } => {
                    // Prefer primary span, fall back to secondary, then unknown
                    // NOTE: HasField constraints should always have at least one span
                    let diagnostic_span = owner_span
                        .or(field_span)
                        .map_or_else(TypeError::unknown_span, TypeError::span_from_span);
                    return Err(TypeError::NotImplementedYet {
                        feature: "constraint type HasField".to_owned(),
                        span: diagnostic_span,
                    });
                }
                TypeConstraint::Callable {
                    callee,
                    arguments,
                    return_type,
                    callee_span,
                    argument_spans,
                    return_span,
                } => {
                    // Apply current substitution to callee type
                    let callee_applied = substitution.apply(&callee);

                    // Extract function signature from callee type
                    if let CoreType::Function {
                        parameters,
                        return_types,
                        error_types: _fn_errors,
                        ..
                    } = callee_applied
                    {
                        // Check arity (number of arguments)
                        if parameters.len() != arguments.len() {
                            let diagnostic_span = callee_span
                                .map_or_else(TypeError::unknown_span, TypeError::span_from_span);
                            return Err(TypeError::ArityMismatch {
                                expected: parameters.len(),
                                found: arguments.len(),
                                span: diagnostic_span,
                            });
                        }

                        // Unify each parameter with corresponding argument
                        for (i, (param_type, arg_type)) in
                            parameters.iter().zip(arguments.iter()).enumerate()
                        {
                            let param_applied = substitution.apply(param_type);
                            let arg_applied = substitution.apply(arg_type);
                            let arg_span = argument_spans.get(i).and_then(|s| *s).or(callee_span);
                            let param_subst =
                                self.unify(&param_applied, &arg_applied, None, arg_span)?;
                            substitution = substitution.compose(&param_subst);
                        }

                        if return_types.len() != 1 {
                            let diagnostic_span = callee_span
                                .map_or_else(TypeError::unknown_span, TypeError::span_from_span);
                            return Err(TypeError::ArityMismatch {
                                expected: 1,
                                found: return_types.len(),
                                span: diagnostic_span,
                            });
                        }

                        if let Some(function_return_type) = return_types.first() {
                            let fn_return_applied = substitution.apply(function_return_type);
                            let return_type_applied = substitution.apply(&return_type);
                            let return_subst = self.unify(
                                &fn_return_applied,
                                &return_type_applied,
                                callee_span,
                                return_span,
                            )?;
                            substitution = substitution.compose(&return_subst);
                        }
                    } else {
                        // Callee is not a function type
                        let diagnostic_span = callee_span
                            .map_or_else(TypeError::unknown_span, TypeError::span_from_span);
                        return Err(TypeError::NotCallable {
                            type_name: format!("{callee_applied}"),
                            span: diagnostic_span,
                        });
                    }
                }
            }
        }

        Ok(substitution)
    }

    /// Generate a fresh type variable
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the type variable
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if type variable ID overflows
    pub fn fresh_type_var(&mut self, name: String, span: Span) -> Result<CoreType, TypeError> {
        let var = TypeVar::new(self.next_var_id, name);
        self.next_var_id =
            self.next_var_id
                .checked_add(1)
                .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                    reason: "type variable counter overflow".to_owned(),
                    span: TypeError::span_from_span(span),
                })?;
        Ok(CoreType::Variable(var))
    }

    /// Generate a fresh type variable with an auto-generated name
    ///
    /// # Arguments
    ///
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if type variable ID overflows
    pub fn fresh_type_var_auto(&mut self, span: Span) -> Result<CoreType, TypeError> {
        self.fresh_type_var(format!("t{}", self.next_var_id), span)
    }

    /// Convert an AST Type to a `CoreType` for validation and instantiation
    /// Supports generics, arrays, and function types.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when type conversion fails
    pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, span } => match name.as_str() {
                "int8" => Ok(CoreType::Int8),
                "int16" => Ok(CoreType::Int16),
                "int32" => Ok(CoreType::Int32),
                "int64" => Ok(CoreType::Int64),
                "uint8" => Ok(CoreType::UInt8),
                "uint16" => Ok(CoreType::UInt16),
                "uint32" => Ok(CoreType::UInt32),
                "uint64" => Ok(CoreType::UInt64),
                "float32" => Ok(CoreType::Float32),
                "float64" => Ok(CoreType::Float64),
                "string" => Ok(CoreType::String),
                "boolean" => Ok(CoreType::Boolean),
                "unit" | "void" => Ok(CoreType::Unit),
                _ => Err(TypeError::TypeNotFound {
                    type_name: name.clone(),
                    span: TypeError::span_from_span(span),
                }),
            },
            Type::Array {
                ref element_type, ..
            } => {
                let core_element = Self::ast_type_to_core_type(element_type.as_ref())?;
                Ok(CoreType::Array(Box::new(core_element)))
            }
            Type::Function {
                ref parameters,
                ref return_types,
                ..
            } => {
                let mut core_params = Vec::with_capacity(parameters.len());
                for param in parameters {
                    core_params.push(Self::ast_type_to_core_type(param)?);
                }
                let mut core_return_types = Vec::with_capacity(return_types.len());
                for return_type in return_types {
                    core_return_types.push(Self::ast_type_to_core_type(return_type)?);
                }
                // NOTE: Function type node may carry an errors clause for pretty-printing
                // and documentation. For conversion, we attempt to resolve these types; if a
                // type is unknown at this stage (e.g., custom ADT not yet declared), we map
                // it into a nominal `Generic { name, args: [] }` form to preserve intent
                // without failing conversion.
                let mut core_errors: Vec<CoreType> = Vec::new();
                if let Type::Function { ref errors, .. } = *ast_type {
                    if let Some(list) = errors.as_ref() {
                        for err_ty in list {
                            match Self::ast_type_to_core_type(err_ty) {
                                Ok(core) => core_errors.push(core),
                                Err(TypeError::TypeNotFound { type_name, .. }) => {
                                    core_errors.push(CoreType::Generic {
                                        name: type_name,
                                        type_args: Vec::new(),
                                    });
                                }
                                Err(other) => return Err(other),
                            }
                        }
                    }
                }

                Ok(CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: core_params,
                    return_types: core_return_types,
                    error_types: core_errors,
                })
            }
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let mut core_args = Vec::with_capacity(type_args.len());
                for arg in type_args {
                    core_args.push(Self::ast_type_to_core_type(arg)?);
                }
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: core_args,
                })
            }
        }
    }

    /// Resolve error type names into nominal [`CoreType`]s using the type environment.
    ///
    /// This ensures that error declarations reference existing types and produces an
    /// [`UndeclaredErrorType`](TypeError::UndeclaredErrorType) diagnostic when a name
    /// cannot be resolved.
    fn resolve_error_types(
        &self,
        error_names: &[String],
        span: Span,
    ) -> Result<Vec<CoreType>, TypeError> {
        let mut resolved = Vec::with_capacity(error_names.len());
        for name in error_names {
            match self.environment.lookup_type(name, span) {
                Ok(core_type) => resolved.push(core_type.clone()),
                Err(_) => {
                    return Err(TypeError::UndeclaredErrorType {
                        name: name.clone(),
                        span: TypeError::span_from_span(span),
                    });
                }
            }
        }
        Ok(resolved)
    }

    /// Validate algebraic data type definitions against the known type environment to ensure all
    /// referenced field and variant types are resolvable.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when ADT validation fails
    pub fn validate_adt_type(type_def: &crate::ast::TypeDef) -> Result<(), TypeError> {
        match *type_def {
            crate::ast::TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        Self::ast_type_to_core_type(&field.type_annotation)?;
                    }
                }
            }
            crate::ast::TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    Self::ast_type_to_core_type(&field.type_annotation)?;
                }
            }
            crate::ast::TypeDef::Alias {
                ref target_type, ..
            } => {
                Self::ast_type_to_core_type(target_type)?;
            }
        }
        Ok(())
    }

    /// Type check a pattern match expression
    /// Ensures all patterns and arms are type compatible
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when pattern matching validation fails
    pub fn type_check_pattern_match(
        &self,
        matched_type: &CoreType,
        matched_span: Span,
        patterns: &[(CoreType, Span)],
        arm_types: &[(CoreType, Span)],
    ) -> Result<(), TypeError> {
        for pattern in patterns {
            let pattern_type = &pattern.0;
            let pattern_span = pattern.1;
            if !self.types_compatible(matched_type, pattern_type) {
                return Err(TypeError::TypeMismatch {
                    expected: matched_type.to_string(),
                    found: pattern_type.to_string(),
                    found_span: TypeError::span_from_span(pattern_span),
                    expected_span: Some(TypeError::span_from_span(matched_span)),
                });
            }
        }

        if let Some(first) = arm_types.first() {
            let first_type = &first.0;
            let first_span = first.1;
            for arm in arm_types {
                let arm_type = &arm.0;
                let arm_span = arm.1;
                if !self.types_compatible(first_type, arm_type) {
                    return Err(TypeError::TypeMismatch {
                        expected: first_type.to_string(),
                        found: arm_type.to_string(),
                        found_span: TypeError::span_from_span(arm_span),
                        expected_span: Some(TypeError::span_from_span(first_span)),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if two core types are structurally compatible (including nested types)
    ///
    /// This method performs deep structural comparison for complex types like
    /// arrays, functions, and generics, ensuring all nested components are compatible.
    /// For simple equality checking, use the `==` operator on `CoreType` directly.
    #[expect(
        clippy::only_used_in_recursion,
        reason = "self parameter needed for structural recursion"
    )]
    pub fn types_compatible(&self, left: &CoreType, right: &CoreType) -> bool {
        // Clone to owned values to allow safe pattern matching without moving out of borrows.
        // This trades some performance for clarity and lint compliance; core types are small.
        let l = left.clone();
        let r = right.clone();
        match (l, r) {
            // All primitive types
            (CoreType::Int8, CoreType::Int8)
            | (CoreType::Int16, CoreType::Int16)
            | (CoreType::Int32, CoreType::Int32)
            | (CoreType::Int64, CoreType::Int64)
            | (CoreType::UInt8, CoreType::UInt8)
            | (CoreType::UInt16, CoreType::UInt16)
            | (CoreType::UInt32, CoreType::UInt32)
            | (CoreType::UInt64, CoreType::UInt64)
            | (CoreType::Float32, CoreType::Float32)
            | (CoreType::Float64, CoreType::Float64)
            | (CoreType::Boolean, CoreType::Boolean)
            | (CoreType::String, CoreType::String)
            | (CoreType::Unit, CoreType::Unit) => true,

            // Type variables are compatible with themselves
            (CoreType::Variable(var1), CoreType::Variable(var2)) => var1.id == var2.id,

            // Arrays are compatible if their element types are compatible
            (CoreType::Array(left_elem), CoreType::Array(right_elem)) => {
                self.types_compatible(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions are compatible if parameters and return types are compatible
            (
                CoreType::Function {
                    parameters: left_params,
                    return_types: left_returns,
                    error_types: left_errors,
                    ..
                },
                CoreType::Function {
                    parameters: right_params,
                    return_types: right_returns,
                    error_types: right_errors,
                    ..
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return false;
                }
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    if !self.types_compatible(left_param, right_param) {
                        return false;
                    }
                }
                if left_returns.len() != right_returns.len() {
                    return false;
                }
                for (left_ret, right_ret) in left_returns.iter().zip(right_returns.iter()) {
                    if !self.types_compatible(left_ret, right_ret) {
                        return false;
                    }
                }
                if left_errors.len() != right_errors.len() {
                    return false;
                }
                for (le, re) in left_errors.iter().zip(right_errors.iter()) {
                    if !self.types_compatible(le, re) {
                        return false;
                    }
                }
                true
            }

            // Generic types are compatible if names and type arguments match
            (
                CoreType::Generic {
                    name: left_name,
                    type_args: left_args,
                },
                CoreType::Generic {
                    name: right_name,
                    type_args: right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return false;
                }
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    if !self.types_compatible(left_arg, right_arg) {
                        return false;
                    }
                }
                true
            }

            // Different types are not compatible
            _ => false,
        }
    }

    /// Validate a cast expression and classify it as safe or unsafe.
    ///
    /// See [`is_safe_cast`](super::checker::helpers::is_safe_cast) for detailed cast safety rules.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::InvalidCast` if the cast is not valid (non-numeric types).
    pub fn validate_cast(
        from_type: &CoreType,
        to_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        use super::checker::helpers::is_valid_cast;

        if !is_valid_cast(from_type, to_type) {
            return Err(TypeError::InvalidCast {
                from_type: from_type.to_string(),
                to_type: to_type.to_string(),
                span: TypeError::span_from_span(span),
            });
        }

        Ok(())
    }

    /// Validate a cast and collect warning diagnostics for non-fatal unsafe conversions.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::InvalidCast` if the cast is not valid.
    pub fn validate_cast_with_warnings(
        &mut self,
        from_type: &CoreType,
        to_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        use super::checker::helpers::is_safe_cast;

        Self::validate_cast(from_type, to_type, span)?;

        if !is_safe_cast(from_type, to_type) {
            self.push_warning(Warning::UnsafeCast {
                from_type: format!("{from_type}"),
                to_type: format!("{to_type}"),
                span: TypeError::span_from_span(span),
                suppression_annotation: None,
            });
        }

        Ok(())
    }

    /// Execute a closure within a fresh lexical scope, ensuring the scope is
    /// entered and exited even when the closure returns early.
    pub(super) fn within_new_scope<F, R>(&mut self, action: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.symbol_table.enter_scope();
        let result = action(self);
        self.symbol_table.exit_scope();
        result
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
