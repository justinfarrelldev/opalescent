#![allow(
    clippy::missing_docs_in_private_items,
    clippy::too_many_lines,
    reason = "string stdlib declarations are explicit runtime ABI tables"
)]

extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::functions_stdlib::declare_fs_result_function;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

pub(super) const STRING_STDLIB_NAMES: &[&str] = &[
    "string_length",
    "string_index",
    "string_find_index_or",
    "string_find_last_index_of_text",
    "string_take_prefix",
    "string_take_suffix",
    "string_extract_range",
    "string_split_lines",
    "string_is_blank",
    "string_trim_whitespace",
    "string_join",
    "string_builder_new",
    "string_builder_push",
    "string_builder_finish",
];

pub(super) fn declare_string_stdlib_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
    let i8_type = ctx.i8_type();
    let i64_type = ctx.i64_type();
    let parse_result_i64_type = ctx.struct_type(&[i64_type.into(), i8_ptr.into()], false);
    let pointer_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let void_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let fs_string_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let fs_string_array_result_type = ctx.struct_type(
        &[
            i8_ptr.ptr_type(AddressSpace::default()).into(),
            i64_type.into(),
            i8_ptr.into(),
        ],
        false,
    );

    match name {
        "string_length" => module.get_function("string_length").or_else(|| {
            let ft = i64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_length", ft, None))
        }),
        "string_index" => module.get_function("string_index").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into(), i64_type.into()], false);
            Some(module.add_function("string_index", ft, None))
        }),
        "string_find_index_or" => module.get_function("string_find_index_or").or_else(|| {
            let ft = i64_type.fn_type(&[i8_ptr.into(), i8_ptr.into(), i64_type.into()], false);
            Some(module.add_function("string_find_index_or", ft, None))
        }),
        "string_find_last_index_of_text" => module
            .get_function("string_find_last_index_of_text")
            .or_else(|| {
                let ft = parse_result_i64_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
                Some(module.add_function("string_find_last_index_of_text", ft, None))
            }),
        "string_split_lines" => module.get_function("string_split_lines").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_split_lines",
                fs_string_array_result_type,
                &[i8_ptr.into()],
            ))
        }),
        "string_is_blank" => module.get_function("string_is_blank").or_else(|| {
            let ft = i8_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_is_blank", ft, None))
        }),
        "string_trim_whitespace" => module
            .get_function("string_trim_whitespace")
            .or_else(|| {
                Some(declare_fs_result_function(
                    codegen_context,
                    "string_trim_whitespace",
                    fs_string_result_type,
                    &[i8_ptr.into()],
                ))
            }),
        "string_take_prefix" => module.get_function("string_take_prefix").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_take_prefix",
                fs_string_result_type,
                &[i8_ptr.into(), i64_type.into()],
            ))
        }),
        "string_take_suffix" => module.get_function("string_take_suffix").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_take_suffix",
                fs_string_result_type,
                &[i8_ptr.into(), i64_type.into()],
            ))
        }),
        "string_extract_range" => module.get_function("string_extract_range").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_extract_range",
                fs_string_result_type,
                &[i8_ptr.into(), i64_type.into(), i64_type.into()],
            ))
        }),
        "string_join" => module.get_function("string_join").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_join",
                fs_string_result_type,
                &[
                    i8_ptr.ptr_type(AddressSpace::default()).into(),
                    i64_type.into(),
                    i8_ptr.into(),
                ],
            ))
        }),
        "string_builder_new" => module.get_function("string_builder_new").or_else(|| {
            let ft = i8_ptr.fn_type(&[], false);
            Some(module.add_function("string_builder_new", ft, None))
        }),
        "string_builder_push" => module.get_function("string_builder_push").or_else(|| {
            Some(declare_fs_result_function(
                codegen_context,
                "string_builder_push",
                void_error_result_type,
                &[i8_ptr.into(), i8_ptr.into()],
            ))
        }),
        "string_builder_finish" => module.get_function("string_builder_finish").or_else(|| {
            let ft = pointer_error_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_builder_finish", ft, None))
        }),
        _ => None,
    }
}
