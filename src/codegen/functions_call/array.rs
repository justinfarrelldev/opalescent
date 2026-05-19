#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]

#[path = "array/helpers.rs"]
mod helpers;
#[path = "array/zip.rs"]
mod zip;
#[path = "array/intrinsics.rs"]
mod intrinsics;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use inkwell::values::BasicValueEnum;

pub(super) fn is_array_intrinsic_name(name: &str) -> bool {
    intrinsics::is_array_intrinsic_name(name)
}

pub(super) fn codegen_array_intrinsic_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    intrinsic_name: &str,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    intrinsics::codegen_array_intrinsic_call(codegen_context, env, intrinsic_name, args)
}

pub(super) fn codegen_array_member_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    member: &str,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    intrinsics::codegen_array_member_call(codegen_context, env, receiver, member, args)
}

