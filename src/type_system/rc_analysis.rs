//! Reference-count insertion planning pass (Perceus-style).
//!
//! This module performs RC analysis over function statements and produces
//! metadata describing where `inc`/`dec`/`drop` operations should be inserted.

extern crate alloc;

use crate::ast::{Parameter, Type};
use crate::ast::{PassingMode, Stmt};
use crate::type_system::heap_class::{HeapClass, classify_core_type};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// RC operation kind to be inserted by codegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RcOp {
    Inc,
    Dec,
    Drop,
}

/// A planned insertion point for an RC operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcInsertionPoint {
    pub variable: String,
    pub op: RcOp,
    pub after_stmt_index: usize,
}

/// Final RC insertion plan for a statement list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RcPlan {
    pub insertions: Vec<RcInsertionPoint>,
}

/// Reuse operation kind emitted by [`ReuseAnalysis`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReuseOp {
    Reuse,
}

/// A detected reuse opportunity: source variable's memory can be reused for target allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseOpportunity {
    /// Variable whose memory is being reused (will be dropped).
    pub source_var: String,
    /// Variable that will reuse the memory (new allocation).
    pub target_var: String,
    /// The type being reused (must match between source and target).
    pub reuse_type: String,
}

/// Extended RC plan with reuse opportunities.
#[derive(Debug, Clone, Default)]
pub struct ReusePlan {
    pub opportunities: Vec<ReuseOpportunity>,
    /// Number of opportunities discovered by this pass.
    pub reuse_count: usize,
}

/// RC analysis entry type.
#[derive(Debug, Clone, Default)]
pub struct RcAnalysis;

/// Internal type metadata tracked for Perceus reuse analysis.
#[derive(Debug, Clone)]
struct ReuseVarInfo {
    type_name: String,
    layout: Option<crate::type_system::memory::MemoryLayout>,
    declared_at: Option<usize>,
    is_ref_like_param: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PathState {
    terminated: bool,
    transferred: bool,
    last_use: Option<usize>,
}

fn is_reference_counted_core_type(core_type: &CoreType) -> bool {
    matches!(classify_core_type(core_type), HeapClass::ReferenceCounted)
}

impl RcAnalysis {
    /// Analyze a function statement list and produce an RC insertion plan.
    #[must_use]
    pub fn analyze_stmts(
        stmts: &[Stmt],
        param_modes: &BTreeMap<String, PassingMode>,
        var_types: &BTreeMap<String, CoreType>,
    ) -> RcPlan {
        let mut insertions = Vec::new();

        if stmts.is_empty() {
            return RcPlan { insertions };
        }

        let scope_end_index = stmts.len() - 1;

        for (variable, variable_type) in var_types {
            if !is_reference_counted_core_type(variable_type) {
                continue;
            }

            if matches!(
                param_modes.get(variable.as_str()),
                Some(PassingMode::Ref | PassingMode::MutableRef)
            ) {
                continue;
            }

            let mut paths = vec![PathState::default()];
            for (stmt_index, stmt) in stmts.iter().enumerate() {
                paths = Self::eval_stmt_paths_for_var(stmt, variable.as_str(), stmt_index, &paths);
            }

            for path in paths {
                if path.transferred {
                    continue;
                }

                let insertion_index = path.last_use.unwrap_or(scope_end_index);
                insertions.push(RcInsertionPoint {
                    variable: variable.clone(),
                    op: RcOp::Dec,
                    after_stmt_index: insertion_index,
                });
            }
        }

        insertions.sort_by(|a, b| {
            a.after_stmt_index
                .cmp(&b.after_stmt_index)
                .then_with(|| a.variable.cmp(&b.variable))
        });
        insertions.dedup();

        RcPlan { insertions }
    }

    /// Convenience wrapper for function-level analysis.
    #[must_use]
    pub fn analyze_function(
        &self,
        stmts: &[Stmt],
        param_modes: &BTreeMap<String, PassingMode>,
        var_types: &BTreeMap<String, CoreType>,
    ) -> RcPlan {
        Self::analyze_stmts(stmts, param_modes, var_types)
    }

    fn eval_stmt_paths_for_var(
        stmt: &Stmt,
        variable: &str,
        stmt_index: usize,
        input_paths: &[PathState],
    ) -> Vec<PathState> {
        let mut out = Vec::new();
        for input in input_paths {
            if input.terminated {
                out.push(*input);
                continue;
            }

            match stmt {
                Stmt::Return { values, .. } => {
                    let transfers = values
                        .iter()
                        .any(|value| Self::expr_uses_var(&value.value, variable));

                    out.push(PathState {
                        terminated: true,
                        transferred: transfers,
                        last_use: if transfers {
                            input.last_use
                        } else {
                            Some(stmt_index)
                        },
                    });
                }
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    let mut base = *input;
                    if Self::expr_uses_var(condition, variable) {
                        base.last_use = Some(stmt_index);
                    }

                    let then_paths = Self::eval_branch_stmt_for_var(
                        then_branch.as_ref(),
                        variable,
                        stmt_index,
                        &[base],
                    );

                    let else_paths = if let Some(else_stmt) = else_branch.as_ref() {
                        Self::eval_branch_stmt_for_var(
                            else_stmt.as_ref(),
                            variable,
                            stmt_index,
                            &[base],
                        )
                    } else {
                        vec![base]
                    };

                    out.extend(then_paths);
                    out.extend(else_paths);
                }
                _ => {
                    let mut next = *input;
                    if Self::stmt_uses_var(stmt, variable) {
                        next.last_use = Some(stmt_index);
                    }
                    out.push(next);
                }
            }
        }

        out
    }

    fn eval_branch_stmt_for_var(
        stmt: &Stmt,
        variable: &str,
        stmt_index: usize,
        input_paths: &[PathState],
    ) -> Vec<PathState> {
        match stmt {
            Stmt::Block { statements, .. } => {
                let mut paths = input_paths.to_vec();
                for nested in statements {
                    paths = Self::eval_stmt_paths_for_var(nested, variable, stmt_index, &paths);
                }
                paths
            }
            _ => Self::eval_stmt_paths_for_var(stmt, variable, stmt_index, input_paths),
        }
    }

    fn stmt_uses_var(stmt: &Stmt, variable: &str) -> bool {
        match stmt {
            Stmt::Let { initializer, .. } => initializer
                .as_ref()
                .is_some_and(|expr| Self::expr_uses_var(expr, variable)),
            Stmt::LetDestructure { initializer, .. } => Self::expr_uses_var(initializer, variable),
            Stmt::Assignment { target, value, .. } => {
                Self::expr_uses_var(target, variable) || Self::expr_uses_var(value, variable)
            }
            Stmt::Return { values, .. } => values
                .iter()
                .any(|value| Self::expr_uses_var(&value.value, variable)),
            Stmt::Expression { expr, .. } => Self::expr_uses_var(expr, variable),
            Stmt::Block { statements, .. } => statements
                .iter()
                .any(|nested| Self::stmt_uses_var(nested, variable)),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                Self::expr_uses_var(condition, variable)
                    || Self::stmt_uses_var(then_branch, variable)
                    || else_branch
                        .as_ref()
                        .is_some_and(|else_stmt| Self::stmt_uses_var(else_stmt, variable))
            }
            Stmt::For { iterable, body, .. } => {
                Self::expr_uses_var(iterable, variable) || Self::stmt_uses_var(body, variable)
            }
            Stmt::While {
                condition, body, ..
            } => Self::expr_uses_var(condition, variable) || Self::stmt_uses_var(body, variable),
            Stmt::Guard {
                expression,
                else_body,
                ..
            } => {
                Self::expr_uses_var(expression, variable)
                    || Self::stmt_uses_var(else_body, variable)
            }
            Stmt::PropagateGuardError { error_binding, .. } => error_binding == variable,
            Stmt::Loop { body, .. } => Self::stmt_uses_var(body, variable),
            Stmt::Break { values, .. } | Stmt::Continue { values, .. } => values
                .iter()
                .any(|value| Self::expr_uses_var(&value.value, variable)),
            Stmt::Comment { .. } => false,
        }
    }

    fn expr_uses_var(expr: &crate::ast::Expr, variable: &str) -> bool {
        use crate::ast::{Expr, LambdaBody, StringPart};

        match expr {
            Expr::Identifier { name, .. } => name.as_str() == variable,
            Expr::Literal { .. } => false,
            Expr::Binary { left, right, .. } => {
                Self::expr_uses_var(left, variable) || Self::expr_uses_var(right, variable)
            }
            Expr::Unary { operand, .. } => Self::expr_uses_var(operand, variable),
            Expr::Call { callee, args, .. } => {
                Self::expr_uses_var(callee, variable)
                    || args.iter().any(|arg| Self::expr_uses_var(arg, variable))
            }
            Expr::Constructor { callee, fields, .. } => {
                Self::expr_uses_var(callee, variable)
                    || fields
                        .iter()
                        .any(|field| Self::expr_uses_var(&field.value, variable))
            }
            Expr::Index { object, index, .. } => {
                Self::expr_uses_var(object, variable) || Self::expr_uses_var(index, variable)
            }
            Expr::Member { object, .. } => Self::expr_uses_var(object, variable),
            Expr::Cast { expr, .. } | Expr::TypeOf { expr, .. } => {
                Self::expr_uses_var(expr, variable)
            }
            Expr::StringInterpolation { parts, .. } => parts.iter().any(|part| match part {
                StringPart::Literal(_) => false,
                StringPart::Expression(expr) => Self::expr_uses_var(expr, variable),
            }),
            Expr::Parenthesized { expr, .. } => Self::expr_uses_var(expr, variable),
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                Self::expr_uses_var(condition, variable)
                    || Self::stmt_uses_var(then_branch, variable)
                    || else_branch
                        .as_ref()
                        .is_some_and(|else_stmt| Self::stmt_uses_var(else_stmt, variable))
            }
            Expr::Array { elements, .. } => elements
                .iter()
                .any(|element| Self::expr_uses_var(element, variable)),
            Expr::Match {
                scrutinee, arms, ..
            } => {
                Self::expr_uses_var(scrutinee, variable)
                    || arms.iter().any(|arm| {
                        arm.guard
                            .as_ref()
                            .is_some_and(|guard| Self::expr_uses_var(guard, variable))
                            || Self::expr_uses_var(&arm.body, variable)
                    })
            }
            Expr::Loop { body, .. } => Self::stmt_uses_var(body, variable),
            Expr::Lambda {
                body,
                captured_variables,
                ..
            } => {
                if captured_variables
                    .iter()
                    .any(|captured| captured == variable)
                {
                    return true;
                }

                match body {
                    LambdaBody::Expression(expr) => Self::expr_uses_var(expr, variable),
                    LambdaBody::Block(statements) => statements
                        .iter()
                        .any(|stmt| Self::stmt_uses_var(stmt, variable)),
                }
            }
            Expr::Guard {
                expr, else_branch, ..
            } => Self::expr_uses_var(expr, variable) || Self::stmt_uses_var(else_branch, variable),
            Expr::Propagate { call, .. } => Self::expr_uses_var(call, variable),
        }
    }
}

/// Perceus-style reuse analysis pass.
///
/// This pass is intentionally separate from [`RcAnalysis`] so reuse optimization can be
/// enabled/disabled independently from baseline RC insertion planning.
///
/// The analysis looks for same-type allocations immediately after the source variable's
/// last use (`last_use == alloc_index - 1`) and only for unique-owner candidates
/// (borrowed `ref`/`mutable ref` parameters are excluded).
pub struct ReuseAnalysis;

impl ReuseAnalysis {
    /// Analyze a sequence of statements for Perceus reuse opportunities.
    #[must_use]
    pub fn analyze_stmts(stmts: &[Stmt], params: &[Parameter]) -> ReusePlan {
        let mut opportunities = Vec::new();
        if stmts.is_empty() {
            return ReusePlan {
                opportunities,
                reuse_count: 0,
            };
        }

        let mut var_info = BTreeMap::new();
        for parameter in params {
            var_info.insert(
                parameter.name.clone(),
                ReuseVarInfo {
                    type_name: parameter.param_type.to_signature_string(),
                    layout: Self::layout_for_ast_type(&parameter.param_type),
                    declared_at: None,
                    is_ref_like_param: matches!(
                        parameter.passing_mode,
                        PassingMode::Ref | PassingMode::MutableRef
                    ),
                },
            );
        }

        let mut alloc_sites = Vec::new();
        for (stmt_index, stmt) in stmts.iter().enumerate() {
            if let Stmt::Let { binding, .. } = stmt {
                let Some(type_annotation) = binding.type_annotation.as_ref() else {
                    continue;
                };

                let variable_name = binding.name.clone();
                var_info.insert(
                    variable_name.clone(),
                    ReuseVarInfo {
                        type_name: type_annotation.to_signature_string(),
                        layout: Self::layout_for_ast_type(type_annotation),
                        declared_at: Some(stmt_index),
                        is_ref_like_param: false,
                    },
                );
                alloc_sites.push((stmt_index, variable_name));
            }
        }

        let last_use = Self::compute_last_use(stmts, &var_info);

        for (alloc_index, target_var) in alloc_sites {
            if alloc_index == 0 {
                continue;
            }

            let Some(target_info) = var_info.get(&target_var) else {
                continue;
            };

            let required_last_use_index = alloc_index - 1;
            let mut selected_source: Option<(String, usize)> = None;

            for (source_var, source_info) in &var_info {
                if source_var == &target_var {
                    continue;
                }

                if source_info.is_ref_like_param {
                    continue;
                }

                if source_info
                    .declared_at
                    .is_some_and(|declared_at| declared_at >= alloc_index)
                {
                    continue;
                }

                if last_use.get(source_var).copied() != Some(required_last_use_index) {
                    continue;
                }

                if !Self::types_compatible_for_reuse(source_info, target_info) {
                    continue;
                }

                let declaration_rank = source_info.declared_at.unwrap_or(0);
                let replace_current = match selected_source.as_ref() {
                    None => true,
                    Some((current_source, current_rank)) => {
                        declaration_rank > *current_rank
                            || (declaration_rank == *current_rank
                                && source_var.as_str() < current_source.as_str())
                    }
                };

                if replace_current {
                    selected_source = Some((source_var.clone(), declaration_rank));
                }
            }

            if let Some((source_var, _)) = selected_source {
                opportunities.push(ReuseOpportunity {
                    source_var,
                    target_var,
                    reuse_type: target_info.type_name.clone(),
                });
            }
        }

        opportunities.sort_by(|a, b| {
            a.source_var
                .cmp(&b.source_var)
                .then_with(|| a.target_var.cmp(&b.target_var))
                .then_with(|| a.reuse_type.cmp(&b.reuse_type))
        });
        opportunities.dedup();

        let reuse_count = opportunities.len();
        ReusePlan {
            opportunities,
            reuse_count,
        }
    }

    fn layout_for_ast_type(ast_type: &Type) -> Option<crate::type_system::memory::MemoryLayout> {
        ast_type_to_core_type(ast_type)
            .ok()
            .map(|core_type| core_type.memory_layout())
    }

    fn types_compatible_for_reuse(source_info: &ReuseVarInfo, target_info: &ReuseVarInfo) -> bool {
        if source_info.type_name != target_info.type_name {
            return false;
        }

        match (source_info.layout, target_info.layout) {
            (Some(source_layout), Some(target_layout)) => {
                source_layout.size == target_layout.size
                    && source_layout.align == target_layout.align
            }
            _ => true,
        }
    }

    fn compute_last_use(
        stmts: &[Stmt],
        variables: &BTreeMap<String, ReuseVarInfo>,
    ) -> BTreeMap<String, usize> {
        let mut last_use = BTreeMap::new();
        for variable in variables.keys() {
            for (stmt_index, stmt) in stmts.iter().enumerate() {
                if RcAnalysis::stmt_uses_var(stmt, variable.as_str()) {
                    last_use.insert(variable.clone(), stmt_index);
                }
            }
        }
        last_use
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, LabeledValue, LetBinding, LiteralValue, NodeId, Parameter, Type};
    use crate::token::{Position, Span};

    fn span() -> Span {
        Span::single(Position::start())
    }

    fn ident(name: &str, id: usize) -> Expr {
        Expr::Identifier {
            name: name.to_string(),
            span: span(),
            id: NodeId(id),
        }
    }

    fn bool_lit(value: bool, id: usize) -> Expr {
        Expr::Literal {
            value: LiteralValue::Boolean(value),
            span: span(),
            id: NodeId(id),
        }
    }

    fn string_lit(value: &str, id: usize) -> Expr {
        Expr::Literal {
            value: LiteralValue::String(value.to_string()),
            span: span(),
            id: NodeId(id),
        }
    }

    fn int_lit(value: i64, id: usize) -> Expr {
        Expr::Literal {
            value: LiteralValue::Integer(value),
            span: span(),
            id: NodeId(id),
        }
    }

    fn basic_type(name: &str) -> Type {
        Type::Basic {
            name: name.to_string(),
            span: span(),
        }
    }

    fn let_stmt(name: &str, initializer: Expr, id: usize) -> Stmt {
        Stmt::Let {
            binding: LetBinding {
                name: name.to_string(),
                type_annotation: None,
                is_mutable: false,
                span: span(),
                id: NodeId(id + 100),
            },
            initializer: Some(initializer),
            span: span(),
            id: NodeId(id),
        }
    }

    fn expr_stmt(expr: Expr, id: usize) -> Stmt {
        Stmt::Expression {
            expr,
            span: span(),
            id: NodeId(id),
        }
    }

    fn typed_let_stmt(name: &str, type_name: &str, initializer: Expr, id: usize) -> Stmt {
        Stmt::Let {
            binding: LetBinding {
                name: name.to_string(),
                type_annotation: Some(basic_type(type_name)),
                is_mutable: false,
                span: span(),
                id: NodeId(id + 300),
            },
            initializer: Some(initializer),
            span: span(),
            id: NodeId(id),
        }
    }

    fn owned_param(name: &str, type_name: &str) -> Parameter {
        Parameter {
            name: name.to_string(),
            param_type: basic_type(type_name),
            passing_mode: PassingMode::Owned,
            span: span(),
        }
    }

    fn ref_param(name: &str, type_name: &str) -> Parameter {
        Parameter {
            name: name.to_string(),
            param_type: basic_type(type_name),
            passing_mode: PassingMode::Ref,
            span: span(),
        }
    }

    fn return_stmt(value: Expr, id: usize) -> Stmt {
        Stmt::Return {
            values: vec![LabeledValue {
                label: String::new(),
                value,
                span: span(),
                id: NodeId(id + 200),
            }],
            span: span(),
            id: NodeId(id),
        }
    }

    #[test]
    fn test_simple_variable_lifecycle_inserts_dec_after_last_use() {
        let stmts = vec![
            let_stmt("x", string_lit("hello", 1), 10),
            expr_stmt(ident("x", 2), 11),
        ];

        let param_modes = BTreeMap::new();
        let mut var_types = BTreeMap::new();
        var_types.insert("x".to_string(), CoreType::String);

        let plan = RcAnalysis::analyze_stmts(&stmts, &param_modes, &var_types);

        assert_eq!(plan.insertions.len(), 1);
        assert_eq!(
            plan.insertions[0],
            RcInsertionPoint {
                variable: "x".to_string(),
                op: RcOp::Dec,
                after_stmt_index: 1,
            }
        );
    }

    #[test]
    fn test_ref_parameter_produces_no_rc_ops() {
        let stmts = vec![expr_stmt(ident("p", 1), 10)];

        let mut param_modes = BTreeMap::new();
        param_modes.insert("p".to_string(), PassingMode::Ref);

        let mut var_types = BTreeMap::new();
        var_types.insert("p".to_string(), CoreType::String);

        let plan = RcAnalysis::analyze_stmts(&stmts, &param_modes, &var_types);
        assert!(plan.insertions.is_empty());
    }

    #[test]
    fn test_if_branch_requires_compensating_dec_when_other_branch_returns_value() {
        let stmts = vec![
            let_stmt("x", string_lit("hello", 1), 10),
            Stmt::If {
                condition: bool_lit(true, 2),
                then_branch: Box::new(Stmt::Block {
                    statements: vec![return_stmt(ident("x", 3), 12)],
                    span: span(),
                    id: NodeId(1000),
                }),
                else_branch: Some(Box::new(Stmt::Block {
                    statements: vec![expr_stmt(string_lit("noop", 4), 13)],
                    span: span(),
                    id: NodeId(1001),
                })),
                span: span(),
                id: NodeId(11),
            },
        ];

        let param_modes = BTreeMap::new();
        let mut var_types = BTreeMap::new();
        var_types.insert("x".to_string(), CoreType::String);

        let plan = RcAnalysis::analyze_stmts(&stmts, &param_modes, &var_types);

        assert!(plan.insertions.iter().any(|insertion| {
            insertion.variable == "x"
                && insertion.op == RcOp::Dec
                && insertion.after_stmt_index == 1
        }));
    }

    #[test]
    fn test_reuse_detected_for_same_size_unique_allocation() {
        let stmts = vec![
            typed_let_stmt("x", "string", string_lit("old", 1), 10),
            expr_stmt(ident("x", 2), 11),
            typed_let_stmt("y", "string", string_lit("new", 3), 12),
        ];

        let params = vec![owned_param("p", "int32")];

        let reuse_plan = ReuseAnalysis::analyze_stmts(&stmts, &params);

        assert!(reuse_plan.opportunities.contains(&ReuseOpportunity {
            source_var: "x".to_string(),
            target_var: "y".to_string(),
            reuse_type: "string".to_string(),
        }));
    }

    #[test]
    fn test_no_reuse_for_different_size_types() {
        let stmts = vec![
            typed_let_stmt("x", "int32", int_lit(1, 1), 10),
            expr_stmt(ident("x", 2), 11),
            typed_let_stmt("y", "int64", int_lit(2, 3), 12),
        ];

        let params = Vec::new();

        let reuse_plan = ReuseAnalysis::analyze_stmts(&stmts, &params);
        assert!(reuse_plan.opportunities.is_empty());
    }

    #[test]
    fn test_no_reuse_for_shared_values() {
        let stmts = vec![
            expr_stmt(ident("x", 1), 10),
            typed_let_stmt("y", "string", string_lit("new", 2), 11),
        ];

        let params = vec![ref_param("x", "string")];

        let reuse_plan = ReuseAnalysis::analyze_stmts(&stmts, &params);
        assert!(reuse_plan.opportunities.is_empty());
    }
}
