//! Typed wrapper argument addresses from compiler-resolved injection positions.
use super::NativeCallbackArgument;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use inkwell::values::{BasicMetadataValueEnum, FunctionValue, PointerValue};

/// Lower known typed injection addresses; source callbacks never receive a map.
pub fn lower_arguments<'context>(
    codegen: &CodegenContext<'context>,
    wrapper: FunctionValue<'context>,
    callback: FunctionValue<'context>,
    plan: &[NativeCallbackArgument],
) -> Result<Vec<BasicMetadataValueEnum<'context>>, CodegenError> {
    let mut result = Vec::new();
    for (argument, expected) in plan.iter().zip(callback.get_type().get_param_types()) {
        if !expected.is_pointer_type() {
            return Err(CodegenError::new(
                "managed callback injection must be an existing reference parameter",
            ));
        }
        let expected = expected.into_pointer_type();
        let address = match *argument {
            NativeCallbackArgument::ContextReference => {
                if !expected.get_element_type().is_pointer_type() {
                    return Err(CodegenError::new(
                        "TestContext reference requires a nominal pointer representation",
                    ));
                }
                crate::codegen::native_context::reference_address(codegen)?
            }
            NativeCallbackArgument::WorldReference(index) => {
                if !expected.get_element_type().is_pointer_type() {
                    return Err(CodegenError::new(
                        "nominal World reference has an incompatible value representation",
                    ));
                }
                let world = world_slot(codegen, wrapper, index)?;
                let slot = codegen
                    .builder
                    .build_alloca(world.get_type(), "callback.world.reference")?;
                codegen.builder.build_store(slot, world)?;
                slot
            }
            NativeCallbackArgument::StateReference(index) => world_slot(codegen, wrapper, index)?,
            NativeCallbackArgument::ExampleReference => wrapper
                .get_nth_param(1)
                .ok_or_else(|| CodegenError::new("example injection has no typed value slot"))?
                .into_pointer_value(),
        };
        result.push(
            codegen
                .builder
                .build_pointer_cast(address, expected, "callback.argument")?
                .into(),
        );
    }
    Ok(result)
}

/// The compiler's checked plan guarantees the index belongs to this case's
/// initialized World vector. The vector contains typed state slots, not names.
fn world_slot<'context>(
    codegen: &CodegenContext<'context>,
    wrapper: FunctionValue<'context>,
    index: usize,
) -> Result<PointerValue<'context>, CodegenError> {
    let worlds = wrapper
        .get_nth_param(0)
        .ok_or_else(|| CodegenError::new("World injection has no lifecycle state vector"))?
        .into_pointer_value();
    let index = u64::try_from(index).map_err(|error| CodegenError::new(error.to_string()))?;
    // SAFETY: The resolved descriptor plan establishes the state vector's index;
    // this is an ordinary reference projection, not a runtime fixture search.
    let slot = unsafe {
        codegen.builder.build_in_bounds_gep(
            worlds,
            &[codegen.context.i64_type().const_int(index, false)],
            "callback.world.slot",
        )?
    };
    Ok(codegen
        .builder
        .build_load(slot, "callback.world.value")?
        .into_pointer_value())
}
