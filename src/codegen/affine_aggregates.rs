//! LLVM helpers for transactional affine aggregate rollback and seal markers.

extern crate alloc;

use crate::ast::{ConstructorField, Expr};
use crate::codegen::adts::declare_or_get_nominal_drop_children_fn;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::expressions_array::requires_rc_runtime_hooks;
use crate::codegen::functions_call::emit_cleanup_aware_error_return;
use crate::codegen::rc_emitter::RcEmitter;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::affine_aggregates::{
    cleanup_operation_for_resource, terminal_affine_aggregate_specs,
};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue, StructValue};

/// One initialized provisional aggregate member awaiting seal or rollback.
#[derive(Clone, Copy)]
pub struct ProvisionalAggregateMember<'context> {
    /// Runtime cleanup operation for this member's resource type.
    pub cleanup_operation: &'static str,
    /// Opaque resource pointer value held provisionally by the compiler.
    pub value: PointerValue<'context>,
}

/// Emit rollback for initialized provisional members in reverse acquisition order.
pub fn emit_transactional_aggregate_rollback<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    current_fn: FunctionValue<'context>,
    original_error: PointerValue<'context>,
    provisional_members: &[ProvisionalAggregateMember<'context>],
) -> Result<(), CodegenError> {
    for member in provisional_members.iter().rev() {
        let cleanup_fn =
            declare_aggregate_cleanup_scaffold(codegen_context, member.cleanup_operation);
        let _cleanup_call = codegen_context.builder.build_call(
            cleanup_fn,
            &[member.value.into()],
            env.next_name("aggregate.rollback.cleanup").as_str(),
        )?;
    }
    emit_cleanup_aware_error_return(codegen_context, env, current_fn, Some(original_error), &[])
}

/// Build an allocation-free struct seal from finalized member values.
pub fn emit_allocation_free_aggregate_seal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[BasicValueEnum<'context>],
) -> Result<StructValue<'context>, CodegenError> {
    let field_types = values
        .iter()
        .map(BasicValueEnum::get_type)
        .collect::<Vec<_>>();
    let aggregate_type = codegen_context
        .context
        .struct_type(field_types.as_slice(), false);
    let mut aggregate = aggregate_type.get_undef();
    for (index, value) in values.iter().enumerate() {
        let field_index = u32::try_from(index)
            .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?;
        aggregate = codegen_context
            .builder
            .build_insert_value(
                aggregate,
                *value,
                field_index,
                env.next_name("aggregate.seal.insert").as_str(),
            )?
            .into_struct_value();
    }
    Ok(aggregate)
}

/// Lower a registered aggregate constructor if `type_name` names one.
pub fn maybe_codegen_transactional_aggregate_constructor<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    type_name: &str,
    fields: &[ConstructorField],
    field_layout: &[(String, CoreType)],
) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
    let Some(spec) = terminal_affine_aggregate_specs()
        .into_iter()
        .find(|spec| spec.type_name == type_name)
    else {
        return Ok(None);
    };
    let field_map = fields
        .iter()
        .map(|field| (field.name.as_str(), &field.value))
        .collect::<BTreeMap<_, _>>();
    let mut lowered_fields = Vec::with_capacity(field_layout.len());
    let mut provisional_members = Vec::new();
    for field in field_layout {
        let field_name = &field.0;
        let field_type = &field.1;
        let field_expr = field_map.get(field_name.as_str()).copied().ok_or_else(|| {
            CodegenError::new(format!(
                "missing field '{field_name}' for transactional aggregate '{}'",
                spec.type_name
            ))
        })?;
        let lowered = codegen_transactional_aggregate_field(
            codegen_context,
            env,
            field_expr,
            field_type,
            provisional_members.as_slice(),
        )?;
        if let Some(slot) = spec
            .slots
            .iter()
            .find(|slot| slot.name == *field_name && slot.owns_obligation)
        {
            if let Some(cleanup_operation) = cleanup_operation_for_resource(slot.type_name.as_str())
            {
                if lowered.is_pointer_value() {
                    provisional_members.push(ProvisionalAggregateMember {
                        cleanup_operation,
                        value: lowered.into_pointer_value(),
                    });
                }
            }
        }
        lowered_fields.push(lowered);
    }
    let sealed =
        emit_allocation_free_aggregate_seal(codegen_context, env, lowered_fields.as_slice())?;
    let payload =
        emit_nominal_aggregate_payload(codegen_context, env, type_name, field_layout, sealed)?;
    Ok(Some(payload.as_basic_value_enum()))
}

/// Store a sealed aggregate value in the nominal heap payload expected by generic ADTs.
fn emit_nominal_aggregate_payload<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    type_name: &str,
    field_layout: &[(String, CoreType)],
    sealed: StructValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let field_types = field_layout
        .iter()
        .map(|field| core_type_to_llvm(codegen_context.context, &field.1))
        .collect::<Vec<_>>();
    let struct_type = codegen_context
        .context
        .struct_type(field_types.as_slice(), false);
    let payload_size = struct_type.size_of().ok_or_else(|| {
        CodegenError::new(format!(
            "could not compute payload size for transactional aggregate '{type_name}'"
        ))
    })?;
    let nominal_core_type = CoreType::Generic {
        name: type_name.to_owned(),
        type_args: Vec::new(),
    };
    let drop_children_fn = if field_layout
        .iter()
        .any(|field| requires_rc_runtime_hooks(&field.1))
    {
        let callback = declare_or_get_nominal_drop_children_fn(
            codegen_context,
            &nominal_core_type,
            field_layout,
        )?;
        let i8_ptr_type = codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default());
        Some(
            callback
                .as_global_value()
                .as_pointer_value()
                .const_cast(i8_ptr_type),
        )
    } else {
        None
    };
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    let payload_ptr = emitter.emit_alloc(payload_size, drop_children_fn)?;
    let typed_ptr = codegen_context.builder.build_pointer_cast(
        payload_ptr,
        struct_type.ptr_type(AddressSpace::default()),
        env.next_name("aggregate.payload.cast").as_str(),
    )?;
    let _store = codegen_context.builder.build_store(typed_ptr, sealed)?;
    Ok(payload_ptr)
}

/// Lower one aggregate field and rollback already-initialized fields on failure.
fn codegen_transactional_aggregate_field<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    expected_type: &CoreType,
    provisional_members: &[ProvisionalAggregateMember<'context>],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let &Expr::Propagate { ref call, .. } = expr else {
        return codegen_expression(codegen_context, env, expr, Some(expected_type));
    };
    let value = codegen_expression(codegen_context, env, call.as_ref(), Some(expected_type))?;
    if !value.is_struct_value() {
        return Ok(value);
    }
    let struct_value = value.into_struct_value();
    let field_count = struct_value.get_type().count_fields();
    if field_count < 2 {
        return Ok(value);
    }
    let error_index = crate::codegen::error_abi::error_field_index(field_count);
    let error_field = codegen_context.builder.build_extract_value(
        struct_value,
        error_index,
        env.next_name("aggregate.field.err").as_str(),
    )?;
    if !error_field.is_pointer_value() {
        return Ok(value);
    }
    let error_ptr = error_field.into_pointer_value();
    let current_fn = current_function(codegen_context)?;
    let rollback_block = codegen_context.context.append_basic_block(
        current_fn,
        env.next_name("aggregate.constructor.rollback").as_str(),
    );
    let continue_block = codegen_context.context.append_basic_block(
        current_fn,
        env.next_name("aggregate.constructor.cont").as_str(),
    );
    let is_error = codegen_context
        .builder
        .build_is_not_null(error_ptr, env.next_name("aggregate.field.is_err").as_str())?;
    let _branch = codegen_context.builder.build_conditional_branch(
        is_error,
        rollback_block,
        continue_block,
    )?;
    codegen_context.builder.position_at_end(rollback_block);
    emit_transactional_aggregate_rollback(
        codegen_context,
        env,
        current_fn,
        error_ptr,
        provisional_members,
    )?;
    codegen_context.builder.position_at_end(continue_block);
    let success_count = crate::codegen::error_abi::error_field_index(field_count);
    if success_count == 1 {
        return codegen_context
            .builder
            .build_extract_value(
                struct_value,
                0,
                env.next_name("aggregate.field.ok").as_str(),
            )
            .map_err(CodegenError::from);
    }
    Ok(value)
}

/// Resolve current enclosing function from insertion point.
fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let Some(block) = codegen_context.builder.get_insert_block() else {
        return Err(CodegenError::new(String::from(
            "builder is not positioned in a block",
        )));
    };
    block.get_parent().ok_or_else(|| {
        CodegenError::new(String::from("insert block does not have a parent function"))
    })
}

/// Declare the private cleanup scaffold used by aggregate rollback.
fn declare_aggregate_cleanup_scaffold<'context>(
    codegen_context: &CodegenContext<'context>,
    cleanup_operation: &str,
) -> FunctionValue<'context> {
    let function_name = format!("__opal_aggregate_cleanup_{cleanup_operation}");
    if let Some(function) = codegen_context.module.get_function(function_name.as_str()) {
        return function;
    }
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    codegen_context.module.add_function(
        function_name.as_str(),
        codegen_context
            .context
            .void_type()
            .fn_type(&[i8_ptr.into()], false),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::context::CodegenContext;
    use crate::codegen::expressions::CodegenEnv;
    use inkwell::context::Context;
    use inkwell::values::{BasicValue, FunctionValue};

    /// Create a function returning the canonical two-field error aggregate.
    fn create_error_return_function<'context>(
        codegen_context: &CodegenContext<'context>,
        function_name: &str,
    ) -> FunctionValue<'context> {
        let i8_ptr = codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default());
        let result_type = codegen_context
            .context
            .struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
        let function = codegen_context.module.add_function(
            function_name,
            result_type.fn_type(&[], false),
            None,
        );
        let entry = codegen_context
            .context
            .append_basic_block(function, "entry");
        codegen_context.builder.position_at_end(entry);
        function
    }

    /// Create a void function with two pointer parameters and position the builder at entry.
    fn create_pointer_pair_function<'context>(
        codegen_context: &CodegenContext<'context>,
        function_name: &str,
    ) -> FunctionValue<'context> {
        let i8_ptr = codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default());
        let function = codegen_context.module.add_function(
            function_name,
            codegen_context
                .context
                .void_type()
                .fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
            None,
        );
        let entry = codegen_context
            .context
            .append_basic_block(function, "entry");
        codegen_context.builder.position_at_end(entry);
        function
    }

    #[test]
    fn terminal_aggregate_rollback_cleans_provisional_members_in_reverse_order() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "terminal_aggregate_rollback");
        let function =
            create_error_return_function(&codegen_context, "terminal_aggregate_rollback_fn");
        let mut env = CodegenEnv::new(true);
        let i8_ptr = context.i8_type().ptr_type(AddressSpace::default());
        let first = i8_ptr.const_null();
        let second = i8_ptr.const_null();
        let original_error = i8_ptr.const_null();

        emit_transactional_aggregate_rollback(
            &codegen_context,
            &mut env,
            function,
            original_error,
            &[
                ProvisionalAggregateMember {
                    cleanup_operation: "terminal_aggregate_first_drop",
                    value: first,
                },
                ProvisionalAggregateMember {
                    cleanup_operation: "terminal_aggregate_second_drop",
                    value: second,
                },
            ],
        )
        .expect("aggregate rollback should lower");

        let ir = codegen_context.module.print_to_string().to_string();
        let second_call = ir
            .find("call void @__opal_aggregate_cleanup_terminal_aggregate_second_drop")
            .expect("second member cleanup should be emitted");
        let first_call = ir
            .find("call void @__opal_aggregate_cleanup_terminal_aggregate_first_drop")
            .expect("first member cleanup should be emitted");
        assert!(
            second_call < first_call,
            "rollback must clean initialized provisional members in reverse acquisition order: {ir}"
        );
        assert!(
            ir.contains("cleanup.body"),
            "rollback should preserve the original constructor failure as the returned error: {ir}"
        );
    }

    #[test]
    fn terminal_aggregate_seal_is_allocation_free_and_publishes_once() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "terminal_aggregate_seal");
        let function = create_pointer_pair_function(&codegen_context, "terminal_aggregate_seal_fn");
        let mut env = CodegenEnv::new(true);
        let first = function
            .get_nth_param(0)
            .expect("seal test function should have a first parameter");
        let second = function
            .get_nth_param(1)
            .expect("seal test function should have a second parameter");
        let first = first.into_pointer_value();
        let second = second.into_pointer_value();

        let sealed = emit_allocation_free_aggregate_seal(
            &codegen_context,
            &mut env,
            &[first.as_basic_value_enum(), second.as_basic_value_enum()],
        )
        .expect("aggregate seal should lower");
        let _use_sealed = codegen_context
            .builder
            .build_extract_value(sealed, 0, "aggregate.seal.read")
            .expect("sealed aggregate field should be readable");

        let ir = codegen_context.module.print_to_string().to_string();
        assert!(
            ir.contains("aggregate.seal.insert"),
            "seal should atomically publish member values into one aggregate value: {ir}"
        );
        assert!(
            !ir.contains("malloc") && !ir.contains("aggregate.rollback.cleanup"),
            "successful seal must be allocation-free and must not leak provisional rollback calls: {ir}"
        );
    }
}
