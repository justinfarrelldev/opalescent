extern crate alloc;

use crate::ast::{Expr, Pattern};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::types::integer_literal_bits;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::IntValue;
use inkwell::values::{BasicValue, BasicValueEnum};

#[doc = "Instantiate a concrete ADT symbol name for generic arguments."]
#[must_use]
pub fn instantiate_generic_adt_name(name: &str, type_args: &[CoreType]) -> String {
    let mut specialized = name.to_owned();
    for type_arg in type_args {
        specialized.push_str("__");
        specialized.push_str(&render_type_arg(type_arg));
    }
    specialized
}

#[doc = "Lower constructor expressions for product and sum ADTs."]
pub fn codegen_constructor_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if let Expr::Constructor {
        ref callee,
        ref fields,
        ..
    } = *expr
    {
        if matches!(callee.as_ref(), &Expr::Member { .. }) {
            return codegen_sum_variant_constructor(codegen_context, env, fields.as_slice());
        }
        return codegen_product_constructor(codegen_context, env, fields.as_slice());
    }
    Err(CodegenError::new(String::from(
        "expected constructor expression",
    )))
}

#[doc = "Lower field access for product ADT values using tracked field indices."]
pub fn codegen_field_access_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let (receiver_name, member_name) = member_parts(expr)?;
    let mut effective_receiver_name = receiver_name.clone();
    let effective_member_name = member_name.as_str();
    if let Some((root, field)) = receiver_name.split_once('.') {
        if let Some(alias_map) = env.variable_field_aliases.get(root) {
            if let Some(target_binding) = alias_map.get(field) {
                effective_receiver_name.clone_from(target_binding);
            }
        }
    }

    let Some(binding) = env.variables.get(effective_receiver_name.as_str()).cloned() else {
        return Err(CodegenError::new(format!(
            "unknown field-access receiver '{receiver_name}'"
        )));
    };

    if let Some(lowered) = codegen_intrinsic_member_access(
        codegen_context,
        env,
        effective_receiver_name.as_str(),
        &binding,
        effective_member_name,
    )? {
        return Ok(lowered);
    }

    let Some(field_indices) = env
        .variable_field_indices
        .get(effective_receiver_name.as_str())
    else {
        return Err(CodegenError::new(format!(
            "receiver '{receiver_name}' does not have tracked product fields"
        )));
    };
    let Some(index) = field_indices.get(effective_member_name) else {
        return Err(CodegenError::new(format!(
            "unknown field '{member_name}' on receiver '{receiver_name}'"
        )));
    };

    // SAFETY: Index comes from tracked constructor field layout for same receiver alloca.
    let field_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            binding.alloca,
            &[
                codegen_context.context.i32_type().const_zero(),
                codegen_context
                    .context
                    .i32_type()
                    .const_int(u64::from(*index), false),
            ],
            &env.next_name("field.gep"),
        )?
    };
    codegen_context
        .builder
        .build_load(field_ptr, &env.next_name("field.load"))
        .map_err(CodegenError::from)
}

#[doc = "Lower intrinsic member access forms that are backed by runtime functions."]
fn codegen_intrinsic_member_access<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver_name: &str,
    binding: &VariableBinding<'context>,
    member_name: &str,
) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
    if let &CoreType::Generic { ref name, ref type_args } = &binding.core_type {
        if name == "Bytes" && type_args.is_empty() && member_name == "length" {
            let runtime_function = crate::codegen::functions_stdlib::declare_stdlib_function(
                codegen_context,
                "bytes_length",
            )
            .ok_or_else(|| CodegenError::new(String::from("bytes_length declaration missing")))?;
            let receiver_value = codegen_context
                .builder
                .build_load(binding.alloca, &env.next_name("bytes.length.receiver"))?;
            let call_site = codegen_context.builder.build_call(
                runtime_function,
                &[receiver_value.into()],
                &env.next_name("bytes.length.call"),
            )?;
            let Some(length_value) = call_site.try_as_basic_value().basic() else {
                return Err(CodegenError::new(String::from(
                    "bytes_length should return an int32 value",
                )));
            };
            return Ok(Some(length_value));
        }
    }

    if binding.core_type == CoreType::String && member_name == "length" {
        let runtime_function = crate::codegen::functions_stdlib::declare_stdlib_function(
            codegen_context,
            "string_length",
        )
        .ok_or_else(|| CodegenError::new(String::from("string_length declaration missing")))?;
        let receiver_value = codegen_context
            .builder
            .build_load(binding.alloca, &env.next_name("string.length.receiver"))?;
        let call_site = codegen_context.builder.build_call(
            runtime_function,
            &[receiver_value.into()],
            &env.next_name("string.length.call"),
        )?;
        let Some(length_value) = call_site.try_as_basic_value().basic() else {
            return Err(CodegenError::new(String::from(
                "string_length should return an int64 value",
            )));
        };
        return Ok(Some(length_value));
    }

    if matches!(binding.core_type, CoreType::Array(_)) && member_name == "length" {
        let length_value =
            resolve_array_length_value(codegen_context, env, receiver_name, binding)?;
        return Ok(Some(length_value.as_basic_value_enum()));
    }

    Ok(None)
}

#[doc = "Resolve the runtime-tracked length value for dynamic array `.length` access."]
fn resolve_array_length_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    receiver_name: &str,
    binding: &VariableBinding<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    if let Some(length) = binding.length {
        return Ok(codegen_context
            .context
            .i64_type()
            .const_int(u64::from(length), false));
    }

    let len_binding_name = format!("{receiver_name}_len");
    let Some(len_binding) = env.variables.get(len_binding_name.as_str()) else {
        return Err(CodegenError::new(String::from(
            "array length binding is missing for intrinsic .length access",
        )));
    };

    Ok(codegen_context
        .builder
        .build_load(len_binding.alloca, len_binding_name.as_str())?
        .into_int_value())
}

#[doc = "Lower match expressions to switch-based control flow."]
#[expect(
    clippy::too_many_lines,
    reason = "Single match lowering routine keeps switch and phi assembly tightly coupled"
)]
pub fn codegen_match_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let Expr::Match {
        ref scrutinee,
        ref arms,
        ..
    } = *expr
    else {
        return Err(CodegenError::new(String::from("expected match expression")));
    };
    let scrutinee_value = codegen_expression(codegen_context, env, scrutinee.as_ref(), None)?;
    let scrutinee_int = scrutinee_value.into_int_value();

    let current_fn = current_function(codegen_context)?;
    let merge_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("match.merge"));
    let mut literal_cases: Vec<(
        usize,
        inkwell::values::IntValue<'context>,
        inkwell::basic_block::BasicBlock<'context>,
    )> = Vec::new();
    let mut default_case: Option<(usize, inkwell::basic_block::BasicBlock<'context>)> = None;

    for (index, arm) in arms.iter().enumerate() {
        if let Pattern::Literal {
            value: crate::ast::LiteralValue::Integer(number),
            ..
        } = arm.pattern
        {
            let block = codegen_context
                .context
                .append_basic_block(current_fn, &env.next_name("match.literal.arm"));
            let literal = codegen_context
                .context
                .i64_type()
                .const_int(integer_literal_bits(number)?, true);
            literal_cases.push((index, literal, block));
        } else {
            let block = codegen_context
                .context
                .append_basic_block(current_fn, &env.next_name("match.default.arm"));
            default_case = Some((index, block));
        }
    }

    let fallback_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("match.fallback"));
    let default_block = default_case.map_or(fallback_block, |pair| pair.1);
    let switch_cases = literal_cases
        .iter()
        .map(|case| (case.1, case.2))
        .collect::<Vec<_>>();
    let _switch = codegen_context.builder.build_switch(
        scrutinee_int,
        default_block,
        switch_cases.as_slice(),
    )?;

    let mut incoming_values: Vec<(
        BasicValueEnum<'context>,
        inkwell::basic_block::BasicBlock<'context>,
    )> = Vec::new();
    for literal_case in &literal_cases {
        codegen_context.builder.position_at_end(literal_case.2);
        let arm_value = codegen_expression(codegen_context, env, &arms[literal_case.0].body, None)?;
        let block_end = codegen_context
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new(String::from("match arm block missing")))?;
        if block_end.get_terminator().is_none() {
            let _to_merge = codegen_context
                .builder
                .build_unconditional_branch(merge_block)?;
        }
        incoming_values.push((arm_value, block_end));
    }

    if let Some(default_pair) = default_case {
        codegen_context.builder.position_at_end(default_pair.1);
        let arm_value = codegen_expression(codegen_context, env, &arms[default_pair.0].body, None)?;
        let block_end = codegen_context
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new(String::from("match default block missing")))?;
        if block_end.get_terminator().is_none() {
            let _to_merge = codegen_context
                .builder
                .build_unconditional_branch(merge_block)?;
        }
        incoming_values.push((arm_value, block_end));
    } else {
        codegen_context.builder.position_at_end(fallback_block);
        let fallback = scrutinee_int.get_type().const_zero().as_basic_value_enum();
        let fallback_end = codegen_context
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new(String::from("match fallback block missing")))?;
        if fallback_end.get_terminator().is_none() {
            let _to_merge = codegen_context
                .builder
                .build_unconditional_branch(merge_block)?;
        }
        incoming_values.push((fallback, fallback_end));
    }

    codegen_context.builder.position_at_end(merge_block);
    let first_value = incoming_values
        .first()
        .map(|pair| pair.0)
        .ok_or_else(|| CodegenError::new(String::from("match expression has no arms")))?;
    let phi = codegen_context
        .builder
        .build_phi(first_value.get_type(), &env.next_name("match.phi"))?;
    let mut phi_incoming: Vec<(
        &dyn BasicValue<'context>,
        inkwell::basic_block::BasicBlock<'context>,
    )> = Vec::new();
    for pair in &incoming_values {
        let incoming_value: &dyn BasicValue<'context> = &pair.0;
        phi_incoming.push((incoming_value, pair.1));
    }
    phi.add_incoming(phi_incoming.as_slice());
    Ok(phi.as_basic_value())
}

#[doc = "Capture field-name to index mapping from product constructors."]
pub fn product_field_indices_from_constructor(constructor: &Expr) -> Option<BTreeMap<String, u32>> {
    if let Expr::Constructor { ref fields, .. } = *constructor {
        let mut indices = BTreeMap::new();
        for (index, field) in fields.iter().enumerate() {
            let converted_index = u32::try_from(index).ok()?;
            indices.insert(field.name.clone(), converted_index);
        }
        return Some(indices);
    }
    None
}

#[doc = "Resolve current enclosing function from insertion point."]
fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<inkwell::values::FunctionValue<'context>, CodegenError> {
    let Some(block) = codegen_context.builder.get_insert_block() else {
        return Err(CodegenError::new(String::from(
            "builder is not positioned in a block",
        )));
    };
    block.get_parent().ok_or_else(|| {
        CodegenError::new(String::from("insert block does not have a parent function"))
    })
}

#[doc = "Extract receiver/member string slices from member access expression."]
fn member_parts(expr: &Expr) -> Result<(String, String), CodegenError> {
    if let Expr::Member {
        ref object,
        ref member,
        ..
    } = *expr
    {
        if let Expr::Identifier { ref name, .. } = *object.as_ref() {
            return Ok((name.clone(), member.clone()));
        }
        if let Expr::Member {
            ref object,
            member: ref inner_member,
            ..
        } = *object.as_ref()
        {
            if let Expr::Identifier { ref name, .. } = **object {
                return Ok((format!("{name}.{inner_member}"), member.clone()));
            }
        }
        return Err(CodegenError::new(String::from(
            "field access requires identifier receiver in task 25",
        )));
    }
    Err(CodegenError::new(String::from(
        "expected member expression",
    )))
}

#[doc = "Lower sum variant constructors into tagged-union struct values."]
fn codegen_sum_variant_constructor<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    fields: &[crate::ast::ConstructorField],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let tagged_type = codegen_context.context.struct_type(
        &[
            codegen_context.context.i64_type().into(),
            codegen_context.context.i8_type().array_type(64).into(),
        ],
        false,
    );
    let alloca = codegen_context
        .builder
        .build_alloca(tagged_type, &env.next_name("sum.alloca"))?;

    // SAFETY: GEP targets the tag field on the same stack-allocated tagged union value.
    let tag_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            alloca,
            &[
                codegen_context.context.i32_type().const_zero(),
                codegen_context.context.i32_type().const_zero(),
            ],
            &env.next_name("sum.tag.ptr"),
        )?
    };
    let _store_tag = codegen_context.builder.build_store(
        tag_ptr,
        codegen_context.context.i64_type().const_int(0, false),
    )?;

    if let Some(first_field) = fields.first() {
        let _payload_value = codegen_expression(codegen_context, env, &first_field.value, None)?;
    }

    codegen_context
        .builder
        .build_load(alloca, &env.next_name("sum.value"))
        .map_err(CodegenError::from)
}

#[doc = "Lower product constructors to plain LLVM struct values."]
fn codegen_product_constructor<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    fields: &[crate::ast::ConstructorField],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let mut lowered_fields = Vec::new();
    for field in fields {
        lowered_fields.push(codegen_expression(
            codegen_context,
            env,
            &field.value,
            None,
        )?);
    }

    let field_types = lowered_fields
        .iter()
        .map(BasicValueEnum::get_type)
        .collect::<Vec<_>>();
    let struct_type = codegen_context
        .context
        .struct_type(field_types.as_slice(), false);
    let alloca = codegen_context
        .builder
        .build_alloca(struct_type, &env.next_name("product.alloca"))?;

    for (index, value) in lowered_fields.iter().enumerate() {
        let converted_index = u64::try_from(index)
            .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?;
        // SAFETY: Field index comes from bounded iteration over the constructor field vector.
        let field_ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                alloca,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context
                        .context
                        .i32_type()
                        .const_int(converted_index, false),
                ],
                &env.next_name("product.field.ptr"),
            )?
        };
        let _store = codegen_context.builder.build_store(field_ptr, *value)?;
    }

    codegen_context
        .builder
        .build_load(alloca, &env.next_name("product.value"))
        .map_err(CodegenError::from)
}

#[doc = "Render type argument to specialization suffix fragment."]
fn render_type_arg(core_type: &CoreType) -> String {
    match *core_type {
        CoreType::Int8 => String::from("int8"),
        CoreType::Int16 => String::from("int16"),
        CoreType::Int32 => String::from("int32"),
        CoreType::Int64 => String::from("int64"),
        CoreType::UInt8 => String::from("uint8"),
        CoreType::UInt16 => String::from("uint16"),
        CoreType::UInt32 => String::from("uint32"),
        CoreType::UInt64 => String::from("uint64"),
        CoreType::Float32 => String::from("float32"),
        CoreType::Float64 => String::from("float64"),
        CoreType::String => String::from("string"),
        CoreType::Boolean => String::from("boolean"),
        CoreType::Unit => String::from("unit"),
        CoreType::Array(ref element) => format!("array_{}", render_type_arg(element.as_ref())),
        CoreType::Variable(ref variable) => format!("var_{}", variable.name),
        CoreType::Function { .. } => String::from("function"),
        CoreType::Generic {
            ref name,
            ref type_args,
        } => {
            if type_args.is_empty() {
                return name.clone();
            }
            let mut rendered = name.clone();
            for type_arg in type_args {
                rendered.push('_');
                rendered.push_str(&render_type_arg(type_arg));
            }
            rendered
        }
    }
}
