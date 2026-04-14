//! Value-generation helper scaffolding for LLVM lowering.

use crate::codegen::context::CodegenContext;
use inkwell::values::PointerValue;

/// Value helper entry points used by future expression and statement lowering.
pub struct ValueBuilder;

impl ValueBuilder {
    /// Build a UTF-8 C-string pointer value.
    ///
    /// Build a UTF-8 C-string pointer for string literal lowering.
    #[must_use]
    pub const fn build_string_placeholder<'context>(
        codegen_context: &CodegenContext<'context>,
    ) -> Option<PointerValue<'context>> {
        let _: &CodegenContext<'context> = codegen_context;
        None
    }
}
