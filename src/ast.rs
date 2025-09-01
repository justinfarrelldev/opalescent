//! Abstract Syntax Tree (AST) definitions for the Opalescent language
//!
//! This module contains all AST node types and related functionality.

#![expect(
    dead_code,
    reason = "AST nodes are partially implemented during language development"
)]

use crate::token::{Span, TokenType};
use core::fmt;

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
        /// The actual value of the literal
        value: LiteralValue,
        /// Source code location of this literal
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Identifiers (variable names, function names)
    Identifier {
        /// The name of the identifier
        name: String,
        /// Source code location of this identifier
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Binary operations (a + b, x < y, p and q)
    Binary {
        /// Left operand expression
        left: Box<Expr>,
        /// Binary operator type
        operator: BinaryOp,
        /// Right operand expression
        right: Box<Expr>,
        /// Source code location of this binary operation
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Unary operations (-x, not p, bnot flags)
    Unary {
        /// Unary operator type
        operator: UnaryOp,
        /// Expression being operated on
        operand: Box<Expr>,
        /// Source code location of this unary operation
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Function calls (print("hello"), math.sqrt(x))
    Call {
        /// Expression that resolves to the function being called
        callee: Box<Expr>,
        /// Arguments passed to the function
        args: Vec<Expr>,
        /// Source code location of this function call
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Array/collection access (arr[0], map\[`"key"`\])
    Index {
        /// Expression being indexed
        object: Box<Expr>,
        /// Index expression
        index: Box<Expr>,
        /// Source code location of this index operation
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Member access (obj.field, module.function)
    Member {
        /// Expression being accessed
        object: Box<Expr>,
        /// Name of the member being accessed
        member: String,
        /// Source code location of this member access
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Type casts (expr as Type)
    Cast {
        /// Expression being cast
        expr: Box<Expr>,
        /// Type to cast to
        target_type: Type,
        /// Source code location of this type cast
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Type checking (`type_of(expr)`)
    TypeOf {
        /// Expression whose type is being queried
        expr: Box<Expr>,
        /// Source code location of this type query
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// String interpolation ('Hello {name}')
    StringInterpolation {
        /// String literal and expression parts
        parts: Vec<StringPart>,
        /// Source code location of this string interpolation
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Parenthesized expressions ((expr))
    Parenthesized {
        /// Expression inside the parentheses
        expr: Box<Expr>,
        /// Source code location of this parenthesized expression
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Array literals ([1, 2, 3])
    Array {
        /// Elements contained in the array
        elements: Vec<Expr>,
        /// Source code location of this array literal
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },
}

/// Statement AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Let bindings (let x = 5)
    Let {
        /// Name of the variable being bound
        name: String,
        /// Optional type annotation
        type_annotation: Option<Type>,
        /// Optional initial value expression
        initializer: Option<Expr>,
        /// Source code location of this let binding
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Mutable variable declarations (let mutable x = 5)
    Mutable {
        /// Name of the mutable variable
        name: String,
        /// Optional type annotation
        type_annotation: Option<Type>,
        /// Optional initial value expression
        initializer: Option<Expr>,
        /// Source code location of this mutable declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Assignment statements (x = 10)
    Assignment {
        /// Left-hand side expression being assigned to
        target: Expr,
        /// Right-hand side value expression
        value: Expr,
        /// Source code location of this assignment
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Return statements (return expr)
    Return {
        /// Optional expression to return
        value: Option<Expr>,
        /// Source code location of this return statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Expression statements (`function_call()`)
    Expression {
        /// Expression being executed as a statement
        expr: Expr,
        /// Source code location of this expression statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Block statements ({ stmt1; stmt2; })
    Block {
        /// Statements contained in this block
        statements: Vec<Stmt>,
        /// Source code location of this block
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// If statements/expressions
    If {
        /// Boolean condition expression
        condition: Expr,
        /// Statement to execute if condition is true
        then_branch: Box<Stmt>,
        /// Optional statement to execute if condition is false
        else_branch: Option<Box<Stmt>>,
        /// Source code location of this if statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// For loops (for item in collection)
    For {
        /// Loop variable name
        variable: String,
        /// Expression that provides the collection to iterate over
        iterable: Expr,
        /// Loop body statement
        body: Box<Stmt>,
        /// Source code location of this for loop
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// While loops (while condition)
    While {
        /// Boolean condition expression
        condition: Expr,
        /// Loop body statement
        body: Box<Stmt>,
        /// Source code location of this while loop
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Loop statements (loop => body)
    Loop {
        /// Infinite loop body statement
        body: Box<Stmt>,
        /// Source code location of this loop statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Break statements
    Break {
        /// Source code location of this break statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Continue statements
    Continue {
        /// Source code location of this continue statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },
}

/// Top-level declaration AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// Function declarations
    Function {
        /// Name of the function
        name: String,
        /// Function parameters
        parameters: Vec<Parameter>,
        /// Optional return type annotation
        return_type: Option<Type>,
        /// Function body statement
        body: Stmt,
        /// Visibility modifier (public/private)
        visibility: Visibility,
        /// Whether this is an entry point function
        is_entry: bool,
        /// Optional documentation comment
        doc_comment: Option<String>,
        /// Source code location of this function declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Type declarations
    Type {
        /// Name of the type being declared
        name: String,
        /// Type definition (sum, product, or alias)
        type_def: TypeDef,
        /// Visibility modifier (public/private)
        visibility: Visibility,
        /// Optional documentation comment
        doc_comment: Option<String>,
        /// Source code location of this type declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Import declarations
    Import {
        /// Items being imported from the source
        items: Vec<ImportItem>,
        /// Source module or file path
        source: String,
        /// Source code location of this import declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    /// 64-bit signed integer literal
    Integer(i64),
    /// 64-bit floating point literal
    Float(f64),
    /// String literal value
    String(String),
    /// Boolean literal (true or false)
    Boolean(bool),
    /// Void/unit literal
    Void,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    /// Addition operator (+)
    Add,
    /// Subtraction operator (-)
    Subtract,
    /// Multiplication operator (*)
    Multiply,
    /// Division operator (/)
    Divide,
    /// Modulo operator (%)
    Modulo,
    /// Exponentiation operator (^)
    Power,

    // Comparison
    /// Equality operator (is)
    Equal,
    /// Inequality operator (is not)
    NotEqual,
    /// Less than operator (<)
    Less,
    /// Less than or equal operator (<=)
    LessEqual,
    /// Greater than operator (>)
    Greater,
    /// Greater than or equal operator (>=)
    GreaterEqual,
    /// Identity comparison operator (is)
    Is,
    /// Negative identity comparison operator (is not)
    IsNot,

    // Logical
    /// Logical AND operator (and)
    And,
    /// Logical OR operator (or)
    Or,
    /// Logical XOR operator (xor)
    Xor,

    // Bitwise
    /// Bitwise AND operator (band)
    BitAnd,
    /// Bitwise OR operator (bor)
    BitOr,
    /// Bitwise XOR operator (bxor)
    BitXor,
    /// Bitwise left shift operator (bshl)
    BitShiftLeft,
    /// Bitwise right shift operator (bshr)
    BitShiftRight,
    /// Bitwise unsigned right shift operator (bushr)
    BitUnsignedShiftRight,

    // Assignment
    /// Assignment operator (=)
    Assign,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    /// Numeric negation operator (-x)
    Negate,
    /// Logical negation operator (not x)
    Not,
    /// Bitwise negation operator (bnot x)
    BitNot,
    /// Unary plus operator (+x)
    Plus,
}

/// Type representations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Basic types
    Basic {
        /// Name of the basic type
        name: String,
        /// Source code location of this type
        span: Span,
    },

    /// Generic types (Array<T>, Result<T, E>)
    Generic {
        /// Name of the generic type
        name: String,
        /// Type arguments for the generic
        type_args: Vec<Type>,
        /// Source code location of this type
        span: Span,
    },

    /// Array types (T[])
    Array {
        /// Type of elements in the array
        element_type: Box<Type>,
        /// Source code location of this type
        span: Span,
    },

    /// Function types (f(int32, string): boolean)
    Function {
        /// Parameter types of the function
        parameters: Vec<Type>,
        /// Return type of the function
        return_type: Box<Type>,
        /// Source code location of this type
        span: Span,
    },
}

/// Function parameters
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// Name of the parameter
    pub name: String,
    /// Type of the parameter
    pub param_type: Type,
    /// Source code location of this parameter
    pub span: Span,
}

/// Type definitions for custom types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDef {
    /// Sum types (enums with variants)
    Sum {
        /// Variants of the sum type
        variants: Vec<Variant>,
        /// Source code location of this type definition
        span: Span,
    },

    /// Product types (structs)
    Product {
        /// Fields of the product type
        fields: Vec<Field>,
        /// Source code location of this type definition
        span: Span,
    },

    /// Type aliases
    Alias {
        /// Target type that this alias refers to
        target_type: Type,
        /// Source code location of this type definition
        span: Span,
    },
}

/// Variant for sum types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// Name of the variant
    pub name: String,
    /// Fields associated with this variant
    pub fields: Vec<Field>,
    /// Source code location of this variant
    pub span: Span,
}

/// Field for product types and variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Name of the field
    pub name: String,
    /// Type of the field
    pub type_annotation: Type,
    /// Source code location of this field
    pub span: Span,
}

/// Import items
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportItem {
    /// Import specific item (import foo from bar)
    Named {
        /// Name of the item being imported
        name: String,
        /// Optional alias for the imported item
        alias: Option<String>,
        /// Source code location of this import item
        span: Span,
    },

    /// Import all (import * from bar)
    Glob {
        /// Source code location of this glob import
        span: Span,
    },

    /// Import type (import type Foo from bar)
    Type {
        /// Name of the type being imported
        name: String,
        /// Optional alias for the imported type
        alias: Option<String>,
        /// Source code location of this type import
        span: Span,
    },
}

/// Visibility modifiers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Public visibility (accessible from outside the module)
    Public,
    /// Private visibility (only accessible within the module)
    Private,
}

/// String interpolation parts
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string part (plain text)
    Literal(String),
    /// Expression part that gets evaluated and interpolated
    Expression(Expr),
}

/// Complete program AST
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level declarations in the program
    pub declarations: Vec<Decl>,
    /// Source code location of the entire program
    pub span: Span,
    /// Unique identifier for this AST node
    pub id: NodeId,
}

impl AstNode for Expr {
    fn span(&self) -> Span {
        match *self {
            Self::Literal { span, .. }
            | Self::Identifier { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Call { span, .. }
            | Self::Index { span, .. }
            | Self::Member { span, .. }
            | Self::Cast { span, .. }
            | Self::TypeOf { span, .. }
            | Self::StringInterpolation { span, .. }
            | Self::Parenthesized { span, .. }
            | Self::Array { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Literal { id, .. }
            | Self::Identifier { id, .. }
            | Self::Binary { id, .. }
            | Self::Unary { id, .. }
            | Self::Call { id, .. }
            | Self::Index { id, .. }
            | Self::Member { id, .. }
            | Self::Cast { id, .. }
            | Self::TypeOf { id, .. }
            | Self::StringInterpolation { id, .. }
            | Self::Parenthesized { id, .. }
            | Self::Array { id, .. } => id,
        }
    }
}

impl AstNode for Stmt {
    fn span(&self) -> Span {
        match *self {
            Self::Let { span, .. }
            | Self::Mutable { span, .. }
            | Self::Assignment { span, .. }
            | Self::Return { span, .. }
            | Self::Expression { span, .. }
            | Self::Block { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Loop { span, .. }
            | Self::Break { span, .. }
            | Self::Continue { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Let { id, .. }
            | Self::Mutable { id, .. }
            | Self::Assignment { id, .. }
            | Self::Return { id, .. }
            | Self::Expression { id, .. }
            | Self::Block { id, .. }
            | Self::If { id, .. }
            | Self::For { id, .. }
            | Self::While { id, .. }
            | Self::Loop { id, .. }
            | Self::Break { id, .. }
            | Self::Continue { id, .. } => id,
        }
    }
}

impl AstNode for Decl {
    fn span(&self) -> Span {
        match *self {
            Self::Function { span, .. }
            | Self::Type { span, .. }
            | Self::Import { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Function { id, .. }
            | Self::Type { id, .. }
            | Self::Import { id, .. } => id,
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
        let symbol = match *self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "^",
            Self::Equal | Self::Is => "is",
            Self::NotEqual | Self::IsNot => "is not",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::BitAnd => "band",
            Self::BitOr => "bor",
            Self::BitXor => "bxor",
            Self::BitShiftLeft => "bshl",
            Self::BitShiftRight => "bshr",
            Self::BitUnsignedShiftRight => "bushr",
            Self::Assign => "=",
        };
        write!(f, "{symbol}")
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match *self {
            Self::Negate => "-",
            Self::Not => "not",
            Self::BitNot => "bnot",
            Self::Plus => "+",
        };
        write!(f, "{symbol}")
    }
}

impl TryFrom<TokenType> for BinaryOp {
    type Error = String;

    fn try_from(token_type: TokenType) -> Result<Self, Self::Error> {
        match token_type {
            TokenType::Plus => Ok(Self::Add),
            TokenType::Minus => Ok(Self::Subtract),
            TokenType::Multiply => Ok(Self::Multiply),
            TokenType::Divide => Ok(Self::Divide),
            TokenType::Modulo => Ok(Self::Modulo),
            TokenType::Power => Ok(Self::Power),
            TokenType::Less => Ok(Self::Less),
            TokenType::LessEqual => Ok(Self::LessEqual),
            TokenType::Greater => Ok(Self::Greater),
            TokenType::GreaterEqual => Ok(Self::GreaterEqual),
            TokenType::Is => Ok(Self::Is),
            TokenType::IsNot => Ok(Self::IsNot),
            TokenType::And => Ok(Self::And),
            TokenType::Or => Ok(Self::Or),
            TokenType::Xor => Ok(Self::Xor),
            TokenType::BitAnd => Ok(Self::BitAnd),
            TokenType::BitOr => Ok(Self::BitOr),
            TokenType::BitXor => Ok(Self::BitXor),
            TokenType::BitShiftLeft => Ok(Self::BitShiftLeft),
            TokenType::BitShiftRight => Ok(Self::BitShiftRight),
            TokenType::BitUnsignedShiftRight => Ok(Self::BitUnsignedShiftRight),
            TokenType::Assign => Ok(Self::Assign),
            _ => Err(format!("Cannot convert {token_type:?} to BinaryOp")),
        }
    }
}

impl TryFrom<TokenType> for UnaryOp {
    type Error = String;

    fn try_from(token_type: TokenType) -> Result<Self, Self::Error> {
        match token_type {
            TokenType::Minus => Ok(Self::Negate),
            TokenType::Plus => Ok(Self::Plus),
            TokenType::Not => Ok(Self::Not),
            TokenType::BitNot => Ok(Self::BitNot),
            _ => Err(format!("Cannot convert {token_type:?} to UnaryOp")),
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
        assert_eq!(BinaryOp::try_from(TokenType::Plus).unwrap(), BinaryOp::Add);
        assert_eq!(BinaryOp::try_from(TokenType::And).unwrap(), BinaryOp::And);
        assert_eq!(
            BinaryOp::try_from(TokenType::BitShiftLeft).unwrap(),
            BinaryOp::BitShiftLeft
        );
    }

    #[test]
    fn test_token_to_unary_op() {
        assert_eq!(UnaryOp::try_from(TokenType::Minus).unwrap(), UnaryOp::Negate);
        assert_eq!(UnaryOp::try_from(TokenType::Not).unwrap(), UnaryOp::Not);
        assert_eq!(UnaryOp::try_from(TokenType::BitNot).unwrap(), UnaryOp::BitNot);
    }
}
