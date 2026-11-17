//! Nominal escaping application-error inference for exact managed boundaries.
extern crate alloc;
use super::TypeChecker;
use crate::token::Span;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeSet;

impl TypeChecker {
    /// Check an exact source boundary, never an enclosing ordinary helper.
    pub(in crate::type_system::checker) fn native_callback_infers_at(&self, span: Span) -> bool {
        self.native_testing
            .active
            .as_ref()
            .is_some_and(|callback| callback.infer_errors && callback.span == span)
    }

    /// Whether application errors may escape the currently checked function
    /// for later nominal inference at its managed callback boundary.
    pub(in crate::type_system::checker) fn native_current_callback_infers_errors(&self) -> bool {
        self.symbol_table()
            .current_function_span()
            .is_some_and(|span| self.native_callback_infers_at(span))
    }

    /// Infer only collected nominal application-error leaves. Test failures have
    /// no application error type and never enter this collection.
    pub(in crate::type_system::checker) fn native_callback_result_errors(
        &self,
        span: Span,
        declared: Vec<CoreType>,
        escaping: &BTreeSet<String>,
    ) -> Result<Vec<CoreType>, TypeError> {
        if self.native_callback_infers_at(span) {
            self.resolve_error_types(&escaping.iter().cloned().collect::<Vec<_>>(), span)
        } else {
            Ok(declared)
        }
    }
}
