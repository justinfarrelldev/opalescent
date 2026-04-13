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
    BinaryOp, ConstructorField, Decl, Expr, LambdaBody, LiteralValue, MatchArm, Pattern, Program,
    Stmt, StringPart, Type, TypeDef, UnaryOp, Variant, Visibility,
};
use crate::formatter::config::FormatterConfig;
use crate::formatter::errors::{FormatterError, FormatterResult};
use crate::formatter::rules;
use crate::lexer::Lexer;
use crate::parser::Parser;

// ─── Free functions (no `self`) ──────────────────────────────────────────────

/// Pretty-print a type annotation.
fn print_type(ty: &Type) -> String {
    match *ty {
        Type::Basic { ref name, .. } => name.clone(),
        Type::Array {
            ref element_type, ..
        } => {
            format!("{}[]", print_type(element_type))
        }
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
fn print_literal(lit: &LiteralValue) -> String {
    match *lit {
        LiteralValue::Integer(n) => format!("{n}"),
        LiteralValue::Float(f) => {
            let s = format!("{f}");
            // Ensure float literals always contain a decimal point.
            if s.contains('.') {
                s
            } else {
                format!("{s}.0")
            }
        }
        LiteralValue::String(ref s) => format!("\"{s}\""),
        LiteralValue::Boolean(b) => (if b { "true" } else { "false" }).to_owned(),
        LiteralValue::Void => String::from("void"),
    }
}

/// Pretty-print a binary operator.
const fn print_binary_op(op: &BinaryOp) -> &'static str {
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
const fn print_unary_op(op: &UnaryOp) -> &'static str {
    match *op {
        UnaryOp::Negate => "-",
        UnaryOp::Plus => "+",
        UnaryOp::Not => "not",
        UnaryOp::BitNot => "bnot",
    }
}

/// Pretty-print a pattern.
fn print_pattern(pattern: &Pattern) -> String {
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
                let fs: Vec<String> = fields
                    .iter()
                    .map(|pair| {
                        pair.0.as_ref().map_or_else(
                            || print_pattern(&pair.1),
                            |field_name| format!("{field_name}: {}", print_pattern(&pair.1)),
                        )
                    })
                    .collect();
                format!("{prefix}{variant_name}({})", fs.join(", "))
            }
        }
        Pattern::Tuple { ref elements, .. } => {
            let ps: Vec<String> = elements.iter().map(print_pattern).collect();
            format!("({})", ps.join(", "))
        }
    }
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
        let lexer = Lexer::new(source);
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
        for decl in &program.declarations {
            parts.push(self.print_decl(decl, 0));
        }
        parts.join("\n\n")
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
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let entry = if *is_entry { "entry " } else { "" };
                let params: Vec<String> = parameters
                    .iter()
                    .map(|p| format!("{}: {}", p.name, print_type(&p.param_type)))
                    .collect();
                let params_str = params.join(", ");
                let returns = match *return_types {
                    Some(ref types) if !types.is_empty() => {
                        let ret_strs: Vec<String> = types.iter().map(print_type).collect();
                        format!(": {}", ret_strs.join(", "))
                    }
                    _ => String::new(),
                };
                let errors = if error_types.is_empty() {
                    String::new()
                } else {
                    format!(" errors {}", error_types.join(", "))
                };
                let body_str = self.print_stmt(body, depth);
                format!(
                    "{indent}{vis}{entry}{name} = f({params_str}){returns}{errors} => {body_str}",
                    indent = self.indent(depth)
                )
            }
            Decl::Type {
                ref name,
                ref type_def,
                ref visibility,
                ..
            } => {
                let vis = if *visibility == Visibility::Public {
                    "public "
                } else {
                    ""
                };
                let body = self.print_type_def(type_def, depth);
                format!("{}{}type {name}:{body}", self.indent(depth), vis)
            }
            Decl::Import {
                ref items,
                ref source,
                ..
            } => {
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
                        } => alias.as_ref().map_or_else(
                            || format!("type {name}"),
                            |a| format!("type {name} as {a}"),
                        ),
                    })
                    .collect();
                format!(
                    "{}import {} from {source}",
                    self.indent(depth),
                    items_str.join(", ")
                )
            }
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
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
                let init_str = self.print_expr(initializer, depth);
                format!(
                    "{}{}let {mutable}{}{type_ann} = {init_str}",
                    self.indent(depth),
                    vis,
                    binding.name
                )
            }
        }
    }

    /// Pretty-print a type definition body (the part after the colon in `type Name: body`).
    fn print_type_def(&self, type_def: &TypeDef, depth: usize) -> String {
        match *type_def {
            TypeDef::Alias {
                ref target_type, ..
            } => format!(" {}", print_type(target_type)),
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
        let fields: Vec<String> = variant
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.name, print_type(&f.type_annotation)))
            .collect();
        if fields.is_empty() {
            format!("{}{}", self.indent(depth), variant.name)
        } else {
            format!(
                "{}{}({})",
                self.indent(depth),
                variant.name,
                fields.join(", ")
            )
        }
    }

    /// Pretty-print a statement at the given indent `depth`.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Stmt variants"
    )]
    fn print_stmt(&self, stmt: &Stmt, depth: usize) -> String {
        let indent = self.indent(depth);
        match *stmt {
            Stmt::Block { ref statements, .. } => {
                if statements.is_empty() {
                    return String::from("{}");
                }
                let mut lines: Vec<String> = Vec::new();
                lines.push(String::from("{"));
                for s in statements {
                    lines.push(self.print_stmt(s, depth.saturating_add(1)));
                }
                lines.push(format!("{indent}}}"));
                lines.join("\n")
            }
            Stmt::Let {
                ref binding,
                ref initializer,
                ..
            } => {
                let mutable = if binding.is_mutable { "mutable " } else { "" };
                let type_ann = binding
                    .type_annotation
                    .as_ref()
                    .map_or_else(String::new, |ta| format!(": {}", print_type(ta)));
                let init = initializer
                    .as_ref()
                    .map_or_else(String::new, |i| format!(" = {}", self.print_expr(i, depth)));
                format!("{indent}let {mutable}{}{type_ann}{init}", binding.name)
            }
            Stmt::LetDestructure {
                ref bindings,
                ref initializer,
                ..
            } => {
                let names: Vec<String> = bindings
                    .iter()
                    .map(|binding| binding.name.clone())
                    .collect();
                format!(
                    "{indent}let {} = {}",
                    names.join(", "),
                    self.print_expr(initializer, depth)
                )
            }
            Stmt::Assignment {
                ref target,
                ref value,
                ..
            } => {
                format!(
                    "{indent}{} = {}",
                    self.print_expr(target, depth),
                    self.print_expr(value, depth)
                )
            }
            Stmt::Return { ref values, .. } => {
                if values.is_empty() {
                    format!("{indent}return")
                } else if values.len() == 1 && values[0].label.is_empty() {
                    format!(
                        "{indent}return {}",
                        self.print_expr(&values[0].value, depth)
                    )
                } else {
                    let parts: Vec<String> = values
                        .iter()
                        .map(|lv| {
                            if lv.label.is_empty() {
                                self.print_expr(&lv.value, depth)
                            } else {
                                format!("{}: {}", lv.label, self.print_expr(&lv.value, depth))
                            }
                        })
                        .collect();
                    format!("{indent}return {}", parts.join(", "))
                }
            }
            Stmt::Expression { ref expr, .. } => {
                format!("{indent}{}", self.print_expr(expr, depth))
            }
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                let cond_str = self.print_expr(condition, depth);
                let then_str = self.print_stmt(then_branch, depth);
                let else_str = else_branch.as_ref().map_or_else(String::new, |eb| {
                    format!(" else {}", self.print_stmt(eb, depth))
                });
                format!("{indent}if {cond_str} {then_str}{else_str}")
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                ..
            } => {
                let iter_str = self.print_expr(iterable, depth);
                let body_str = self.print_stmt(body, depth);
                format!("{indent}for {variable} in {iter_str} {body_str}")
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                let cond_str = self.print_expr(condition, depth);
                let body_str = self.print_stmt(body, depth);
                format!("{indent}while {cond_str} {body_str}")
            }
            Stmt::Guard {
                ref expression,
                ref success_binding,
                ref error_binding,
                ref else_body,
                ..
            } => {
                let expression_str = self.print_expr(expression, depth);
                let else_body_str = self.print_stmt(else_body, depth);
                format!(
                    "{indent}guard {expression_str} into {success_binding} else {error_binding} => {else_body_str}"
                )
            }
            Stmt::Loop { ref body, .. } => {
                let body_str = self.print_stmt(body, depth);
                format!("{indent}loop {body_str}")
            }
            Stmt::Break { ref values, .. } => {
                if values.is_empty() {
                    format!("{indent}break")
                } else {
                    let parts: Vec<String> = values
                        .iter()
                        .map(|lv| {
                            if lv.label.is_empty() {
                                self.print_expr(&lv.value, depth)
                            } else {
                                format!("{}: {}", lv.label, self.print_expr(&lv.value, depth))
                            }
                        })
                        .collect();
                    format!("{indent}break {}", parts.join(", "))
                }
            }
            Stmt::Continue { ref values, .. } => {
                if values.is_empty() {
                    format!("{indent}continue")
                } else {
                    let parts: Vec<String> = values
                        .iter()
                        .map(|lv| {
                            if lv.label.is_empty() {
                                self.print_expr(&lv.value, depth)
                            } else {
                                format!("{}: {}", lv.label, self.print_expr(&lv.value, depth))
                            }
                        })
                        .collect();
                    format!("{indent}continue {}", parts.join(", "))
                }
            }
        }
    }

    /// Pretty-print an expression at the given indent `depth`.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Expr variants"
    )]
    fn print_expr(&self, expr: &Expr, depth: usize) -> String {
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
                let args_str: Vec<String> =
                    args.iter().map(|a| self.print_expr(a, depth)).collect();
                let generics = generic_args.as_ref().map_or_else(String::new, |ga| {
                    let g: Vec<String> = ga.iter().map(print_type).collect();
                    format!("::<{}>", g.join(", "))
                });
                format!("{callee_str}{generics}({})", args_str.join(", "))
            }
            Expr::Constructor {
                ref callee,
                ref fields,
                ..
            } => {
                let callee_str = self.print_expr(callee, depth);
                let fields_str = self.print_constructor_fields(fields, depth);
                format!("{callee_str} {{{fields_str}}}")
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
            Expr::Cast {
                ref expr,
                ref target_type,
                ..
            } => {
                let inner = self.print_expr(expr, depth);
                let ty = print_type(target_type);
                format!("{inner} as {ty}")
            }
            Expr::TypeOf { ref expr, .. } => {
                let inner = self.print_expr(expr, depth);
                format!("type_of({inner})")
            }
            Expr::StringInterpolation { ref parts, .. } => {
                let mut s = String::from("'");
                for part in parts {
                    match *part {
                        StringPart::Literal(ref lit) => s.push_str(lit),
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
                let elems: Vec<String> =
                    elements.iter().map(|e| self.print_expr(e, depth)).collect();
                format!("[{}]", elems.join(", "))
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
                let body_str = self.print_stmt(body, depth);
                format!("loop => {body_str}")
            }
            Expr::Lambda {
                ref params,
                ref return_types,
                ref body,
                ref error_types,
                ..
            } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, print_type(&p.param_type)))
                    .collect();
                let ret_strs: Vec<String> = return_types.iter().map(print_type).collect();
                let errors = if error_types.is_empty() {
                    String::new()
                } else {
                    format!(" errors {}", error_types.join(", "))
                };
                let body_str = match *body {
                    LambdaBody::Expression(ref e) => self.print_expr(e, depth),
                    LambdaBody::Block(ref stmts) => {
                        let inner: Vec<String> = stmts
                            .iter()
                            .map(|s| self.print_stmt(s, depth.saturating_add(1)))
                            .collect();
                        format!("{{\n{}\n{}}}", inner.join("\n"), self.indent(depth))
                    }
                };
                let ret = ret_strs.join(", ");
                format!("f({}): {ret}{errors} => {body_str}", params_str.join(", "))
            }
            Expr::Guard {
                ref expr,
                ref binding_name,
                ref binding_type,
                ref is_mutable,
                ref else_branch,
                ..
            } => {
                let inner = self.print_expr(expr, depth);
                let mutable = if *is_mutable { "mutable " } else { "" };
                let ty = binding_type
                    .as_ref()
                    .map_or_else(String::new, |t| format!(": {}", print_type(t)));
                let else_str = self.print_stmt(else_branch, depth);
                format!("guard {inner} into {mutable}{binding_name}{ty} else {else_str}")
            }
            Expr::Propagate { ref call, .. } => {
                let inner = self.print_expr(call, depth);
                format!("propagate {inner}")
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
                format!("{}{pat} => {body}", self.indent(depth.saturating_add(1)))
            })
            .collect();
        format!(
            "match {scrut} {{\n{}\n{}}}",
            arm_strs.join(",\n"),
            self.indent(depth)
        )
    }

    /// Pretty-print constructor fields.
    fn print_constructor_fields(&self, fields: &[ConstructorField], depth: usize) -> String {
        if fields.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = fields
            .iter()
            .map(|f| format!("{}: {}", f.name, self.print_expr(&f.value, depth)))
            .collect();
        format!(" {} ", parts.join(", "))
    }
}
