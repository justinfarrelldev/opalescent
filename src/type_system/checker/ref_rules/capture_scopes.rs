//! Lexical capture traversal for existing second-class references and owners.
extern crate alloc;
use super::{Expr, LambdaBody, Span, Stmt, TypeChecker, TypeError};
use crate::ast::StringPart;
use alloc::collections::BTreeSet;

impl TypeChecker {
    /// Distinguish local parameters/bindings from captured outer references.
    pub(super) fn check_lambda_body_for_capture(
        &self,
        body: &LambdaBody,
        parameters: &[&str],
        _span: Span,
    ) -> Result<(), TypeError> {
        let mut locals: BTreeSet<String> =
            parameters.iter().map(|name| (*name).to_owned()).collect();
        match *body {
            LambdaBody::Expression(ref expression) => self.capture_expr(expression, &locals),
            LambdaBody::Block(ref statements) => self.capture_statements(statements, &mut locals),
        }
    }

    /// Initializers see the preceding scope; their new bindings become local
    /// only afterward, preventing both false captures and unsafe shadowing.
    fn capture_statements(
        &self,
        statements: &[Stmt],
        locals: &mut BTreeSet<String>,
    ) -> Result<(), TypeError> {
        for statement in statements {
            self.capture_statement(statement, locals)?;
            match *statement {
                Stmt::Let { ref binding, .. } => {
                    locals.insert(binding.name.clone());
                }
                Stmt::LetDestructure { ref bindings, .. } => {
                    locals.extend(bindings.iter().map(|binding| binding.name.clone()));
                }
                Stmt::Guard {
                    ref success_binding,
                    ref success_bindings,
                    ..
                } => {
                    locals.extend(success_binding.iter().cloned());
                    locals.extend(success_bindings.iter().map(|binding| binding.name.clone()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Control-flow scopes preserve local parameter shadowing in native
    /// assertions/configurations without allowing reference capture to escape.
    fn capture_statement(
        &self,
        statement: &Stmt,
        locals: &BTreeSet<String>,
    ) -> Result<(), TypeError> {
        match *statement {
            Stmt::Testing { ref statement, .. } => {
                for expression in statement.expressions() {
                    self.capture_expr(expression, locals)?;
                }
            }
            Stmt::Let {
                ref initializer, ..
            } => {
                if let Some(ref initializer_expr) = *initializer {
                    self.capture_expr(initializer_expr, locals)?;
                }
            }
            Stmt::LetDestructure {
                ref initializer, ..
            } => self.capture_expr(initializer, locals)?,
            Stmt::Expression { ref expr, .. } => self.capture_expr(expr, locals)?,
            Stmt::Assignment {
                ref target,
                ref value,
                ..
            } => {
                self.capture_expr(target, locals)?;
                self.capture_expr(value, locals)?;
            }
            Stmt::Return { ref values, .. }
            | Stmt::Break { ref values, .. }
            | Stmt::Continue { ref values, .. } => {
                for value in values {
                    self.capture_expr(&value.value, locals)?;
                }
            }
            Stmt::Block { ref statements, .. } => {
                self.capture_statements(statements, &mut locals.clone())?;
            }
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                self.capture_expr(condition, locals)?;
                self.capture_statement(then_branch, locals)?;
                if let Some(ref else_statement) = *else_branch {
                    self.capture_statement(else_statement, locals)?;
                }
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                ..
            } => {
                self.capture_expr(iterable, locals)?;
                let mut scope = locals.clone();
                scope.insert(variable.clone());
                self.capture_statement(body, &scope)?;
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                self.capture_expr(condition, locals)?;
                self.capture_statement(body, locals)?;
            }
            Stmt::Loop { ref body, .. } => self.capture_statement(body, locals)?,
            Stmt::Guard {
                ref expression,
                ref error_binding,
                ref else_body,
                ..
            } => {
                self.capture_expr(expression, locals)?;
                let mut scope = locals.clone();
                scope.insert(error_binding.clone());
                self.capture_statement(else_body, &scope)?;
            }
            Stmt::Using {
                ref binding,
                ref acquisition,
                ref body,
                ..
            } => {
                self.capture_expr(acquisition, locals)?;
                let mut scope = locals.clone();
                scope.insert(binding.name.clone());
                self.capture_statement(body, &scope)?;
            }
            Stmt::PropagateGuardError { .. } | Stmt::Comment { .. } => {}
        }
        Ok(())
    }

    /// Interpolated values obey the same lexical capture rules as other values.
    fn capture_string_parts(
        &self,
        parts: &[StringPart],
        locals: &BTreeSet<String>,
    ) -> Result<(), TypeError> {
        for part in parts {
            if let StringPart::Expression(ref interpolated) = *part {
                self.capture_expr(interpolated, locals)?;
            }
        }
        Ok(())
    }

    /// Read projections/temporary call arguments in their actual lexical scope.
    /// A nested lambda performs its own capture check against its own parameters.
    fn capture_expr(&self, expression: &Expr, locals: &BTreeSet<String>) -> Result<(), TypeError> {
        match *expression {
            Expr::Identifier { ref name, .. } if locals.contains(name) => Ok(()),
            Expr::BorrowArgument { ref target, .. } => self.capture_expr(target, locals),
            Expr::Member { ref object, .. } => self.capture_expr(object, locals),
            Expr::Index {
                ref object,
                ref index,
                ..
            } => {
                self.capture_expr(object, locals)?;
                self.capture_expr(index, locals)
            }
            Expr::Call {
                ref callee,
                ref args,
                ..
            } => {
                self.capture_expr(callee, locals)?;
                for argument in args {
                    self.capture_expr(argument, locals)?;
                }
                Ok(())
            }
            Expr::Parenthesized { ref expr, .. }
            | Expr::TypeOf { ref expr, .. }
            | Expr::Cast { ref expr, .. } => self.capture_expr(expr, locals),
            Expr::Unary { ref operand, .. } => self.capture_expr(operand, locals),
            Expr::Binary {
                ref left,
                ref right,
                ..
            } => {
                self.capture_expr(left, locals)?;
                self.capture_expr(right, locals)
            }
            Expr::Array { ref elements, .. } => {
                for element in elements {
                    self.capture_expr(element, locals)?;
                }
                Ok(())
            }
            Expr::Constructor {
                ref callee,
                ref fields,
                ..
            }
            | Expr::RecordUpdate {
                receiver: ref callee,
                ref fields,
                ..
            } => {
                self.capture_expr(callee, locals)?;
                for field in fields {
                    self.capture_expr(&field.value, locals)?;
                }
                Ok(())
            }
            Expr::Constrain { ref value, .. } | Expr::Refinement { ref value, .. } => {
                self.capture_expr(value, locals)
            }
            Expr::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                self.capture_expr(condition, locals)?;
                self.capture_statement(then_branch, locals)?;
                if let Some(ref else_statement) = *else_branch {
                    self.capture_statement(else_statement, locals)?;
                }
                Ok(())
            }
            Expr::Loop { ref body, .. } => self.capture_statement(body, locals),
            Expr::Guard {
                ref expr,
                ref else_branch,
                ..
            } => {
                self.capture_expr(expr, locals)?;
                self.capture_statement(else_branch, locals)
            }
            Expr::Propagate {
                ref call,
                ref cause,
                ..
            } => {
                self.capture_expr(call, locals)?;
                if let Some(ref cause_expr) = *cause {
                    self.capture_expr(cause_expr, locals)?;
                }
                Ok(())
            }
            Expr::StringInterpolation { ref parts, .. } => self.capture_string_parts(parts, locals),
            _ => self.check_affine_or_borrow_escape(expression, "be captured by a lambda"),
        }
    }
}
