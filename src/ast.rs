//! Abstract Syntax Tree (AST) definitions for the Opalescent language
//!
//! This module contains all AST node types and related functionality.

#![expect(
    dead_code,
    reason = "AST nodes are partially implemented during language development"
)]
#![expect(
    clippy::pub_use,
    reason = "Re-exporting from submodules maintains a clean public API - submodules are implementation details"
)]

mod documentation;
mod helpers;
mod metadata;
mod node_impls;
mod operators;
mod types;

extern crate alloc;
use crate::token::Span;
use alloc::string::String;

// Re-export operators from the operators module for public use
pub use self::operators::{BinaryOp, UnaryOp};

// Re-export type structures from the types module for public use
pub use self::types::{Field, Parameter, Type, TypeDef, Variant};

// Re-export documentation structures for public use
pub use self::documentation::Documentation;

// Re-export hot-reload metadata structures for public use
pub use self::metadata::{HotReloadMetadata, ModulePath, SymbolInfo};

/// A unique identifier for AST nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Base trait for all AST nodes, extended for hot-reload metadata
///
/// # Hot Reload Metadata
/// This trait is designed to support Opalescent's hot-reloading architecture.
/// It provides ABI symbol info, dependency tracking, and reloadable status for each node.
///
/// All core methods are required for ABI signature generation and dynamic reload.
pub trait AstNode {
    /// Returns the source span of this AST node
    fn span(&self) -> Span;
    /// Returns the unique node ID of this AST node
    fn node_id(&self) -> NodeId;
    /// Returns ABI-relevant symbol info for this node (for ABI signature generation)
    fn abi_symbols(&self) -> alloc::vec::Vec<SymbolInfo> {
        alloc::vec::Vec::new()
    }
    /// Returns a list of module dependencies for this node
    fn dependencies(&self) -> alloc::vec::Vec<ModulePath> {
        alloc::vec::Vec::new()
    }
    /// Returns true if this node is hot-reloadable (eligible for dynamic reload)
    fn is_hot_reloadable(&self) -> bool {
        false
    }
}

impl Expr {
    /// Retrieve the source span associated with this expression in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
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
            | Self::Array { span, .. }
            | Self::Lambda { span, .. }
            | Self::Guard { span, .. }
            | Self::Propagate { span, .. } => span,
        }
    }

    /// Retrieve the unique identifier associated with this expression in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
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
            | Self::Array { id, .. }
            | Self::Lambda { id, .. }
            | Self::Guard { id, .. }
            | Self::Propagate { id, .. } => id,
        }
    }
}

impl Stmt {
    /// Retrieve the source span associated with this statement in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Let { span, .. }
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

    /// Retrieve the unique identifier associated with this statement in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        match *self {
            Self::Let { id, .. }
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

impl Decl {
    /// Retrieve the source span associated with this declaration in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Function { span, .. }
            | Self::Type { span, .. }
            | Self::Import { span, .. }
            | Self::Let { span, .. } => span,
        }
    }

    /// Retrieve the unique identifier associated with this declaration in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        match *self {
            Self::Function { id, .. }
            | Self::Type { id, .. }
            | Self::Import { id, .. }
            | Self::Let { id, .. } => id,
        }
    }
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

    /// Lambda expressions (f(x: T): U => expr, f<T, U>(x: T): U => block)
    ///
    /// Lambda expressions represent first-class functions in Opalescent. They can be:
    /// - Assigned to variables: `let add = f(x: int32, y: int32): int32 => x + y`
    /// - Passed as arguments: `map(arr, f(x: int32): int32 => x * 2)`
    /// - Used inline: `filter(items, f(item: T): boolean => item.is_valid())`
    ///
    /// Syntax variations:
    /// - Simple lambda: `f(x: T): U => expression`
    /// - Generic lambda: `f<T, U>(x: T, fn: f(T): U): U => fn(x)`
    /// - No parameters: `f(): T => constant_value`
    /// - Block body: `f(x: T): U => { statements; return result; }`
    ///
    /// Integration with type system:
    /// - Parameters are fully typed (no inference across lambda boundaries)
    /// - Return type is explicit (required for clarity and hot-reload compatibility)
    /// - Generic parameters are resolved at call site
    /// - Function types can be used as parameter types: `f(callback: f(T): U)`
    ///
    /// Hot-reload considerations:
    /// - Lambda expressions maintain ABI compatibility through explicit typing
    /// - Generic instantiations are tracked for dependency management
    /// - Closure captures (if implemented) affect hot-reload boundaries
    Lambda {
        /// Optional generic type parameters (<T, U>)
        ///
        /// When present, these define type variables that can be used in parameter
        /// and return types. Generic parameters are resolved at the call site.
        /// Example: `f<T, U>(mapper: f(T): U, value: T): U`
        generic_params: Option<Vec<String>>,
        /// Function parameters with types
        ///
        /// All parameters must have explicit types. Parameter names follow
        /// `snake_case` convention. Types can reference generic parameters.
        /// Example: `[Parameter { name: "x", param_type: Type::Basic("T") }]`
        params: Vec<Parameter>,
        /// Return types of the lambda in declaration order.
        ///
        /// Backward compatibility: single-return lambdas use a vector with one element.
        return_types: Vec<Type>,
        /// Error types that this lambda may produce
        ///
        /// Stores raw type names from parsing (e.g., `["ParseError", "IoError"]`).
        /// Resolution to `CoreType` happens during type checking.
        /// Empty vector indicates no errors declared (default).
        /// Used for error propagation analysis in functional composition.
        error_types: Vec<String>,
        /// Lambda body (expression or block)
        ///
        /// Single expressions are more common for functional programming patterns.
        /// Block bodies are used for complex logic with multiple statements.
        /// See `LambdaBody` for details on each variant.
        body: LambdaBody,
        /// Captured variables from enclosing scope (TODO: implement in closure phase)
        ///
        /// When closures are implemented, this will track which variables from
        /// the enclosing scope are captured by this lambda. Critical for:
        /// - Hot-reload dependency tracking
        /// - Memory management in LLVM backend
        /// - ABI signature generation for module boundaries
        captured_variables: Vec<String>, // TODO: Phase 4-5
        /// ABI compatibility metadata for hot-reload (TODO: implement in hot-reload phase)  
        ///
        /// Metadata needed for hot-reload compatibility, including:
        /// - Function signature hash for ABI compatibility checks
        /// - Dependency tracking for incremental compilation
        /// - Symbol information for dynamic linking
        ///
        /// Boxed to reduce the size of the Lambda variant in the Expr enum.
        metadata: Box<HotReloadMetadata>,
        /// Source code location of this lambda expression
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Guard expression for error handling (guard expr into name else handler)
    ///
    /// Guard expressions provide structured error handling by branching on success/error.
    /// They bind the success value to a name and execute an else branch on error.
    ///
    /// Syntax variations:
    /// - Basic: `guard read_line() into line else handle_error()`
    /// - With type annotation: `guard parse(s) into value: int32 else return default`
    /// - Mutable binding: `guard get_config() into mutable cfg else fallback_config`
    /// - Block else: `guard validate(data) into clean_data else { log_error(); return; }`
    ///
    /// Type checking semantics:
    /// - The guarded expression must be a call to a function with declared error types
    /// - Success value is bound to `binding_name` with inferred or annotated type
    /// - Else branch must handle the error type(s) from the guarded expression
    /// - Result type of the guard expression is the success type
    /// - Symbol table registers `binding_name` for the scope following the guard
    ///
    /// Error handling integration:
    /// - Guard is used when the caller wants to handle errors locally
    /// - Propagate is used when errors should bubble up to the caller
    /// - Together they provide explicit, type-safe error handling without exceptions
    Guard {
        /// Expression being guarded (typically a call that may produce errors)
        expr: Box<Expr>,
        /// Name to bind the success value to
        binding_name: String,
        /// Optional type annotation for the success value
        binding_type: Option<Type>,
        /// Whether the binding is mutable
        is_mutable: bool,
        /// Else branch executed on error (statement or expression)
        ///
        /// Stores a statement to handle both expression and block forms:
        /// - Expression form: wrapped in `Stmt::Expression`
        /// - Block form: stored as `Stmt::Block`
        else_branch: Box<Stmt>,
        /// Source code location of this guard expression
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Propagate expression for error bubbling (`propagate call_expr`)
    ///
    /// Propagate expressions automatically bubble errors up to the caller when the
    /// current function declares compatible error types.
    ///
    /// Syntax:
    /// - `let n = propagate string_to_int32(s)`
    /// - `let data = propagate read_file(path)`
    ///
    /// Type checking semantics:
    /// - Only valid inside a function/lambda that declares error types
    /// - Inner expression must be a call to a function with error types
    /// - Error types of inner call must be a subset of current function's error types
    /// - Result type is the success type of the inner call
    /// - Errors are automatically propagated to caller without explicit handling
    ///
    /// Error compatibility:
    /// - Requires: `inner.error_types ⊆ current_function.error_types`
    /// - Prevents accidental error suppression
    /// - Provides clear error flow through the call stack
    /// - Works with guard to give complete error handling coverage
    Propagate {
        /// Call expression whose errors should be propagated
        ///
        /// Parser validates this is an `Expr::Call` variant.
        /// Type checker ensures the call's function has error types.
        call: Box<Expr>,
        /// Source code location of this propagate expression
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },
}

/// Shared metadata for `let` bindings used in statements and declarations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetBinding {
    /// Name of the variable being bound
    pub name: String,
    /// Optional explicit type annotation
    pub type_annotation: Option<Type>,
    /// Whether the binding is mutable
    pub is_mutable: bool,
    /// Source code location of this binding
    pub span: Span,
    /// Unique identifier for this binding
    pub id: NodeId,
}

/// Labeled control-flow payload produced by `break` or `continue`.
#[derive(Debug, Clone, PartialEq)]
pub struct LabeledValue {
    /// Name assigned to the payload (e.g., `result` in `break result: value`).
    pub label: String,
    /// Expression evaluated to produce the payload.
    pub value: Expr,
    /// Source span covering the entire `label: value` pair.
    pub span: Span,
    /// Unique identifier for this payload node.
    pub id: NodeId,
}

/// Statement AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Let bindings (let x = 5, let mutable y = 10)
    Let {
        /// Shared binding metadata
        binding: LetBinding,
        /// Optional initial value expression
        initializer: Option<Expr>,
        /// Source code span for the full statement
        span: Span,
        /// Unique identifier for this statement node
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

    /// Return statements (`return expr`, `return label1: expr1, label2: expr2`)
    Return {
        /// Return payload values in declaration order.
        ///
        /// Single unlabeled returns are represented as a vector with one value where
        /// `label` is empty.
        values: Vec<LabeledValue>,
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
        /// Labeled payload values returned alongside the loop exit.
        values: Vec<LabeledValue>,
        /// Source code location of this break statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },

    /// Continue statements
    Continue {
        /// Labeled payload values forwarded to the next loop iteration.
        values: Vec<LabeledValue>,
        /// Source code location of this continue statement
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
    },
}

impl Program {
    /// Retrieve the source span associated with the entire program in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Retrieve the unique identifier associated with this program in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        self.id
    }
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
        /// Optional return type annotations in declaration order.
        ///
        /// Backward compatibility: single-return functions use a vector with one element.
        return_types: Option<Vec<Type>>,
        /// Error types that this function may produce
        ///
        /// Stores raw type names from parsing (e.g., `["ParseError", "IoError"]`).
        /// Resolution to `CoreType` happens during type checking.
        /// Empty vector indicates no errors declared (default).
        /// Used for error propagation analysis and ABI signature generation.
        error_types: Vec<String>,
        /// Function body statement
        body: Stmt,
        /// Visibility modifier (public/private)
        visibility: Visibility,
        /// Whether this is an entry point function
        is_entry: bool,
        /// Optional structured documentation derived from doc comments
        doc_comment: Option<Documentation>,
        /// Source code location of this function declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
        /// Hot-reload metadata
        metadata: HotReloadMetadata,
    },

    /// Type declarations
    Type {
        /// Name of the type being declared
        name: String,
        /// Type definition (sum, product, or alias)
        type_def: TypeDef,
        /// Visibility modifier (public/private)
        visibility: Visibility,
        /// Optional structured documentation derived from doc comments
        doc_comment: Option<Documentation>,
        /// Source code location of this type declaration
        span: Span,
        /// Unique identifier for this AST node
        id: NodeId,
        /// Hot-reload metadata
        metadata: HotReloadMetadata,
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
        /// Hot-reload metadata
        metadata: HotReloadMetadata,
    },

    /// Let declarations (variable declarations that can include lambda expressions)
    ///
    /// Let declarations create immutable bindings by default, with optional mutability.
    /// They are commonly used to assign lambda expressions to named variables,
    /// creating reusable functions within the module scope.
    ///
    /// Syntax variations:
    /// - Simple binding: `let x = 42`
    /// - With type annotation: `let x: int32 = 42`
    /// - Mutable binding: `let mutable x = 42`
    /// - Lambda assignment: `let add = f(x: int32, y: int32): int32 => x + y`
    /// - Generic lambda: `let map = f<T, U>(arr: T[], fn: f(T): U): U[] => ...`
    /// - Public declaration: `public let utility_fn = f(...): ... => ...`
    ///
    /// Type inference:
    /// - Type annotations are optional when the type can be inferred from initializer
    /// - Lambda expressions have explicit types, making inference straightforward
    /// - Complex expressions may require explicit annotations for clarity
    ///
    /// Hot-reload integration:
    /// - Public let declarations become part of the module's ABI
    /// - Lambda assignments are tracked for dependency analysis
    /// - Mutable bindings may affect hot-reload compatibility
    /// - Generic lambdas require special handling for type instantiation tracking
    Let {
        /// Shared binding metadata
        binding: LetBinding,
        /// Initializer expression (required for let declarations)
        initializer: Expr,
        /// Visibility modifier (public/private)
        visibility: Visibility,
        /// Optional structured documentation derived from doc comments
        doc_comment: Option<Documentation>,
        /// Source span for this declaration
        span: Span,
        /// Unique identifier for this declaration node
        id: NodeId,
        /// Hot-reload metadata
        metadata: HotReloadMetadata,
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

/// Lambda body representation
///
/// Lambda bodies can be either single expressions or blocks of statements.
/// The choice affects both syntax and semantics:
///
/// Expression bodies:
/// - Syntax: `f(x: T): U => expression`
/// - Direct evaluation of the expression as the return value
/// - Common for functional programming patterns (map, filter, reduce)
/// - More concise for simple transformations
/// - Example: `f(x: int32): int32 => x * 2`
///
/// Block bodies:
/// - Syntax: `f(x: T): U => { statements; return result; }`
/// - Multiple statements with explicit `return` required
/// - Used for complex logic, local variables, control flow
/// - Better for imperative-style implementations
/// - Example: `f(x: int32): int32 => { let doubled = x * 2; return doubled + 1; }`
///
/// Type checking considerations:
/// - Expression bodies: the expression's type must match the return type
/// - Block bodies: all return statements must have compatible types
/// - Both forms must have explicit return types (no inference across lambda boundaries)
#[derive(Debug, Clone, PartialEq)]
pub enum LambdaBody {
    /// Single expression body (f(x): T => expr)
    ///
    /// The expression is evaluated and its result becomes the lambda's return value.
    /// The expression's type must be assignable to the declared return type.
    /// This is the preferred form for functional programming patterns.
    Expression(Box<Expr>),
    /// Block body with statements (f(x): T => { statements; return expr; })
    ///
    /// A sequence of statements that must end with a `return` statement.
    /// Allows for local variable declarations, control flow, and complex logic.
    /// All `return` statements within the block must have compatible types.
    Block(Vec<Stmt>),
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
    fn test_statement_span_and_node_id() {
        let binding = LetBinding {
            name: "value".to_owned(),
            type_annotation: None,
            is_mutable: false,
            span: dummy_span(),
            id: dummy_node_id(),
        };

        let stmt = Stmt::Let {
            binding,
            initializer: None,
            span: dummy_span(),
            id: dummy_node_id(),
        };

        assert_eq!(stmt.span(), dummy_span());
        assert_eq!(stmt.node_id(), dummy_node_id());
    }

    #[test]
    fn test_declaration_span_and_node_id() {
        let decl = Decl::Import {
            items: alloc::vec![ImportItem::Glob { span: dummy_span() }],
            source: "./module".to_owned(),
            span: dummy_span(),
            id: dummy_node_id(),
            metadata: HotReloadMetadata::for_import(),
        };

        assert_eq!(decl.span(), dummy_span());
        assert_eq!(decl.node_id(), dummy_node_id());
    }

    #[test]
    fn test_import_item_span_accessor() {
        let item = ImportItem::Named {
            name: "value".to_owned(),
            alias: Some("alias".to_owned()),
            span: dummy_span(),
        };

        assert_eq!(item.span(), dummy_span());
    }
}
