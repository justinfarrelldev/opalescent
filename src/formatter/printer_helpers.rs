//! Helper renderers for the AST pretty-printer.

extern crate alloc;

use crate::ast::{BinaryOp, LiteralValue, Type, UnaryOp};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Pretty-print a type annotation.
pub(super) fn print_type(ty: &Type) -> String {
    match *ty {
        Type::Basic { ref name, .. } => name.clone(),
        Type::Array {
            ref element_type, ..
        } => format!("{}[]", print_type(element_type)),
        Type::Function {
            ref parameters,
            ref return_types,
            ..
        } => {
            let param_strs: Vec<String> = parameters.iter().map(print_type).collect();
            let ret_strs: Vec<String> = return_types.iter().map(print_type).collect();
            format!("f({}): {}", param_strs.join(", "), ret_strs.join(", "))
        }
        Type::Generic {
            ref name,
            ref type_args,
            ..
        } => {
            let arg_strs: Vec<String> = type_args.iter().map(print_type).collect();
            format!("{name}<{}>", arg_strs.join(", "))
        }
    }
}

/// Pretty-print a literal value.
pub(super) fn print_literal(lit: &LiteralValue) -> String {
    match *lit {
        LiteralValue::Integer(n) => format!("{n}"),
        LiteralValue::Float(f) => {
            let s = format!("{f}");
            if s.contains('.') { s } else { format!("{s}.0") }
        }
        LiteralValue::String(ref s) => format!("'{}'", escape_single_quoted_string(s)),
        LiteralValue::Boolean(b) => (if b { "true" } else { "false" }).to_owned(),
        LiteralValue::Void => String::from("void"),
    }
}

/// Pretty-print a binary operator.
pub(super) const fn print_binary_op(op: &BinaryOp) -> &'static str {
    match *op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::DivEuclid => "div_euclid",
        BinaryOp::ModEuclid => "mod_euclid",
        BinaryOp::Power => "^",
        BinaryOp::Equal | BinaryOp::Is => "is",
        BinaryOp::NotEqual | BinaryOp::IsNot => "is not",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Xor => "xor",
        BinaryOp::BitAnd => "band",
        BinaryOp::BitOr => "bor",
        BinaryOp::BitXor => "bxor",
        BinaryOp::BitShiftLeft => "bshl",
        BinaryOp::BitShiftRight => "bshr",
        BinaryOp::BitUnsignedShiftRight => "bushr",
        BinaryOp::Assign => "=",
    }
}

/// Pretty-print a unary operator.
pub(super) const fn print_unary_op(op: &UnaryOp) -> &'static str {
    match *op {
        UnaryOp::Negate => "-",
        UnaryOp::Plus => "+",
        UnaryOp::Not => "not",
        UnaryOp::BitNot => "bnot",
    }
}

/// Escape a string for single-quoted literal rendering.
pub(super) fn escape_single_quoted_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}
