//! Signature validation for precompiled, attempt-local typed doubles.
extern crate alloc;
use super::{NativeHandleKind, TypeChecker, native_error};
use crate::ast::testing::{ExpectedCallCount, Matcher, StubBehavior, TestingStatement};
use crate::ast::{AstNode, BorrowKind, ConstructorField, Expr};
use crate::token::Span;
use crate::type_system::NativeDoubleSignature;
use crate::type_system::checker::FallibleExpressionContext;
use crate::type_system::checker::helpers::{
    coerce_literal_to_expected, is_integer_type, type_mismatch_error,
};
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

impl TypeChecker {
    /// Register the generated nominal type and concrete application signature.
    pub(crate) fn register_native_test_double(
        &mut self,
        local_name: &str,
        nominal_name: &str,
        signature: NativeDoubleSignature,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let core_type = self.native_declare_double_alias(local_name, nominal_name, span)?;
        self.native_define_double_signature(nominal_name, signature, span)?;
        Ok(core_type)
    }

    /// `new Double` is a runner activation operation, not a forgeable record.
    pub(in crate::type_system::checker) fn native_double_constructor(
        &self,
        name: &str,
        fields: &[ConstructorField],
        span: Span,
    ) -> Result<Option<CoreType>, TypeError> {
        let Some(core_type) = self.native_source_type(name) else {
            return Ok(None);
        };
        if !matches!(core_type, CoreType::Generic { name: ref nominal_name, .. } if self.native_testing.kinds.get(nominal_name) == Some(&NativeHandleKind::Double))
        {
            return Ok(None);
        }
        if !fields.is_empty() {
            return Err(native_error(
                span,
                "double activation takes no user-defined handle fields",
            ));
        }
        Ok(Some(core_type))
    }

    /// Validate one typed configuration or lifecycle operation. Exact rule use,
    /// activation conflicts and unexpected calls are checked by the runtime.
    pub(super) fn type_check_native_double_statement(
        &mut self,
        statement: &TestingStatement,
        span: Span,
    ) -> Result<(), TypeError> {
        match *statement {
            TestingStatement::Verify(ref target) | TestingStatement::Restore(ref target) => {
                if matches!(*statement, TestingStatement::Restore(_)) {
                    self.validate_native_world_mutation(target)?;
                }
                let actual = self.type_check_expr(target)?;
                self.native_double_signature(&actual, span)?;
                Ok(())
            }
            TestingStatement::Stub {
                ref target,
                ref arguments,
                ref behavior,
            } => {
                self.validate_native_world_mutation(target)?;
                let actual = self.type_check_expr(target)?;
                let signature = self.native_double_signature(&actual, span)?;
                if let Some(ref values) = *arguments {
                    self.check_native_double_arguments(&signature, values, true, span)?;
                }
                self.check_native_double_behavior(&signature, behavior, span)
            }
            TestingStatement::Expect(_) => {
                Err(native_error(span, "expected a typed double operation"))
            }
        }
    }

    /// Count-only history remains available for unsupported snapshot types.
    pub(super) fn type_check_native_call_matcher(
        &mut self,
        actual: &CoreType,
        matcher: &Matcher,
        span: Span,
    ) -> Result<(), TypeError> {
        let signature = self.native_double_signature(actual, span)?;
        match *matcher {
            Matcher::CallCount {
                count: ExpectedCallCount::Exactly(ref count),
            } => {
                if !is_integer_type(&self.type_check_expr(count)?) {
                    return Err(native_error(
                        count.span(),
                        "call-count assertions require an integer count",
                    ));
                }
                Ok(())
            }
            Matcher::CallCount { .. } => Ok(()),
            Matcher::CalledWith { ref arguments } => {
                self.check_native_double_arguments(&signature, arguments, false, span)
            }
            _ => Err(native_error(
                span,
                "expected a supported call-history matcher",
            )),
        }
    }

    /// Resolve a genuine generated double handle rather than a display name.
    fn native_double_signature(
        &self,
        core_type: &CoreType,
        span: Span,
    ) -> Result<NativeDoubleSignature, TypeError> {
        let CoreType::Generic {
            ref name,
            ref type_args,
        } = *core_type
        else {
            return Err(native_error(
                span,
                "operation requires a typed double handle",
            ));
        };
        if !type_args.is_empty() {
            return Err(native_error(
                span,
                "double handles are concrete nominal types",
            ));
        }
        self.native_testing
            .doubles
            .get(name)
            .cloned()
            .ok_or_else(|| native_error(span, "operation requires a resolved typed double handle"))
    }

    /// Exact rules use the application reference modes. History assertions
    /// compare owned snapshots, never retained borrowed addresses.
    fn check_native_double_arguments(
        &mut self,
        signature: &NativeDoubleSignature,
        arguments: &[Expr],
        application_modes: bool,
        span: Span,
    ) -> Result<(), TypeError> {
        if arguments.len() != signature.parameters.len() {
            return Err(TypeError::ArityMismatch {
                expected: signature.parameters.len(),
                found: arguments.len(),
                span: TypeError::span_from_span(span),
            });
        }
        if application_modes {
            self.validate_native_world_call_borrows(arguments)?;
        }
        for (index, (argument, expected)) in arguments.iter().zip(&signature.parameters).enumerate()
        {
            if !crate::type_system::native_double_signature::supports_argument_snapshot(expected) {
                return Err(native_error(
                    argument.span(),
                    "this argument type does not support owned snapshots and typed exact matching; count-only history or a replacement/spy fallback remains available",
                ));
            }
            let mode = if application_modes {
                signature.parameter_modes[index]
            } else {
                BorrowKind::Owned
            };
            let actual = match mode {
                BorrowKind::Ref | BorrowKind::MutableRef => {
                    self.type_check_explicit_borrow_argument(argument, mode)?
                }
                BorrowKind::Owned => {
                    if matches!(*argument, Expr::BorrowArgument { .. }) {
                        return Err(native_error(
                            argument.span(),
                            "argument reference mode does not match the double signature",
                        ));
                    }
                    self.type_check_expr(argument)?
                }
            };
            let compatible =
                coerce_literal_to_expected(expected, argument, &actual).unwrap_or(actual);
            if !self.types_compatible(expected, &compatible) {
                return Err(type_mismatch_error(
                    expected,
                    None,
                    &compatible,
                    argument.span(),
                ));
            }
        }
        Ok(())
    }

    /// Return values and application errors are ordinary typed source values;
    /// neither is used to represent an unexpected call or assertion failure.
    fn check_native_double_behavior(
        &mut self,
        signature: &NativeDoubleSignature,
        behavior: &StubBehavior,
        span: Span,
    ) -> Result<(), TypeError> {
        match *behavior {
            StubBehavior::FailTest | StubBehavior::SpyOriginal => Ok(()),
            StubBehavior::Return(ref value) => {
                if signature
                    .return_types
                    .iter()
                    .any(|result| self.core_type_contains_affine_resource(result))
                {
                    return Err(native_error(
                        value.span(),
                        "stored stub results cannot duplicate affine owners; use a replacement or spy to produce each result",
                    ));
                }
                let actual_results =
                    self.native_stub_return_types(value, signature.return_types.len())?;
                if actual_results.len() != signature.return_types.len() {
                    return Err(native_error(
                        value.span(),
                        "stub return count does not match the replaced function",
                    ));
                }
                for (expected, actual) in signature.return_types.iter().zip(actual_results) {
                    let actual =
                        coerce_literal_to_expected(expected, value, &actual).unwrap_or(actual);
                    if !self.types_compatible(expected, &actual) {
                        return Err(type_mismatch_error(expected, None, &actual, value.span()));
                    }
                }
                Ok(())
            }
            StubBehavior::ApplicationError(ref reference) => {
                let name = reference.path.join(".");
                if self.error_sets.contains_key(&name) {
                    return Err(native_error(
                        reference.span,
                        "a stub must fail with a nominal application error variant, not an error-set alias",
                    ));
                }
                let errors = self.resolve_error_types(&[name], reference.span)?;
                self.validate_native_mock_error_variant(reference, &errors)?;
                if errors.len() != 1
                    || !errors.iter().all(|error| {
                        signature
                            .error_types
                            .iter()
                            .any(|allowed| Self::declared_error_type_covers(error, allowed))
                    })
                {
                    return Err(native_error(
                        reference.span,
                        "stub error is not an allowed nominal application error of the replaced function",
                    ));
                }
                Ok(())
            }
            StubBehavior::Replacement(ref replacement) => {
                self.check_native_replacement(signature, replacement, span)
            }
        }
    }

    /// A multi-result configuration expression uses the existing ordered return
    /// representation rather than creating source-level tuple values.
    fn native_stub_return_types(
        &mut self,
        value: &Expr,
        count: usize,
    ) -> Result<Vec<CoreType>, TypeError> {
        if count == 1 {
            return self.type_check_expr(value).map(|core_type| vec![core_type]);
        }
        match *value {
            Expr::Call {
                ref callee,
                ref generic_args,
                ref args,
                span,
                ..
            } => {
                self.validate_call_preconditions(callee)?;
                self.resolve_call_type(callee, generic_args.as_deref(), args, span)
                    .map(|resolved| resolved.return_types)
            }
            Expr::Propagate {
                ref call,
                ref cause,
                span,
                ..
            } => {
                let shape =
                    self.classify_fallible_call_shape(call, FallibleExpressionContext::Propagate)?;
                self.ensure_propagate_error_types_allowed(call, &shape.error_types, span)?;
                if let Some(cause) = cause.as_deref() {
                    self.type_check_propagate_cause_expr(call, cause, span)?;
                }
                Ok(shape.success_types)
            }
            Expr::Parenthesized { ref expr, .. } => self.native_stub_return_types(expr, count),
            _ => Err(native_error(
                value.span(),
                "multi-result stubs require an ordinary multi-return call expression or a signature-compatible replacement",
            )),
        }
    }

    /// Validate typed custom handlers, including borrowed parameter modes and
    /// the subset of ordinary application errors they may produce.
    fn check_native_replacement(
        &mut self,
        signature: &NativeDoubleSignature,
        replacement: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        let replacement_type = self.type_check_expr(replacement)?;
        let CoreType::Function {
            ref generic_params,
            ref parameters,
            ref return_types,
            ref error_types,
        } = replacement_type
        else {
            return Err(native_error(
                span,
                "double replacement must be a typed callable",
            ));
        };
        if !generic_params.is_empty()
            || parameters != &signature.parameters
            || return_types != &signature.return_types
            || !error_types.iter().all(|error| {
                signature
                    .error_types
                    .iter()
                    .any(|allowed| Self::declared_error_type_covers(error, allowed))
            })
        {
            return Err(native_error(
                replacement.span(),
                "replacement signature or nominal application errors do not match the replaced function",
            ));
        }
        for (index, expected) in signature.parameter_modes.iter().enumerate() {
            let actual = match *replacement {
                Expr::Lambda { ref params, .. } => params[index].borrow_kind,
                _ => self
                    .expected_borrow_kind_for_call(replacement, index)
                    .unwrap_or(BorrowKind::Owned),
            };
            if actual != *expected {
                return Err(native_error(
                    replacement.span(),
                    "replacement parameter reference modes do not match the replaced function",
                ));
            }
        }
        Ok(())
    }
}
