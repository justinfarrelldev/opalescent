//! Type checker implementation for the Opalescent type system

extern crate alloc;

use super::constraints::TypeConstraint;
use super::environment::TypeEnvironment;
use super::errors::TypeError;
use super::substitution::Substitution;
use super::symbol_table::{SymbolInfo, SymbolTable, SymbolType, Visibility};
use super::types::{CoreType, NumericKind, TypeVar};
use crate::ast::{
    AstNode, BinaryOp, Decl, Expr, LambdaBody, LetBinding, LiteralValue, Parameter, Program, Stmt,
    StringPart, Type, UnaryOp, Visibility as AstVisibility,
};
use crate::token::Span;
use alloc::{boxed::Box, format, string::String, vec, vec::Vec};

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
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        Self {
            environment: TypeEnvironment::new(),
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
        }
    }

    /// Create a type checker with a specific environment
    pub fn with_environment(environment: TypeEnvironment) -> Self {
        Self {
            environment,
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
        }
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
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if constraints cannot be satisfied.
    pub fn solve_constraints(&mut self) -> Result<Substitution, TypeError> {
        let pending_constraints = core::mem::take(&mut self.constraints);
        let mut substitution = Substitution::empty();

        for constraint in pending_constraints {
            match constraint {
                TypeConstraint::Equality(left, right) => {
                    let left_applied = substitution.apply(&left);
                    let right_applied = substitution.apply(&right);
                    let new_substitution = self.unify(&left_applied, &right_applied)?;
                    substitution = new_substitution.compose(&substitution);
                }
                TypeConstraint::HasField(_, field, _) => {
                    return Err(TypeError::NotImplementedYet {
                        feature: format!("has-field constraint solving for field '{field}'"),
                        span: TypeError::unknown_span(),
                    });
                }
                TypeConstraint::Callable(_, _, _) => {
                    return Err(TypeError::NotImplementedYet {
                        feature: "callable constraint solving".to_owned(),
                        span: TypeError::unknown_span(),
                    });
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
    pub fn fresh_type_var(&mut self, name: String, span: Span) -> Result<CoreType, TypeError> {
        let var = TypeVar::new(self.next_var_id, name);
        self.next_var_id =
            self.next_var_id
                .checked_add(1)
                .ok_or_else(|| TypeError::TypeVariableOverflow {
                    span: TypeError::span_from_span(span),
                })?;
        Ok(CoreType::Variable(var))
    }

    /// Generate a fresh type variable with an auto-generated name
    ///
    /// # Arguments
    ///
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    pub fn fresh_type_var_auto(&mut self, span: Span) -> Result<CoreType, TypeError> {
        self.fresh_type_var(format!("t{}", self.next_var_id), span)
    }

    /// Convert an AST Type to a `CoreType` for validation and instantiation
    /// Supports generics, arrays, and function types.
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
                "unit" => Ok(CoreType::Unit),
                _ => Err(TypeError::TypeNotFound {
                    type_name: name.clone(),
                    span: TypeError::span_from_span(span),
                }),
            },
            Type::Array {
                ref element_type, ..
            } => {
                let elem_core = Self::ast_type_to_core_type(element_type.as_ref())?;
                Ok(CoreType::Array(Box::new(elem_core)))
            }
            Type::Function {
                ref parameters,
                ref return_type,
                ..
            } => {
                let mut param_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    param_types.push(Self::ast_type_to_core_type(param)?);
                }
                let ret_type = Self::ast_type_to_core_type(return_type.as_ref())?;
                Ok(CoreType::Function {
                    parameters: param_types,
                    return_type: Box::new(ret_type),
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

    /// Validate algebraic data type definitions against the known type environment to ensure all
    /// referenced field and variant types are resolvable.
    pub fn validate_adt_type(&self, type_def: &crate::ast::TypeDef) -> Result<(), TypeError> {
        match *type_def {
            crate::ast::TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                        self.validate_type_name(&field.name, &core_field_type, field.span)?;
                    }
                }
                Ok(())
            }
            crate::ast::TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                    self.validate_type_name(&field.name, &core_field_type, field.span)?;
                }
                Ok(())
            }
            crate::ast::TypeDef::Alias {
                ref target_type, ..
            } => {
                let _: CoreType = Self::ast_type_to_core_type(target_type)?;
                Ok(())
            }
        }
    }
    /// Type check a pattern match expression
    /// Ensures all patterns and arms are type compatible
    pub fn type_check_pattern_match(
        &self,
        matched_type: &CoreType,
        patterns: &[CoreType],
        arm_types: &[CoreType],
    ) -> Result<(), TypeError> {
        // Each pattern must be compatible with matched_type
        for pat in patterns {
            if !self.types_compatible(matched_type, pat) {
                return Err(TypeError::TypeMismatch {
                    expected: matched_type.to_string(),
                    found: pat.to_string(),
                    found_span: TypeError::unknown_span(),
                    expected_span: None,
                });
            }
        }
        // All arm types must be compatible with each other
        if let Some(first) = arm_types.first() {
            for arm in arm_types {
                if !self.types_compatible(first, arm) {
                    return Err(TypeError::TypeMismatch {
                        expected: first.to_string(),
                        found: arm.to_string(),
                        found_span: TypeError::unknown_span(),
                        expected_span: None,
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
        match (left, right) {
            // All primitive types
            (&CoreType::Int8, &CoreType::Int8)
            | (&CoreType::Int16, &CoreType::Int16)
            | (&CoreType::Int32, &CoreType::Int32)
            | (&CoreType::Int64, &CoreType::Int64)
            | (&CoreType::UInt8, &CoreType::UInt8)
            | (&CoreType::UInt16, &CoreType::UInt16)
            | (&CoreType::UInt32, &CoreType::UInt32)
            | (&CoreType::UInt64, &CoreType::UInt64)
            | (&CoreType::Float32, &CoreType::Float32)
            | (&CoreType::Float64, &CoreType::Float64)
            | (&CoreType::String, &CoreType::String)
            | (&CoreType::Boolean, &CoreType::Boolean)
            | (&CoreType::Unit, &CoreType::Unit) => true,

            // Variables are equal if they have the same ID
            (&CoreType::Variable(ref left_var), &CoreType::Variable(ref right_var)) => {
                left_var.id == right_var.id
            }

            // Arrays are compatible if element types are compatible
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.types_compatible(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions are compatible if parameters and return types are compatible
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                left_params.len() == right_params.len()
                    && left_params
                        .iter()
                        .zip(right_params.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
                    && self.types_compatible(left_ret.as_ref(), right_ret.as_ref())
            }

            // Generic types are compatible if names and type arguments are compatible
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                left_name == right_name
                    && left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .zip(right_args.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
            }

            // Different types are not compatible
            _ => false,
        }
    }

    /// Type check an expression and return its [`CoreType`]
    ///
    /// # Errors
    /// Returns `TypeError` variants when expression typing fails.
    pub fn type_check_expr(&mut self, expr: &Expr) -> Result<CoreType, TypeError> {
        match *expr {
            Expr::Literal { ref value, .. } => Ok(Self::literal_to_core_type(value)),
            Expr::Identifier { ref name, span, .. } => self.resolve_identifier(name, span),
            Expr::Parenthesized { ref expr, .. } => self.type_check_expr(expr),
            Expr::Binary {
                ref left,
                ref operator,
                ref right,
                span,
                ..
            } => self.type_check_binary_expr(left.as_ref(), operator, right.as_ref(), span),
            Expr::Unary {
                ref operator,
                ref operand,
                span,
                ..
            } => self.type_check_unary_expr(operator, operand.as_ref(), span),
            Expr::Call {
                ref callee,
                ref args,
                span,
                ..
            } => self.type_check_call_expr(callee.as_ref(), args.as_slice(), span),
            Expr::Index {
                ref object,
                ref index,
                span,
                ..
            } => self.type_check_index_expr(object.as_ref(), index.as_ref(), span),
            Expr::Member { span, .. } => Err(TypeError::NotImplementedYet {
                feature: "member access type checking".to_owned(),
                span: TypeError::span_from_span(span),
            }),
            Expr::Cast {
                ref expr,
                ref target_type,
                span,
                ..
            } => self.type_check_cast_expr(expr.as_ref(), target_type, span),
            Expr::TypeOf { ref expr, .. } => {
                self.type_check_expr(expr.as_ref())?;
                Ok(CoreType::String)
            }
            Expr::StringInterpolation {
                ref parts, span, ..
            } => {
                self.type_check_string_interpolation(parts.as_slice(), span)?;
                Ok(CoreType::String)
            }
            Expr::Array {
                ref elements, span, ..
            } => self.type_check_array_expr(elements.as_slice(), span),
            Expr::Lambda {
                ref generic_params,
                ref params,
                ref return_type,
                ref body,
                span,
                ..
            } => self.type_check_lambda_expr(
                generic_params.as_deref(),
                params.as_slice(),
                return_type,
                body,
                span,
            ),
        }
    }

    /// Determine the canonical [`CoreType`] for a literal value.
    const fn literal_to_core_type(value: &LiteralValue) -> CoreType {
        match *value {
            LiteralValue::Integer(_) => CoreType::Int64,
            LiteralValue::Float(_) => CoreType::Float64,
            LiteralValue::String(_) => CoreType::String,
            LiteralValue::Boolean(_) => CoreType::Boolean,
            LiteralValue::Void => CoreType::Unit,
        }
    }

    /// Resolve an identifier to its registered core type or emit a symbol error.
    fn resolve_identifier(&self, name: &str, span: Span) -> Result<CoreType, TypeError> {
        self.symbol_table()
            .lookup(name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::SymbolNotFound {
                name: name.to_owned(),
                span: TypeError::span_from_span(span),
            })
    }

    /// Categorize a core type into a numeric family when applicable.
    const fn classify_numeric(core_type: &CoreType) -> Option<NumericKind> {
        match *core_type {
            CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64 => {
                Some(NumericKind::SignedInt)
            }
            CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64 => {
                Some(NumericKind::UnsignedInt)
            }
            CoreType::Float32 | CoreType::Float64 => Some(NumericKind::Float),
            _ => None,
        }
    }

    /// Check whether the provided type belongs to any numeric family.
    const fn is_numeric_type(core_type: &CoreType) -> bool {
        Self::classify_numeric(core_type).is_some()
    }

    /// Check whether the provided type is an integer (signed or unsigned).
    const fn is_integer_type(core_type: &CoreType) -> bool {
        matches!(
            Self::classify_numeric(core_type),
            Some(NumericKind::SignedInt | NumericKind::UnsignedInt)
        )
    }

    /// Check whether the provided type is a floating point primitive.
    const fn is_float_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::Float32 | &CoreType::Float64)
    }

    /// Check whether the provided type is the boolean primitive.
    const fn is_boolean_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::Boolean)
    }

    /// Check whether the provided type is the string primitive.
    const fn is_string_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::String)
    }

    /// Construct a type error describing an invalid operation on a type.
    fn invalid_operation_error(operation: &str, core_type: &CoreType, span: Span) -> TypeError {
        TypeError::InvalidOperation {
            operation: operation.to_owned(),
            type_name: core_type.to_string(),
            span: TypeError::span_from_span(span),
        }
    }

    /// Construct a type mismatch diagnostic with consistent formatting.
    fn type_mismatch_error(
        expected: &CoreType,
        expected_span: Option<Span>,
        found: &CoreType,
        found_span: Span,
    ) -> TypeError {
        TypeError::TypeMismatch {
            expected: expected.to_string(),
            found: found.to_string(),
            found_span: TypeError::span_from_span(found_span),
            expected_span: expected_span.map(TypeError::span_from_span),
        }
    }

    /// Attempt to coerce a literal expression's type to match an expected core type.
    fn coerce_literal_to_expected(
        expected: &CoreType,
        expr: &Expr,
        actual: &CoreType,
    ) -> Option<CoreType> {
        match *expr {
            Expr::Literal { ref value, .. } => match *value {
                LiteralValue::Integer(_) => (Self::is_integer_type(expected)
                    && Self::is_integer_type(actual))
                .then(|| expected.clone()),
                LiteralValue::Float(_) => (Self::is_float_type(expected)
                    && Self::is_float_type(actual))
                .then(|| expected.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Ensure that two resolved operand types are identical, capturing precise source spans
    /// for a subsequent diagnostic when they differ.
    fn ensure_same_type(
        expected: &CoreType,
        expected_span: Span,
        actual: &CoreType,
        actual_span: Span,
    ) -> Result<(), TypeError> {
        if expected == actual {
            Ok(())
        } else {
            Err(Self::type_mismatch_error(
                expected,
                Some(expected_span),
                actual,
                actual_span,
            ))
        }
    }

    /// Validate that a core type belongs to one of the numeric families prior to a numeric
    /// operation, preserving architectural guarantees about arithmetic safety.
    fn ensure_numeric_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_numeric_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Ensure that the provided type is an integer (signed or unsigned) before executing an
    /// integer-only operation, preventing silent lossy conversions.
    fn ensure_integer_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_integer_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Guard boolean-only operations so that only strict `boolean` operands are permitted,
    /// preserving logical semantics for control-flow constructs.
    fn ensure_boolean_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_boolean_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Provide a human-readable description for a binary operator, feeding into diagnostics
    /// and future telemetry without repeating strings across the code base.
    const fn binary_operation_name(operator: &BinaryOp) -> &'static str {
        match *operator {
            BinaryOp::Add => "addition",
            BinaryOp::Subtract => "subtraction",
            BinaryOp::Multiply => "multiplication",
            BinaryOp::Divide => "division",
            BinaryOp::Modulo => "modulo",
            BinaryOp::Power => "exponentiation",
            BinaryOp::Equal => "equality comparison",
            BinaryOp::NotEqual => "inequality comparison",
            BinaryOp::Less => "less-than comparison",
            BinaryOp::LessEqual => "less-or-equal comparison",
            BinaryOp::Greater => "greater-than comparison",
            BinaryOp::GreaterEqual => "greater-or-equal comparison",
            BinaryOp::Is => "identity comparison",
            BinaryOp::IsNot => "negative identity comparison",
            BinaryOp::And => "logical and",
            BinaryOp::Or => "logical or",
            BinaryOp::Xor => "logical xor",
            BinaryOp::BitAnd => "bitwise and",
            BinaryOp::BitOr => "bitwise or",
            BinaryOp::BitXor => "bitwise xor",
            BinaryOp::BitShiftLeft => "left shift",
            BinaryOp::BitShiftRight => "right shift",
            BinaryOp::BitUnsignedShiftRight => "unsigned right shift",
            BinaryOp::Assign => "assignment",
        }
    }

    /// Provide a human-readable description for unary operators so that diagnostics can
    /// reference intent rather than symbolic tokens alone.
    const fn unary_operation_name(operator: &UnaryOp) -> &'static str {
        match *operator {
            UnaryOp::Negate => "numeric negation",
            UnaryOp::Not => "logical not",
            UnaryOp::BitNot => "bitwise not",
            UnaryOp::Plus => "unary plus",
        }
    }

    /// Determine the numeric family and bit width for cast validation, enabling widening rules
    /// that mirror the language specification while keeping the data in a const context.
    const fn numeric_bit_width(core_type: &CoreType) -> Option<(NumericKind, u8)> {
        match *core_type {
            CoreType::Int8 => Some((NumericKind::SignedInt, 8)),
            CoreType::Int16 => Some((NumericKind::SignedInt, 16)),
            CoreType::Int32 => Some((NumericKind::SignedInt, 32)),
            CoreType::Int64 => Some((NumericKind::SignedInt, 64)),
            CoreType::UInt8 => Some((NumericKind::UnsignedInt, 8)),
            CoreType::UInt16 => Some((NumericKind::UnsignedInt, 16)),
            CoreType::UInt32 => Some((NumericKind::UnsignedInt, 32)),
            CoreType::UInt64 => Some((NumericKind::UnsignedInt, 64)),
            CoreType::Float32 => Some((NumericKind::Float, 32)),
            CoreType::Float64 => Some((NumericKind::Float, 64)),
            _ => None,
        }
    }

    /// Determine whether an implicit cast between numeric types is permitted under the
    /// language's widening rules. This intentionally excludes narrowing conversions and
    /// mixed-family casts unless explicitly sanctioned by the specification.
    fn is_cast_allowed(from: &CoreType, to: &CoreType) -> bool {
        if from == to {
            return true;
        }
        match (Self::numeric_bit_width(from), Self::numeric_bit_width(to)) {
            (
                Some((NumericKind::SignedInt, from_bits)),
                Some((NumericKind::SignedInt, to_bits)),
            )
            | (
                Some((NumericKind::UnsignedInt, from_bits)),
                Some((NumericKind::UnsignedInt, to_bits)),
            )
            | (Some((NumericKind::Float, from_bits)), Some((NumericKind::Float, to_bits))) => {
                from_bits <= to_bits
            }
            (
                Some((NumericKind::SignedInt | NumericKind::UnsignedInt, _)),
                Some((NumericKind::Float, _)),
            ) => true,
            _ => false,
        }
    }

    /// Type check a binary expression, enforcing operand compatibility, recording inference
    /// constraints, and returning the resulting core type for subsequent analysis.
    fn type_check_binary_expr(
        &mut self,
        left: &Expr,
        operator: &BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let left_type = self.type_check_expr(left)?;
        let right_type = self.type_check_expr(right)?;
        let op_name = Self::binary_operation_name(operator);

        match *operator {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Power => {
                if Self::is_string_type(&left_type) && Self::is_string_type(&right_type) {
                    return Ok(CoreType::String);
                }
                Self::ensure_numeric_type(&left_type, left.span(), op_name)?;
                Self::ensure_numeric_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::Modulo => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Is | BinaryOp::IsNot => {
                if !self.types_compatible(&left_type, &right_type) {
                    return Err(Self::type_mismatch_error(
                        &left_type,
                        Some(left.span()),
                        &right_type,
                        right.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(CoreType::Boolean)
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                Self::ensure_numeric_type(&left_type, left.span(), op_name)?;
                Self::ensure_numeric_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
                Self::ensure_boolean_type(&left_type, left.span(), op_name)?;
                Self::ensure_boolean_type(&right_type, right.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::BitShiftLeft | BinaryOp::BitShiftRight | BinaryOp::BitUnsignedShiftRight => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Ok(left_type)
            }
            BinaryOp::Assign => Err(Self::invalid_operation_error(op_name, &left_type, span)),
        }
    }

    /// Type check a unary expression, returning the deduced result type while enforcing the
    /// operator's domain constraints.
    fn type_check_unary_expr(
        &mut self,
        operator: &UnaryOp,
        operand: &Expr,
        _span: Span,
    ) -> Result<CoreType, TypeError> {
        let operand_type = self.type_check_expr(operand)?;
        let op_name = Self::unary_operation_name(operator);
        match *operator {
            UnaryOp::Negate | UnaryOp::Plus => {
                Self::ensure_numeric_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
            UnaryOp::Not => {
                Self::ensure_boolean_type(&operand_type, operand.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            UnaryOp::BitNot => {
                Self::ensure_integer_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
        }
    }

    /// Validate a function call, ensuring arity matches, arguments conform to parameter types,
    /// and recording equality constraints for the inference engine.
    fn type_check_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let callee_type = self.type_check_expr(callee)?;
        match callee_type {
            CoreType::Function {
                parameters,
                return_type,
            } => {
                if parameters.len() != args.len() {
                    return Err(TypeError::InvalidOperation {
                        operation: format!(
                            "function call expected {} arguments but received {}",
                            parameters.len(),
                            args.len()
                        ),
                        type_name: "function".to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                }

                for (index, arg_expr) in args.iter().enumerate() {
                    let param_type = parameters[index].clone();
                    let arg_type = self.type_check_expr(arg_expr)?;
                    let reconciled_type = if self.types_compatible(&param_type, &arg_type) {
                        arg_type
                    } else if let Some(adjusted) =
                        Self::coerce_literal_to_expected(&param_type, arg_expr, &arg_type)
                    {
                        adjusted
                    } else {
                        return Err(Self::type_mismatch_error(
                            &param_type,
                            None,
                            &arg_type,
                            arg_expr.span(),
                        ));
                    };
                    self.add_constraint(TypeConstraint::Equality(
                        param_type.clone(),
                        reconciled_type,
                    ));
                }

                Ok(*return_type)
            }
            other => Err(Self::invalid_operation_error("function call", &other, span)),
        }
    }

    /// Type check an array indexing operation, confirming integer indices and yielding the
    /// element type for subsequent evaluation.
    fn type_check_index_expr(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let object_type = self.type_check_expr(object)?;
        let index_type = self.type_check_expr(index)?;
        Self::ensure_integer_type(&index_type, index.span(), "indexing")?;
        match object_type {
            CoreType::Array(element_type) => Ok(*element_type),
            other => Err(Self::invalid_operation_error("indexing", &other, span)),
        }
    }

    /// Type check an explicit cast expression, leveraging the numeric widening rules to
    /// determine whether the conversion is permitted.
    fn type_check_cast_expr(
        &mut self,
        expr: &Expr,
        target_type: &Type,
        _span: Span,
    ) -> Result<CoreType, TypeError> {
        let source_type = self.type_check_expr(expr)?;
        let target_core_type = Self::ast_type_to_core_type(target_type)?;
        if Self::is_cast_allowed(&source_type, &target_core_type) {
            Ok(target_core_type)
        } else {
            Err(Self::type_mismatch_error(
                &target_core_type,
                Some(target_type.span()),
                &source_type,
                expr.span(),
            ))
        }
    }

    /// Validate each interpolated expression, ensuring only display-safe primitives appear
    /// inside a string literal interpolation sequence.
    fn type_check_string_interpolation(
        &mut self,
        parts: &[StringPart],
        _span: Span,
    ) -> Result<(), TypeError> {
        for part in parts {
            if let StringPart::Expression(ref expr) = *part {
                let expr_type = self.type_check_expr(expr)?;
                if !(Self::is_numeric_type(&expr_type)
                    || Self::is_boolean_type(&expr_type)
                    || Self::is_string_type(&expr_type))
                {
                    return Err(Self::invalid_operation_error(
                        "string interpolation",
                        &expr_type,
                        expr.span(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Type check an array literal, deriving a unified element type and generating equality
    /// constraints between each element and the inferred element type.
    fn type_check_array_expr(
        &mut self,
        elements: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let mut element_type: Option<CoreType> = None;
        for element in elements {
            let element_core_type = self.type_check_expr(element)?;
            if let Some(existing_type) = element_type.as_ref() {
                if !self.types_compatible(existing_type, &element_core_type) {
                    return Err(Self::type_mismatch_error(
                        existing_type,
                        Some(element.span()),
                        &element_core_type,
                        element.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::Equality(
                    existing_type.clone(),
                    element_core_type,
                ));
            } else {
                element_type = Some(element_core_type);
            }
        }

        let resolved = match element_type {
            Some(core_type) => core_type,
            None => self.fresh_type_var_auto(span)?,
        };

        Ok(CoreType::Array(Box::new(resolved)))
    }

    /// Type check a lambda expression by establishing a scoped environment for its parameters and body.
    fn type_check_lambda_expr(
        &mut self,
        generic_params: Option<&[String]>,
        parameters: &[Parameter],
        return_type: &Type,
        body: &LambdaBody,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if let Some(params) = generic_params {
            if !params.is_empty() {
                return Err(TypeError::NotImplementedYet {
                    feature: "generic lambda type checking".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = Self::ast_type_to_core_type(return_type)?;
        let return_span = return_type.span();

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            match *body {
                LambdaBody::Expression(ref expr) => {
                    let expr_type = checker.type_check_expr(expr)?;
                    if !checker.types_compatible(&return_core, &expr_type) {
                        return Err(Self::type_mismatch_error(
                            &return_core,
                            Some(return_span),
                            &expr_type,
                            expr.span(),
                        ));
                    }
                    checker
                        .add_constraint(TypeConstraint::Equality(return_core.clone(), expr_type));
                    Ok(())
                }
                LambdaBody::Block(ref statements) => {
                    checker.type_check_statements(statements, Some(&return_core))
                }
            }
        })?;

        Ok(CoreType::Function {
            parameters: parameter_types,
            return_type: Box::new(return_core),
        })
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

    /// Type check a slice of statements while propagating the expected return
    /// type for the enclosing function or lambda.
    fn type_check_statements(
        &mut self,
        statements: &[Stmt],
        expected_return: Option<&CoreType>,
    ) -> Result<(), TypeError> {
        for statement in statements {
            self.type_check_stmt_with_return(statement, expected_return)?;
        }
        Ok(())
    }

    /// Type check a single statement, validating it within the context of an
    /// optional expected return type.
    pub(super) fn type_check_stmt_with_return(
        &mut self,
        stmt: &Stmt,
        expected_return: Option<&CoreType>,
    ) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Let {
                ref binding,
                ref initializer,
                ..
            } => self.type_check_let_statement(binding, initializer.as_ref()),
            Stmt::Assignment {
                ref target,
                ref value,
                span,
                ..
            } => self.type_check_assignment(target, value, span),
            Stmt::Return {
                ref value, span, ..
            } => self.type_check_return(value.as_ref(), expected_return, span),
            Stmt::Expression { ref expr, .. } => {
                self.type_check_expr(expr)?;
                Ok(())
            }
            Stmt::Block { ref statements, .. } => self.within_new_scope(|checker| {
                checker.type_check_statements(statements, expected_return)
            }),
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                Self::ensure_boolean_type(&condition_type, condition.span(), "if condition")?;
                self.within_new_scope(|checker| {
                    checker.type_check_stmt_with_return(then_branch.as_ref(), expected_return)
                })?;
                if let Some(else_branch_stmt) = else_branch.as_deref() {
                    self.within_new_scope(|checker| {
                        checker.type_check_stmt_with_return(else_branch_stmt, expected_return)
                    })?;
                }
                Ok(())
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                span,
                ..
            } => {
                let iterable_type = self.type_check_expr(iterable)?;
                match iterable_type {
                    CoreType::Array(element_type) => {
                        let element_core = *element_type;
                        let variable_name = variable.clone();
                        self.within_new_scope(move |checker| {
                            checker.symbol_table.register(SymbolInfo {
                                name: variable_name.clone(),
                                symbol_type: SymbolType::Variable,
                                core_type: element_core,
                                visibility: Visibility::Private,
                                source_location: span,
                            });
                            checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                        })
                    }
                    _ => Err(Self::invalid_operation_error(
                        "for loop iteration",
                        &iterable_type,
                        span,
                    )),
                }
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                Self::ensure_boolean_type(&condition_type, condition.span(), "while condition")?;
                self.within_new_scope(|checker| {
                    checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                })
            }
            Stmt::Loop { ref body, .. } => self.within_new_scope(|checker| {
                checker.type_check_stmt_with_return(body.as_ref(), expected_return)
            }),
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
        }
    }

    /// Validate a `let` statement by resolving optional type annotations,
    /// initializer compatibility, and registering the binding in the current
    /// scope.
    fn type_check_let_statement(
        &mut self,
        binding: &LetBinding,
        initializer: Option<&Expr>,
    ) -> Result<(), TypeError> {
        let annotated_type = binding
            .type_annotation
            .as_ref()
            .map(Self::ast_type_to_core_type)
            .transpose()?;

        let initializer_info = match initializer {
            Some(expr) => Some((self.type_check_expr(expr)?, expr)),
            None => None,
        };

        let final_type = match (annotated_type, initializer_info) {
            (Some(expected), Some((actual, expr))) => {
                let reconciled = if self.types_compatible(&expected, &actual) {
                    actual
                } else if let Some(adjusted) =
                    Self::coerce_literal_to_expected(&expected, expr, &actual)
                {
                    adjusted
                } else {
                    return Err(Self::type_mismatch_error(
                        &expected,
                        binding.type_annotation.as_ref().map(Type::span),
                        &actual,
                        expr.span(),
                    ));
                };
                self.add_constraint(TypeConstraint::Equality(expected.clone(), reconciled));
                expected
            }
            (Some(expected), None) => expected,
            (None, Some((actual, _))) => actual,
            (None, None) => {
                return Err(TypeError::ConstraintSolvingFailed {
                    reason: format!(
                        "Cannot infer type for binding '{}' without annotation or initializer",
                        binding.name
                    ),
                    span: TypeError::span_from_span(binding.span),
                });
            }
        };

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };

        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: final_type,
            visibility: Visibility::Private,
            source_location: binding.span,
        });

        Ok(())
    }

    /// Ensure an assignment statement has a valid target and a value that is
    /// type compatible with that target.
    fn type_check_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        let target_type = self.type_check_expr(target)?;
        let value_type = self.type_check_expr(value)?;
        let reconciled_value_type = if self.types_compatible(&target_type, &value_type) {
            value_type
        } else if let Some(adjusted) =
            Self::coerce_literal_to_expected(&target_type, value, &value_type)
        {
            adjusted
        } else {
            return Err(Self::type_mismatch_error(
                &target_type,
                Some(target.span()),
                &value_type,
                value.span(),
            ));
        };
        let validity = match *target {
            Expr::Identifier { .. } | Expr::Member { .. } | Expr::Index { .. } => Ok(()),
            _ => Err(Self::invalid_operation_error(
                "assignment target",
                &target_type,
                span,
            )),
        };

        if validity.is_ok() {
            self.add_constraint(TypeConstraint::Equality(target_type, reconciled_value_type));
        }

        validity
    }

    /// Validate a return statement against the function's expected return type,
    /// guaranteeing both presence and compatibility.
    fn type_check_return(
        &mut self,
        value: Option<&Expr>,
        expected_return: Option<&CoreType>,
        span: Span,
    ) -> Result<(), TypeError> {
        let expected = expected_return.ok_or_else(|| TypeError::InvalidOperation {
            operation: "return outside of function".to_owned(),
            type_name: "<unknown>".to_owned(),
            span: TypeError::span_from_span(span),
        })?;

        match value {
            Some(expr) => {
                let value_type = self.type_check_expr(expr)?;
                let reconciled_type = if self.types_compatible(expected, &value_type) {
                    value_type
                } else if let Some(adjusted) =
                    Self::coerce_literal_to_expected(expected, expr, &value_type)
                {
                    adjusted
                } else {
                    return Err(Self::type_mismatch_error(
                        expected,
                        None,
                        &value_type,
                        expr.span(),
                    ));
                };
                self.add_constraint(TypeConstraint::Equality(expected.clone(), reconciled_type));
                Ok(())
            }
            None => {
                if matches!(expected, &CoreType::Unit) {
                    Ok(())
                } else {
                    Err(Self::type_mismatch_error(
                        expected,
                        None,
                        &CoreType::Unit,
                        span,
                    ))
                }
            }
        }
    }

    /// Type check a statement and update the symbol table as needed.
    ///
    /// # Errors
    /// Returns `TypeError` variants when statement typing fails.
    pub fn type_check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        self.type_check_stmt_with_return(stmt, None)
    }

    /// Convert AST-level visibility into the internal representation, accounting for entry points.
    const fn convert_visibility(visibility: &AstVisibility, is_entry: bool) -> Visibility {
        if is_entry {
            Visibility::Entry
        } else {
            match *visibility {
                AstVisibility::Public => Visibility::Public,
                AstVisibility::Private => Visibility::Private,
            }
        }
    }

    /// Register a declaration's symbol signature prior to body checking so forward references succeed.
    fn register_declaration_signature(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match decl {
            &Decl::Function {
                ref name,
                ref parameters,
                ref return_type,
                ref visibility,
                is_entry,
                ref span,
                ..
            } => {
                let mut parameter_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
                }

                let return_core = return_type
                    .as_ref()
                    .map(Self::ast_type_to_core_type)
                    .transpose()?
                    .unwrap_or(CoreType::Unit);

                let function_type = CoreType::Function {
                    parameters: parameter_types,
                    return_type: Box::new(return_core),
                };

                let visibility = Self::convert_visibility(visibility, is_entry);
                self.symbol_table.register(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Function,
                    core_type: function_type,
                    visibility,
                    source_location: *span,
                });
                Ok(())
            }
            &Decl::Let {
                ref binding,
                ref visibility,
                ..
            } => {
                if let Some(annotation) = binding.type_annotation.as_ref() {
                    let annotated_type = Self::ast_type_to_core_type(annotation)?;
                    let symbol_type = if binding.is_mutable {
                        SymbolType::Variable
                    } else {
                        SymbolType::Constant
                    };
                    let visibility = Self::convert_visibility(visibility, false);
                    self.symbol_table.register(SymbolInfo {
                        name: binding.name.clone(),
                        symbol_type,
                        core_type: annotated_type,
                        visibility,
                        source_location: binding.span,
                    });
                }
                Ok(())
            }
            &Decl::Type { .. } | &Decl::Import { .. } => Ok(()),
        }
    }

    /// Type check a top-level declaration and update symbol/type environments accordingly.
    fn type_check_declaration(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match *decl {
            Decl::Function {
                ref parameters,
                ref return_type,
                ref body,
                ..
            } => self.type_check_function_declaration(
                parameters.as_slice(),
                return_type.as_ref(),
                body,
            ),
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ..
            } => self.type_check_let_declaration(binding, initializer, visibility),
            Decl::Type { ref type_def, .. } => self.validate_adt_type(type_def),
            Decl::Import { .. } => {
                // Phase 4 will introduce full import validation; until then we simply acknowledge the declaration.
                Ok(())
            }
        }
    }

    /// Type check a function body within a dedicated parameter scope, enforcing return compatibility.
    fn type_check_function_declaration(
        &mut self,
        parameters: &[Parameter],
        return_type: Option<&Type>,
        body: &Stmt,
    ) -> Result<(), TypeError> {
        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = return_type
            .map(Self::ast_type_to_core_type)
            .transpose()?
            .unwrap_or(CoreType::Unit);

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            checker.type_check_stmt_with_return(body, Some(&return_core))
        })
    }

    /// Type check a module-level let declaration and ensure the registered symbol honors visibility.
    fn type_check_let_declaration(
        &mut self,
        binding: &LetBinding,
        initializer: &Expr,
        visibility: &AstVisibility,
    ) -> Result<(), TypeError> {
        self.type_check_let_statement(binding, Some(initializer))?;

        let inferred_type = self
            .symbol_table()
            .lookup(&binding.name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "Binding '{}' failed to register during top-level let processing",
                    binding.name
                ),
                span: TypeError::span_from_span(binding.span),
            })?;

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };
        let visibility = Self::convert_visibility(visibility, false);
        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: inferred_type,
            visibility,
            source_location: binding.span,
        });

        Ok(())
    }

    /// Type check an entire program, collecting all discovered errors.
    pub fn type_check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.clear_constraints();

        let mut errors: Vec<TypeError> = Vec::new();
        let mut skipped_decls: Vec<usize> = Vec::new();

        for decl in &program.declarations {
            if let Err(error) = self.register_declaration_signature(decl) {
                skipped_decls.push(decl.node_id().0);
                errors.push(error);
            }
        }

        for decl in &program.declarations {
            if skipped_decls.contains(&decl.node_id().0) {
                continue;
            }

            if let Err(error) = self.type_check_declaration(decl) {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            if let Err(error) = self.solve_constraints() {
                errors.push(error);
            }
        } else {
            self.clear_constraints();
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate that a type name is valid for the given core type
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the type to validate
    /// * `core_type` - The core type definition
    /// * `span` - Source location of the type definition (for error reporting)
    pub fn validate_type_name(
        &self,
        name: &str,
        core_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Ok(existing_type) = self.environment.lookup_type(name, span) {
            if existing_type != core_type {
                return Err(TypeError::TypeMismatch {
                    expected: existing_type.to_string(),
                    found: core_type.to_string(),
                    found_span: TypeError::span_from_span(span),
                    expected_span: None,
                });
            }
        }
        Ok(())
    }

    /// Unify two types, returning a substitution that makes them equal
    pub fn unify(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        self.unify_impl(left, right)
    }

    /// Internal implementation of unification algorithm
    fn unify_impl(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        match (left, right) {
            // Same primitive types unify with empty substitution
            (l, r) if self.types_compatible(l, r) => Ok(Substitution::empty()),

            // Variable unifies with any type (with occurs check)
            (&CoreType::Variable(ref var), other) | (other, &CoreType::Variable(ref var)) => {
                if Self::occurs_check(var.id, other) {
                    Err(TypeError::OccursCheckFailed {
                        var_name: var.name.clone(),
                        type_name: other.to_string(),
                        span: TypeError::unknown_span(),
                    })
                } else {
                    Ok(Substitution::single(var.id, other.clone()))
                }
            }

            // Arrays unify if their element types unify
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.unify_impl(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions unify if parameters and return types unify
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all parameters
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    let left_applied = combined_subst.apply(left_param);
                    let right_applied = combined_subst.apply(right_param);
                    let param_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&param_subst);
                }

                // Unify return types
                let left_ret_applied = combined_subst.apply(left_ret.as_ref());
                let right_ret_applied = combined_subst.apply(right_ret.as_ref());
                let ret_subst = self.unify_impl(&left_ret_applied, &right_ret_applied)?;
                combined_subst = combined_subst.compose(&ret_subst);

                Ok(combined_subst)
            }

            // Generic types unify if names match and type arguments unify
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all type arguments
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    let left_applied = combined_subst.apply(left_arg);
                    let right_applied = combined_subst.apply(right_arg);
                    let arg_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&arg_subst);
                }

                Ok(combined_subst)
            }

            // Different types cannot be unified
            _ => Err(TypeError::UnificationFailed {
                left: left.to_string(),
                right: right.to_string(),
                left_span: TypeError::unknown_span(),
                right_span: TypeError::unknown_span(),
            }),
        }
    }

    /// Check if a type variable occurs in a type (prevents infinite types)
    /// Uses iterative approach to avoid stack overflow with deeply nested types
    pub(super) fn occurs_check(var_id: usize, initial_type: &CoreType) -> bool {
        let mut work_queue = vec![initial_type];

        while let Some(current_type) = work_queue.pop() {
            match *current_type {
                CoreType::Variable(ref var) => {
                    if var.id == var_id {
                        return true;
                    }
                }
                CoreType::Array(ref element_type) => {
                    work_queue.push(element_type.as_ref());
                }
                CoreType::Function {
                    parameters: ref params,
                    return_type: ref ret_type,
                } => {
                    work_queue.push(ret_type.as_ref());
                    work_queue.extend(params.iter());
                }
                CoreType::Generic {
                    type_args: ref args,
                    ..
                } => {
                    work_queue.extend(args.iter());
                }
                // Primitive types don't contain variables - skip them
                _ => {}
            }
        }

        false
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
