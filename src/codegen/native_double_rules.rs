//! Precompiled typed rule layouts and pure argument snapshot/matching callbacks.
use super::native_callback_boundary::c_function;
use super::{context::CodegenContext, error::CodegenError, expressions::CodegenEnv};
use crate::ast::BorrowKind;
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

/// Exact layouts derive from the checked ordinary signature, not runtime type guesses.
#[derive(Clone, Copy)]
pub struct RuleLayout<'context> {
    /// By-value expected argument snapshots followed by an optional application result.
    pub payload: StructType<'context>,
    /// Original physical argument representations, including reference addresses.
    pub physical: StructType<'context>,
    /// Copied logical argument values; never retained borrowed addresses.
    pub logical: StructType<'context>,
    /// Unchanged ordinary application return/error ABI, absent for plain void.
    pub application: Option<BasicTypeEnum<'context>>,
    /// Explicit target-native replacement function ABI.
    pub replacement: inkwell::types::PointerType<'context>,
}

impl RuleLayout<'_> {
    /// Replacement pointers occupy the final explicitly generated payload field.
    pub fn replacement_index(self) -> Result<u32, CodegenError> {
        self.payload
            .count_fields()
            .checked_sub(1)
            .ok_or_else(|| CodegenError::new("replacement payload has no function pointer field"))
    }
}

/// Compiler-only pure C callback headers.
#[derive(Clone, Copy)]
pub struct RuleCallbacks<'context> {
    /// Logical expected arguments versus original physical call arguments.
    pub matches: FunctionValue<'context>,
    /// Logical argument equality for duplicate rules and history assertions.
    pub overlap: FunctionValue<'context>,
    /// Release framework-owned scalar payload memory.
    pub release: FunctionValue<'context>,
    /// Snapshot logical arguments before the original operation can mutate them.
    pub snapshot: FunctionValue<'context>,
}

/// Explicit initial snapshot subset; unsupported signatures remain count-only spies.
pub const fn supported(core: &CoreType) -> bool {
    matches!(
        *core,
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
            | CoreType::Float64
    )
}

/// Reproduce native declaration parameter and application return representations.
pub fn layout<'context>(
    ctx: &CodegenContext<'context>,
    signature: &NativeDoubleSignature,
) -> Result<RuleLayout<'context>, CodegenError> {
    if signature.parameters.len() != signature.parameter_modes.len() {
        return Err(CodegenError::new(
            "double signature parameter modes do not match the checked parameter count",
        ));
    }
    let logical = signature
        .parameters
        .iter()
        .map(|core| super::types::core_type_to_llvm(ctx.context, core))
        .collect::<Vec<_>>();
    let physical = logical
        .iter()
        .zip(&signature.parameter_modes)
        .map(|(&value, &mode)| {
            if mode == BorrowKind::Owned {
                value
            } else {
                value.ptr_type(AddressSpace::default()).into()
            }
        })
        .collect::<Vec<_>>();
    let parameters = physical.iter().copied().map(Into::into).collect::<Vec<_>>();
    let application = super::functions_call::build_function_type(
        ctx,
        &parameters,
        &signature.return_types,
        &signature.error_types,
    )?
    .get_return_type();
    let replacement = super::native_outcome::NativeOutcomeType::new(ctx.context, application)
        .llvm_type()
        .fn_type(&parameters, false)
        .ptr_type(AddressSpace::default());
    let mut payload = logical.clone();
    payload.extend(application);
    payload.push(replacement.into());
    Ok(RuleLayout {
        payload: ctx.context.struct_type(&payload, false),
        physical: ctx.context.struct_type(&physical, false),
        logical: ctx.context.struct_type(&logical, false),
        application,
        replacement,
    })
}

/// Generate ordinary C helpers once, preserving the caller's builder position.
pub fn callbacks<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    key: &str,
    signature: &NativeDoubleSignature,
) -> Result<RuleCallbacks<'context>, CodegenError> {
    if !signature.parameters.iter().all(supported) {
        return Err(CodegenError::new(
            "typed double argument snapshots require supported concrete value types; count-only spying remains available",
        ));
    }
    let previous = ctx.builder.get_insert_block();
    let result = emit_callbacks(ctx, env, key, signature);
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result
}

/// Each callback is generated from fixed type metadata, never interpreted source syntax.
fn emit_callbacks<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    key: &str,
    signature: &NativeDoubleSignature,
) -> Result<RuleCallbacks<'context>, CodegenError> {
    let name = super::native_project::symbol("double_rule", &key)?;
    let plan = layout(ctx, signature)?;
    let matches = matching(ctx, env, &format!("{name}.match"), signature, plan, true)?;
    let overlap = matching(ctx, env, &format!("{name}.overlap"), signature, plan, false)?;
    let release = free_callback(ctx, &format!("{name}.drop"))?;
    let snapshot = copy_arguments(ctx, &format!("{name}.snapshot"), signature, plan)?;
    Ok(RuleCallbacks {
        matches,
        overlap,
        release,
        snapshot,
    })
}

/// Release only framework allocation memory, preserving the current source builder.
pub fn free_callback<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
) -> Result<FunctionValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let release = c_function(
        ctx,
        name,
        ctx.context.void_type().fn_type(&[pointer.into()], false),
    )?;
    if release.count_basic_blocks() != 0 {
        return Ok(release);
    }
    let previous = ctx.builder.get_insert_block();
    release.set_linkage(inkwell::module::Linkage::Private);
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(release, "entry"));
    let result = (|| {
        let free = c_function(
            ctx,
            "free",
            ctx.context.void_type().fn_type(&[pointer.into()], false),
        )?;
        let argument = release
            .get_first_param()
            .ok_or_else(|| CodegenError::new("value release omitted its owned pointer"))?;
        ctx.builder.build_call(free, &[argument.into()], "")?;
        ctx.builder.build_return(None)?;
        Ok(())
    })();
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result.map(|()| release)
}

/// Read one checked argument position, dereferencing a borrow only during the call.
pub fn argument<'context>(
    ctx: &CodegenContext<'context>,
    structure: StructType<'context>,
    data: PointerValue<'context>,
    index: usize,
    borrowed: bool,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let data = ctx.builder.build_pointer_cast(
        data,
        structure.ptr_type(AddressSpace::default()),
        "double.arguments.typed",
    )?;
    let slot = ctx.builder.build_struct_gep(
        data,
        u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
        "double.argument.slot",
    )?;
    let value = ctx.builder.build_load(slot, "double.argument")?;
    if borrowed {
        Ok(ctx
            .builder
            .build_load(value.into_pointer_value(), "double.borrowed.value")?)
    } else {
        Ok(value)
    }
}

/// C matcher callbacks are pure typed equality and cannot propagate application errors.
fn matching<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    name: &str,
    signature: &NativeDoubleSignature,
    plan: RuleLayout<'context>,
    physical: bool,
) -> Result<FunctionValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let function = c_function(
        ctx,
        name,
        ctx.context
            .i32_type()
            .fn_type(&[pointer.into(), pointer.into()], false),
    )?;
    if function.count_basic_blocks() != 0 {
        return Ok(function);
    }
    function.set_linkage(inkwell::module::Linkage::Private);
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(function, "entry"));
    let expected = function
        .get_nth_param(0)
        .ok_or_else(|| CodegenError::new("rule matcher omitted expected arguments"))?
        .into_pointer_value();
    let actual = function
        .get_nth_param(1)
        .ok_or_else(|| CodegenError::new("rule matcher omitted actual arguments"))?
        .into_pointer_value();
    let mut matches = ctx.context.bool_type().const_int(1, false);
    for (index, core) in signature.parameters.iter().enumerate() {
        let left = argument(ctx, plan.logical, expected, index, false)?;
        let right = argument(
            ctx,
            if physical {
                plan.physical
            } else {
                plan.logical
            },
            actual,
            index,
            physical && signature.parameter_modes[index] != BorrowKind::Owned,
        )?;
        let equal = super::native_assertions::typed_equal(ctx, env, core, left, right)?;
        matches = ctx
            .builder
            .build_and(matches, equal, "double.argument.equal")?;
    }
    let result =
        ctx.builder
            .build_int_z_extend(matches, ctx.context.i32_type(), "double.matches")?;
    ctx.builder.build_return(Some(&result))?;
    Ok(function)
}

/// Owned scalar snapshots use fixed storage and do not save any borrowed address.
fn copy_arguments<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
    signature: &NativeDoubleSignature,
    plan: RuleLayout<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let function = c_function(ctx, name, pointer.fn_type(&[pointer.into()], false))?;
    if function.count_basic_blocks() != 0 {
        return Ok(function);
    }
    function.set_linkage(inkwell::module::Linkage::Private);
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(function, "entry"));
    let input = function
        .get_first_param()
        .ok_or_else(|| CodegenError::new("snapshot omitted incoming argument storage"))?
        .into_pointer_value();
    let size = plan
        .logical
        .size_of()
        .ok_or_else(|| CodegenError::new("snapshot has an unsized argument representation"))?;
    let empty = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        size,
        size.get_type().const_zero(),
        "double.arguments.empty",
    )?;
    let size = ctx.builder.build_select(
        empty,
        size.get_type().const_int(1, false),
        size,
        "double.snapshot.size",
    )?;
    let malloc = c_function(
        ctx,
        "malloc",
        pointer.fn_type(&[ctx.context.i64_type().into()], false),
    )?;
    let storage = ctx
        .builder
        .build_call(malloc, &[size.into()], "double.snapshot.storage")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snapshot allocation omitted its result"))?
        .into_pointer_value();
    let failed = ctx
        .context
        .append_basic_block(function, "allocation.failed");
    let ready = ctx.context.append_basic_block(function, "allocation.ready");
    let missing = ctx.builder.build_is_null(storage, "snapshot.missing")?;
    ctx.builder
        .build_conditional_branch(missing, failed, ready)?;
    ctx.builder.position_at_end(failed);
    ctx.builder.build_return(Some(&pointer.const_null()))?;
    ctx.builder.position_at_end(ready);
    let typed = ctx.builder.build_pointer_cast(
        storage,
        plan.logical.ptr_type(AddressSpace::default()),
        "double.snapshot.typed",
    )?;
    for (index, mode) in signature.parameter_modes.iter().enumerate() {
        let value = argument(ctx, plan.physical, input, index, *mode != BorrowKind::Owned)?;
        let slot = ctx.builder.build_struct_gep(
            typed,
            u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
            "double.snapshot.slot",
        )?;
        ctx.builder.build_store(slot, value)?;
    }
    ctx.builder.build_return(Some(&storage))?;
    Ok(function)
}
