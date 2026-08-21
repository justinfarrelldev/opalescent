//! `using` statement lowering through existing lexical scope cleanup.

use crate::ast::{Expr, LetBinding, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;

/// Lower `using` as a lexical scope-bound acquisition.
pub(super) fn codegen_using_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding: &LetBinding,
    acquisition: &Expr,
    body: &Stmt,
) -> Result<(), CodegenError> {
    let _using_scope_depth = env.enter_scope();
    super::codegen_let_statement(codegen_context, env, binding, Some(acquisition))?;
    super::codegen_statement(codegen_context, env, body)?;
    if let Some(current_block) = codegen_context.builder.get_insert_block() {
        if current_block.get_terminator().is_none() {
            crate::codegen::scope_tracker::cleanup_scopes_to_depth_with_malloc_string_release(
                codegen_context,
                env,
                env.current_scope_depth().saturating_sub(1),
                &[],
            )?;
        } else {
            super::unwind_scope_without_cleanup(env);
        }
    } else {
        super::unwind_scope_without_cleanup(env);
    }
    Ok(())
}
