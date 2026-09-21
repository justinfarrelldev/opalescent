//! Branch-local codegen support for nominal sum variant refinements.

extern crate alloc;

use crate::ast::{BinaryOp, Expr};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub(super) struct VariantRefinementRequest {
    value: Expr,
    narrowed_identifier: Option<String>,
    variant_type_name: String,
    payload_binding: Option<String>,
}

pub(super) struct AppliedVariantRefinement<'context> {
    narrowed_identifier: Option<(String, VariableBinding<'context>)>,
    payload_binding: Option<(String, Option<VariableBinding<'context>>)>,
}

fn variant_type_expression_name(expr: &Expr) -> Option<String> {
    let Expr::Member { object, member, .. } = expr else {
        return None;
    };
    let Expr::Identifier { name, .. } = object.as_ref() else {
        return None;
    };
    crate::type_system::terminal_proposal_variant_id(name.as_str(), member.as_str())?;
    Some(format!("{name}.{member}"))
}

pub(super) fn variant_refinement_request(condition: &Expr) -> Option<VariantRefinementRequest> {
    match condition {
        Expr::Refinement {
            value,
            variant,
            payload_binding,
            ..
        } => {
            let variant_type_name = variant_type_expression_name(variant.as_ref())?;
            let narrowed_identifier = if let Expr::Identifier { name, .. } = value.as_ref() {
                Some(name.clone())
            } else {
                None
            };
            Some(VariantRefinementRequest {
                value: value.as_ref().clone(),
                narrowed_identifier,
                variant_type_name,
                payload_binding: Some(payload_binding.clone()),
            })
        }
        Expr::Binary {
            left,
            operator: BinaryOp::Is,
            right,
            ..
        } => {
            let variant_type_name = variant_type_expression_name(right.as_ref())?;
            let Expr::Identifier { name, .. } = left.as_ref() else {
                return None;
            };
            Some(VariantRefinementRequest {
                value: left.as_ref().clone(),
                narrowed_identifier: Some(name.clone()),
                variant_type_name,
                payload_binding: None,
            })
        }
        _ => None,
    }
}

pub(super) fn apply_variant_refinement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    request: Option<&VariantRefinementRequest>,
) -> Result<Option<AppliedVariantRefinement<'context>>, CodegenError> {
    let Some(request) = request else {
        return Ok(None);
    };
    let variant_core_type = CoreType::Generic {
        name: request.variant_type_name.clone(),
        type_args: Vec::new(),
    };

    let mut applied = AppliedVariantRefinement {
        narrowed_identifier: None,
        payload_binding: None,
    };

    if let Some(identifier) = &request.narrowed_identifier {
        if let Some(previous) = env.variables.get(identifier.as_str()).cloned() {
            let mut narrowed = previous.clone();
            narrowed.core_type = variant_core_type.clone();
            env.variables.insert(identifier.clone(), narrowed);
            applied.narrowed_identifier = Some((identifier.clone(), previous));
        }
    }

    if let Some(payload_binding) = &request.payload_binding {
        let previous_payload = env.variables.get(payload_binding.as_str()).cloned();
        let payload_alloca = if let Some(identifier) = &request.narrowed_identifier {
            env.variables
                .get(identifier.as_str())
                .map(|binding| binding.alloca)
        } else {
            None
        };
        let payload_alloca = if let Some(alloca) = payload_alloca {
            alloca
        } else {
            let llvm_type = core_type_to_llvm(codegen_context.context, &variant_core_type);
            let alloca = codegen_context
                .builder
                .build_alloca(llvm_type, &env.next_name("refinement.payload.alloca"))?;
            let value = codegen_expression(codegen_context, env, &request.value, None)?;
            let _store = codegen_context.builder.build_store(alloca, value)?;
            alloca
        };
        env.variables.insert(
            payload_binding.clone(),
            VariableBinding {
                alloca: payload_alloca,
                core_type: variant_core_type,
                length: None,
                capacity: None,
                is_mutable: false,
            },
        );
        env.register_scope_binding(payload_binding.as_str());
        applied.payload_binding = Some((payload_binding.clone(), previous_payload));
    }

    Ok(Some(applied))
}

pub(super) fn restore_variant_refinement<'context>(
    env: &mut CodegenEnv<'context>,
    applied: Option<AppliedVariantRefinement<'context>>,
) {
    let Some(applied) = applied else {
        return;
    };
    if let Some((payload_name, previous_payload)) = applied.payload_binding {
        if let Some(previous) = previous_payload {
            env.variables.insert(payload_name, previous);
        } else {
            env.variables.remove(payload_name.as_str());
        }
    }
    if let Some((name, previous)) = applied.narrowed_identifier {
        env.variables.insert(name, previous);
    }
}
