//! Bounded owned diagnostics rendered from checked types, never used for equality.
use super::{CodegenContext, CodegenEnv, CodegenError, value_equality};
use crate::ast::testing::{Expectation, Matcher};
use crate::codegen::native_callback_boundary::c_function;
use crate::type_system::types::CoreType;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

/// Render both retained operands only after the last assertion mismatch.
pub(super) fn failure<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
    core: &CoreType,
    actual: BasicValueEnum<'context>,
    expected: Option<BasicValueEnum<'context>>,
) -> Result<IntValue<'context>, CodegenError> {
    crate::codegen::native_polling::retry_mismatch(ctx, env)?;
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let new = c_function(ctx, "opal_test_snapshot_new", pointer.fn_type(&[], false))?;
    let left = ctx
        .builder
        .build_call(new, &[], "snapshot.actual")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snapshot omitted its storage"))?
        .into_pointer_value();
    let right = ctx
        .builder
        .build_call(new, &[], "snapshot.expected")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snapshot omitted its storage"))?
        .into_pointer_value();
    render(ctx, env, left, core, actual)?;
    if let Some(expected) = expected {
        render(ctx, env, right, core, expected)?;
    } else {
        text(ctx, right, &constraint_description(&expectation.matcher))?;
    }
    let matcher = match expectation.matcher {
        Matcher::Equal { negated: true, .. } => "inequality",
        Matcher::Equal { .. } => "equality",
        Matcher::Compare { .. } => "numeric comparison",
        Matcher::Sequence { .. } => "exact order",
        Matcher::Variant { .. } => "nominal variant",
        Matcher::CallCount { .. } => "call count",
        Matcher::CalledWith { .. } => "call history",
        Matcher::Empty { .. } => "emptiness",
    };
    let matcher = ctx
        .builder
        .build_global_string_ptr(matcher, "snapshot.matcher")?
        .as_pointer_value();
    let name = ctx
        .builder
        .build_global_string_ptr(
            &display_type(core).chars().take(64).collect::<String>(),
            "snapshot.type",
        )?
        .as_pointer_value();
    let file = ctx
        .builder
        .build_global_string_ptr(&env.current_source_path, "snapshot.file")?
        .as_pointer_value();
    let span = expectation.span;
    let helper = c_function(
        ctx,
        "opal_test_snapshot_failure",
        ctx.context.i64_type().fn_type(
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                pointer.into(),
                pointer.into(),
                ctx.context.i64_type().into(),
                ctx.context.i64_type().into(),
            ],
            false,
        ),
    )?;
    Ok(ctx
        .builder
        .build_call(
            helper,
            &[
                left.into(),
                right.into(),
                matcher.into(),
                name.into(),
                file.into(),
                ctx.context
                    .i64_type()
                    .const_int(
                        u64::try_from(span.start.offset)
                            .map_err(|e| CodegenError::new(e.to_string()))?,
                        false,
                    )
                    .into(),
                ctx.context
                    .i64_type()
                    .const_int(
                        u64::try_from(span.end.offset.saturating_sub(span.start.offset))
                            .map_err(|e| CodegenError::new(e.to_string()))?,
                        false,
                    )
                    .into(),
            ],
            "snapshot.failure",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snapshot failure omitted its identity"))?
        .into_int_value())
}

/// Human-readable type labels omit encoded module paths, not nominal identity in checking.
fn display_type(core: &CoreType) -> String {
    match *core {
        CoreType::Generic { ref name, .. } => name.rsplit("::").next().unwrap_or(name).to_owned(),
        CoreType::Array(ref element) => format!("{}[]", display_type(element)),
        _ => format!("{core}"),
    }
}

/// Non-value constraints retain the genuine expected identity or emptiness clause.
fn constraint_description(matcher: &Matcher) -> String {
    match *matcher {
        Matcher::Variant {
            ref nominal_type,
            ref variant,
        } => format!("{}.{}", nominal_type.path.join("."), variant),
        Matcher::Empty { negated: true } => "not empty".to_owned(),
        Matcher::Empty { negated: false } => "empty".to_owned(),
        _ => "<none>".to_owned(),
    }
}

/// Append immutable compiler-owned punctuation or field names.
fn text<'context>(
    ctx: &CodegenContext<'context>,
    snapshot: PointerValue<'context>,
    value: &str,
) -> Result<(), CodegenError> {
    let value = ctx
        .builder
        .build_global_string_ptr(value, "snapshot.text")?
        .as_pointer_value();
    append(ctx, snapshot, value)
}
/// Copy a live C string into bounded runtime-owned diagnostic storage.
fn append<'context>(
    ctx: &CodegenContext<'context>,
    snapshot: PointerValue<'context>,
    value: PointerValue<'context>,
) -> Result<(), CodegenError> {
    let helper = c_function(
        ctx,
        "opal_test_snapshot_text",
        ctx.context.void_type().fn_type(
            &[snapshot.get_type().into(), value.get_type().into()],
            false,
        ),
    )?;
    ctx.builder
        .build_call(helper, &[snapshot.into(), value.into()], "")?;
    Ok(())
}
/// Type-directed rendering never participates in comparison semantics.
fn render<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    snapshot: PointerValue<'context>,
    core: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    match *core {
        CoreType::String => {
            text(ctx, snapshot, "\"")?;
            append(ctx, snapshot, value.into_pointer_value())?;
            text(ctx, snapshot, "\"")
        }
        CoreType::Array(ref element) => {
            array(ctx, env, snapshot, element, value.into_pointer_value())
        }
        CoreType::Generic { .. } if crate::codegen::native_path_values::is_path(core) => {
            text(ctx, snapshot, "FilesystemPath(\"")?;
            append(ctx, snapshot, value.into_pointer_value())?;
            text(ctx, snapshot, "\")")
        }
        CoreType::Generic { ref name, .. } => nominal(ctx, env, snapshot, name, value),
        CoreType::Boolean
        | CoreType::Int8
        | CoreType::Int16
        | CoreType::Int32
        | CoreType::Int64
        | CoreType::UInt8
        | CoreType::UInt16
        | CoreType::UInt32
        | CoreType::UInt64
        | CoreType::Float32
        | CoreType::Float64 => scalar(ctx, snapshot, core, value),
        // Empty arrays may retain an unconstrained element representation;
        // their unreachable element renderer must not invent a scalar layout.
        _ => text(ctx, snapshot, "<no displayable value>"),
    }
}
/// Preserve signedness, boolean spelling and floating-point value bits.
fn scalar<'context>(
    ctx: &CodegenContext<'context>,
    snapshot: PointerValue<'context>,
    core: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    let unsigned = matches!(
        *core,
        CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
            | CoreType::Boolean
    );
    let (bits, kind) = if value.is_float_value() {
        let value = value.into_float_value();
        let value = if value.get_type() == ctx.context.f64_type() {
            value
        } else {
            ctx.builder
                .build_float_ext(value, ctx.context.f64_type(), "snapshot.double")?
        };
        (
            ctx.builder
                .build_bit_cast(value, ctx.context.i64_type(), "snapshot.float.bits")?
                .into_int_value(),
            3_i32,
        )
    } else {
        let value = value.into_int_value();
        let bits = if value.get_type() == ctx.context.i64_type() {
            value
        } else if unsigned {
            ctx.builder
                .build_int_z_extend(value, ctx.context.i64_type(), "snapshot.unsigned")?
        } else {
            ctx.builder
                .build_int_s_extend(value, ctx.context.i64_type(), "snapshot.signed")?
        };
        (
            bits,
            if *core == CoreType::Boolean {
                2_i32
            } else {
                i32::from(unsigned)
            },
        )
    };
    let helper = c_function(
        ctx,
        "opal_test_snapshot_scalar",
        ctx.context.void_type().fn_type(
            &[
                snapshot.get_type().into(),
                ctx.context.i64_type().into(),
                ctx.context.i32_type().into(),
            ],
            false,
        ),
    )?;
    ctx.builder.build_call(
        helper,
        &[
            snapshot.into(),
            bits.into(),
            ctx.context
                .i32_type()
                .const_int(
                    u64::try_from(kind).map_err(|e| CodegenError::new(e.to_string()))?,
                    false,
                )
                .into(),
        ],
        "",
    )?;
    Ok(())
}
/// Products use declared field positions; variants expose identity without payload reads.
fn nominal<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    snapshot: PointerValue<'context>,
    name: &str,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    let owner = env.adt_layout_key(name).to_owned();
    let variants = env
        .adt_variant_discriminants
        .iter()
        .filter(|&(key, _)| key.starts_with(&format!("{owner}.")))
        .map(|(key, &tag)| (key.clone(), tag))
        .collect::<Vec<_>>();
    if !variants.is_empty() {
        let tag = value_equality::variant_tag(ctx, value)?;
        let mut label = ctx
            .builder
            .build_global_string_ptr("<unknown variant>", "snapshot.variant.unknown")?
            .as_pointer_value();
        for (variant, id) in variants {
            let matches = ctx.builder.build_int_compare(
                IntPredicate::EQ,
                tag,
                tag.get_type()
                    .const_int(crate::codegen::types::integer_literal_bits(id)?, true),
                "snapshot.variant.is",
            )?;
            let variant_name = variant.rsplit("::").next().unwrap_or(&variant);
            let candidate = ctx
                .builder
                .build_global_string_ptr(variant_name, "snapshot.variant.name")?
                .as_pointer_value();
            label = ctx
                .builder
                .build_select(matches, candidate, label, "snapshot.variant")?
                .into_pointer_value();
        }
        return append(ctx, snapshot, label);
    }
    let Some(fields) = env.adt_field_layout(&owner).cloned() else {
        // Emptiness may involve opaque/resource elements. Never discover
        // resources or dereference an unknown foreign layout for diagnostics.
        return text(ctx, snapshot, "<opaque value>");
    };
    text(
        ctx,
        snapshot,
        &format!("{}{{", owner.rsplit("::").next().unwrap_or(&owner)),
    )?;
    let structure = ctx.context.struct_type(
        &fields
            .iter()
            .map(|&(_, ref core)| crate::codegen::types::core_type_to_llvm(ctx.context, core))
            .collect::<Vec<_>>(),
        false,
    );
    for (index, &(ref field, ref core)) in fields.iter().enumerate() {
        if index != 0 {
            text(ctx, snapshot, ", ")?;
        }
        text(ctx, snapshot, &format!("{field}: "))?;
        let value = value_equality::product_field(
            ctx,
            value,
            structure,
            u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
        )?;
        render(ctx, env, snapshot, core, value)?;
    }
    text(ctx, snapshot, "}")
}
/// Bounds-check each array read and stop traversing once the shared buffer fills.
fn array<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    snapshot: PointerValue<'context>,
    element: &CoreType,
    value: PointerValue<'context>,
) -> Result<(), CodegenError> {
    use crate::codegen::expressions_array::{
        load_array_data_ptr_for_element_type, load_array_length_from_value,
    };
    text(ctx, snapshot, "[")?;
    let length = load_array_length_from_value(ctx, env, value, "snapshot.array")?;
    let data = load_array_data_ptr_for_element_type(ctx, env, value, element, "snapshot.array")?;
    let entry = ctx
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new("snapshot array has no entry"))?;
    let function = entry
        .get_parent()
        .ok_or_else(|| CodegenError::new("snapshot array has no function"))?;
    let head = ctx
        .context
        .append_basic_block(function, "snapshot.array.head");
    let body = ctx
        .context
        .append_basic_block(function, "snapshot.array.body");
    let finished = ctx
        .context
        .append_basic_block(function, "snapshot.array.end");
    ctx.builder.build_unconditional_branch(head)?;
    ctx.builder.position_at_end(head);
    let index = ctx
        .builder
        .build_phi(length.get_type(), "snapshot.array.index")?;
    index.add_incoming(&[(&length.get_type().const_zero(), entry)]);
    let current = index.as_basic_value().into_int_value();
    let in_range = ctx.builder.build_int_compare(
        IntPredicate::ULT,
        current,
        length,
        "snapshot.array.in_range",
    )?;
    let helper = c_function(
        ctx,
        "opal_test_snapshot_available",
        ctx.context
            .i32_type()
            .fn_type(&[snapshot.get_type().into()], false),
    )?;
    let available = ctx
        .builder
        .build_call(helper, &[snapshot.into()], "snapshot.available")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snapshot omitted its bound"))?
        .into_int_value();
    let available = ctx.builder.build_int_compare(
        IntPredicate::NE,
        available,
        available.get_type().const_zero(),
        "snapshot.has_room",
    )?;
    let iterate = ctx
        .builder
        .build_and(in_range, available, "snapshot.iterate")?;
    ctx.builder
        .build_conditional_branch(iterate, body, finished)?;
    ctx.builder.position_at_end(body);
    // SAFETY: The loop condition proves the typed array element index is in range.
    let slot = unsafe {
        ctx.builder
            .build_in_bounds_gep(data, &[current], "snapshot.element")?
    };
    let item = ctx.builder.build_load(slot, "snapshot.value")?;
    render(ctx, env, snapshot, element, item)?;
    text(ctx, snapshot, ", ")?;
    let tail = ctx
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new("snapshot element has no continuation"))?;
    let next = ctx.builder.build_int_add(
        current,
        length.get_type().const_int(1, false),
        "snapshot.next",
    )?;
    index.add_incoming(&[(&next, tail)]);
    ctx.builder.build_unconditional_branch(head)?;
    ctx.builder.position_at_end(finished);
    text(ctx, snapshot, "]")
}
