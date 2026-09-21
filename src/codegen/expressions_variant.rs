//! Runtime tag checks for generated nominal sum variants.

extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::types::integer_literal_bits;
use alloc::string::String;
use inkwell::values::{BasicValue, BasicValueEnum};
use inkwell::{AddressSpace, IntPredicate};

fn variant_type_expression_parts(expr: &Expr) -> Option<(&str, &str)> {
    let Expr::Member { object, member, .. } = expr else {
        return None;
    };
    let Expr::Identifier { name, .. } = object.as_ref() else {
        return None;
    };
    Some((name.as_str(), member.as_str()))
}

pub(super) fn codegen_variant_tag_compare<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value_expr: &Expr,
    variant_expr: &Expr,
    invert: bool,
) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
    let Some((type_name, variant_name)) = variant_type_expression_parts(variant_expr) else {
        return Ok(None);
    };
    let Some(variant_tag) =
        crate::type_system::terminal_proposal_variant_id(type_name, variant_name)
    else {
        return Ok(None);
    };

    let value = codegen_expression(codegen_context, env, value_expr, None)?;
    let i64_type = codegen_context.context.i64_type();
    let tag_value = if value.is_pointer_value() {
        let tagged_type = codegen_context.context.struct_type(
            &[
                i64_type.into(),
                codegen_context.context.i8_type().array_type(64).into(),
            ],
            false,
        );
        let tagged_ptr = codegen_context.builder.build_pointer_cast(
            value.into_pointer_value(),
            tagged_type.ptr_type(AddressSpace::default()),
            &env.next_name("variant.test.tagged.cast"),
        )?;
        // SAFETY: GEP selects the tag field within the tagged nominal payload layout.
        let tag_ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                tagged_ptr,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_zero(),
                ],
                &env.next_name("variant.test.tag.ptr"),
            )?
        };
        codegen_context
            .builder
            .build_load(tag_ptr, &env.next_name("variant.test.tag.load"))?
            .into_int_value()
    } else if value.is_struct_value() {
        codegen_context
            .builder
            .build_extract_value(
                value.into_struct_value(),
                0,
                &env.next_name("variant.test.tag.extract"),
            )?
            .into_int_value()
    } else {
        return Err(CodegenError::new(String::from(
            "variant test expects a tagged nominal value",
        )));
    };

    let expected_bits =
        integer_literal_bits(variant_tag).map_err(|error| CodegenError::new(error.to_string()))?;
    let predicate = if invert {
        IntPredicate::NE
    } else {
        IntPredicate::EQ
    };
    let comparison = codegen_context.builder.build_int_compare(
        predicate,
        tag_value,
        i64_type.const_int(expected_bits, true),
        &env.next_name("variant.test"),
    )?;
    Ok(Some(comparison.as_basic_value_enum()))
}
