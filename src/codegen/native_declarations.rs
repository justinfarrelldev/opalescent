//! Lower checked source function definitions with explicit native parameter ABI.
use super::binding_store::initialize_binding_value;
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::{CodegenEnv, VariableBinding};
use super::functions_call::{build_function_type, emit_default_return};
use super::statements::{codegen_statement, unwind_scope_without_cleanup};
use super::types::core_type_to_llvm;
use crate::ast::{BorrowKind, Parameter, Stmt};
use crate::type_system::types::CoreType;
use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::FunctionValue;

/// Declare a checked concrete source ABI before emitting any function bodies.
///
/// Native owned arrays are opaque value pointers; reference parameters
/// are addresses of original values, not copied callback-local substitutes.
///
/// # Errors
/// Rejects inconsistent signatures, unresolved generics and invalid lowering.
pub fn declare_source_function<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    parameters: &[Parameter],
    signature: &CoreType,
) -> Result<FunctionValue<'context>, CodegenError> {
    let &CoreType::Function {
        ref generic_params,
        parameters: ref parameter_types,
        ref return_types,
        ref error_types,
    } = signature
    else {
        return Err(CodegenError::new(
            "native source declaration requires a checked function signature",
        ));
    };
    if !generic_params.is_empty() {
        return Err(CodegenError::new(
            "generic source functions require concrete monomorphization before native declaration",
        ));
    }
    if parameters.len() != parameter_types.len() {
        return Err(CodegenError::new(
            "native source parameter metadata does not match its signature",
        ));
    }
    let llvm_parameters: Vec<BasicMetadataTypeEnum<'context>> = parameters
        .iter()
        .zip(parameter_types)
        .map(|(parameter, core)| {
            let value = core_type_to_llvm(codegen.context, core);
            match parameter.borrow_kind {
                BorrowKind::Owned => value.into(),
                BorrowKind::Ref | BorrowKind::MutableRef => {
                    value.ptr_type(AddressSpace::default()).into()
                }
            }
        })
        .collect();
    let original = build_function_type(codegen, &llvm_parameters, return_types, error_types)?;
    codegen.declare_native_test_function(name, original, None)
}

/// Compile a checked source body against its previously declared native ABI.
///
/// # Errors
/// Rejects conflicting declarations, duplicate bodies and invalid lowering.
pub fn emit_source_function<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    name: &str,
    parameters: &[Parameter],
    signature: &CoreType,
    body: &Stmt,
) -> Result<FunctionValue<'context>, CodegenError> {
    let function = declare_source_function(codegen, name, parameters, signature)?;
    let &CoreType::Function {
        parameters: ref parameter_types,
        ref return_types,
        ..
    } = signature
    else {
        return Err(CodegenError::new(
            "native source body requires a checked function signature",
        ));
    };
    if function.count_basic_blocks() != 0 {
        return Err(CodegenError::new(
            "native source function body was already emitted",
        ));
    }
    codegen
        .builder
        .position_at_end(codegen.context.append_basic_block(function, "entry"));
    env.imported_signatures
        .insert(name.to_owned(), signature.clone());
    let _parameter_scope = env.enter_scope();
    let previous = bind_parameters(codegen, env, function, parameters, parameter_types)?;
    env.current_function_return_types.push(return_types.clone());
    let result =
        env.with_loop_isolated(|environment| codegen_statement(codegen, environment, body));
    env.current_function_return_types.pop();
    result?;
    if codegen
        .builder
        .get_insert_block()
        .is_some_and(|block| block.get_terminator().is_none())
    {
        emit_default_return(codegen, env, return_types)?;
    }
    unwind_scope_without_cleanup(env);
    for (parameter_name, original_binding) in previous {
        if let Some(binding) = original_binding {
            env.variables.insert(parameter_name, binding);
        } else {
            env.variables.remove(&parameter_name);
        }
    }
    Ok(function)
}

/// Previous visible bindings are restored after the parameter scope ends.
type ShadowedParameters<'context> = Vec<(String, Option<VariableBinding<'context>>)>;

/// Borrowed parameters alias original storage and do not acquire RC ownership.
fn bind_parameters<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    function: FunctionValue<'context>,
    parameters: &[Parameter],
    types: &[CoreType],
) -> Result<ShadowedParameters<'context>, CodegenError> {
    let mut previous = Vec::new();
    for (index, (parameter, core)) in parameters.iter().zip(types).enumerate() {
        let index = u32::try_from(index).map_err(|error| CodegenError::new(error.to_string()))?;
        let argument = function.get_nth_param(index).ok_or_else(|| {
            CodegenError::new("native source function omitted a declared parameter")
        })?;
        let borrowed = parameter.borrow_kind != BorrowKind::Owned;
        let slot = if borrowed {
            argument.into_pointer_value()
        } else {
            codegen
                .builder
                .build_alloca(argument.get_type(), &parameter.name)?
        };
        previous.push((
            parameter.name.clone(),
            env.variables.insert(
                parameter.name.clone(),
                VariableBinding {
                    alloca: slot,
                    core_type: core.clone(),
                    length: None,
                    capacity: None,
                    is_mutable: parameter.borrow_kind == BorrowKind::MutableRef,
                },
            ),
        ));
        if !borrowed {
            env.register_scope_binding(&parameter.name);
            initialize_binding_value(
                codegen,
                env,
                &parameter.name,
                argument,
                "native parameter",
                matches!(*core, CoreType::String | CoreType::Array(_)),
            )?;
        }
    }
    Ok(previous)
}
