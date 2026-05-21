#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]

extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::rc_emitter::RcEmitter;
use crate::type_system::heap_class::{HeapClass, classify_core_type};
use crate::type_system::types::CoreType;
use alloc::format;
use inkwell::values::BasicValueEnum;

pub(crate) fn initialize_binding_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    value: BasicValueEnum<'context>,
    operation: &str,
    retain_new_value: bool,
) -> Result<(), CodegenError> {
    let Some(binding_snapshot) = env.variables.get(binding_name).cloned() else {
        return Err(CodegenError::new(format!(
            "{operation} target '{binding_name}' not found"
        )));
    };

    if retain_new_value {
        retain_new_binding_value_if_needed(codegen_context, &binding_snapshot.core_type, value)?;
    }

    codegen_context
        .builder
        .build_store(binding_snapshot.alloca, value)?;
    clear_array_binding_metadata(env, binding_name);
    Ok(())
}

pub(crate) fn store_binding_overwrite_rc_safe<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    value: BasicValueEnum<'context>,
    operation: &str,
) -> Result<(), CodegenError> {
    let Some(binding_snapshot) = env.variables.get(binding_name).cloned() else {
        return Err(CodegenError::new(format!(
            "{operation} target '{binding_name}' not found"
        )));
    };

    let rc_bearing_binding = is_rc_bearing_binding_core_type(&binding_snapshot.core_type);
    let old_value = rc_bearing_binding.then(|| {
        codegen_context.builder.build_load(
            binding_snapshot.alloca,
            &env.next_name(format!("{operation}.old.load").as_str()),
        )
    }).transpose()?;

    retain_new_binding_value_if_needed(codegen_context, &binding_snapshot.core_type, value)?;
    codegen_context
        .builder
        .build_store(binding_snapshot.alloca, value)?;
    if let Some(previous_value) = old_value {
        release_old_binding_value_if_needed(
            codegen_context,
            &binding_snapshot.core_type,
            previous_value,
        )?;
    }
    clear_array_binding_metadata(env, binding_name);
    Ok(())
}

fn clear_array_binding_metadata(env: &mut CodegenEnv<'_>, binding_name: &str) {
    if let Some(binding) = env.variables.get_mut(binding_name) {
        if matches!(binding.core_type, CoreType::Array(_)) {
            binding.length = None;
            binding.capacity = None;
        }
    }
}

fn binding_heap_class(core_type: &CoreType) -> HeapClass {
    classify_core_type(core_type)
}

fn is_rc_bearing_binding_core_type(core_type: &CoreType) -> bool {
    matches!(binding_heap_class(core_type), HeapClass::ReferenceCounted)
}

fn retain_new_binding_value_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    if !is_rc_bearing_binding_core_type(core_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "RC-bearing binding type '{core_type}' expected pointer value during overwrite"
        )));
    }

    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_inc(value.into_pointer_value())
}

fn release_old_binding_value_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    if !is_rc_bearing_binding_core_type(core_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "RC-bearing binding type '{core_type}' expected pointer value during overwrite"
        )));
    }

    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_dec(value.into_pointer_value())
}
