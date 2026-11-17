//! Ordinary lexical assertion values use one stack slot per compiled operand.
use super::context::CodegenContext;
use super::error::CodegenError;
use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;

/// Place a slot in the enclosing entry so polling iterations do not grow stack.
pub fn entry_slot<'context>(
    codegen: &CodegenContext<'context>,
    value_type: BasicTypeEnum<'context>,
    name: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    let entry = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .and_then(inkwell::values::FunctionValue::get_first_basic_block)
        .ok_or_else(|| {
            CodegenError::new("assertion storage requires an enclosing function entry")
        })?;
    let builder = codegen.context.create_builder();
    if let Some(first) = entry.get_first_instruction() {
        builder.position_before(&first);
    } else {
        builder.position_at_end(entry);
    }
    Ok(builder.build_alloca(value_type, name)?)
}
