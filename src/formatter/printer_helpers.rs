//! Helper renderers for the AST pretty-printer.

extern crate alloc;

use crate::ast::{
    BinaryOp, DeclarationAnnotation, FunctionModifier, LetBinding, LiteralValue, Pattern, Type,
    TypeDeclarationForm, UnaryOp,
};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Format the optional `errors ...` clause and function arrow for a signature.
pub(super) fn format_signature_errors_and_arrow(
    prefix: &str,
    error_types: &[String],
    max_line_width: usize,
) -> String {
    if error_types.is_empty() {
        return format!("{prefix} =>");
    }

    let one_line = format!("{prefix} errors {} =>", error_types.join(", "));
    if one_line.chars().count() <= max_line_width || error_types.len() == 1 {
        return one_line;
    }

    let error_prefix = format!("{prefix} errors ");
    let continuation_indent = " ".repeat(error_prefix.chars().count());
    let mut lines = Vec::with_capacity(error_types.len());
    for (index, error_type) in error_types.iter().enumerate() {
        let line_prefix = if index == 0 {
            error_prefix.as_str()
        } else {
            continuation_indent.as_str()
        };
        let suffix = if index.saturating_add(1) == error_types.len() {
            " =>"
        } else {
            ","
        };
        lines.push(format!("{line_prefix}{error_type}{suffix}"));
    }
    lines.join("\n")
}

/// Format a function declaration modifier prefix.
pub(super) fn print_function_modifier_prefix(
    modifiers: &[FunctionModifier],
    is_entry: bool,
) -> String {
    let modifier_parts = modifiers
        .iter()
        .filter_map(|modifier| match *modifier {
            FunctionModifier::Pure => Some("pure"),
            FunctionModifier::Untested if !is_entry => Some("untested"),
            FunctionModifier::Untested => None,
        })
        .collect::<Vec<_>>();
    if modifier_parts.is_empty() {
        String::new()
    } else {
        format!("{} ", modifier_parts.join(" "))
    }
}

/// Format a statement guard header, including multiline guarded expressions.
pub(super) fn format_guard_header(
    indent: &str,
    expression: &str,
    success_binding: Option<&String>,
    success_binding_type: Option<&Type>,
    success_binding_is_mutable: bool,
    success_bindings: &[LetBinding],
    error_binding: &str,
) -> String {
    let multiline_separator = if expression.contains('\n') { "\n" } else { " " };
    if success_bindings.len() > 1
        || success_bindings
            .first()
            .is_some_and(|binding| binding.returned_label.is_some())
    {
        let rendered_bindings = success_bindings
            .iter()
            .map(|binding| {
                binding.returned_label.as_ref().map_or_else(
                    || binding.name.clone(),
                    |returned_label| format!("{returned_label}: {}", binding.name),
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "{indent}guard {expression}{multiline_separator}{indent}into {rendered_bindings} else {error_binding} =>"
        );
    }

    success_binding.map_or_else(
        || {
            if expression.contains('\n') {
                format!("{indent}guard {expression}\n{indent}else {error_binding} =>")
            } else {
                format!("{indent}guard {expression} else {error_binding} =>")
            }
        },
        |binding| {
            let binding_type = success_binding_type
                .as_ref()
                .map_or_else(String::new, |ty| format!(": {}", print_type(ty)));
            let mutable = if success_binding_is_mutable {
                " mutable"
            } else {
                ""
            };
            format!(
                "{indent}guard {expression}{multiline_separator}{indent}into {binding}{binding_type}{mutable} else {error_binding} =>"
            )
        },
    )
}

/// Format an array literal from already formatted element strings.
pub(super) fn format_array_literal(elements: &[String], depth: usize, indent_unit: &str) -> String {
    if elements.is_empty() {
        return String::from("[]");
    }
    if elements.iter().all(|element| !element.contains('\n')) {
        return format!("[{}]", elements.join(", "));
    }

    let element_indent = indent_unit.repeat(depth.saturating_add(1));
    let closing_indent = indent_unit.repeat(depth);
    let mut lines = vec![String::from("[")];
    for (index, element) in elements.iter().enumerate() {
        for (line_index, line) in element.lines().enumerate() {
            if line_index == 0 {
                lines.push(format!("{element_indent}{line}"));
            } else {
                lines.push(line.to_owned());
            }
        }
        if index.saturating_add(1) < elements.len() {
            lines.push(format!("{element_indent},"));
        }
    }
    lines.push(format!("{closing_indent}]"));
    lines.join("\n")
}

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

/// Pretty-print a pattern.
pub(super) fn print_pattern(pattern: &Pattern) -> String {
    match *pattern {
        Pattern::Literal { ref value, .. } => print_literal(value),
        Pattern::Binding { ref name, .. } => name.clone(),
        Pattern::Wildcard { .. } => String::from("_"),
        Pattern::Variant {
            ref type_name,
            ref variant_name,
            ref fields,
            ..
        } => {
            let prefix = type_name
                .as_ref()
                .map_or_else(String::new, |tn| format!("{tn}."));
            if fields.is_empty() {
                format!("{prefix}{variant_name}")
            } else {
                let field_strings: Vec<String> = fields
                    .iter()
                    .map(|pair| {
                        pair.0.as_ref().map_or_else(
                            || print_pattern(&pair.1),
                            |field_name| format!("{field_name}: {}", print_pattern(&pair.1)),
                        )
                    })
                    .collect();
                format!("{prefix}{variant_name}({})", field_strings.join(", "))
            }
        }
        Pattern::Tuple { ref elements, .. } => {
            let pattern_strings: Vec<String> = elements.iter().map(print_pattern).collect();
            format!("({})", pattern_strings.join(", "))
        }
    }
}

/// Pretty-print a proposal declaration annotation.
pub(super) fn print_declaration_annotation(annotation: &DeclarationAnnotation) -> String {
    match *annotation {
        DeclarationAnnotation::Availability { ref value, .. } => {
            format!("@availability({value})")
        }
        DeclarationAnnotation::ConstructorVisibility { ref value, .. } => {
            format!("@constructor_visibility({value})")
        }
        DeclarationAnnotation::AbiTypeId { value, .. } => format!("@abi_type_id({value})"),
        DeclarationAnnotation::AbiEvolution { ref value, .. } => {
            format!("@abi_evolution({value})")
        }
    }
}

/// Prefix written before `type` for proposal-specific declaration forms.
pub(super) const fn print_type_declaration_form(form: TypeDeclarationForm) -> &'static str {
    match form {
        TypeDeclarationForm::Nominal => "",
        TypeDeclarationForm::Constrained => "constrained ",
        TypeDeclarationForm::OpaqueImmutable => "opaque immutable ",
        TypeDeclarationForm::CompilerRegisteredAffineResource => {
            "compiler_registered affine resource "
        }
        TypeDeclarationForm::NonExhaustive => "non_exhaustive ",
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
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}
