#![allow(
    clippy::pattern_type_mismatch,
    reason = "Matching on references to AST enum variants; adding & to all patterns reduces readability"
)]
#![allow(
    clippy::missing_docs_in_private_items,
    reason = "Private helper functions for capture analysis; names are self-documenting"
)]

use crate::ast::{Expr, LambdaBody, Parameter, Stmt, StringPart};

pub(super) fn collect_captured_variables(body: &LambdaBody, params: &[Parameter]) -> Vec<String> {
    let param_names: std::collections::HashSet<&str> =
        params.iter().map(|p| p.name.as_str()).collect();
    let mut captures = Vec::new();
    let mut seen = std::collections::HashSet::new();
    match body {
        LambdaBody::Expression(expr) => {
            collect_identifiers_in_expr(expr, &param_names, &mut seen, &mut captures);
        }
        LambdaBody::Block(stmts) => {
            for stmt in stmts {
                collect_identifiers_in_stmt(stmt, &param_names, &mut seen, &mut captures);
            }
        }
    }
    captures
}

fn collect_identifiers_in_expr(
    expr: &Expr,
    param_names: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<String>,
    captures: &mut Vec<String>,
) {
    match expr {
        Expr::Identifier { name, .. } => {
            if !param_names.contains(name.as_str()) && seen.insert(name.clone()) {
                captures.push(name.clone());
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_identifiers_in_expr(left, param_names, seen, captures);
            collect_identifiers_in_expr(right, param_names, seen, captures);
        }
        Expr::Unary { operand, .. } => {
            collect_identifiers_in_expr(operand, param_names, seen, captures);
        }
        Expr::Call { callee, args, .. } => {
            collect_identifiers_in_expr(callee, param_names, seen, captures);
            for arg in args {
                collect_identifiers_in_expr(arg, param_names, seen, captures);
            }
        }
        Expr::Index { object, index, .. } => {
            collect_identifiers_in_expr(object, param_names, seen, captures);
            collect_identifiers_in_expr(index, param_names, seen, captures);
        }
        Expr::Member { object, .. } => {
            collect_identifiers_in_expr(object, param_names, seen, captures);
        }
        Expr::Cast { expr, .. } | Expr::TypeOf { expr, .. } | Expr::Parenthesized { expr, .. } => {
            collect_identifiers_in_expr(expr, param_names, seen, captures);
        }
        Expr::Array { elements, .. } => {
            for elem in elements {
                collect_identifiers_in_expr(elem, param_names, seen, captures);
            }
        }
        Expr::Constructor { fields, .. } => {
            for field in fields {
                collect_identifiers_in_expr(&field.value, param_names, seen, captures);
            }
        }
        Expr::StringInterpolation { parts, .. } => {
            for part in parts {
                if let StringPart::Expression(e) = part {
                    collect_identifiers_in_expr(e, param_names, seen, captures);
                }
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_identifiers_in_expr(condition, param_names, seen, captures);
            collect_identifiers_in_stmt(then_branch, param_names, seen, captures);
            if let Some(eb) = else_branch {
                collect_identifiers_in_stmt(eb, param_names, seen, captures);
            }
        }
        Expr::Loop { body, .. } => {
            collect_identifiers_in_stmt(body, param_names, seen, captures);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_identifiers_in_expr(scrutinee, param_names, seen, captures);
            for arm in arms {
                collect_identifiers_in_expr(&arm.body, param_names, seen, captures);
            }
        }
        Expr::Lambda {
            body,
            params: inner_params,
            ..
        } => {
            let inner_param_names: std::collections::HashSet<&str> =
                inner_params.iter().map(|p| p.name.as_str()).collect();
            let combined: std::collections::HashSet<&str> =
                param_names.union(&inner_param_names).copied().collect();
            match body {
                LambdaBody::Expression(e) => {
                    collect_identifiers_in_expr(e, &combined, seen, captures);
                }
                LambdaBody::Block(stmts) => {
                    for stmt in stmts {
                        collect_identifiers_in_stmt(stmt, &combined, seen, captures);
                    }
                }
            }
        }
        Expr::Guard {
            expr, else_branch, ..
        } => {
            collect_identifiers_in_expr(expr, param_names, seen, captures);
            collect_identifiers_in_stmt(else_branch, param_names, seen, captures);
        }
        Expr::Propagate { call, .. } => {
            collect_identifiers_in_expr(call, param_names, seen, captures);
        }
        Expr::Literal { .. } => {}
    }
}

fn collect_identifiers_in_stmt(
    stmt: &Stmt,
    param_names: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<String>,
    captures: &mut Vec<String>,
) {
    match stmt {
        Stmt::Let { initializer, .. } => {
            if let Some(init) = initializer {
                collect_identifiers_in_expr(init, param_names, seen, captures);
            }
        }
        Stmt::LetDestructure { initializer, .. } => {
            collect_identifiers_in_expr(initializer, param_names, seen, captures);
        }
        Stmt::Assignment { target, value, .. } => {
            collect_identifiers_in_expr(target, param_names, seen, captures);
            collect_identifiers_in_expr(value, param_names, seen, captures);
        }
        Stmt::Return { values, .. }
        | Stmt::Break { values, .. }
        | Stmt::Continue { values, .. } => {
            for lv in values {
                collect_identifiers_in_expr(&lv.value, param_names, seen, captures);
            }
        }
        Stmt::Expression { expr, .. } => {
            collect_identifiers_in_expr(expr, param_names, seen, captures);
        }
        Stmt::Block { statements, .. } => {
            for s in statements {
                collect_identifiers_in_stmt(s, param_names, seen, captures);
            }
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_identifiers_in_expr(condition, param_names, seen, captures);
            collect_identifiers_in_stmt(then_branch, param_names, seen, captures);
            if let Some(eb) = else_branch {
                collect_identifiers_in_stmt(eb, param_names, seen, captures);
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_identifiers_in_expr(iterable, param_names, seen, captures);
            collect_identifiers_in_stmt(body, param_names, seen, captures);
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_identifiers_in_expr(condition, param_names, seen, captures);
            collect_identifiers_in_stmt(body, param_names, seen, captures);
        }
        Stmt::Guard {
            expression,
            else_body,
            ..
        } => {
            collect_identifiers_in_expr(expression, param_names, seen, captures);
            collect_identifiers_in_stmt(else_body, param_names, seen, captures);
        }
        Stmt::Loop { body, .. } => {
            collect_identifiers_in_stmt(body, param_names, seen, captures);
        }
        Stmt::Comment { .. } => {}
    }
}
