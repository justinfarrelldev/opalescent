extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenError;
use alloc::format;
use alloc::string::String;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;

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
        _ => None,
    }
}

#[doc = "Resolve imported stdlib symbol to concrete runtime function name."]
pub fn resolve_imported_runtime_name(
    module_name: &str,
    symbol_name: &str,
) -> Result<String, CodegenError> {
    match (module_name, symbol_name) {
        ("standard", "take_input") => Ok("take_input".to_owned()),
        ("standard", "print") => Ok("print".to_owned()),
        ("standard", "print_string") => Ok("print_string".to_owned()),
        ("standard", "print_int8") => Ok("print_int8".to_owned()),
        ("standard", "print_int16") => Ok("print_int16".to_owned()),
        ("standard", "print_int32") => Ok("print_int32".to_owned()),
        ("standard", "print_int64") => Ok("print_int64".to_owned()),
        ("standard", "print_uint8") => Ok("print_uint8".to_owned()),
        ("standard", "print_uint16") => Ok("print_uint16".to_owned()),
        ("standard", "print_uint32") => Ok("print_uint32".to_owned()),
        ("standard", "print_uint64") => Ok("print_uint64".to_owned()),
        ("standard", "print_float32") => Ok("print_float32".to_owned()),
        ("standard", "print_float64") => Ok("print_float64".to_owned()),
        ("standard", "string_to_int8") => Ok("string_to_int8".to_owned()),
        ("standard", "string_to_int16") => Ok("string_to_int16".to_owned()),
        ("standard", "string_to_int32") => Ok("string_to_int32".to_owned()),
        ("standard", "string_to_int64") => Ok("string_to_int64".to_owned()),
        ("standard", "string_to_uint8") => Ok("string_to_uint8".to_owned()),
        ("standard", "string_to_uint16") => Ok("string_to_uint16".to_owned()),
        ("standard", "string_to_uint32") => Ok("string_to_uint32".to_owned()),
        ("standard", "string_to_uint64") => Ok("string_to_uint64".to_owned()),
        ("standard", "string_to_float32") => Ok("string_to_float32".to_owned()),
        ("standard", "string_to_float64") => Ok("string_to_float64".to_owned()),
        ("standard", "int8_to_string") => Ok("int8_to_string".to_owned()),
        ("standard", "int16_to_string") => Ok("int16_to_string".to_owned()),
        ("standard", "int32_to_string") => Ok("int32_to_string".to_owned()),
        ("standard", "int64_to_string") => Ok("int64_to_string".to_owned()),
        ("standard", "uint8_to_string") => Ok("uint8_to_string".to_owned()),
        ("standard", "uint16_to_string") => Ok("uint16_to_string".to_owned()),
        ("standard", "uint32_to_string") => Ok("uint32_to_string".to_owned()),
        ("standard", "uint64_to_string") => Ok("uint64_to_string".to_owned()),
        ("standard", "float32_to_string") => Ok("float32_to_string".to_owned()),
        ("standard", "float64_to_string") => Ok("float64_to_string".to_owned()),
        ("standard", "bool_to_string") => Ok("bool_to_string".to_owned()),
        ("math", "random_int8") => Ok("random_int8".to_owned()),
        ("math", "random_int16") => Ok("random_int16".to_owned()),
        ("math", "random_int32") => Ok("random_int32".to_owned()),
        ("math", "random_int64") => Ok("random_int64".to_owned()),
        ("math", "random_uint8") => Ok("random_uint8".to_owned()),
        ("math", "random_uint16") => Ok("random_uint16".to_owned()),
        ("math", "random_uint32") => Ok("random_uint32".to_owned()),
        ("math", "random_uint64") => Ok("random_uint64".to_owned()),
        _ => Err(CodegenError::new(format!(
            "unknown import symbol '{symbol_name}' in module '{module_name}'"
        ))),
    }
}
