//! Helper renderers for the AST pretty-printer.

extern crate alloc;

use crate::ast::{
    BinaryOp, DeclarationAnnotation, FunctionModifier, LetBinding, LiteralValue, Pattern, Type,
    TypeDeclarationForm, UnaryOp,
};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Return the character width of the final physical line in `text`.
fn last_line_width(text: &str) -> usize {
    text.lines().last().map_or(0, |line| line.chars().count())
}

/// Append `suffix` to the last line in `text`.
fn append_to_last_line(text: &str, suffix: &str) -> String {
    let Some((before, last)) = text.rsplit_once('\n') else {
        return format!("{text}{suffix}");
    };
    format!("{before}\n{last}{suffix}")
}

/// Format a complete function/lambda signature through its `=>` arrow.
pub(super) fn format_function_signature(
    prefix_before_f: &str,
    parameters: &[String],
    returns: &str,
    error_types: &[String],
    depth: usize,
    indent_unit: &str,
    max_line_width: usize,
) -> String {
    let flat_head = format!("{prefix_before_f}f({}){returns}", parameters.join(", "));
    let flat_full = if error_types.is_empty() {
        format!("{flat_head} =>")
    } else {
        format!("{flat_head} errors {} =>", error_types.join(", "))
    };

    if flat_full.chars().count() <= max_line_width {
        return flat_full;
    }

    let base_indent = indent_unit.repeat(depth);
    let param_indent = indent_unit.repeat(depth.saturating_add(1));
    let should_wrap_parameters = !parameters.is_empty()
        && (flat_head.chars().count() > max_line_width || parameters.len() > 1);
    let head = if should_wrap_parameters {
        let mut lines = Vec::with_capacity(parameters.len().saturating_add(2));
        lines.push(format!("{prefix_before_f}f("));
        for (index, parameter) in parameters.iter().enumerate() {
            let suffix = if index.saturating_add(1) == parameters.len() {
                ""
            } else {
                ","
            };
            lines.push(format!("{param_indent}{parameter}{suffix}"));
        }
        lines.push(format!("{base_indent}){returns}"));
        lines.join("\n")
    } else {
        flat_head
    };

    append_errors_and_arrow(
        &head,
        error_types,
        &base_indent,
        &param_indent,
        max_line_width,
    )
}

/// Append an `errors` clause and/or `=>` arrow to a formatted signature head.
fn append_errors_and_arrow(
    head: &str,
    error_types: &[String],
    base_indent: &str,
    error_indent: &str,
    max_line_width: usize,
) -> String {
    if error_types.is_empty() {
        return append_to_last_line(head, " =>");
    }

    let one_line_suffix = format!(" errors {} =>", error_types.join(", "));
    if last_line_width(head).saturating_add(one_line_suffix.chars().count()) <= max_line_width {
        return append_to_last_line(head, &one_line_suffix);
    }

    let head_with_errors =
        if last_line_width(head).saturating_add(" errors".len()) <= max_line_width {
            append_to_last_line(head, " errors")
        } else {
            format!("{head}\n{base_indent}errors")
        };
    let mut lines = head_with_errors
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for (index, error_type) in error_types.iter().enumerate() {
        let suffix = if index.saturating_add(1) == error_types.len() {
            ""
        } else {
            ","
        };
        lines.push(format!("{error_indent}{error_type}{suffix}"));
    }
    lines.push(format!("{base_indent}=>"));
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

/// Return the separator before a guard header suffix (`into`/`else`).
fn guard_suffix_separator(indent: &str, expression: &str) -> String {
    if !expression.contains('\n') {
        return String::from(" ");
    }

    let last_line = expression.lines().last().map_or("", str::trim_end);
    if last_line.ends_with(')') || last_line.ends_with(']') {
        String::from(" ")
    } else {
        format!("\n{indent}")
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
    let suffix_separator = guard_suffix_separator(indent, expression);
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
            "{indent}guard {expression}{suffix_separator}into {rendered_bindings} else {error_binding} =>"
        );
    }

    success_binding.map_or_else(
        || format!("{indent}guard {expression}{suffix_separator}else {error_binding} =>"),
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
                "{indent}guard {expression}{suffix_separator}into {binding}{binding_type}{mutable} else {error_binding} =>"
            )
        },
    )
}

/// Format a call expression from already formatted argument strings.
pub(super) fn format_call_expression(
    callee: &str,
    generics: &str,
    arguments: &[String],
    depth: usize,
    indent_unit: &str,
    max_line_width: usize,
    prefix_width: usize,
) -> String {
    let flat = format!("{callee}{generics}({})", arguments.join(", "));
    if arguments.is_empty()
        || (!arguments.iter().any(|argument| argument.contains('\n'))
            && prefix_width.saturating_add(flat.chars().count()) <= max_line_width)
    {
        return flat;
    }

    let argument_indent = indent_unit.repeat(depth.saturating_add(1));
    let closing_indent = indent_unit.repeat(depth);
    let mut lines = vec![format!("{callee}{generics}(")];
    push_multiline_items(&mut lines, arguments, &argument_indent);
    lines.push(format!("{closing_indent})"));
    lines.join("\n")
}

/// Format an array literal from already formatted element strings.
pub(super) fn format_array_literal(
    elements: &[String],
    depth: usize,
    indent_unit: &str,
    max_line_width: usize,
    prefix_width: usize,
) -> String {
    if elements.is_empty() {
        return String::from("[]");
    }
    let flat = format!("[{}]", elements.join(", "));
    if !elements.iter().any(|element| element.contains('\n'))
        && prefix_width.saturating_add(flat.chars().count()) <= max_line_width
    {
        return flat;
    }

    let element_indent = indent_unit.repeat(depth.saturating_add(1));
    let closing_indent = indent_unit.repeat(depth);
    let mut lines = vec![String::from("[")];
    push_multiline_items(&mut lines, elements, &element_indent);
    lines.push(format!("{closing_indent}]"));
    lines.join("\n")
}

/// Format an import declaration, splitting oversized import groups into chunks.
pub(super) fn format_import_lines(
    indent: &str,
    type_prefix: &str,
    items: &[String],
    source: &str,
    max_line_width: usize,
) -> String {
    let one_line = format!(
        "{indent}import {type_prefix}{} from {source}",
        items.join(", ")
    );
    if one_line.chars().count() <= max_line_width || items.len() <= 1 {
        return one_line;
    }

    let mut lines = Vec::new();
    let mut current_items: Vec<String> = Vec::new();
    for item in items {
        let mut candidate_items = current_items.clone();
        candidate_items.push(item.clone());
        let candidate = format!(
            "{indent}import {type_prefix}{} from {source}",
            candidate_items.join(", ")
        );
        if candidate.chars().count() > max_line_width && !current_items.is_empty() {
            lines.push(format!(
                "{indent}import {type_prefix}{} from {source}",
                current_items.join(", ")
            ));
            current_items.clear();
        }
        current_items.push(item.clone());
    }
    if !current_items.is_empty() {
        lines.push(format!(
            "{indent}import {type_prefix}{} from {source}",
            current_items.join(", ")
        ));
    }
    lines.join("\n")
}

/// Push multiline collection items with commas attached to each item's final line.
fn push_multiline_items(lines: &mut Vec<String>, items: &[String], item_indent: &str) {
    for (index, item) in items.iter().enumerate() {
        let has_comma = index.saturating_add(1) < items.len();
        let mut item_lines = item.lines().map(str::to_owned).collect::<Vec<_>>();
        if let Some(first_line) = item_lines.first_mut() {
            *first_line = format!("{item_indent}{first_line}");
        }
        let is_multiline = item_lines.len() > 1;
        lines.extend(item_lines);
        if has_comma {
            if is_multiline {
                lines.push(format!("{item_indent},"));
            } else if let Some(last_line) = lines.last_mut() {
                last_line.push(',');
            }
        }
    }
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
