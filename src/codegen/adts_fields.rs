//! Field evaluation and ownership at nominal ADT construction boundaries.

extern crate alloc;

use crate::ast::{ConstructorField, Expr};
use crate::codegen::binding_store::initialize_binding_value;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::statements::unwind_scope_without_cleanup;
use crate::codegen::string_ownership::{
    retain_string_for_expr_boundary, string_expr_needs_retain_at_ownership_boundary,
};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use inkwell::values::BasicValueEnum;

/// Evaluate fields in layout order. Hold array references in a provisional
/// scope until all initializers succeed, then transfer them to the product.
/// A return or native-abort edge in a later initializer cleans up that scope.
pub(super) fn lower_product_fields<'context>(
    context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    name: &str,
    fields: &[ConstructorField],
    layout: &[(String, CoreType)],
) -> Result<Vec<BasicValueEnum<'context>>, CodegenError> {
    let field_map = fields
        .iter()
        .map(|field| (field.name.as_str(), &field.value))
        .collect::<BTreeMap<_, _>>();
    let mut values = Vec::with_capacity(layout.len());
    env.scope_stack.push(Vec::new());
    for field in layout {
        let (field_name, field_type) = (&field.0, &field.1);
        let expr = field_map.get(field_name.as_str()).copied().ok_or_else(|| {
            CodegenError::new(format!(
                "missing field '{field_name}' for constructor '{name}'"
            ))
        })?;
        let value = codegen_expression(context, env, expr, Some(field_type))?;
        match *field_type {
            CoreType::String => retain_string_for_expr_boundary(context, env, expr, value)?,
            CoreType::Array(_) => own_array_field(context, env, field_type, expr, value)?,
            _ => {}
        }
        values.push(value);
    }
    unwind_scope_without_cleanup(env);
    Ok(values)
}

/// Retain a borrowed array once; take over an owned expression's reference.
/// Register either form for release if construction does not complete.
pub(super) fn own_array_field<'context>(
    context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    field_type: &CoreType,
    expr: &Expr,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    let name = env.next_name("adt.array.field");
    let alloca = context.builder.build_alloca(value.get_type(), &name)?;
    env.variables.insert(
        name.clone(),
        VariableBinding {
            alloca,
            core_type: field_type.clone(),
            length: None,
            capacity: None,
            is_mutable: false,
        },
    );
    env.register_scope_binding(&name);
    // The same syntactic boundary classifies borrowed arrays and strings for
    // these initializers: identifiers and projections are not fresh owners.
    initialize_binding_value(
        context,
        env,
        &name,
        value,
        "adt.array.field.init",
        string_expr_needs_retain_at_ownership_boundary(expr),
    )
}
