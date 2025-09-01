//! Abstract Syntax Tree (AST) definitions for the Opalescent language
//! 
//! This module contains all AST node types and related functionality.

#![expect(dead_code, reason = "AST nodes are partially implemented during language development")]

use crate::token::{Span, TokenType};
use std::fmt;

/// A unique identifier for AST nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Base trait for all AST nodes
pub trait AstNode {
    /// Returns the source span of this AST node
    fn span(&self) -> Span;
    /// Returns the unique node ID of this AST node
    fn node_id(&self) -> NodeId;
}

/// Expression AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal values (42, 3.14, "hello", true)
    Literal {
        value: LiteralValue,
        span: Span,
        id: NodeId,
    },

    /// Identifiers (variable names, function names)
    Identifier {
        name: String,
        span: Span,
        id: NodeId,
    },

    /// Binary operations (a + b, x < y, p and q)
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Unary operations (-x, not p, bnot flags)
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Function calls (print("hello"), math.sqrt(x))
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Array/collection access (arr[0], map["key"])
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Member access (obj.field, module.function)
    Member {
        object: Box<Expr>,
        member: String,
        span: Span,
        id: NodeId,
    },

    /// Type casts (expr as Type)
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
        id: NodeId,
    },

    /// Type checking (type_of(expr))
    TypeOf {
        expr: Box<Expr>,
        span: Span,
        id: NodeId,
    },

    /// String interpolation ('Hello {name}')
    StringInterpolation {
        parts: Vec<StringPart>,
        span: Span,
        id: NodeId,
    },

    /// Parenthesized expressions ((expr))
    Parenthesized {
        expr: Box<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Array literals ([1, 2, 3])
    Array {
        elements: Vec<Expr>,
        span: Span,
        id: NodeId,
    },
}

/// Statement AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Let bindings (let x = 5)
    Let {
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Mutable variable declarations (let mutable x = 5)
    Mutable {
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Assignment statements (x = 10)
    Assignment {
        target: Expr,
        value: Expr,
        span: Span,
        id: NodeId,
    },

    /// Return statements (return expr)
    Return {
        value: Option<Expr>,
        span: Span,
        id: NodeId,
    },

    /// Expression statements (function_call())
    Expression { expr: Expr, span: Span, id: NodeId },

    /// Block statements ({ stmt1; stmt2; })
    Block {
        statements: Vec<Stmt>,
        span: Span,
        id: NodeId,
    },

    /// If statements/expressions
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
        id: NodeId,
    },

    /// For loops (for item in collection)
    For {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
        span: Span,
        id: NodeId,
    },

    /// While loops (while condition)
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
        id: NodeId,
    },

    /// Break statements
    Break { span: Span, id: NodeId },

    /// Continue statements
    Continue { span: Span, id: NodeId },
}

/// Top-level declaration AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// Function declarations
    Function {
        name: String,
        parameters: Vec<Parameter>,
        return_type: Option<Type>,
        body: Stmt,
        visibility: Visibility,
        is_entry: bool,
        doc_comment: Option<String>,
        span: Span,
        id: NodeId,
    },

    /// Type declarations
    Type {
        name: String,
        type_def: TypeDef,
        visibility: Visibility,
        doc_comment: Option<String>,
        span: Span,
        id: NodeId,
    },

    /// Import declarations
    Import {
        items: Vec<ImportItem>,
        source: String,
        span: Span,
        id: NodeId,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Void,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Is,
    IsNot,

    // Logical
    And,
    Or,
    Xor,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    BitShiftLeft,
    BitShiftRight,
    BitUnsignedShiftRight,

    // Assignment
    Assign,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Negate, // -x
    Not,    // not x
    BitNot, // bnot x
    Plus,   // +x
}

/// Type representations
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Basic types
    Basic { name: String, span: Span },

    /// Generic types (Array<T>, Result<T, E>)
    Generic {
        name: String,
        type_args: Vec<Type>,
        span: Span,
    },

    /// Array types (T[])
    Array { element_type: Box<Type>, span: Span },

    /// Function types (f(int32, string): boolean)
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
        span: Span,
    },
}

/// Function parameters
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub span: Span,
}

/// Type definitions for custom types
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef {
    /// Sum types (enums with variants)
    Sum { variants: Vec<Variant>, span: Span },

    /// Product types (structs)
    Product { fields: Vec<Field>, span: Span },

    /// Type aliases
    Alias { target_type: Type, span: Span },
}

/// Variant for sum types
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// Field for product types and variants
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub field_type: Type,
    pub span: Span,
}

/// Import items
#[derive(Debug, Clone, PartialEq)]
pub enum ImportItem {
    /// Import specific item (import foo from bar)
    Named {
        name: String,
        alias: Option<String>,
        span: Span,
    },

    /// Import all (import * from bar)
    Glob { span: Span },

    /// Import type (import type Foo from bar)
    Type {
        name: String,
        alias: Option<String>,
        span: Span,
    },
}

/// Visibility modifiers
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
}

/// String interpolation parts
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string part
    Literal(String),
    /// Expression part ({expr})
    Expression(Expr),
}

/// Complete program AST
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Decl>,
    pub span: Span,
    pub id: NodeId,
}

impl AstNode for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Identifier { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Member { span, .. } => *span,
            Expr::Cast { span, .. } => *span,
            Expr::TypeOf { span, .. } => *span,
            Expr::StringInterpolation { span, .. } => *span,
            Expr::Parenthesized { span, .. } => *span,
            Expr::Array { span, .. } => *span,
        }
    }

    fn node_id(&self) -> NodeId {
        match self {
            Expr::Literal { id, .. } => *id,
            Expr::Identifier { id, .. } => *id,
            Expr::Binary { id, .. } => *id,
            Expr::Unary { id, .. } => *id,
            Expr::Call { id, .. } => *id,
            Expr::Index { id, .. } => *id,
            Expr::Member { id, .. } => *id,
            Expr::Cast { id, .. } => *id,
            Expr::TypeOf { id, .. } => *id,
            Expr::StringInterpolation { id, .. } => *id,
            Expr::Parenthesized { id, .. } => *id,
            Expr::Array { id, .. } => *id,
        }
    }
}

impl AstNode for Stmt {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Mutable { span, .. } => *span,
            Stmt::Assignment { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Expression { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::Break { span, .. } => *span,
            Stmt::Continue { span, .. } => *span,
        }
    }

    fn node_id(&self) -> NodeId {
        match self {
            Stmt::Let { id, .. } => *id,
            Stmt::Mutable { id, .. } => *id,
            Stmt::Assignment { id, .. } => *id,
            Stmt::Return { id, .. } => *id,
            Stmt::Expression { id, .. } => *id,
            Stmt::Block { id, .. } => *id,
            Stmt::If { id, .. } => *id,
            Stmt::For { id, .. } => *id,
            Stmt::While { id, .. } => *id,
            Stmt::Break { id, .. } => *id,
            Stmt::Continue { id, .. } => *id,
        }
    }
}

impl AstNode for Decl {
    fn span(&self) -> Span {
        match self {
            Decl::Function { span, .. } => *span,
            Decl::Type { span, .. } => *span,
            Decl::Import { span, .. } => *span,
        }
    }

    fn node_id(&self) -> NodeId {
        match self {
            Decl::Function { id, .. } => *id,
            Decl::Type { id, .. } => *id,
            Decl::Import { id, .. } => *id,
        }
    }
}

impl AstNode for Program {
    fn span(&self) -> Span {
        self.span
    }

    fn node_id(&self) -> NodeId {
        self.id
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Modulo => "%",
            BinaryOp::Power => "^",
            BinaryOp::Equal => "is",
            BinaryOp::NotEqual => "is not",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Is => "is",
            BinaryOp::IsNot => "is not",
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
        };
        write!(f, "{symbol}")
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match self {
            UnaryOp::Negate => "-",
            UnaryOp::Not => "not",
            UnaryOp::BitNot => "bnot",
            UnaryOp::Plus => "+",
        };
        write!(f, "{symbol}")
    }
}

impl From<TokenType> for BinaryOp {
    fn from(token_type: TokenType) -> Self {
        match token_type {
            TokenType::Plus => BinaryOp::Add,
            TokenType::Minus => BinaryOp::Subtract,
            TokenType::Multiply => BinaryOp::Multiply,
            TokenType::Divide => BinaryOp::Divide,
            TokenType::Modulo => BinaryOp::Modulo,
            TokenType::Power => BinaryOp::Power,
            TokenType::Less => BinaryOp::Less,
            TokenType::LessEqual => BinaryOp::LessEqual,
            TokenType::Greater => BinaryOp::Greater,
            TokenType::GreaterEqual => BinaryOp::GreaterEqual,
            TokenType::Is => BinaryOp::Is,
            TokenType::IsNot => BinaryOp::IsNot,
            TokenType::And => BinaryOp::And,
            TokenType::Or => BinaryOp::Or,
            TokenType::Xor => BinaryOp::Xor,
            TokenType::BitAnd => BinaryOp::BitAnd,
            TokenType::BitOr => BinaryOp::BitOr,
            TokenType::BitXor => BinaryOp::BitXor,
            TokenType::BitShiftLeft => BinaryOp::BitShiftLeft,
            TokenType::BitShiftRight => BinaryOp::BitShiftRight,
            TokenType::BitUnsignedShiftRight => BinaryOp::BitUnsignedShiftRight,
            TokenType::Assign => BinaryOp::Assign,
            _ => panic!("Cannot convert {token_type:?} to BinaryOp"),
        }
    }
}

impl From<TokenType> for UnaryOp {
    fn from(token_type: TokenType) -> Self {
        match token_type {
            TokenType::Minus => UnaryOp::Negate,
            TokenType::Plus => UnaryOp::Plus,
            TokenType::Not => UnaryOp::Not,
            TokenType::BitNot => UnaryOp::BitNot,
            _ => panic!("Cannot convert {token_type:?} to UnaryOp"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Position;

    fn dummy_span() -> Span {
        Span::single(Position::start())
    }

    fn dummy_node_id() -> NodeId {
        NodeId(0)
    }

    #[test]
    fn test_literal_expr() {
        let expr = Expr::Literal {
            value: LiteralValue::Integer(42),
            span: dummy_span(),
            id: dummy_node_id(),
        };

        assert_eq!(expr.span(), dummy_span());
        assert_eq!(expr.node_id(), dummy_node_id());
    }

    #[test]
    fn test_binary_expr() {
        let left = Box::new(Expr::Literal {
            value: LiteralValue::Integer(1),
            span: dummy_span(),
            id: dummy_node_id(),
        });

        let right = Box::new(Expr::Literal {
            value: LiteralValue::Integer(2),
            span: dummy_span(),
            id: dummy_node_id(),
        });

        let expr = Expr::Binary {
            left,
            operator: BinaryOp::Add,
            right,
            span: dummy_span(),
            id: dummy_node_id(),
        };

        assert_eq!(expr.span(), dummy_span());
        assert_eq!(expr.node_id(), dummy_node_id());
    }

    #[test]
    fn test_binary_op_display() {
        assert_eq!(format!("{}", BinaryOp::Add), "+");
        assert_eq!(format!("{}", BinaryOp::And), "and");
        assert_eq!(format!("{}", BinaryOp::BitShiftLeft), "bshl");
    }

    #[test]
    fn test_unary_op_display() {
        assert_eq!(format!("{}", UnaryOp::Negate), "-");
        assert_eq!(format!("{}", UnaryOp::Not), "not");
        assert_eq!(format!("{}", UnaryOp::BitNot), "bnot");
    }

    #[test]
    fn test_token_to_binary_op() {
        assert_eq!(BinaryOp::from(TokenType::Plus), BinaryOp::Add);
        assert_eq!(BinaryOp::from(TokenType::And), BinaryOp::And);
        assert_eq!(
            BinaryOp::from(TokenType::BitShiftLeft),
            BinaryOp::BitShiftLeft
        );
    }

    #[test]
    fn test_token_to_unary_op() {
        assert_eq!(UnaryOp::from(TokenType::Minus), UnaryOp::Negate);
        assert_eq!(UnaryOp::from(TokenType::Not), UnaryOp::Not);
        assert_eq!(UnaryOp::from(TokenType::BitNot), UnaryOp::BitNot);
    }
}
