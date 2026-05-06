#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::Type;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::type_mapping::{AstTypeMappingError, ast_type_to_core_type};
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, FunctionValue};

pub(super) fn declare_external_imported_function<'context>(
    codegen_context: &CodegenContext<'context>,
    function_name: &str,
    signature: &CoreType,
) -> Result<FunctionValue<'context>, CodegenError> {
    if let Some(existing) = codegen_context.module.get_function(function_name) {
        return Ok(existing);
    }

    let CoreType::Function {
        ref parameters,
        ref return_types,
        ref error_types,
        ..
    } = *signature
    else {
        return Err(CodegenError::new(format!(
            "imported symbol '{function_name}' is not callable"
        )));
    };

    let mut lowered_parameter_core_types = Vec::new();
    for core_type in parameters {
        lowered_parameter_core_types.push(core_type.clone());
        if matches!(*core_type, CoreType::Array(_)) {
            lowered_parameter_core_types.push(CoreType::Int64);
        }
    }

    let parameter_types = lowered_parameter_core_types
        .iter()
        .map(|core_type| core_type_to_llvm(codegen_context.context, core_type).into())
        .collect::<Vec<BasicMetadataTypeEnum<'context>>>();
    let function_type =
        build_function_type(codegen_context, &parameter_types, return_types, error_types)?;

    Ok(codegen_context.module.add_function(
        function_name,
        function_type,
        Some(inkwell::module::Linkage::External),
    ))
}

pub fn build_function_type<'context>(
    codegen_context: &CodegenContext<'context>,
    parameters: &[BasicMetadataTypeEnum<'context>],
    returns: &[CoreType],
    error_types: &[CoreType],
) -> Result<inkwell::types::FunctionType<'context>, CodegenError> {
    if error_types.is_empty() {
        if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
            return Ok(codegen_context
                .context
                .void_type()
                .fn_type(parameters, false));
        }
        if returns.len() == 1 {
            let return_type = core_type_to_llvm(codegen_context.context, &returns[0]);
            return Ok(return_type.fn_type(parameters, false));
        }
        let aggregate_fields = returns
            .iter()
            .map(|core_type| core_type_to_llvm(codegen_context.context, core_type))
            .collect::<Vec<_>>();
        let aggregate = codegen_context
            .context
            .struct_type(aggregate_fields.as_slice(), false);
        return Ok(aggregate.fn_type(parameters, false));
    }

    if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
        return Ok(crate::codegen::error_abi::build_error_return_type(
            codegen_context.context,
            None,
        )
        .fn_type(parameters, false));
    }

    if returns.len() == 1 {
        let success_type = core_type_to_llvm(codegen_context.context, &returns[0]);
        if matches!(
            success_type,
            inkwell::types::BasicTypeEnum::ArrayType(_)
                | inkwell::types::BasicTypeEnum::StructType(_)
                | inkwell::types::BasicTypeEnum::VectorType(_)
                | inkwell::types::BasicTypeEnum::ScalableVectorType(_)
        ) {
            return unsupported_error_return_type(&returns[0]);
        }
        return Ok(crate::codegen::error_abi::build_error_return_type(
            codegen_context.context,
            Some(success_type),
        )
        .fn_type(parameters, false));
    }

    unsupported_error_return_type(&CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: returns.to_vec(),
        error_types: error_types.to_vec(),
    })
}

fn unsupported_error_return_type<'context>(
    return_type: &CoreType,
) -> Result<inkwell::types::FunctionType<'context>, CodegenError> {
    Err(CodegenError::new(format!(
        "aggregate error return type '{return_type:?}' not yet supported; only Unit and scalar/pointer returns can use an errors ABI"
    )))
}

pub fn emit_default_return(
    codegen_context: &CodegenContext<'_>,
    env: &mut CodegenEnv<'_>,
    returns: &[CoreType],
) -> Result<(), CodegenError> {
    if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }

    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context
        .builder
        .build_global_string_ptr("missing return statement", &env.next_name("ret.msg"))?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        runtime_fn,
        &[msg.into()],
        &env.next_name("ret.call"),
    )?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;
    Ok(())
}

pub fn ast_type_to_core_type_for_signature(ast_type: &Type) -> Result<CoreType, CodegenError> {
    if matches!(*ast_type, Type::Function { .. }) {
        return Err(CodegenError::new(String::from(
            "unsupported function type annotation",
        )));
    }

    ast_type_to_core_type(ast_type).map_err(|error| match error {
        AstTypeMappingError::TypeNotFound { type_name, .. } => {
            CodegenError::new(format!("unsupported type '{type_name}'"))
        }
    })
}

/// Emit C ABI main wrapper that dispatches to the compiled Opalescent entry.
///
/// # Panics
/// Panics if the synthesized C `main` function is missing its expected argc/argv parameters.
pub fn emit_c_main_wrapper<'context>(
    codegen_context: &CodegenContext<'context>,
    entry_function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    if codegen_context.module.get_function("main").is_some() {
        return Ok(());
    }
    let i32_type = codegen_context.context.i32_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let argv_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let c_main_type = i32_type.fn_type(&[i32_type.into(), argv_type.into()], false);
    let c_main = codegen_context
        .module
        .add_function("main", c_main_type, None);
    let block = codegen_context.context.append_basic_block(c_main, "entry");
    codegen_context.builder.position_at_end(block);

    let argc_param = c_main
        .get_nth_param(0)
        .expect("main must have argc param")
        .into_int_value();
    let argv_param = c_main
        .get_nth_param(1)
        .expect("main must have argv param")
        .into_pointer_value();

    let parameter_types = entry_function.get_type().get_param_types();
    let mut call_args: Vec<BasicMetadataValueEnum<'context>> =
        Vec::with_capacity(parameter_types.len());
    let mut argv_forwarded = false;
    for parameter_type in parameter_types {
        let argument = match parameter_type {
            BasicMetadataTypeEnum::PointerType(pointer_type) if !argv_forwarded => {
                argv_forwarded = true;
                argv_param.const_cast(pointer_type).into()
            }
            BasicMetadataTypeEnum::IntType(int_type)
                if argv_forwarded && int_type.get_bit_width() == 64 =>
            {
                codegen_context
                    .builder
                    .build_int_z_extend(argc_param, int_type, "entry.argc.i64")?
                    .into()
            }
            BasicMetadataTypeEnum::FloatType(float_type) => float_type.const_zero().into(),
            BasicMetadataTypeEnum::IntType(int_type) => int_type.const_zero().into(),
            BasicMetadataTypeEnum::PointerType(pointer_type) => pointer_type.const_null().into(),
            BasicMetadataTypeEnum::ArrayType(array_type) => array_type.const_zero().into(),
            BasicMetadataTypeEnum::StructType(struct_type) => struct_type.const_zero().into(),
            BasicMetadataTypeEnum::VectorType(vector_type) => vector_type.const_zero().into(),
            BasicMetadataTypeEnum::ScalableVectorType(vector_type) => {
                vector_type.get_undef().into()
            }
            BasicMetadataTypeEnum::MetadataType(_) => {
                return Err(CodegenError::new(String::from(
                    "entry function cannot use metadata parameters",
                )));
            }
        };
        call_args.push(argument);
    }

    let _call =
        codegen_context
            .builder
            .build_call(entry_function, call_args.as_slice(), "entry.call")?;
    let _ret = codegen_context.builder.build_return(Some(
        &codegen_context.context.i32_type().const_int(0, false),
    ))?;
    Ok(())
}
