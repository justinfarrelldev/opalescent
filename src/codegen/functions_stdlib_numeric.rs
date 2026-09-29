//! LLVM declarations for checked numeric conversion runtime functions.

extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::numeric_conversions::numeric_conversion_spec;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

/// Declare a checked numeric conversion runtime function when `name` is registered.
pub(super) fn declare_numeric_conversion_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let spec = numeric_conversion_spec(name)?;
    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
    let i8_type = ctx.i8_type();
    let i16_type = ctx.i16_type();
    let i32_type = ctx.i32_type();
    let i64_type = ctx.i64_type();
    let f32_type = ctx.f32_type();
    let f64_type = ctx.f64_type();

    let source_type = match spec.source {
        "int8" | "uint8" => i8_type.into(),
        "int16" | "uint16" => i16_type.into(),
        "int32" | "uint32" => i32_type.into(),
        "int64" | "uint64" => i64_type.into(),
        "float32" => f32_type.into(),
        "float64" => f64_type.into(),
        _ => return None,
    };
    let result_type = match spec.destination {
        "int8" | "uint8" => ctx.struct_type(&[i8_type.into(), i8_ptr.into()], false),
        "int16" | "uint16" => ctx.struct_type(&[i16_type.into(), i8_ptr.into()], false),
        "int32" | "uint32" => ctx.struct_type(&[i32_type.into(), i8_ptr.into()], false),
        "int64" | "uint64" => ctx.struct_type(&[i64_type.into(), i8_ptr.into()], false),
        "float32" => ctx.struct_type(&[f32_type.into(), i8_ptr.into()], false),
        "float64" => ctx.struct_type(&[f64_type.into(), i8_ptr.into()], false),
        _ => return None,
    };

    module.get_function(name).or_else(|| {
        let ft = result_type.fn_type(&[source_type], false);
        Some(module.add_function(name, ft, None))
    })
}

/// Return whether `name` is an implemented checked numeric conversion runtime symbol.
pub(super) fn is_numeric_conversion_runtime_name(name: &str) -> bool {
    numeric_conversion_spec(name).is_some()
}
