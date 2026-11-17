//! Shared, scalar C lifecycle boundary and explicit outcome outputs.
use super::context::CodegenContext;
use super::error::CodegenError;
use inkwell::AddressSpace;
use inkwell::types::FunctionType;
use inkwell::values::{FunctionValue, PointerValue};

/// Outputs are initialized before invoking any source callback.
pub(super) struct CallbackBoundary<'context> {
    /// C-compatible invocation entry.
    pub function: FunctionValue<'context>,
    /// Successfully created typed value slot, or null.
    pub created: PointerValue<'context>,
    /// Independent native abort identity, or zero.
    pub failure: PointerValue<'context>,
    /// Ordinary application error, or null.
    pub error: PointerValue<'context>,
}

/// Declare the uniform boundary, initialize outputs and position at its entry.
pub(super) fn declare_boundary<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
) -> Result<CallbackBoundary<'context>, CodegenError> {
    if codegen.module.get_function(name).is_some() {
        return Err(CodegenError::new(
            "native callback wrapper name conflicts with an existing function",
        ));
    }
    let llvm = codegen.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let pointer_slot = pointer.ptr_type(AddressSpace::default());
    let failure_slot = llvm.i64_type().ptr_type(AddressSpace::default());
    let signature = llvm.i32_type().fn_type(
        &[
            pointer_slot.into(),
            pointer.into(),
            pointer_slot.into(),
            failure_slot.into(),
            pointer_slot.into(),
        ],
        false,
    );
    let function = codegen.module.add_function(name, signature, None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let output = |index| {
        function
            .get_nth_param(index)
            .map(inkwell::values::BasicValueEnum::into_pointer_value)
            .ok_or_else(|| CodegenError::new("lifecycle callback output is missing"))
    };
    let created = output(2)?;
    let failure = output(3)?;
    let error = output(4)?;
    codegen.builder.build_store(created, pointer.const_null())?;
    codegen
        .builder
        .build_store(failure, llvm.i64_type().const_zero())?;
    codegen.builder.build_store(error, pointer.const_null())?;
    Ok(CallbackBoundary {
        function,
        created,
        failure,
        error,
    })
}

/// Never confuse an instrumented source function with a scalar C runtime call.
pub(super) fn c_function<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    signature: FunctionType<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    if let Some(function) = codegen.module.get_function(name) {
        if function.get_type() != signature || codegen.native_function_outcome(function).is_some() {
            return Err(CodegenError::new(
                "native lifecycle runtime function has an incompatible ABI",
            ));
        }
        Ok(function)
    } else {
        Ok(codegen.module.add_function(name, signature, None))
    }
}
