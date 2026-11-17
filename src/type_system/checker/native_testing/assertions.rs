//! Typed fixed-vocabulary assertion validation for native test artifacts.
extern crate alloc;
use super::{TypeChecker, native_error};
use crate::ast::testing::{
    Expectation, Matcher, SequenceRelation, SourceReference, TestingStatement,
};
use crate::ast::{AstNode, Expr};
use crate::token::Span;
use crate::type_system::checker::helpers::{
    coerce_literal_to_expected, is_numeric_type, type_mismatch_error,
};
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeSet;

impl TypeChecker {
    /// Validate test-only operations without broadening application error sets.
    pub(in crate::type_system::checker) fn type_check_native_statement(
        &mut self,
        statement: &TestingStatement,
        span: Span,
    ) -> Result<(), TypeError> {
        if !self.native_testing.enabled {
            return Err(native_error(
                span,
                "test operations require an explicitly instrumented test artifact",
            ));
        }
        match *statement {
            TestingStatement::Expect(ref expectation) => {
                self.type_check_native_expectation(expectation)
            }
            TestingStatement::Stub { .. }
            | TestingStatement::Restore(_)
            | TestingStatement::Verify(_) => {
                self.type_check_native_double_statement(statement, span)
            }
        }
    }

    /// All actual/expected operands remain ordinary, statically typed expressions.
    fn type_check_native_expectation(
        &mut self,
        expectation: &Expectation,
    ) -> Result<(), TypeError> {
        let actual = self.type_check_expr(&expectation.actual)?;
        match expectation.matcher {
            Matcher::Equal { ref expected, .. } => {
                self.check_native_expected_type(&actual, expected, expectation.actual.span())?;
                if !self.native_equality_supported(&actual, &mut BTreeSet::new()) {
                    return Err(native_error(
                        expectation.span,
                        "unsupported typed equality operands",
                    ));
                }
            }
            Matcher::Compare { ref expected, .. } => {
                if !is_numeric_type(&actual) {
                    return Err(native_error(
                        expectation.actual.span(),
                        "numeric assertion requires compatible numeric operands",
                    ));
                }
                self.check_native_expected_type(&actual, expected, expectation.actual.span())?;
            }
            Matcher::Sequence {
                ref expected,
                relation,
                ..
            } => {
                if relation == SequenceRelation::ExactlyInOrder {
                    if !matches!(actual, CoreType::Array(_))
                        || !self.native_equality_supported(&actual, &mut BTreeSet::new())
                    {
                        return Err(native_error(
                            expectation.actual.span(),
                            "exact ordered matching requires arrays with supported element equality",
                        ));
                    }
                } else if actual != CoreType::String {
                    return Err(native_error(
                        expectation.actual.span(),
                        "string matcher requires string operands",
                    ));
                }
                self.check_native_expected_type(&actual, expected, expectation.actual.span())?;
            }
            Matcher::Empty { .. } => {
                if !matches!(actual, CoreType::String | CoreType::Array(_)) {
                    return Err(native_error(
                        expectation.actual.span(),
                        "empty matching supports strings and arrays",
                    ));
                }
            }
            Matcher::Variant {
                ref variant,
                ref nominal_type,
            } => self.check_native_variant(
                &actual,
                variant,
                nominal_type,
                expectation.actual.span(),
            )?,
            Matcher::CallCount { .. } | Matcher::CalledWith { .. } => self
                .type_check_native_call_matcher(&actual, &expectation.matcher, expectation.span)?,
        }
        let extent = expectation.actual.span();
        self.native_testing
            .assertion_operands
            .insert((extent.start.offset, extent.end.offset), actual);
        if let Some(ref deadline) = expectation.eventually_seconds {
            let deadline_type = self.type_check_expr(deadline)?;
            if !is_numeric_type(&deadline_type) {
                return Err(native_error(
                    deadline.span(),
                    "polling deadline requires a numeric duration in seconds",
                ));
            }
        }
        Ok(())
    }

    /// Resolve a genuine variant of the named nominal type, including aliases.
    fn check_native_variant(
        &self,
        actual: &CoreType,
        variant: &str,
        nominal_type: &SourceReference,
        actual_span: Span,
    ) -> Result<(), TypeError> {
        let name = nominal_type.path.join(".");
        let expected = self
            .symbol_table()
            .lookup(&name)
            .map(|symbol| symbol.core_type.clone())
            .ok_or_else(|| {
                native_error(
                    nominal_type.span,
                    "variant matcher requires a declared nominal type",
                )
            })?;
        let qualified = format!("{name}.{variant}");
        if !self
            .adt_variants
            .get(&name)
            .is_some_and(|variants| variants.contains(&qualified))
        {
            return Err(native_error(
                nominal_type.span,
                "variant is not declared by the named nominal type",
            ));
        }
        if self.types_compatible(&expected, actual) {
            Ok(())
        } else {
            Err(type_mismatch_error(
                &expected,
                Some(nominal_type.span),
                actual,
                actual_span,
            ))
        }
    }

    /// Reuse ordinary literal compatibility rather than stringifying values.
    fn check_native_expected_type(
        &mut self,
        actual: &CoreType,
        expected: &Expr,
        actual_span: Span,
    ) -> Result<(), TypeError> {
        let expected_type = self.type_check_expr(expected)?;
        let compatible =
            coerce_literal_to_expected(actual, expected, &expected_type).unwrap_or(expected_type);
        if self.types_compatible(actual, &compatible) {
            Ok(())
        } else {
            Err(type_mismatch_error(
                actual,
                Some(actual_span),
                &compatible,
                expected.span(),
            ))
        }
    }

    /// V1 equality: primitive values, arrays and acyclic products of supported
    /// values, plus payload-free enums. Handles and recursive shapes are excluded.
    pub(super) fn native_equality_supported(
        &self,
        core_type: &CoreType,
        active: &mut BTreeSet<String>,
    ) -> bool {
        match *core_type {
            CoreType::String | CoreType::Boolean => true,
            CoreType::Array(ref element) => self.native_equality_supported(element, active),
            CoreType::Generic {
                ref name,
                ref type_args,
            } => {
                if !type_args.is_empty()
                    || self.native_testing.worlds.contains_key(name)
                    || !active.insert(name.clone())
                {
                    return false;
                }
                let supported = self.adt_variants.get(name).map_or_else(
                    || {
                        self.adt_fields.get(name).is_some_and(|fields| {
                            fields
                                .values()
                                .all(|field| self.native_equality_supported(field, active))
                        })
                    },
                    |variants| {
                        variants.iter().all(|variant| {
                            self.adt_fields
                                .get(variant)
                                .is_some_and(alloc::collections::BTreeMap::is_empty)
                        })
                    },
                );
                active.remove(name);
                supported
            }
            _ => is_numeric_type(core_type),
        }
    }
}
