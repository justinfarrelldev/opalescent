//! LLVM declarations for Task 16 immutable error inspector runtime helpers.

use crate::codegen::context::CodegenContext;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

/// Runtime/helper names implemented for immutable error attachments.
pub(super) const ERROR_ATTACHMENT_STDLIB_NAMES: &[&str] = &[
    "opal_error_new",
    "opal_error_attach_cause",
    "error_cause",
    "error_suppressed_length",
    "error_suppressed_at",
    "error_attachment_truncation",
    "error_attachment_truncation_cause_depth",
    "error_attachment_truncation_suppressed_count",
    "error_attachment_truncation_bytes",
];

/// Declare one Task 16 error runtime function if it is implemented.
pub(super) fn declare_error_inspector_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_type = ctx.i8_type();
    let i8_ptr = i8_type.ptr_type(AddressSpace::default());
    let i64_type = ctx.i64_type();
    let pointer_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);

    match name {
        "opal_error_new" => module.get_function(name).or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "opal_error_attach_cause" => module.get_function(name).or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "error_cause" => module.get_function(name).or_else(|| {
            let ft = pointer_error_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "error_suppressed_at" => module.get_function(name).or_else(|| {
            let ft = pointer_error_result_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "error_suppressed_length" => module.get_function(name).or_else(|| {
            let ft = i64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "error_attachment_truncation" => module.get_function(name).or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        "error_attachment_truncation_cause_depth"
        | "error_attachment_truncation_suppressed_count"
        | "error_attachment_truncation_bytes" => module.get_function(name).or_else(|| {
            let ft = i8_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function(name, ft, None))
        }),
        _ => None,
    }
}
