extern crate alloc;

use crate::ast::{Expr, Stmt, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{codegen_expression, CodegenEnv, CodegenError, VariableBinding};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::string::String;

/// Lower one typed statement into LLVM IR side effects.
pub fn codegen_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    stmt: &Stmt,
) -> Result<(), CodegenError> {
    match *stmt {
        Stmt::Let {
            ref binding,
            ref initializer,
            ..
        } => {
            let declared_type = if let Some(ref annotation) = binding.type_annotation {
                ast_type_to_core_type(annotation)?
            } else if let Some(ref init_expr) = *initializer {
                infer_core_type_from_expr(init_expr)
            } else {
                CoreType::Unit
            };

            let alloca_type = core_type_to_llvm(codegen_context.context, &declared_type);
            let alloca = codegen_context
                .builder
                .build_alloca(alloca_type, binding.name.as_str())?;

            if let Some(ref init_expr) = *initializer {
                let value =
                    codegen_expression(codegen_context, env, init_expr, Some(&declared_type))?;
                let _store_instruction = codegen_context.builder.build_store(alloca, value)?;
            }

            env.variables.insert(
                binding.name.clone(),
                VariableBinding {
                    alloca,
                    core_type: declared_type,
                },
            );
            Ok(())
        }
        Stmt::Assignment {
            ref target,
            ref value,
            ..
        } => codegen_assignment(codegen_context, env, target, value),
        _ => Ok(()),
    }
}

/// Lower a simple identifier assignment into a store.
fn codegen_assignment<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    value: &Expr,
) -> Result<(), CodegenError> {
    if let Expr::Identifier { ref name, .. } = *target {
        let Some(binding_snapshot) = env.variables.get(name) else {
            return Err(CodegenError::new(format!(
                "assignment target '{name}' not found"
            )));
        };
        let binding_alloca = binding_snapshot.alloca;
        let binding_type = binding_snapshot.core_type.clone();

        let rhs_value = codegen_expression(codegen_context, env, value, Some(&binding_type))?;
        let _store_instruction = codegen_context
            .builder
            .build_store(binding_alloca, rhs_value)?;
        return Ok(());
    }

    Err(CodegenError::new(String::from(
        "assignment target must be an identifier in task 22",
    )))
}

/// Convert parsed AST type annotations into backend core types.
fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, CodegenError> {
    match *ast_type {
        Type::Basic { ref name, .. } => match name.as_str() {
            "int8" => Ok(CoreType::Int8),
            "int16" => Ok(CoreType::Int16),
            "int32" => Ok(CoreType::Int32),
            "int64" => Ok(CoreType::Int64),
            "uint8" => Ok(CoreType::UInt8),
            "uint16" => Ok(CoreType::UInt16),
            "uint32" => Ok(CoreType::UInt32),
            "uint64" => Ok(CoreType::UInt64),
            "float32" => Ok(CoreType::Float32),
            "float64" => Ok(CoreType::Float64),
            "string" => Ok(CoreType::String),
            "boolean" => Ok(CoreType::Boolean),
            "void" | "unit" => Ok(CoreType::Unit),
            _ => Err(CodegenError::new(format!("unsupported type '{name}'"))),
        },
        Type::Array {
            ref element_type, ..
        } => Ok(CoreType::Array(alloc::boxed::Box::new(
            ast_type_to_core_type(element_type)?,
        ))),
        _ => Err(CodegenError::new(String::from(
            "unsupported let type annotation for task 22",
        ))),
    }
}

/// Infer a fallback core type for let initializers without explicit annotations.
fn infer_core_type_from_expr(expr: &Expr) -> CoreType {
    match *expr {
        Expr::Literal { ref value, .. } => match *value {
            crate::ast::LiteralValue::Integer(_) => CoreType::Int64,
            crate::ast::LiteralValue::Float(_) => CoreType::Float64,
            crate::ast::LiteralValue::String(_) => CoreType::String,
            crate::ast::LiteralValue::Boolean(_) => CoreType::Boolean,
            crate::ast::LiteralValue::Void => CoreType::Unit,
        },
        Expr::Array { .. } => CoreType::Array(alloc::boxed::Box::new(CoreType::Int64)),
        _ => CoreType::Int64,
    }
}
