//! Top-level expression codegen dispatcher.

use super::{
    BasicMetadataValueEnum, BasicValueEnum, CodegenContext, CodegenEnv, CodegenError, CoreType,
    Expr, String, codegen_array_access, codegen_array_literal, codegen_binary,
    codegen_call_expression, codegen_cast, codegen_constructor_expression,
    codegen_field_access_expression, codegen_guard_expression, codegen_identifier,
    codegen_if_expression, codegen_literal, codegen_match_expression, codegen_propagate_expression,
    codegen_refinement, codegen_string_access, codegen_string_interpolation, codegen_unary, format,
    infer_expression_core_type,
};

fn codegen_terminal_constrain_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target_type: &crate::ast::Type,
    value: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let (runtime_symbol, value_expected_type, lower_bound, upper_bound, is_control_code) =
        match target_type {
            crate::ast::Type::Basic { name, .. } if name == "TerminalControlCode" => (
                "opal_terminal_constrain_u8_control_code",
                CoreType::UInt8,
                0_i32,
                0_i32,
                true,
            ),
            crate::ast::Type::Basic { name, .. } => {
                let bounds = match name.as_str() {
                    "TerminalFunctionKeyNumber" => Some((1_i32, 0x7FFF_i32)),
                    "TerminalColumnCount" | "TerminalRowCount" => Some((1_i32, i32::MAX)),
                    "TerminalColumnIndex" | "TerminalRowIndex" => Some((0_i32, i32::MAX)),
                    "TerminalKeyRepeatCount" | "TerminalWaitMilliseconds" => {
                        Some((1_i32, i32::MAX))
                    }
                    "TerminalInputSequenceTimeoutMilliseconds" => Some((1_i32, 60_000_i32)),
                    "TerminalCommittedTextByteLimit"
                    | "TerminalCompositionPreeditByteLimit"
                    | "TerminalPendingSequenceByteLimit" => Some((4_i32, 0x0010_0000_i32)),
                    "TerminalPasteChunkByteLimit" | "TerminalColorCount" => {
                        Some((1_i32, 0x0100_0000_i32))
                    }
                    "TerminalUnknownByteChunkLimit" => Some((1_i32, 0x0010_0000_i32)),
                    "TerminalRetainedEventLimit" => Some((8_i32, 0x0010_0000_i32)),
                    "TerminalRetainedByteLimit" => Some((4_096_i32, 0x4000_0000_i32)),
                    "TerminalCorrelatedEventLimit" => Some((2_i32, 0x0001_0000_i32)),
                    "TerminalCorrelatedByteLimit" => Some((64_i32, 0x0100_0000_i32)),
                    "TerminalDiagnosticCountLimit" => Some((1_i32, 256_i32)),
                    "TerminalDiagnosticCollectionByteLimit" => Some((256_i32, 0x0010_0000_i32)),
                    _ => None,
                };
                let Some((minimum, maximum)) = bounds else {
                    return Err(CodegenError::new(format!(
                        "runtime lowering for constrained type '{name}' is not implemented yet"
                    )));
                };
                (
                    "opal_terminal_constrain_i32_range",
                    CoreType::Int32,
                    minimum,
                    maximum,
                    false,
                )
            }
            _ => {
                return Err(CodegenError::new(String::from(
                    "constrain target must lower from a basic nominal terminal type",
                )));
            }
        };

    let runtime_function =
        crate::codegen::functions_stdlib::declare_stdlib_function(codegen_context, runtime_symbol)
            .ok_or_else(|| CodegenError::new(format!("{runtime_symbol} declaration missing")))?;
    let lowered_value =
        super::codegen_expression(codegen_context, env, value, Some(&value_expected_type))?;
    let mut args: Vec<BasicMetadataValueEnum<'context>> = vec![lowered_value.into()];
    if !is_control_code {
        let lowered_minimum = crate::codegen::types::integer_literal_bits(i64::from(lower_bound))
            .map_err(|error| CodegenError::new(error.to_string()))?;
        args.push(
            codegen_context
                .context
                .i32_type()
                .const_int(lowered_minimum, true)
                .into(),
        );
        let lowered_maximum = crate::codegen::types::integer_literal_bits(i64::from(upper_bound))
            .map_err(|error| CodegenError::new(error.to_string()))?;
        args.push(
            codegen_context
                .context
                .i32_type()
                .const_int(lowered_maximum, true)
                .into(),
        );
    }
    let call = codegen_context.builder.build_call(
        runtime_function,
        args.as_slice(),
        &env.next_name("terminal.constrain"),
    )?;
    call.try_as_basic_value().basic().ok_or_else(|| {
        CodegenError::new(String::from(
            "terminal constrained runtime helper should return an error-bearing aggregate",
        ))
    })
}

pub fn codegen_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match *expr {
        Expr::Literal { ref value, .. } => {
            codegen_literal(codegen_context, env, value, expected_type)
        }
        Expr::Identifier { ref name, .. } => {
            codegen_identifier(codegen_context, env, name, expected_type)
        }
        Expr::Parenthesized { ref expr, .. } => {
            codegen_expression(codegen_context, env, expr, expected_type)
        }
        Expr::BorrowArgument { ref target, .. } => {
            codegen_expression(codegen_context, env, target.as_ref(), expected_type)
        }
        Expr::Binary {
            ref left,
            ref operator,
            ref right,
            ..
        } => codegen_binary(codegen_context, env, left, operator, right, expected_type),
        Expr::Unary {
            ref operator,
            ref operand,
            ..
        } => codegen_unary(codegen_context, env, operator, operand, expected_type),
        Expr::Cast {
            ref expr,
            ref target_type,
            ..
        } => codegen_cast(codegen_context, env, expr, target_type),
        Expr::Constrain {
            ref target_type,
            ref value,
            ..
        } => codegen_terminal_constrain_expression(codegen_context, env, target_type, value),
        Expr::Refinement {
            ref value,
            ref variant,
            ..
        } => codegen_refinement(codegen_context, env, value.as_ref(), variant.as_ref()),
        Expr::Array { ref elements, .. } => {
            codegen_array_literal(codegen_context, env, elements.as_slice(), expected_type)
        }
        Expr::Index {
            ref object,
            ref index,
            span,
            ..
        } => match infer_expression_core_type(env, object.as_ref()) {
            Some(CoreType::String) => {
                codegen_string_access(codegen_context, env, object, index, span, expected_type)
            }
            Some(CoreType::Array(_)) => {
                codegen_array_access(codegen_context, env, object, index, expected_type)
            }
            Some(other) => Err(CodegenError::new(format!(
                "index access expects array or string receiver, found '{other}'"
            ))),
            None => Err(CodegenError::new(String::from(
                "index access receiver type could not be inferred",
            ))),
        },
        Expr::Call {
            ref callee,
            ref generic_args,
            ref args,
            ..
        } => codegen_call_expression(
            codegen_context,
            env,
            callee.as_ref(),
            generic_args.as_deref(),
            args.as_slice(),
            expected_type,
        ),
        Expr::Constructor { .. } => {
            codegen_constructor_expression(codegen_context, env, expr, expected_type)
        }
        Expr::Match { .. } => codegen_match_expression(codegen_context, env, expr),
        Expr::Loop { .. } => Err(CodegenError::new(String::from(
            "loop expressions are lowered in statement context",
        ))),
        Expr::Member { .. } => codegen_field_access_expression(codegen_context, env, expr),
        Expr::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => codegen_if_expression(
            codegen_context,
            env,
            condition.as_ref(),
            then_branch.as_ref(),
            else_branch.as_deref(),
        ),
        Expr::Guard {
            ref expr,
            ref binding_name,
            ref else_branch,
            ..
        } => codegen_guard_expression(
            codegen_context,
            env,
            expr.as_ref(),
            binding_name.as_str(),
            else_branch.as_ref(),
            expected_type,
        ),
        Expr::Propagate {
            ref call,
            ref cause,
            ..
        } => codegen_propagate_expression(
            codegen_context,
            env,
            call.as_ref(),
            cause.as_deref(),
            expected_type,
        ),
        Expr::StringInterpolation { ref parts, .. } => {
            codegen_string_interpolation(codegen_context, env, parts.as_slice())
        }
        _ => Err(CodegenError::new(String::from(
            "unsupported expression kind",
        ))),
    }
}
