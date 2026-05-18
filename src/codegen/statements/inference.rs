#![allow(
    clippy::all,
    clippy::needless_pass_by_ref_mut,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::runtime_type_info::{known_runtime_return_type, llvm_return_type_to_core_type};
use crate::ast::{Expr, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::propertyless_constructors::lookup_propertyless_constructor;
use crate::type_system::type_mapping::{AstTypeMappingError, ast_type_to_core_type};
use crate::type_system::types::CoreType;

/// Map a let-binding annotation into the lowered core type used during code generation.
pub(super) fn ast_type_to_core_type_for_let(ast_type: &Type) -> Result<CoreType, CodegenError> {
    if matches!(*ast_type, Type::Function { .. } | Type::Generic { .. }) {
        return Err(CodegenError::new(String::from(
            "unsupported type annotation in let binding",
        )));
    }

    match ast_type_to_core_type(ast_type) {
        Ok(core_type) => Ok(core_type),
        Err(AstTypeMappingError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
            name: type_name,
            type_args: alloc::vec::Vec::new(),
        }),
    }
}

/// Infer the lowered core type for an expression when statement lowering must allocate storage.
pub(super) fn infer_core_type_from_expr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expr: &Expr,
) -> CoreType {
    match *expr {
        Expr::Literal { ref value, .. } => match *value {
            crate::ast::LiteralValue::Integer(_) => CoreType::Int64,
            crate::ast::LiteralValue::Float(_) => CoreType::Float64,
            crate::ast::LiteralValue::String(_) => CoreType::String,
            crate::ast::LiteralValue::Boolean(_) => CoreType::Boolean,
            crate::ast::LiteralValue::Void => CoreType::Unit,
        },
        Expr::Array { ref elements, .. } => elements.first().map_or_else(
            || CoreType::Array(alloc::boxed::Box::new(CoreType::Int64)),
            |first| {
                let element_core = infer_core_type_from_expr(codegen_context, env, first);
                CoreType::Array(alloc::boxed::Box::new(element_core))
            },
        ),
        Expr::Call {
            ref callee,
            ref args,
            ..
        } => infer_call_return_type(codegen_context, env, callee, args).unwrap_or(CoreType::Int64),
        Expr::Propagate { ref call, .. } => infer_core_type_from_expr(codegen_context, env, call),
        Expr::Identifier { ref name, .. } => env
            .variables
            .get(name)
            .map_or(CoreType::Int64, |binding| binding.core_type.clone()),
        Expr::Member {
            ref object,
            ref member,
            ..
        } => {
            let object_type = infer_core_type_from_expr(codegen_context, env, object);
            match object_type {
                CoreType::String | CoreType::Array(_) if member == "length" => CoreType::Int64,
                CoreType::Generic {
                    ref name,
                    ref type_args,
                } if name == "Bytes" && type_args.is_empty() && member == "length" => {
                    CoreType::Int32
                }
                CoreType::Generic {
                    ref name,
                    ref type_args,
                } if name == "Pair" && type_args.len() == 2 => match member.as_str() {
                    "first" => type_args[0].clone(),
                    "second" => type_args[1].clone(),
                    _ => CoreType::Int64,
                },
                _ => CoreType::Int64,
            }
        }
        Expr::Constructor {
            ref callee,
            ref fields,
            ..
        } => {
            if fields.is_empty() {
                if let Expr::Identifier { ref name, .. } = **callee {
                    if lookup_propertyless_constructor(name.as_str()).is_some() {
                        return CoreType::Generic {
                            name: name.clone(),
                            type_args: alloc::vec::Vec::new(),
                        };
                    }
                }
            }
            CoreType::Int64
        }
        _ => CoreType::Int64,
    }
}

#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed signatures is clearer than manual dereferencing"
)]
/// Infer the return type for a call expression from imported signatures, runtime metadata, or emitted functions.
fn infer_call_return_type<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
) -> Option<CoreType> {
    if let Expr::Member {
        ref object,
        ref member,
        ..
    } = *callee
    {
        let receiver_type = infer_core_type_from_expr(codegen_context, env, object);
        return infer_member_call_return_type(env, &receiver_type, member.as_str(), args);
    }

    let Expr::Identifier { ref name, .. } = *callee else {
        return None;
    };

    if name == "append" {
        return args.first().and_then(|array_expr| {
            let array_type = infer_core_type_from_expr(codegen_context, env, array_expr);
            matches!(array_type, CoreType::Array(_)).then_some(array_type)
        });
    }

    if let Some(imported_signature) = env.imported_signatures.get(name) {
        if let CoreType::Function { return_types, .. } = imported_signature {
            if let Some(first_return) = return_types.first() {
                return Some(first_return.clone());
            }
        }
    }

    if let Some(runtime_name) = env.imported_functions.get(name) {
        if let Some(runtime_return_type) = known_runtime_return_type(runtime_name.as_str()) {
            return Some(runtime_return_type);
        }
    }
    if let Some(runtime_return_type) = known_runtime_return_type(name) {
        return Some(runtime_return_type);
    }

    if let Some(function) = codegen_context.module.get_function(name) {
        return llvm_return_type_to_core_type(function.get_type().get_return_type());
    }

    env.imported_functions.get(name).and_then(|runtime_name| {
        codegen_context
            .module
            .get_function(runtime_name.as_str())
            .and_then(|function| {
                llvm_return_type_to_core_type(function.get_type().get_return_type())
            })
    })
}

#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed receiver core types is clearer than manual dereferencing"
)]
/// Infer the result type of collection-style member calls used by let bindings.
fn infer_member_call_return_type(
    env: &CodegenEnv<'_>,
    receiver_type: &CoreType,
    member: &str,
    args: &[Expr],
) -> Option<CoreType> {
    match (receiver_type, member) {
        (&CoreType::Array(_), "length") => Some(CoreType::Int64),
        (CoreType::Array(_), "push") => Some(CoreType::Unit),
        (CoreType::Array(element_type), "pop") => Some(element_type.as_ref().clone()),
        (CoreType::Array(_), "map") => infer_callback_return_core_type(env, args.first()?)
            .map(|callback_return| CoreType::Array(Box::new(callback_return))),
        (CoreType::Array(element_type), "filter") => {
            Some(CoreType::Array(Box::new(element_type.as_ref().clone())))
        }
        (CoreType::Array(_), "reduce") => infer_callback_return_core_type(env, args.get(1)?),
        (CoreType::Array(left_type), "zip") => {
            let Expr::Identifier { ref name, .. } = *args.first()? else {
                return None;
            };
            let right_type = env.variables.get(name)?.core_type.clone();
            let CoreType::Array(right_element_type) = right_type else {
                return None;
            };
            Some(CoreType::Array(alloc::boxed::Box::new(CoreType::Generic {
                name: String::from("Pair"),
                type_args: vec![
                    left_type.as_ref().clone(),
                    right_element_type.as_ref().clone(),
                ],
            })))
        }
        (CoreType::Generic { name, type_args }, "length")
            if name == "Bytes" && type_args.is_empty() =>
        {
            Some(CoreType::Int32)
        }
        _ => None,
    }
}

#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed function signatures is clearer than manual dereferencing"
)]
/// Infer the callback return type for higher-order collection methods.
fn infer_callback_return_core_type(env: &CodegenEnv<'_>, callback: &Expr) -> Option<CoreType> {
    match *callback {
        Expr::Lambda {
            ref return_types, ..
        } => return_types
            .first()
            .and_then(|return_type| ast_type_to_core_type_for_let(return_type).ok()),
        Expr::Identifier { ref name, .. } => env
            .variables
            .get(name.as_str())
            .and_then(|binding| match &binding.core_type {
                CoreType::Function { return_types, .. } => return_types.first().cloned(),
                _ => None,
            })
            .or_else(|| {
                env.imported_signatures
                    .get(name.as_str())
                    .and_then(|signature| match signature {
                        CoreType::Function { return_types, .. } => return_types.first().cloned(),
                        _ => None,
                    })
            }),
        _ => None,
    }
}
