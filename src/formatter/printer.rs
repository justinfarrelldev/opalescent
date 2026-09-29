//! AST pretty-printer for the Opalescent code formatter.
//!
//! The [`Formatter`] struct traverses an [`ast::Program`] and produces
//! consistently-styled source code.  It is intentionally idempotent: parsing
//! the output and re-formatting it produces identical output.
//!
//! Formatting is performed by converting the AST back to source text using a
//! configurable [`FormatterConfig`].  The textual rules from
//! [`crate::formatter::rules`] are then applied as a post-processing step.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{
    BorrowKind, Decl, Expr, LambdaBody, MatchArm, Program, Stmt, StringPart, TypeDef, UnaryOp,
    Variant, Visibility,
};
use crate::formatter::config::FormatterConfig;
use crate::formatter::errors::{FormatterError, FormatterResult};
use crate::formatter::printer_helpers::{
    escape_single_quoted_string, format_array_literal, format_call_expression,
    format_function_signature, format_import_lines, print_binary_op, print_declaration_annotation,
    print_function_modifier_prefix, print_literal, print_pattern, print_type,
    print_type_declaration_form, print_unary_op,
};
use crate::formatter::rules;
use crate::lexer::Lexer;
use crate::parser::Parser;

mod statements;
use statements::FormatterStatementPrinter;

// ─── Free functions (no `self`) ──────────────────────────────────────────────

/// Normalise line endings and indentation so the source can be safely lexed.
///
/// This pre-pass performs two transformations before the source is handed to
/// the lexer:
///
/// 1. **CRLF → LF**: Windows-style `\r\n` line endings (and any bare `\r`
///    characters) are replaced with plain `\n`.  Without this step a `\r`
///    that appears immediately before a closing quote is treated as an
///    unexpected character by the lexer, producing spurious parse errors.
///
/// 2. **Leading tabs → spaces**: Every tab character that appears in the
///    leading whitespace of a line is replaced with `indent_size` spaces.
///    This allows files that use tab indentation (or mix tabs and spaces) to
///    pass through the formatter without triggering a `MixedWhitespace` lex
///    error.  Tabs that appear *after* the first non-whitespace character
///    (e.g. inside string literals) are left untouched.
fn normalize_indentation(source: &str, indent_size: usize) -> String {
    let cr_stripped: alloc::borrow::Cow<'_, str> = if source.contains('\r') {
        alloc::borrow::Cow::Owned(source.replace('\r', ""))
    } else {
        alloc::borrow::Cow::Borrowed(source)
    };
    let lf_source = cr_stripped.as_ref();

    let indent_spaces = " ".repeat(indent_size);
    let mut normalized = String::with_capacity(lf_source.len());

    for line in lf_source.split_inclusive('\n') {
        let (line_content, line_ending) = line
            .strip_suffix('\n')
            .map_or((line, ""), |content| (content, "\n"));

        let first_non_whitespace = line_content
            .char_indices()
            .find(|&(_, ch)| !ch.is_whitespace())
            .map_or(line_content.len(), |(idx, _)| idx);

        let (leading_whitespace, rest) = line_content.split_at(first_non_whitespace);
        for ch in leading_whitespace.chars() {
            if ch == '\t' {
                normalized.push_str(&indent_spaces);
            } else {
                normalized.push(ch);
            }
        }

        normalized.push_str(rest);
        normalized.push_str(line_ending);
    }

    normalized
}

// ─── Formatter struct ─────────────────────────────────────────────────────────

/// Idempotent pretty-printer for Opalescent source code.
///
/// # Usage
///
/// ```ignore
/// let formatter = Formatter::new(FormatterConfig::default());
/// let output = formatter.format_source("entry main = f(): unit => return")?;
/// ```
pub struct Formatter {
    /// Configuration controlling indentation width, line width, and tab usage.
    config: FormatterConfig,
}

impl Formatter {
    /// Create a new [`Formatter`] with the given configuration.
    #[must_use]
    pub const fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Create a new [`Formatter`] with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(FormatterConfig::default())
    }

    /// Parse `source`, pretty-print the resulting AST, and apply textual
    /// formatting rules.
    ///
    /// # Errors
    ///
    /// Returns [`FormatterError::ParseError`] when the source fails to parse.
    pub fn format_source(&self, source: &str) -> FormatterResult<String> {
        let normalized = normalize_indentation(source, self.config.indent_size);
        let lexer = Lexer::new(&normalized);
        let (tokens, lex_errors) = lexer.tokenize();

        if !lex_errors.errors.is_empty() {
            let msgs: Vec<String> = lex_errors.errors.iter().map(|e| format!("{e:?}")).collect();
            return Err(FormatterError::ParseError(msgs.join("; ")));
        }

        let parser = Parser::new(tokens);
        let (program_opt, parse_errors) = parser.parse();

        if !parse_errors.errors.is_empty() {
            let msgs: Vec<String> = parse_errors
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect();
            return Err(FormatterError::ParseError(msgs.join("; ")));
        }

        let Some(program) = program_opt else {
            return Err(FormatterError::ParseError(
                "no program produced by parser".to_owned(),
            ));
        };

        let raw = self.print_program(&program);
        Ok(rules::apply_all(&raw))
    }

    /// Return the indent string for `depth` levels of indentation.
    fn indent(&self, depth: usize) -> String {
        self.config.indent_unit().repeat(depth)
    }

    /// Pretty-print a complete [`Program`].
    fn print_program(&self, program: &Program) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (idx, decl) in program.declarations.iter().enumerate() {
            if idx > 0 {
                let prev = &program.declarations[idx.saturating_sub(1)];
                let prev_is_comment = matches!(*prev, Decl::Comment { .. });
                let current_is_comment = matches!(*decl, Decl::Comment { .. });

                if (prev_is_comment && current_is_comment)
                    || (matches!(*prev, Decl::Import { .. })
                        && matches!(*decl, Decl::Import { .. }))
                {
                    parts.push(String::from("\n"));
                } else {
                    parts.push(String::from("\n\n"));
                }
            }

            parts.push(self.print_decl(decl, 0));
        }
        parts.concat()
    }

    /// Pretty-print a declaration at the given indent `depth`.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Decl variants"
    )]
    fn print_decl(&self, decl: &Decl, depth: usize) -> String {
        match *decl {
            Decl::Function {
                ref name,
                ref parameters,
                ref return_types,
                ref error_types,
                ref body,
                ref visibility,
                ref is_entry,
                ref modifiers,
                ref doc_comment,
                ref metadata,
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let entry = if *is_entry { "entry " } else { "" };
                let modifier_prefix = print_function_modifier_prefix(modifiers, *is_entry);
                let params: Vec<String> = parameters
                    .iter()
                    .map(crate::ast::Parameter::to_signature_string)
                    .collect();
                let returns = match *return_types {
                    Some(ref types) if !types.is_empty() => {
                        let ret_strs: Vec<String> = types
                            .iter()
                            .enumerate()
                            .map(|(index, ty)| {
                                metadata.return_labels.get(index).map_or_else(
                                    || print_type(ty),
                                    |label| format!("{label}: {}", print_type(ty)),
                                )
                            })
                            .collect();
                        format!(": {}", ret_strs.join(", "))
                    }
                    _ => String::new(),
                };
                let signature_prefix = format!(
                    "{indent}{vis}{modifier_prefix}{entry}{name} = ",
                    indent = self.indent(depth)
                );
                let signature = format_function_signature(
                    &signature_prefix,
                    &params,
                    &returns,
                    error_types,
                    depth,
                    &self.config.indent_unit(),
                    self.config.max_line_width,
                );
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                let decl_str = format!("{signature}\n{body_str}");
                if let Some(ref doc) = *doc_comment {
                    let doc_lines: Vec<String> = doc
                        .raw
                        .lines()
                        .map(|line| format!("{}{}", self.indent(depth), line))
                        .collect();
                    format!(
                        "{}##\n{}\n{}##\n{}",
                        self.indent(depth),
                        doc_lines.join("\n"),
                        self.indent(depth),
                        decl_str
                    )
                } else {
                    decl_str
                }
            }
            Decl::Type {
                ref name,
                ref type_def,
                ref annotations,
                ref form,
                ref visibility,
                ref doc_comment,
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let form_prefix = print_type_declaration_form(*form);
                let body = self.print_type_def(type_def, depth);
                let decl_str = if matches!(*type_def, TypeDef::Opaque { .. }) {
                    format!("{}{}{form_prefix}type {name}", self.indent(depth), vis)
                } else {
                    format!(
                        "{}{}{form_prefix}type {name}:{body}",
                        self.indent(depth),
                        vis
                    )
                };
                let annotation_lines = annotations
                    .iter()
                    .map(|annotation| {
                        format!(
                            "{}{}",
                            self.indent(depth),
                            print_declaration_annotation(annotation)
                        )
                    })
                    .collect::<Vec<_>>();
                let decl_with_annotations = if annotation_lines.is_empty() {
                    decl_str
                } else {
                    format!("{}\n{decl_str}", annotation_lines.join("\n"))
                };
                if let Some(ref doc) = *doc_comment {
                    let doc_lines: Vec<String> = doc
                        .raw
                        .lines()
                        .map(|line| format!("{}{}", self.indent(depth), line))
                        .collect();
                    format!(
                        "{}##\n{}\n{}##\n{}",
                        self.indent(depth),
                        doc_lines.join("\n"),
                        self.indent(depth),
                        decl_with_annotations
                    )
                } else {
                    decl_with_annotations
                }
            }
            Decl::ErrorSet {
                ref name,
                ref members,
                ref visibility,
                ref doc_comment,
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let member_lines = members
                    .iter()
                    .map(|member| {
                        format!("{}{},", self.indent(depth.saturating_add(1)), member.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let decl_str = format!(
                    "{}{}error set {name} =\n{member_lines}",
                    self.indent(depth),
                    vis
                );
                if let Some(ref doc) = *doc_comment {
                    let doc_lines: Vec<String> = doc
                        .raw
                        .lines()
                        .map(|line| format!("{}{}", self.indent(depth), line))
                        .collect();
                    format!(
                        "{}##\n{}\n{}##\n{}",
                        self.indent(depth),
                        doc_lines.join("\n"),
                        self.indent(depth),
                        decl_str
                    )
                } else {
                    decl_str
                }
            }
            Decl::Import {
                ref items,
                ref source,
                ..
            } => {
                let all_type_imports = items
                    .iter()
                    .all(|item| matches!(*item, crate::ast::ImportItem::Type { .. }));
                let items_str: Vec<String> = items
                    .iter()
                    .map(|item| match *item {
                        crate::ast::ImportItem::Named {
                            ref name,
                            ref alias,
                            ..
                        } => alias
                            .as_ref()
                            .map_or_else(|| name.clone(), |a| format!("{name} as {a}")),
                        crate::ast::ImportItem::Glob { .. } => String::from("*"),
                        crate::ast::ImportItem::Type {
                            ref name,
                            ref alias,
                            ..
                        } => alias
                            .as_ref()
                            .map_or_else(|| name.clone(), |a| format!("{name} as {a}")),
                    })
                    .collect();
                let type_prefix = if all_type_imports { "type " } else { "" };
                format_import_lines(
                    &self.indent(depth),
                    type_prefix,
                    &items_str,
                    source,
                    self.config.max_line_width,
                )
            }
            Decl::Namespace { ref path, .. } => {
                format!("{}namespace {}", self.indent(depth), path.join("."))
            }
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ref doc_comment,
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let mutable = if binding.is_mutable { "mutable " } else { "" };
                let type_ann = binding
                    .type_annotation
                    .as_ref()
                    .map_or_else(String::new, |ta| format!(": {}", print_type(ta)));
                let prefix = format!(
                    "{}{}let {mutable}{}{type_ann} = ",
                    self.indent(depth),
                    vis,
                    binding.name
                );
                let decl_str = self
                    .print_lambda_with_prefix(&prefix, initializer, depth)
                    .map_or_else(
                        || self.format_prefixed_expression(&prefix, initializer, depth, true),
                        |lambda| lambda,
                    );
                if let Some(ref doc) = *doc_comment {
                    let doc_lines: Vec<String> = doc
                        .raw
                        .lines()
                        .map(|line| format!("{}{}", self.indent(depth), line))
                        .collect();
                    format!(
                        "{}##\n{}\n{}##\n{}",
                        self.indent(depth),
                        doc_lines.join("\n"),
                        self.indent(depth),
                        decl_str
                    )
                } else {
                    decl_str
                }
            }
            Decl::Comment { ref text, .. } => {
                format!("{}{}", self.indent(depth), text)
            }
        }
    }

    /// Pretty-print a type definition body (the part after the colon in `type Name: body`).
    fn print_type_def(&self, type_def: &TypeDef, depth: usize) -> String {
        match *type_def {
            TypeDef::Alias {
                ref target_type,
                ref constraint,
                ..
            } => {
                let where_clause = constraint.as_ref().map_or_else(String::new, |predicate| {
                    format!(" where {}", predicate.expression)
                });
                format!(" {}{where_clause}", print_type(target_type))
            }
            TypeDef::Opaque { .. } => String::new(),
            TypeDef::Sum { ref variants, .. } => {
                let variant_strs: Vec<String> = variants
                    .iter()
                    .map(|v| self.print_variant(v, depth.saturating_add(1)))
                    .collect();
                if variant_strs.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", variant_strs.join("\n"))
                }
            }
            TypeDef::Product { ref fields, .. } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}{}: {}",
                            self.indent(depth.saturating_add(1)),
                            f.name,
                            print_type(&f.type_annotation)
                        )
                    })
                    .collect();
                if field_strs.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", field_strs.join("\n"))
                }
            }
        }
    }

    /// Pretty-print a variant.
    fn print_variant(&self, variant: &Variant, depth: usize) -> String {
        let explicit_id = variant
            .explicit_id
            .map_or_else(String::new, |id| format!(" = {id}"));
        if variant.fields.is_empty() {
            return format!("{}{}{}", self.indent(depth), variant.name, explicit_id);
        }

        let mut lines = vec![format!(
            "{}{}{}:",
            self.indent(depth),
            variant.name,
            explicit_id
        )];
        lines.extend(variant.fields.iter().map(|field| {
            format!(
                "{}{}: {}",
                self.indent(depth.saturating_add(1)),
                field.name,
                print_type(&field.type_annotation)
            )
        }));
        lines.join("\n")
    }

    /// Pretty-print a lambda expression using a caller-provided prefix before `f`.
    fn print_lambda_with_prefix(
        &self,
        prefix_before_f: &str,
        expr: &Expr,
        depth: usize,
    ) -> Option<String> {
        let Expr::Lambda {
            ref params,
            ref return_types,
            ref body,
            ref error_types,
            ref metadata,
            ..
        } = *expr
        else {
            return None;
        };

        let params_str: Vec<String> = params
            .iter()
            .map(crate::ast::Parameter::to_signature_string)
            .collect();
        let ret_strs: Vec<String> = return_types
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                metadata.return_labels.get(index).map_or_else(
                    || print_type(ty),
                    |label| format!("{label}: {}", print_type(ty)),
                )
            })
            .collect();
        let returns = format!(": {}", ret_strs.join(", "));
        let signature = format_function_signature(
            prefix_before_f,
            &params_str,
            &returns,
            error_types,
            depth,
            &self.config.indent_unit(),
            self.config.max_line_width,
        );
        Some(match *body {
            LambdaBody::Expression(ref e) => format!("{signature} {}", self.print_expr(e, depth)),
            LambdaBody::Block(ref statements) => {
                let body_str = statements
                    .iter()
                    .map(|statement| self.print_stmt(statement, depth.saturating_add(1)))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{signature}\n{body_str}")
            }
        })
    }

    /// Pretty-print an expression at the given indent `depth`.
    fn print_expr(&self, expr: &Expr, depth: usize) -> String {
        self.print_expr_with_prefix_width(expr, depth, 0)
    }

    /// Pretty-print an expression with the width already occupied on its first line.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Expr variants"
    )]
    fn print_expr_with_prefix_width(
        &self,
        expr: &Expr,
        depth: usize,
        prefix_width: usize,
    ) -> String {
        match *expr {
            Expr::Literal { ref value, .. } => print_literal(value),
            Expr::Identifier { ref name, .. } => name.clone(),
            Expr::Binary {
                ref left,
                ref operator,
                ref right,
                ..
            } => {
                let l = self.print_expr(left, depth);
                let op = print_binary_op(operator);
                let r = self.print_expr(right, depth);
                format!("{l} {op} {r}")
            }
            Expr::Unary {
                ref operator,
                ref operand,
                ..
            } => {
                let op = print_unary_op(operator);
                let operand_str = self.print_expr(operand, depth);
                match *operator {
                    UnaryOp::Not | UnaryOp::BitNot => {
                        format!("{op} {operand_str}")
                    }
                    UnaryOp::Negate | UnaryOp::Plus => format!("{op}{operand_str}"),
                }
            }
            Expr::Call {
                ref callee,
                ref args,
                ref generic_args,
                ..
            } => {
                let callee_str = self.print_expr(callee, depth);
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.print_expr_with_prefix_width(a, depth.saturating_add(1), 0))
                    .collect();
                let generics = generic_args.as_ref().map_or_else(String::new, |ga| {
                    let g: Vec<String> = ga.iter().map(print_type).collect();
                    format!("::<{}>", g.join(", "))
                });
                format_call_expression(
                    &callee_str,
                    &generics,
                    &args_str,
                    depth,
                    &self.config.indent_unit(),
                    self.config.max_line_width,
                    prefix_width,
                )
            }
            Expr::Constructor {
                ref callee,
                ref fields,
                ..
            } => {
                let callee_str = self.print_expr(callee, depth);
                if fields.is_empty() {
                    return format!("new {callee_str}");
                }

                // Fields always live in an indented block one level past the
                // statement that owns the expression. Using `depth + 1` keeps
                // nested constructors visually aligned with the surrounding
                // block grammar (`let`, `return`, etc.).
                let field_indent = self.config.indent_unit().repeat(depth.saturating_add(1));
                let mut out = format!("new {callee_str}:");
                for field in fields {
                    let value_prefix_width = field_indent
                        .chars()
                        .count()
                        .saturating_add(field.name.chars().count())
                        .saturating_add(": ".chars().count());
                    let value_str = self.print_expr_with_prefix_width(
                        &field.value,
                        depth.saturating_add(1),
                        value_prefix_width,
                    );
                    out.push('\n');
                    out.push_str(&field_indent);
                    out.push_str(&field.name);
                    out.push_str(": ");
                    out.push_str(&value_str);
                }
                out
            }
            Expr::RecordUpdate {
                ref receiver,
                ref fields,
                ..
            } => {
                let receiver_str = self.print_expr(receiver, depth);
                let field_indent = self.config.indent_unit().repeat(depth.saturating_add(1));
                let mut out = format!("{receiver_str} with:");
                for field in fields {
                    let value_prefix_width = field_indent
                        .chars()
                        .count()
                        .saturating_add(field.name.chars().count())
                        .saturating_add(": ".chars().count());
                    let value_str = self.print_expr_with_prefix_width(
                        &field.value,
                        depth.saturating_add(1),
                        value_prefix_width,
                    );
                    out.push('\n');
                    out.push_str(&field_indent);
                    out.push_str(&field.name);
                    out.push_str(": ");
                    out.push_str(&value_str);
                }
                out
            }
            Expr::Index {
                ref object,
                ref index,
                ..
            } => {
                let obj = self.print_expr(object, depth);
                let idx = self.print_expr(index, depth);
                format!("{obj}[{idx}]")
            }
            Expr::Member {
                ref object,
                ref member,
                ..
            } => {
                let obj = self.print_expr(object, depth);
                format!("{obj}.{member}")
            }
            Expr::BorrowArgument {
                ref target,
                ref borrow_kind,
                ..
            } => {
                let target_str = self.print_expr(target, depth);
                match *borrow_kind {
                    BorrowKind::Owned => target_str,
                    BorrowKind::Ref => format!("ref {target_str}"),
                    BorrowKind::MutableRef => format!("mutable ref {target_str}"),
                }
            }
            Expr::Cast {
                ref expr,
                ref target_type,
                ..
            } => {
                let inner = self.print_expr(expr, depth);
                let ty = print_type(target_type);
                format!("{inner} as {ty}")
            }
            Expr::Constrain {
                ref target_type,
                ref value,
                ..
            } => {
                let ty = print_type(target_type);
                let value_str = self.print_expr(value, depth);
                format!("constrain {ty} from {value_str}")
            }
            Expr::Refinement {
                ref value,
                ref variant,
                ref payload_binding,
                ..
            } => {
                let value_str = self.print_expr(value, depth);
                let variant_str = self.print_expr(variant, depth);
                format!("{value_str} is {variant_str} into {payload_binding}")
            }
            Expr::TypeOf { ref expr, .. } => {
                let inner = self.print_expr(expr, depth);
                format!("type_of({inner})")
            }
            Expr::StringInterpolation { ref parts, .. } => {
                let mut s = String::from("'");
                for part in parts {
                    match *part {
                        StringPart::Literal(ref lit) => {
                            s.push_str(&escape_single_quoted_string(lit));
                        }
                        StringPart::Expression(ref e) => {
                            s.push('{');
                            s.push_str(&self.print_expr(e, depth));
                            s.push('}');
                        }
                    }
                }
                s.push('\'');
                s
            }
            Expr::Parenthesized { ref expr, .. } => {
                format!("({})", self.print_expr(expr, depth))
            }
            Expr::Array { ref elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|element| self.print_expr(element, depth.saturating_add(1)))
                    .collect();
                format_array_literal(
                    &elems,
                    depth,
                    &self.config.indent_unit(),
                    self.config.max_line_width,
                    prefix_width,
                )
            }
            Expr::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                let cond = self.print_expr(condition, depth);
                let then = self.print_stmt(then_branch, depth);
                let else_part = else_branch.as_ref().map_or_else(String::new, |eb| {
                    format!(" else {}", self.print_stmt(eb, depth))
                });
                format!("if {cond} {then}{else_part}")
            }
            Expr::Match {
                ref scrutinee,
                ref arms,
                ..
            } => self.print_match_expr(scrutinee, arms, depth),
            Expr::Loop { ref body, .. } => {
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                format!("loop =>\n{body_str}")
            }
            Expr::Lambda { .. } => self
                .print_lambda_with_prefix("", expr, depth)
                .expect("lambda expression arm should format lambda"),
            Expr::Guard {
                ref expr,
                ref binding_name,
                ref binding_type,
                ref is_mutable,
                ref else_branch,
                ..
            } => {
                let guard_prefix_width = prefix_width.saturating_add("guard ".chars().count());
                let inner = self.print_expr_with_prefix_width(expr, depth, guard_prefix_width);
                let mutable = if *is_mutable { "mutable " } else { "" };
                let ty = binding_type
                    .as_ref()
                    .map_or_else(String::new, |t| format!(": {}", print_type(t)));
                let header = format!("guard {inner} into {mutable}{binding_name}{ty} else");
                let inline_else = if let Stmt::Expression {
                    expr: ref else_expr,
                    ..
                } = **else_branch
                {
                    self.print_expr_with_prefix_width(
                        else_expr,
                        depth,
                        prefix_width
                            .saturating_add(header.chars().count())
                            .saturating_add(1),
                    )
                } else {
                    self.print_stmt(else_branch, depth)
                };
                let inline = format!("{header} {inline_else}");
                if !inline.contains('\n')
                    && prefix_width.saturating_add(inline.chars().count())
                        <= self.config.max_line_width
                {
                    inline
                } else {
                    format!(
                        "{header}\n{}",
                        self.print_stmt(else_branch, depth.saturating_add(1))
                    )
                }
            }
            Expr::Propagate {
                ref call,
                ref cause,
                ..
            } => {
                let propagate_width = prefix_width.saturating_add("propagate ".chars().count());
                let inner = self.print_expr_with_prefix_width(call, depth, propagate_width);
                cause.as_ref().map_or_else(
                    || format!("propagate {inner}"),
                    |cause_expr| {
                        let cause_prefix = propagate_width
                            .saturating_add(
                                inner.lines().last().map_or(0, |line| line.chars().count()),
                            )
                            .saturating_add(" cause ".chars().count());
                        let cause_str =
                            self.print_expr_with_prefix_width(cause_expr, depth, cause_prefix);
                        format!("propagate {inner} cause {cause_str}")
                    },
                )
            }
        }
    }

    /// Pretty-print a match expression.
    fn print_match_expr(&self, scrutinee: &Expr, arms: &[MatchArm], depth: usize) -> String {
        let scrut = self.print_expr(scrutinee, depth);
        let arm_strs: Vec<String> = arms
            .iter()
            .map(|arm| {
                let pat = print_pattern(&arm.pattern);
                let body = self.print_expr(&arm.body, depth.saturating_add(1));
                format!("{pat} => {body}")
            })
            .collect();
        format!("match {scrut} {{ {} }}", arm_strs.join(", "))
    }
}
