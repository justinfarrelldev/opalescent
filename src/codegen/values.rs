//! Value-generation helper scaffolding for LLVM lowering.

use crate::codegen::context::CodegenContext;
use inkwell::values::PointerValue;

/// Value helper entry points used by future expression and statement lowering.
pub struct ValueBuilder;

impl ValueBuilder {
    /// Build a UTF-8 C-string pointer value.
    ///
    /// This is a placeholder API for Task 22 expression lowering work.
    #[must_use]
    pub const fn build_string_placeholder<'context>(
        codegen_context: &CodegenContext<'context>,
    ) -> Option<PointerValue<'context>> {
        let _: &CodegenContext<'context> = codegen_context;
        None
    }
}
