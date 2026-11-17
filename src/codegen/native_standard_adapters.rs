//! Native original-call bridges reuse ordinary standard-library lowering unchanged.
use super::{context::CodegenContext, error::CodegenError, expressions::CodegenEnv};
use crate::ast::{BorrowKind, Expr, LabeledValue, LiteralValue, NodeId, Parameter, Stmt, Type};
use crate::testing::native::doubles::ResolvedDouble;
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::values::FunctionValue;

/// Synthesize only a typed compiler forwarding container, never rewrite user source.
pub fn emit<'context>(
    ctx: &CodegenContext<'context>,
    parent: &CodegenEnv<'context>,
    declaration: &ResolvedDouble,
    signature: &NativeDoubleSignature,
) -> Result<FunctionValue<'context>, CodegenError> {
    let name = super::native_project::symbol("standard_original", &declaration.edge)?;
    if let Some(function) = ctx.module.get_function(&name) {
        if ctx.native_function_outcome(function).is_none() {
            return Err(CodegenError::new(
                "standard bridge conflicts with a non-native ABI",
            ));
        }
        return Ok(function);
    }
    let runtime = declaration
        .import_names
        .iter()
        .find_map(|alias| parent.imported_functions.get(alias))
        .ok_or_else(|| {
            CodegenError::new("standard double edge has no resolved original runtime mapping")
        })?
        .clone();
    let span = crate::token::Span::single(crate::token::Position::start());
    let parameters = signature
        .parameter_modes
        .iter()
        .enumerate()
        .map(|(index, &mode)| Parameter {
            name: format!("__native_standard_argument_{index}"),
            param_type: Type::Basic {
                name: "__compiler_resolved".to_owned(),
                span,
            },
            borrow_kind: mode,
            span,
        })
        .collect::<Vec<_>>();
    let core = CoreType::Function {
        generic_params: Vec::new(),
        parameters: signature.parameters.clone(),
        return_types: signature.return_types.clone(),
        error_types: signature.error_types.clone(),
    };
    let original_alias = "__native_standard_original";
    let mut env = CodegenEnv::new(parent.debug_mode);
    env.current_source_path
        .clone_from(&parent.current_source_path);
    env.current_source_text
        .clone_from(&parent.current_source_text);
    env.adt_field_layouts.clone_from(&parent.adt_field_layouts);
    env.adt_field_indices.clone_from(&parent.adt_field_indices);
    env.adt_product_layouts
        .clone_from(&parent.adt_product_layouts);
    env.adt_variant_discriminants
        .clone_from(&parent.adt_variant_discriminants);
    env.adt_layout_aliases
        .clone_from(&parent.adt_layout_aliases);
    env.imported_functions
        .insert(original_alias.to_owned(), runtime);
    env.imported_signatures
        .insert(original_alias.to_owned(), core.clone());
    let statement = forwarding_body(&parameters, signature, span, original_alias);
    let previous = ctx.builder.get_insert_block();
    let result = super::native_declarations::emit_source_function(
        ctx,
        &mut env,
        &name,
        &parameters,
        &core,
        &statement,
    );
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result
}

/// Forward through the existing typed expression/error lowering, including void.
fn forwarding_body(
    parameters: &[Parameter],
    signature: &NativeDoubleSignature,
    span: crate::token::Span,
    original_alias: &str,
) -> Stmt {
    let arguments = parameters
        .iter()
        .map(|parameter| {
            let target = Expr::Identifier {
                name: parameter.name.clone(),
                span,
                id: NodeId(0),
            };
            if parameter.borrow_kind == BorrowKind::Owned {
                target
            } else {
                Expr::BorrowArgument {
                    target: Box::new(target),
                    borrow_kind: parameter.borrow_kind,
                    span,
                    id: NodeId(0),
                }
            }
        })
        .collect();
    let call = Expr::Call {
        callee: Box::new(Expr::Identifier {
            name: original_alias.to_owned(),
            span,
            id: NodeId(0),
        }),
        generic_args: None,
        args: arguments,
        span,
        id: NodeId(0),
    };
    let call = if signature.error_types.is_empty() {
        call
    } else {
        Expr::Propagate {
            call: Box::new(call),
            cause: None,
            span,
            id: NodeId(0),
        }
    };
    let void = signature.return_types.is_empty() || signature.return_types == [CoreType::Unit];
    if void {
        Stmt::Block {
            statements: vec![
                Stmt::Expression {
                    expr: call,
                    span,
                    id: NodeId(0),
                },
                Stmt::Return {
                    values: vec![LabeledValue {
                        label: String::new(),
                        value: Expr::Literal {
                            value: LiteralValue::Void,
                            span,
                            id: NodeId(0),
                        },
                        span,
                        id: NodeId(0),
                    }],
                    span,
                    id: NodeId(0),
                },
            ],
            span,
            id: NodeId(0),
        }
    } else {
        Stmt::Return {
            values: vec![LabeledValue {
                label: String::new(),
                value: call,
                span,
                id: NodeId(0),
            }],
            span,
            id: NodeId(0),
        }
    }
}
