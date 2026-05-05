extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use alloc::format;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::values::FunctionValue;

#[doc = "Declare a stdlib function in the LLVM module if not already present."]
#[expect(
    clippy::too_many_lines,
    reason = "stdlib function declarations are necessarily verbose"
)]
#[expect(
    clippy::similar_names,
    reason = "numeric type names are intentionally similar"
)]
pub fn declare_stdlib_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    if !STDLIB_NAMES.contains(&name) {
        return None;
    }

    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
    let void_type = ctx.void_type();
    let i8_type = ctx.i8_type();
    let i16_type = ctx.i16_type();
    let i32_type = ctx.i32_type();
    let i64_type = ctx.i64_type();
    let f32_type = ctx.f32_type();
    let f64_type = ctx.f64_type();
    let parse_result_i8_type = ctx.struct_type(&[i8_type.into(), i8_ptr.into()], false);
    let parse_result_i16_type = ctx.struct_type(&[i16_type.into(), i8_ptr.into()], false);
    let parse_result_i32_type = ctx.struct_type(&[i32_type.into(), i8_ptr.into()], false);
    let parse_result_i64_type = ctx.struct_type(&[i64_type.into(), i8_ptr.into()], false);
    let parse_result_f32_type = ctx.struct_type(&[f32_type.into(), i8_ptr.into()], false);
    let parse_result_f64_type = ctx.struct_type(&[f64_type.into(), i8_ptr.into()], false);
    // Bytes fallible operations mirror the `{value_ptr, error_cstr}` shape
    // used by `string_to_intN`, letting `guard`/`propagate` lower identically.
    let bytes_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let fs_void_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let fs_boolean_result_type = ctx.struct_type(&[i8_type.into(), i8_ptr.into()], false);
    let fs_metadata_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let fs_path_array_result_type = ctx.struct_type(
        &[
            i8_ptr.ptr_type(AddressSpace::default()).into(),
            i64_type.into(),
            i8_ptr.into(),
        ],
        false,
    );

    // Helper: get existing or add a new void(T) function.
    macro_rules! void_fn {
        ($nm:expr, $param:expr) => {
            module.get_function($nm).or_else(|| {
                let ft = void_type.fn_type(&[$param.into()], false);
                Some(module.add_function($nm, ft, None))
            })
        };
    }
    // Helper: get existing or add T(T, T) function.
    macro_rules! binary_int_fn {
        ($nm:expr, $t:expr) => {
            module.get_function($nm).or_else(|| {
                let ft = $t.fn_type(&[$t.into(), $t.into()], false);
                Some(module.add_function($nm, ft, None))
            })
        };
    }
    match name {
        "print" => module.get_function("puts").or_else(|| {
            let ft = i32_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("puts", ft, None))
        }),
        "printf" => module.get_function("printf").or_else(|| {
            let ft = i32_type.fn_type(&[i8_ptr.into()], true);
            Some(module.add_function("printf", ft, None))
        }),
        "take_input" => module.get_function("take_input").or_else(|| {
            let ft = i8_ptr.fn_type(&[], false);
            Some(module.add_function("take_input", ft, None))
        }),
        "print_string" => void_fn!("print_string", i8_ptr),
        "print_int8" => void_fn!("print_int8", i8_type),
        "print_int16" => void_fn!("print_int16", i16_type),
        "print_int32" => void_fn!("print_int32", i32_type),
        "print_int64" => void_fn!("print_int64", i64_type),
        "print_uint8" => void_fn!("print_uint8", i8_type),
        "print_uint16" => void_fn!("print_uint16", i16_type),
        "print_uint32" => void_fn!("print_uint32", i32_type),
        "print_uint64" => void_fn!("print_uint64", i64_type),
        "print_float32" => void_fn!("print_float32", f32_type),
        "print_float64" => void_fn!("print_float64", f64_type),
        "random_int8" => binary_int_fn!("random_int8", i8_type),
        "random_int16" => binary_int_fn!("random_int16", i16_type),
        "random_int32" => binary_int_fn!("random_int32", i32_type),
        "random_int64" => binary_int_fn!("random_int64", i64_type),
        "random_uint8" => binary_int_fn!("random_uint8", i8_type),
        "random_uint16" => binary_int_fn!("random_uint16", i16_type),
        "random_uint32" => binary_int_fn!("random_uint32", i32_type),
        "random_uint64" => binary_int_fn!("random_uint64", i64_type),
        "string_to_int8" => module.get_function("string_to_int8").or_else(|| {
            let ft = parse_result_i8_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_int8", ft, None))
        }),
        "string_to_int16" => module.get_function("string_to_int16").or_else(|| {
            let ft = parse_result_i16_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_int16", ft, None))
        }),
        "string_to_int32" => module.get_function("string_to_int32").or_else(|| {
            let ft = parse_result_i32_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_int32", ft, None))
        }),
        "string_to_int64" => module.get_function("string_to_int64").or_else(|| {
            let ft = parse_result_i64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_int64", ft, None))
        }),
        "string_to_uint8" => module.get_function("string_to_uint8").or_else(|| {
            let ft = parse_result_i8_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_uint8", ft, None))
        }),
        "string_to_uint16" => module.get_function("string_to_uint16").or_else(|| {
            let ft = parse_result_i16_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_uint16", ft, None))
        }),
        "string_to_uint32" => module.get_function("string_to_uint32").or_else(|| {
            let ft = parse_result_i32_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_uint32", ft, None))
        }),
        "string_to_uint64" => module.get_function("string_to_uint64").or_else(|| {
            let ft = parse_result_i64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_uint64", ft, None))
        }),
        "string_to_float32" => module.get_function("string_to_float32").or_else(|| {
            let ft = parse_result_f32_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_float32", ft, None))
        }),
        "string_to_float64" => module.get_function("string_to_float64").or_else(|| {
            let ft = parse_result_f64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_to_float64", ft, None))
        }),
        "int8_to_string" => module.get_function("int8_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_type.into()], false);
            Some(module.add_function("int8_to_string", ft, None))
        }),
        "int16_to_string" => module.get_function("int16_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i16_type.into()], false);
            Some(module.add_function("int16_to_string", ft, None))
        }),
        "int32_to_string" => module.get_function("int32_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i32_type.into()], false);
            Some(module.add_function("int32_to_string", ft, None))
        }),
        "int64_to_string" => module.get_function("int64_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i64_type.into()], false);
            Some(module.add_function("int64_to_string", ft, None))
        }),
        "uint8_to_string" => module.get_function("uint8_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_type.into()], false);
            Some(module.add_function("uint8_to_string", ft, None))
        }),
        "uint16_to_string" => module.get_function("uint16_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i16_type.into()], false);
            Some(module.add_function("uint16_to_string", ft, None))
        }),
        "uint32_to_string" => module.get_function("uint32_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i32_type.into()], false);
            Some(module.add_function("uint32_to_string", ft, None))
        }),
        "uint64_to_string" => module.get_function("uint64_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i64_type.into()], false);
            Some(module.add_function("uint64_to_string", ft, None))
        }),
        "float32_to_string" => module.get_function("float32_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[f32_type.into()], false);
            Some(module.add_function("float32_to_string", ft, None))
        }),
        "float64_to_string" => module.get_function("float64_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[f64_type.into()], false);
            Some(module.add_function("float64_to_string", ft, None))
        }),
        "bool_to_string" => module.get_function("bool_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_type.into()], false);
            Some(module.add_function("bool_to_string", ft, None))
        }),
        "string_length" => module.get_function("string_length").or_else(|| {
            let ft = i64_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("string_length", ft, None))
        }),
        "array_length" => module.get_function("array_length").or_else(|| {
            let ft = i64_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
            Some(module.add_function("array_length", ft, None))
        }),
        "opal_array_bounds_error" => module.get_function("opal_array_bounds_error").or_else(|| {
            let ft = void_type.fn_type(&[i64_type.into(), i64_type.into()], false);
            Some(module.add_function("opal_array_bounds_error", ft, None))
        }),
        "opal_runtime_error" => void_fn!("opal_runtime_error", i8_ptr),
        "bytes_new" => module.get_function("bytes_new").or_else(|| {
            let ft = i8_ptr.fn_type(&[], false);
            Some(module.add_function("bytes_new", ft, None))
        }),
        "bytes_length" => module.get_function("bytes_length").or_else(|| {
            let ft = i32_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("bytes_length", ft, None))
        }),
        "bytes_to_hex" => module.get_function("bytes_to_hex").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("bytes_to_hex", ft, None))
        }),
        "bytes_concatenate" => module.get_function("bytes_concatenate").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("bytes_concatenate", ft, None))
        }),
        "bytes_from_hex" => module.get_function("bytes_from_hex").or_else(|| {
            let ft = bytes_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("bytes_from_hex", ft, None))
        }),
        "bytes_slice" => module.get_function("bytes_slice").or_else(|| {
            let ft = bytes_result_type
                .fn_type(&[i8_ptr.into(), i32_type.into(), i32_type.into()], false);
            Some(module.add_function("bytes_slice", ft, None))
        }),
        // ── T5: Path manipulation ─────────────────────────────────────────────
        // FsPathResult { char* value; const char* error; }
        "path_from" => module.get_function("path_from").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_from", ft, None))
        }),
        "join_path_components" => module.get_function("join_path_components").or_else(|| {
            // base: i8*, components: i8**, count: i64
            let ft = i8_ptr.fn_type(
                &[
                    i8_ptr.into(),
                    i8_ptr.ptr_type(AddressSpace::default()).into(),
                    i64_type.into(),
                ],
                false,
            );
            Some(module.add_function("join_path_components", ft, None))
        }),
        "path_parent_directory" => module.get_function("path_parent_directory").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_parent_directory", ft, None))
        }),
        "path_file_name" => module.get_function("path_file_name").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_file_name", ft, None))
        }),
        "path_file_extension" => module.get_function("path_file_extension").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_file_extension", ft, None))
        }),
        "normalize_path" => module.get_function("normalize_path").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("normalize_path", ft, None))
        }),
        "path_to_string" => module.get_function("path_to_string").or_else(|| {
            let ft = i8_ptr.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_to_string", ft, None))
        }),
        "absolute_path_sync" => module.get_function("absolute_path_sync").or_else(|| {
            let fs_path_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
            let ft = fs_path_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("absolute_path_sync", ft, None))
        }),
        "read_contents_sync" => module.get_function("read_contents_sync").or_else(|| {
            let fs_bytes_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
            let ft = fs_bytes_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("read_contents_sync", ft, None))
        }),
        "read_text_sync" => module.get_function("read_text_sync").or_else(|| {
            let fs_string_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
            let ft = fs_string_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("read_text_sync", ft, None))
        }),
        "read_first_line_sync" => module.get_function("read_first_line_sync").or_else(|| {
            let fs_string_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
            let ft = fs_string_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("read_first_line_sync", ft, None))
        }),
        "read_lines_sync" => module.get_function("read_lines_sync").or_else(|| {
            let fs_string_array_result_type = ctx.struct_type(
                &[
                    i8_ptr.ptr_type(AddressSpace::default()).into(),
                    i64_type.into(),
                    i8_ptr.into(),
                ],
                false,
            );
            let ft = void_type.fn_type(
                &[
                    fs_string_array_result_type
                        .ptr_type(AddressSpace::default())
                        .into(),
                    i8_ptr.into(),
                ],
                false,
            );
            let function = module.add_function("read_lines_sync", ft, None);
            apply_sret_attr_if_needed(
                codegen_context,
                function,
                fs_string_array_result_type.into(),
            );
            Some(function)
        }),
        "read_bytes_at_offset_sync" => {
            module
                .get_function("read_bytes_at_offset_sync")
                .or_else(|| {
                    let fs_bytes_result_type =
                        ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
                    let ft = fs_bytes_result_type
                        .fn_type(&[i8_ptr.into(), i64_type.into(), i64_type.into()], false);
                    Some(module.add_function("read_bytes_at_offset_sync", ft, None))
                })
        }
        "write_contents_sync" => module.get_function("write_contents_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("write_contents_sync", ft, None))
        }),
        "write_text_sync" => module.get_function("write_text_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("write_text_sync", ft, None))
        }),
        "write_contents_atomic_sync" => {
            module
                .get_function("write_contents_atomic_sync")
                .or_else(|| {
                    let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
                    Some(module.add_function("write_contents_atomic_sync", ft, None))
                })
        }
        "write_text_atomic_sync" => module.get_function("write_text_atomic_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("write_text_atomic_sync", ft, None))
        }),
        "append_contents_sync" => module.get_function("append_contents_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("append_contents_sync", ft, None))
        }),
        "append_text_sync" => module.get_function("append_text_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("append_text_sync", ft, None))
        }),
        "write_bytes_at_offset_sync" => {
            module
                .get_function("write_bytes_at_offset_sync")
                .or_else(|| {
                    let ft = fs_void_result_type
                        .fn_type(&[i8_ptr.into(), i64_type.into(), i8_ptr.into()], false);
                    Some(module.add_function("write_bytes_at_offset_sync", ft, None))
                })
        }
        "create_file_sync" => module.get_function("create_file_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("create_file_sync", ft, None))
        }),
        "delete_file_sync" => module.get_function("delete_file_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("delete_file_sync", ft, None))
        }),
        "copy_file_sync" => module.get_function("copy_file_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("copy_file_sync", ft, None))
        }),
        "move_path_sync" => module.get_function("move_path_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            Some(module.add_function("move_path_sync", ft, None))
        }),
        "path_exists_sync" => module.get_function("path_exists_sync").or_else(|| {
            let ft = fs_boolean_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("path_exists_sync", ft, None))
        }),
        "read_metadata_sync" => module.get_function("read_metadata_sync").or_else(|| {
            let ft = fs_metadata_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("read_metadata_sync", ft, None))
        }),
        "read_metadata_nofollow_sync" => module
            .get_function("read_metadata_nofollow_sync")
            .or_else(|| {
                let ft = fs_metadata_result_type.fn_type(&[i8_ptr.into()], false);
                Some(module.add_function("read_metadata_nofollow_sync", ft, None))
            }),
        "create_directory_sync" => module.get_function("create_directory_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("create_directory_sync", ft, None))
        }),
        "create_directory_recursive_sync" => module
            .get_function("create_directory_recursive_sync")
            .or_else(|| {
                let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
                Some(module.add_function("create_directory_recursive_sync", ft, None))
            }),
        "delete_directory_sync" => module.get_function("delete_directory_sync").or_else(|| {
            let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("delete_directory_sync", ft, None))
        }),
        "delete_directory_recursive_sync" => module
            .get_function("delete_directory_recursive_sync")
            .or_else(|| {
                let ft = fs_void_result_type.fn_type(&[i8_ptr.into()], false);
                Some(module.add_function("delete_directory_recursive_sync", ft, None))
            }),
        "list_directory_sync" => module.get_function("list_directory_sync").or_else(|| {
            let ft = void_type.fn_type(
                &[
                    fs_path_array_result_type
                        .ptr_type(AddressSpace::default())
                        .into(),
                    i8_ptr.into(),
                ],
                false,
            );
            let function = module.add_function("list_directory_sync", ft, None);
            apply_sret_attr_if_needed(codegen_context, function, fs_path_array_result_type.into());
            Some(function)
        }),
        "is_file_sync" => module.get_function("is_file_sync").or_else(|| {
            let ft = fs_boolean_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("is_file_sync", ft, None))
        }),
        "is_file_nofollow_sync" => module.get_function("is_file_nofollow_sync").or_else(|| {
            let ft = fs_boolean_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("is_file_nofollow_sync", ft, None))
        }),
        "is_directory_sync" => module.get_function("is_directory_sync").or_else(|| {
            let ft = fs_boolean_result_type.fn_type(&[i8_ptr.into()], false);
            Some(module.add_function("is_directory_sync", ft, None))
        }),
        "is_directory_nofollow_sync" => {
            module
                .get_function("is_directory_nofollow_sync")
                .or_else(|| {
                    let ft = fs_boolean_result_type.fn_type(&[i8_ptr.into()], false);
                    Some(module.add_function("is_directory_nofollow_sync", ft, None))
                })
        }
        _ => None,
    }
}

#[doc = "Apply the LLVM `sret` attribute to the hidden result pointer when the ABI requires it."]
fn apply_sret_attr_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    function: FunctionValue<'context>,
    struct_type: inkwell::types::AnyTypeEnum<'context>,
) {
    if codegen_context.target.platform != crate::build_system::targets::Platform::Linux {
        return;
    }

    let sret_kind = Attribute::get_named_enum_kind_id("sret");
    if sret_kind == 0 {
        return;
    }

    let sret_attr = codegen_context
        .context
        .create_type_attribute(sret_kind, struct_type);
    function.add_attribute(AttributeLoc::Param(0), sret_attr);
}

#[doc = "Resolve imported stdlib symbol to concrete runtime function name."]
pub fn resolve_imported_runtime_name(
    module_name: &str,
    symbol_name: &str,
) -> Result<String, CodegenError> {
    match (module_name, symbol_name) {
        ("standard" | "math", name) if STDLIB_NAMES.contains(&name) => Ok(name.to_owned()),
        _ => Err(CodegenError::new(format!(
            "unknown import symbol '{symbol_name}' in module '{module_name}'"
        ))),
    }
}

#[doc = "Authoritative list of all stdlib function names."]
pub const STDLIB_NAMES: &[&str] = &[
    "print",
    "printf",
    "take_input",
    "print_string",
    "print_int8",
    "print_int16",
    "print_int32",
    "print_int64",
    "print_uint8",
    "print_uint16",
    "print_uint32",
    "print_uint64",
    "print_float32",
    "print_float64",
    "random_int8",
    "random_int16",
    "random_int32",
    "random_int64",
    "random_uint8",
    "random_uint16",
    "random_uint32",
    "random_uint64",
    "string_to_int8",
    "string_to_int16",
    "string_to_int32",
    "string_to_int64",
    "string_to_uint8",
    "string_to_uint16",
    "string_to_uint32",
    "string_to_uint64",
    "string_to_float32",
    "string_to_float64",
    "int8_to_string",
    "int16_to_string",
    "int32_to_string",
    "int64_to_string",
    "uint8_to_string",
    "uint16_to_string",
    "uint32_to_string",
    "uint64_to_string",
    "float32_to_string",
    "float64_to_string",
    "bool_to_string",
    "string_length",
    "array_length",
    "opal_array_bounds_error",
    "opal_runtime_error",
    "bytes_new",
    "bytes_length",
    "bytes_to_hex",
    "bytes_concatenate",
    "bytes_from_hex",
    "bytes_slice",
    "path_from",
    "join_path_components",
    "path_parent_directory",
    "path_file_name",
    "path_file_extension",
    "normalize_path",
    "path_to_string",
    "absolute_path_sync",
    "read_contents_sync",
    "read_text_sync",
    "read_first_line_sync",
    "read_lines_sync",
    "read_bytes_at_offset_sync",
    "write_contents_sync",
    "write_text_sync",
    "write_contents_atomic_sync",
    "write_text_atomic_sync",
    "append_contents_sync",
    "append_text_sync",
    "write_bytes_at_offset_sync",
    "create_file_sync",
    "delete_file_sync",
    "copy_file_sync",
    "move_path_sync",
    "path_exists_sync",
    "read_metadata_sync",
    "read_metadata_nofollow_sync",
    "create_directory_sync",
    "create_directory_recursive_sync",
    "delete_directory_sync",
    "delete_directory_recursive_sync",
    "list_directory_sync",
    "is_file_sync",
    "is_file_nofollow_sync",
    "is_directory_sync",
    "is_directory_nofollow_sync",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdlib_names_registry_exists_and_has_correct_count() {
        assert_eq!(
            STDLIB_NAMES.len(),
            89,
            "stdlib registry should have 89 names"
        );
        assert!(
            STDLIB_NAMES.contains(&"opal_runtime_error"),
            "opal_runtime_error should be in registry"
        );
        assert!(
            STDLIB_NAMES.contains(&"print"),
            "print should be in registry"
        );
        assert!(
            STDLIB_NAMES.contains(&"random_int32"),
            "random_int32 should be in registry"
        );
    }
}
