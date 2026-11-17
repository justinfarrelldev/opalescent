//! Address projections for existing typed product-reference arguments.
use super::{adts_sum, infer_product_core_type};
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::type_system::types::CoreType;
use inkwell::values::PointerValue;

/// Project the actual field address, never a copied local value.
///
/// # Errors
/// Rejects unknown nominal layouts and escaping addresses of value temporaries.
pub fn field_address<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
) -> Result<PointerValue<'context>, CodegenError> {
    let Expr::Member {
        ref object,
        ref member,
        ..
    } = *expression
    else {
        return Err(CodegenError::new(
            "native borrow requires a product-field projection",
        ));
    };
    let core = infer_product_core_type(env, object)
        .ok_or_else(|| CodegenError::new("native borrow receiver has no typed product layout"))?;
    let index = product_field_index_for_core_type(env, &core, member)
        .ok_or_else(|| CodegenError::new("native borrow field is not declared by its receiver"))?;
    let value = codegen_expression(codegen, env, object, Some(&core))?;
    let pointer = if value.is_pointer_value() {
        let CoreType::Generic { ref name, .. } = core else {
            return Err(CodegenError::new(
                "native pointer field borrow requires a nominal product",
            ));
        };
        let layout = env
            .adt_field_layout(name)
            .ok_or_else(|| CodegenError::new("native borrow product field metadata is missing"))?;
        let fields: Vec<_> = layout
            .iter()
            .map(|&(_, ref field)| crate::codegen::types::core_type_to_llvm(codegen.context, field))
            .collect();
        let structure = codegen.context.struct_type(&fields, false);
        adts_sum::pointer_for_pointer_backed_field_access(
            codegen,
            env,
            name,
            value.into_pointer_value(),
            structure,
        )?
    } else if let Expr::Identifier { ref name, .. } = **object {
        env.variables
            .get(name)
            .ok_or_else(|| CodegenError::new("native value product has no original storage"))?
            .alloca
    } else {
        return Err(CodegenError::new(
            "native borrow cannot retain the address of a temporary product value",
        ));
    };
    // SAFETY: The checked nominal field layout and declared field index establish the
    // in-bounds projection, exactly as ordinary product read/write lowering.
    Ok(unsafe {
        codegen.builder.build_in_bounds_gep(
            pointer,
            &[
                codegen.context.i32_type().const_zero(),
                codegen
                    .context
                    .i32_type()
                    .const_int(u64::from(index), false),
            ],
            "native.borrow.field",
        )?
    })
}

/// Resolve a declared field index, including the existing inline Pair layout.
pub(super) fn product_field_index_for_core_type(
    env: &CodegenEnv<'_>,
    core: &CoreType,
    member: &str,
) -> Option<u32> {
    match *core {
        CoreType::Generic {
            ref name,
            ref type_args,
        } if name == "Pair" && type_args.len() == 2 => match member {
            "first" => Some(0),
            "second" => Some(1),
            _ => None,
        },
        CoreType::Generic { ref name, .. } => env
            .adt_field_indices_for(name)
            .and_then(|indices| indices.get(member).copied()),
        _ => None,
    }
}
