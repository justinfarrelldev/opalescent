//! Naming convention checker for the Opalescent code formatter.
//!
//! This module detects naming-convention violations in source code by inspecting
//! AST identifiers and type names:
//!
//! - Variable and function names should use `snake_case`.
//! - Type names should use `PascalCase`.
//!
//! Violations are returned as a `Vec<`[`NamingViolation`]`>` so the caller can
//! decide whether to error, warn, or auto-fix them.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{Decl, Expr, LambdaBody, Program, Stmt};

/// Describes the expected naming style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingStyle {
    /// Identifiers should be `snake_case`.
    SnakeCase,
    /// Identifiers should be `PascalCase`.
    PascalCase,
}

impl core::fmt::Display for NamingStyle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::SnakeCase => f.write_str("snake_case"),
            Self::PascalCase => f.write_str("PascalCase"),
        }
    }
}

/// A single naming-convention violation found in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingViolation {
    /// The identifier that violates the convention.
    pub name: String,
    /// The convention that the identifier should follow.
    pub expected: NamingStyle,
    /// A human-readable description of where the violation occurs.
    pub location: String,
}

impl core::fmt::Display for NamingViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "`{}` in {} should use {} naming",
            self.name, self.location, self.expected
        )
    }
}

/// Returns `true` when `name` conforms to `snake_case`.
///
/// A `snake_case` identifier consists of lowercase ASCII letters, ASCII digits,
/// and underscores, and does **not** contain uppercase letters.  Single and
/// double underscores are permitted for conventional uses (`_unused`,
/// `__reserved`).
#[must_use]
pub fn is_snake_case(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    name.chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

/// Returns `true` when `name` conforms to `PascalCase`.
///
/// A `PascalCase` identifier starts with an uppercase ASCII letter and does not
/// contain underscores.
#[must_use]
pub fn is_pascal_case(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    first.is_ascii_uppercase() && !name.contains('_')
}

/// Checks all declarations in a [`Program`] for naming-convention violations.
///
/// Returns a (possibly empty) vector of [`NamingViolation`]s.
#[must_use]
pub fn check_program(program: &Program) -> Vec<NamingViolation> {
    let mut violations = Vec::new();
    for decl in &program.declarations {
        check_decl(decl, &mut violations);
    }
    violations
}

/// Checks a single declaration for naming violations, appending any found to
/// `violations`.
fn check_decl(decl: &Decl, violations: &mut Vec<NamingViolation>) {
    match *decl {
        Decl::Function {
            ref name,
            ref parameters,
            ref body,
            ..
        } => {
            if !is_snake_case(name) {
                violations.push(NamingViolation {
                    name: name.clone(),
                    expected: NamingStyle::SnakeCase,
                    location: "function declaration".to_owned(),
                });
            }
            for param in parameters {
                if !is_snake_case(&param.name) {
                    violations.push(NamingViolation {
                        name: param.name.clone(),
                        expected: NamingStyle::SnakeCase,
                        location: format!("parameter of function `{name}`"),
                    });
                }
            }
            check_stmt(body, violations);
        }
        Decl::Type { ref name, .. } => {
            if !is_pascal_case(name) {
                violations.push(NamingViolation {
                    name: name.clone(),
                    expected: NamingStyle::PascalCase,
                    location: "type declaration".to_owned(),
                });
            }
        }
        Decl::Let {
            ref binding,
            ref initializer,
            ..
        } => {
            if !is_snake_case(&binding.name) {
                violations.push(NamingViolation {
                    name: binding.name.clone(),
                    expected: NamingStyle::SnakeCase,
                    location: "let declaration".to_owned(),
                });
            }
            check_expr(initializer, violations);
        }
        Decl::Import { .. } => {}
    }
}

/// Checks a statement for naming violations, appending any found to
/// `violations`.
fn check_stmt(stmt: &Stmt, violations: &mut Vec<NamingViolation>) {
    match *stmt {
        Stmt::Let {
            ref binding,
            ref initializer,
            ..
        } => {
            if !is_snake_case(&binding.name) {
                violations.push(NamingViolation {
                    name: binding.name.clone(),
                    expected: NamingStyle::SnakeCase,
                    location: "let binding".to_owned(),
                });
            }
            if let Some(ref init) = *initializer {
                check_expr(init, violations);
            }
        }
        Stmt::Block { ref statements, .. } => {
            for s in statements {
                check_stmt(s, violations);
            }
        }
        Stmt::Return { ref values, .. }
        | Stmt::Break { ref values, .. }
        | Stmt::Continue { ref values, .. } => {
            for lv in values {
                check_expr(&lv.value, violations);
            }
        }
        Stmt::Expression { ref expr, .. } => check_expr(expr, violations),
        Stmt::Assignment {
            ref target,
            ref value,
            ..
        } => {
            check_expr(target, violations);
            check_expr(value, violations);
        }
        Stmt::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => {
            check_expr(condition, violations);
            check_stmt(then_branch, violations);
            if let Some(ref eb) = *else_branch {
                check_stmt(eb, violations);
            }
        }
        Stmt::For {
            ref variable,
            ref iterable,
            ref body,
            ..
        } => {
            if !is_snake_case(variable) {
                violations.push(NamingViolation {
                    name: variable.clone(),
                    expected: NamingStyle::SnakeCase,
                    location: "for-loop variable".to_owned(),
                });
            }
            check_expr(iterable, violations);
            check_stmt(body, violations);
        }
        Stmt::While {
            ref condition,
            ref body,
            ..
        } => {
            check_expr(condition, violations);
            check_stmt(body, violations);
        }
        Stmt::Loop { ref body, .. } => check_stmt(body, violations),
    }
}

/// Checks an expression for naming violations, appending any found to
/// `violations`.
#[expect(
    clippy::too_many_lines,
    reason = "exhaustive match over all Expr variants"
)]
fn check_expr(expr: &Expr, violations: &mut Vec<NamingViolation>) {
    match *expr {
        Expr::Identifier { .. } | Expr::Literal { .. } | Expr::StringInterpolation { .. } => {}
        Expr::Binary {
            ref left,
            ref right,
            ..
        } => {
            check_expr(left, violations);
            check_expr(right, violations);
        }
        Expr::Unary { ref operand, .. } => check_expr(operand, violations),
        Expr::Call {
            ref callee,
            ref args,
            ..
        } => {
            check_expr(callee, violations);
            for arg in args {
                check_expr(arg, violations);
            }
        }
        Expr::Constructor {
            ref callee,
            ref fields,
            ..
        } => {
            check_expr(callee, violations);
            for field in fields {
                check_expr(&field.value, violations);
            }
        }
        Expr::Index {
            ref object,
            ref index,
            ..
        } => {
            check_expr(object, violations);
            check_expr(index, violations);
        }
        Expr::Member { ref object, .. } => check_expr(object, violations),
        Expr::Cast {
            expr: ref inner, ..
        }
        | Expr::TypeOf {
            expr: ref inner, ..
        }
        | Expr::Parenthesized {
            expr: ref inner, ..
        } => check_expr(inner, violations),
        Expr::Array { ref elements, .. } => {
            for elem in elements {
                check_expr(elem, violations);
            }
        }
        Expr::Match {
            ref scrutinee,
            ref arms,
            ..
        } => {
            check_expr(scrutinee, violations);
            for arm in arms {
                check_expr(&arm.body, violations);
            }
        }
        Expr::Lambda {
            ref params,
            ref body,
            ..
        } => {
            for param in params {
                if !is_snake_case(&param.name) {
                    violations.push(NamingViolation {
                        name: param.name.clone(),
                        expected: NamingStyle::SnakeCase,
                        location: "lambda parameter".to_owned(),
                    });
                }
            }
            match *body {
                LambdaBody::Expression(ref inner_expr) => {
                    check_expr(inner_expr, violations);
                }
                LambdaBody::Block(ref stmts) => {
                    for s in stmts {
                        check_stmt(s, violations);
                    }
                }
            }
        }
        Expr::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => {
            check_expr(condition, violations);
            check_stmt(then_branch, violations);
            if let Some(ref eb) = *else_branch {
                check_stmt(eb, violations);
            }
        }
        Expr::Guard {
            expr: ref inner,
            ref else_branch,
            ..
        } => {
            check_expr(inner, violations);
            check_stmt(else_branch, violations);
        }
        Expr::Propagate { ref call, .. } => check_expr(call, violations),
    }
}
