//! Exact nominal parameter order and reference contracts for test bodies/hooks.
use super::{TypeChecker, native_error};
use crate::ast::testing::ManagedCallback;
use crate::ast::{AstNode, BorrowKind, Expr};
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

impl TypeChecker {
    /// Check a case body against ordered nominal injections and optional example
    /// data, followed only by an optional final mutable test context reference.
    ///
    /// # Errors
    /// Rejects missing/extra/reordered Worlds, wrong reference modes, wrong
    /// example/context slots, unused Worlds and ordinary callback type errors.
    pub fn type_check_native_case_callback(
        &mut self,
        callback: &ManagedCallback,
        expected_worlds: &[CoreType],
        example_type: Option<&CoreType>,
    ) -> Result<CoreType, TypeError> {
        self.enable_native_test_mode()?;
        let Expr::Lambda {
            ref params,
            ref return_types,
            ..
        } = callback.function
        else {
            return Err(native_error(
                callback.function.span(),
                "case callback must be an ordinary function literal",
            ));
        };
        if return_types.len() != 1
            || self.ast_type_to_core_type_with_generics(&return_types[0], &[])? != CoreType::Unit
        {
            return Err(native_error(
                callback.function.span(),
                "test and hook callbacks must return void",
            ));
        }
        for (index, world) in expected_worlds.iter().enumerate() {
            if expected_worlds
                .get(..index)
                .is_some_and(|previous| previous.contains(world))
            {
                return Err(native_error(
                    callback.function.span(),
                    "duplicate canonical World injection",
                ));
            }
        }
        let context_type = params
            .last()
            .map(|parameter| self.ast_type_to_core_type_with_generics(&parameter.param_type, &[]))
            .transpose()?;
        let has_context = matches!(context_type, Some(CoreType::Generic { ref name, .. }) if name == "__opal_testing.TestContext");
        let expected_count = expected_worlds
            .len()
            .saturating_add(usize::from(example_type.is_some()))
            .saturating_add(usize::from(has_context));
        if params.len() != expected_count {
            return Err(TypeError::ArityMismatch {
                expected: expected_count,
                found: params.len(),
                span: TypeError::span_from_span(callback.function.span()),
            });
        }
        let mut required = Vec::new();
        for (parameter, expected) in params.iter().zip(expected_worlds) {
            let actual = self.ast_type_to_core_type_with_generics(&parameter.param_type, &[])?;
            if actual != *expected || parameter.borrow_kind == BorrowKind::Owned {
                return Err(native_error(
                    parameter.span,
                    "World parameters must use ref or mutable ref and match exact provider identity and subscription order",
                ));
            }
            required.push(parameter.name.clone());
        }
        if let Some(expected_example) = example_type {
            let parameter = &params[expected_worlds.len()];
            let actual = self.ast_type_to_core_type_with_generics(&parameter.param_type, &[])?;
            if actual != *expected_example || parameter.borrow_kind != BorrowKind::Ref {
                return Err(native_error(
                    parameter.span,
                    "example data must follow Worlds as ref example: ExampleType",
                ));
            }
        }
        if has_context
            && params
                .last()
                .is_some_and(|parameter| parameter.borrow_kind != BorrowKind::MutableRef)
        {
            return Err(native_error(
                callback.function.span(),
                "test context must be a final mutable ref parameter",
            ));
        }
        self.type_check_native_callback(callback, &required)
    }
}
