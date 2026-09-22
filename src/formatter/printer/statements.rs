//! Statement and block rendering for the formatter printer.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{Expr, LabeledValue, Stmt};
use crate::formatter::printer::Formatter;
use crate::formatter::printer_helpers::{format_guard_header, print_type};

/// Extension methods that render statements and block bodies for [`Formatter`].
pub(super) trait FormatterStatementPrinter {
    /// Print the body statements of a block at the given indent depth.
    fn print_block_body_indented(&self, block: &Stmt, depth: usize) -> String;

    /// Pretty-print a statement at the given indent `depth`.
    fn print_stmt(&self, stmt: &Stmt, depth: usize) -> String;

    /// Pretty-print a return/break/continue statement with optional labeled values.
    fn print_labeled_values(&self, keyword: &str, values: &[LabeledValue], depth: usize) -> String;

    /// Format an expression after a fixed statement prefix.
    fn format_prefixed_expression(
        &self,
        prefix: &str,
        expr: &Expr,
        depth: usize,
        allow_break_after_prefix: bool,
    ) -> String;
}

impl FormatterStatementPrinter for Formatter {
    /// Print the body statements of a block at the given indent depth.
    ///
    /// This helper is used by control flow statements (if/while/for/loop) to
    /// print block contents WITHOUT surrounding braces. The caller is responsible
    /// for emitting the header line (e.g., `if cond:` or `loop =>`).
    ///
    /// # Language Spec Compliance
    /// Control flow uses colon-block syntax per the Opalescent language spec.
    fn print_block_body_indented(&self, block: &Stmt, depth: usize) -> String {
        if let Stmt::Block { ref statements, .. } = *block {
            if statements.is_empty() {
                // Per language spec: empty block uses comment placeholder
                return format!("{}# empty", self.indent(depth));
            }
            statements
                .iter()
                .map(|s| self.print_stmt(s, depth))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            // Non-block body: print as single statement
            self.print_stmt(block, depth)
        }
    }

    /// Pretty-print a statement at the given indent `depth`.
    ///
    /// # Language Spec Compliance
    ///
    /// This method outputs control flow statements using Opalescent's colon-block
    /// syntax per the language specification:
    /// - `if condition:` followed by indented body (no braces)
    /// - `while condition:` followed by indented body (no braces)
    /// - `for var in iter:` followed by indented body (no braces)
    /// - `loop =>` followed by indented body (no braces)
    ///
    /// See `language-spec/*.op` for canonical examples.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match over all Stmt variants"
    )]
    fn print_stmt(&self, stmt: &Stmt, depth: usize) -> String {
        let indent = self.indent(depth);
        match *stmt {
            Stmt::Block { ref statements, .. } => {
                if statements.is_empty() {
                    return format!("{indent}# empty");
                }
                statements
                    .iter()
                    .map(|statement| self.print_stmt(statement, depth))
                    .collect::<Vec<_>>()
                    .join("\n")
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
                let prefix = format!("{indent}let {mutable}{}{type_ann}", binding.name);
                initializer.as_ref().map_or_else(
                    || prefix.clone(),
                    |i| {
                        let initializer_prefix = format!("{prefix} = ");
                        self.print_lambda_with_prefix(&initializer_prefix, i, depth)
                            .map_or_else(
                                || {
                                    self.format_prefixed_expression(
                                        &initializer_prefix,
                                        i,
                                        depth,
                                        true,
                                    )
                                },
                                |lambda| lambda,
                            )
                    },
                )
            }
            Stmt::LetDestructure {
                ref bindings,
                ref initializer,
                ..
            } => {
                let names: Vec<String> = bindings
                    .iter()
                    .map(|binding| {
                        binding.returned_label.as_ref().map_or_else(
                            || binding.name.clone(),
                            |returned_label| format!("{returned_label}: {}", binding.name),
                        )
                    })
                    .collect();
                let prefix = format!("{indent}let {} = ", names.join(", "));
                self.format_prefixed_expression(&prefix, initializer, depth, true)
            }
            Stmt::Assignment {
                ref target,
                ref value,
                ..
            } => {
                let prefix = format!("{indent}{} = ", self.print_expr(target, depth));
                self.format_prefixed_expression(&prefix, value, depth, true)
            }
            Stmt::Return { ref values, .. } => self.print_labeled_values("return", values, depth),
            Stmt::Expression { ref expr, .. } => {
                self.format_prefixed_expression(&indent, expr, depth, false)
            }
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                // Per language spec: colon-block syntax
                let cond_str = self.print_expr_with_prefix_width(
                    condition,
                    depth,
                    indent.chars().count().saturating_add("if ".chars().count()),
                );
                let body_str = self.print_block_body_indented(then_branch, depth.saturating_add(1));
                let else_str = else_branch.as_ref().map_or_else(String::new, |eb| {
                    let else_body = self.print_block_body_indented(eb, depth.saturating_add(1));
                    format!("\n{indent}else:\n{else_body}")
                });
                format!("{indent}if {cond_str}:\n{body_str}{else_str}")
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                ..
            } => {
                // Per language spec: colon-block syntax
                let iter_prefix_width = indent
                    .chars()
                    .count()
                    .saturating_add("for ".chars().count())
                    .saturating_add(variable.chars().count())
                    .saturating_add(" in ".chars().count());
                let iter_str =
                    self.print_expr_with_prefix_width(iterable, depth, iter_prefix_width);
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                format!("{indent}for {variable} in {iter_str}:\n{body_str}")
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                // Per language spec: colon-block syntax
                let cond_str = self.print_expr_with_prefix_width(
                    condition,
                    depth,
                    indent
                        .chars()
                        .count()
                        .saturating_add("while ".chars().count()),
                );
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                format!("{indent}while {cond_str}:\n{body_str}")
            }
            Stmt::Guard {
                ref expression,
                ref success_binding,
                ref success_binding_type,
                ref success_binding_is_mutable,
                ref success_bindings,
                ref error_binding,
                ref else_body,
                ..
            } => {
                let expression_str = self.print_expr_with_prefix_width(
                    expression,
                    depth,
                    indent
                        .chars()
                        .count()
                        .saturating_add("guard ".chars().count()),
                );
                let guard_header = format_guard_header(
                    &indent,
                    &expression_str,
                    success_binding.as_ref(),
                    success_binding_type.as_ref(),
                    *success_binding_is_mutable,
                    success_bindings,
                    error_binding,
                );
                if let Stmt::Block { ref statements, .. } = **else_body {
                    let mut lines = vec![guard_header];
                    lines.extend(
                        statements
                            .iter()
                            .map(|statement| self.print_stmt(statement, depth.saturating_add(1))),
                    );
                    lines.join("\n")
                } else {
                    let else_body_str = self.print_stmt(else_body, depth.saturating_add(1));
                    format!("{guard_header}\n{else_body_str}")
                }
            }
            Stmt::PropagateGuardError {
                ref error_binding, ..
            } => {
                format!("{indent}propagate {error_binding}")
            }
            Stmt::Loop { ref body, .. } => {
                // Per language spec: colon-block syntax
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                format!("{indent}loop =>\n{body_str}")
            }
            Stmt::Using {
                ref binding,
                ref acquisition,
                ref body,
                ..
            } => {
                let acquisition_prefix_width = indent
                    .chars()
                    .count()
                    .saturating_add("using ".chars().count())
                    .saturating_add(binding.name.chars().count())
                    .saturating_add(" = ".chars().count());
                let acquisition_str =
                    self.print_expr_with_prefix_width(acquisition, depth, acquisition_prefix_width);
                let body_str = self.print_block_body_indented(body, depth.saturating_add(1));
                format!(
                    "{indent}using {} = {acquisition_str}:\n{body_str}",
                    binding.name
                )
            }
            Stmt::Break { ref values, .. } => self.print_labeled_values("break", values, depth),
            Stmt::Continue { ref values, .. } => {
                self.print_labeled_values("continue", values, depth)
            }
            Stmt::Comment { ref text, .. } => {
                format!("{indent}{text}")
            }
        }
    }

    /// Pretty-print a return/break/continue statement with optional labeled values.
    fn print_labeled_values(&self, keyword: &str, values: &[LabeledValue], depth: usize) -> String {
        let indent = self.indent(depth);
        if values.is_empty() {
            return format!("{indent}{keyword}");
        }

        let flat_parts = values
            .iter()
            .map(|value| {
                if value.label.is_empty() {
                    self.print_expr(&value.value, depth)
                } else {
                    format!("{}: {}", value.label, self.print_expr(&value.value, depth))
                }
            })
            .collect::<Vec<_>>();
        let flat = format!("{indent}{keyword} {}", flat_parts.join(", "));
        if flat
            .lines()
            .all(|line| line.chars().count() <= self.config.max_line_width)
        {
            return flat;
        }

        let label_indent = format!(
            "{indent}{}",
            " ".repeat(keyword.chars().count().saturating_add(1))
        );
        let mut lines = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            let item_prefix = if index == 0 {
                format!("{indent}{keyword} ")
            } else {
                label_indent.clone()
            };
            let label_prefix = if value.label.is_empty() {
                String::new()
            } else {
                format!("{}: ", value.label)
            };
            let expr_prefix_width = item_prefix
                .chars()
                .count()
                .saturating_add(label_prefix.chars().count());
            let expr = self.print_expr_with_prefix_width(
                &value.value,
                depth.saturating_add(1),
                expr_prefix_width,
            );
            let mut item_lines = format!("{item_prefix}{label_prefix}{expr}")
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if index.saturating_add(1) < values.len() {
                if let Some(last_line) = item_lines.last_mut() {
                    last_line.push(',');
                }
            }
            lines.extend(item_lines);
        }
        lines.join("\n")
    }

    /// Format an expression after a fixed statement prefix.
    fn format_prefixed_expression(
        &self,
        prefix: &str,
        expr: &Expr,
        depth: usize,
        allow_break_after_prefix: bool,
    ) -> String {
        let expr_str = self.print_expr_with_prefix_width(expr, depth, prefix.chars().count());
        let candidate = format!("{prefix}{expr_str}");
        let first_line_width = candidate
            .lines()
            .next()
            .map_or(0, |line| line.chars().count());
        if allow_break_after_prefix && first_line_width > self.config.max_line_width {
            let continuation_indent = self.indent(depth.saturating_add(1));
            let continued_expr = self.print_expr_with_prefix_width(
                expr,
                depth.saturating_add(1),
                continuation_indent.chars().count(),
            );
            format!(
                "{}\n{continuation_indent}{continued_expr}",
                prefix.trim_end()
            )
        } else {
            candidate
        }
    }
}
