extern crate alloc;

use crate::ast::{Expr, LetBinding, Stmt, Type};
use crate::codegen::adts::product_field_indices_from_constructor;
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_statement, codegen_loop_expression_into_slots, codegen_loop_statement,
    codegen_return_statement,
};
use crate::codegen::expressions::{codegen_expression, CodegenEnv, CodegenError, VariableBinding};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;

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
        } => codegen_let_statement(codegen_context, env, binding, initializer.as_ref()),
        Stmt::LetDestructure {
            ref bindings,
            ref initializer,
            ..
        } => codegen_let_destructure_statement(
            codegen_context,
            env,
            bindings.as_slice(),
            initializer,
        ),
        Stmt::Assignment {
            ref target,
            ref value,
            ..
        } => codegen_assignment(codegen_context, env, target, value),
        Stmt::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => codegen_if_statement(
            codegen_context,
            env,
            condition,
            then_branch.as_ref(),
            else_branch.as_deref(),
        ),
        Stmt::For { .. } | Stmt::While { .. } | Stmt::Loop { .. } => {
            codegen_loop_statement(codegen_context, env, stmt)
        }
        Stmt::Return { ref values, .. } => {
            codegen_return_statement(codegen_context, env, values.as_slice())
        }
        Stmt::Block { ref statements, .. } => {
            for statement in statements {
                codegen_statement(codegen_context, env, statement)?;
            }
            Ok(())
        }
        Stmt::Expression { ref expr, .. } => {
            let _value = codegen_expression(codegen_context, env, expr, None)?;
            Ok(())
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
    }
}

/// Lower a `let` statement by allocating storage and binding initializer values.
fn codegen_let_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding: &LetBinding,
    initializer: Option<&Expr>,
) -> Result<(), CodegenError> {
    let (declared_type, lowered_initializer) = if let Some(ref annotation) = binding.type_annotation
    {
        let declared_type = ast_type_to_core_type(annotation)?;
        let lowered = if let Some(init_expr) = initializer {
            Some(codegen_expression(
                codegen_context,
                env,
                init_expr,
                Some(&declared_type),
            )?)
        } else {
            None
        };
        (declared_type, lowered)
    } else if let Some(init_expr) = initializer {
        (
            infer_core_type_from_expr(init_expr),
            Some(codegen_expression(codegen_context, env, init_expr, None)?),
        )
    } else {
        (CoreType::Unit, None)
    };

    let alloca = if let Some(initializer_value) = lowered_initializer {
        let alloca = codegen_context
            .builder
            .build_alloca(initializer_value.get_type(), binding.name.as_str())?;
        let _store_instruction = codegen_context
            .builder
            .build_store(alloca, initializer_value)?;
        alloca
    } else {
        let alloca_type = core_type_to_llvm(codegen_context.context, &declared_type);
        codegen_context
            .builder
            .build_alloca(alloca_type, binding.name.as_str())?
    };

    env.variables.insert(
        binding.name.clone(),
        VariableBinding {
            alloca,
            core_type: declared_type,
        },
    );
    if let Some(&Expr::Constructor { .. }) = initializer {
        if let Some(field_indices) = initializer.and_then(product_field_indices_from_constructor) {
            env.variable_field_indices
                .insert(binding.name.clone(), field_indices);
        }
    }

    Ok(())
}

/// Lower a destructuring `let` from a loop expression into preallocated slots.
fn codegen_let_destructure_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    bindings: &[LetBinding],
    initializer: &Expr,
) -> Result<(), CodegenError> {
    let Expr::Loop { ref body, .. } = *initializer else {
        return Err(CodegenError::new(String::from(
            "destructuring let currently requires loop expression initializer",
        )));
    };

    let mut slots = Vec::new();
    let mut labels = Vec::new();
    for binding in bindings {
        let binding_type = binding
            .type_annotation
            .as_ref()
            .map(ast_type_to_core_type)
            .transpose()?
            .unwrap_or(CoreType::Int64);
        let slot_type = core_type_to_llvm(codegen_context.context, &binding_type);
        let alloca = codegen_context
            .builder
            .build_alloca(slot_type, binding.name.as_str())?;
        slots.push(alloca);
        labels.push(binding.name.clone());
        env.variables.insert(
            binding.name.clone(),
            VariableBinding {
                alloca,
                core_type: binding_type,
            },
        );
    }

    codegen_loop_expression_into_slots(
        codegen_context,
        env,
        body.as_ref(),
        slots.as_slice(),
        labels.as_slice(),
    )
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
