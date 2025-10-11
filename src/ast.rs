//! Abstract Syntax Tree (AST) definitions for the Opalescent language
//!
//! This module contains all AST node types and related functionality.

#![expect(
    dead_code,
    reason = "AST nodes are partially implemented during language development"
)]

extern crate alloc;
use crate::token::{Span, TokenType};
use core::fmt;

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

/// Symbol information for ABI signature generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Name of the symbol (function/type/variable/constant)
    pub name: alloc::string::String,
    /// Type of the symbol
    pub symbol_type: SymbolType,
    /// Type signature for ABI compatibility
    pub signature: TypeSignature,
    /// Visibility of the symbol
    pub visibility: Visibility,
    /// Source location of the symbol
    pub source_location: Span,
}

/// Symbol type for ABI signature
#[derive(Debug, Clone, PartialEq, Eq)]
/// Symbol type for ABI signature
pub enum SymbolType {
    /// Function symbol (represents a function declaration)
    Function,
    /// Type symbol (represents a type declaration)
    Type,
    /// Variable symbol (represents a variable declaration)
    Variable,
    /// Constant symbol (represents a constant declaration)
    Constant,
}

/// ABI type signature for hot-reload compatibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSignature {
    /// String representation of the type signature
    pub signature: alloc::string::String,
}

/// Module path for dependency tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath(pub alloc::string::String);

/// Reusable metadata for hot-reload aware AST nodes
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HotReloadMetadata {
    /// Optional ABI symbol information for this node
    pub abi_symbol: Option<SymbolInfo>,
    /// Other modules this node depends on for hot-reload safety
    pub dependencies: alloc::vec::Vec<ModulePath>,
    /// Whether this node is eligible for hot reload without restart
    pub is_hot_reloadable: bool,
}

impl HotReloadMetadata {
    /// Metadata with defaults for functions (not hot-reloadable until validated)
    pub const fn for_function() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata with defaults for top-level `let` declarations
    pub const fn for_let_declaration() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: true,
        }
    }

    /// Metadata with defaults for expressions (e.g., lambdas)
    pub const fn for_expression() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata defaults for type declarations (not hot-reloadable by default)
    pub const fn for_type_declaration() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata defaults for imports (never hot-reloadable)
    pub const fn for_import() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
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
        /// Return type of the lambda
        ///
        /// Explicitly required for all lambdas to ensure type safety and
        /// hot-reload compatibility. Can be a primitive, generic, or complex type.
        /// Example: `Type::Basic("int32")` or `Type::Generic("U")`
        return_type: Type,
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
        metadata: HotReloadMetadata,
        /// Source code location of this lambda expression
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
        /// Optional documentation comment
        doc_comment: Option<String>,
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
        /// Optional documentation comment
        doc_comment: Option<String>,
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

impl Type {
    /// Convenience accessor for the span associated with this type node
    #[must_use]
    pub const fn span(&self) -> Span {
        match *self {
            Self::Basic { span, .. }
            | Self::Generic { span, .. }
            | Self::Array { span, .. }
            | Self::Function { span, .. } => span,
        }
    }
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
            | Self::Array { span, .. }
            | Self::Lambda { span, .. } => span,
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
            | Self::Array { id, .. }
            | Self::Lambda { id, .. } => id,
        }
    }

    fn abi_symbols(&self) -> alloc::vec::Vec<SymbolInfo> {
        match *self {
            Self::Lambda { ref metadata, .. } => metadata.abi_symbol.iter().cloned().collect(),
            _ => alloc::vec::Vec::new(),
        }
    }

    fn dependencies(&self) -> alloc::vec::Vec<ModulePath> {
        match *self {
            Self::Lambda { ref metadata, .. } => metadata.dependencies.clone(),
            _ => alloc::vec::Vec::new(),
        }
    }

    fn is_hot_reloadable(&self) -> bool {
        matches!(*self, Self::Lambda { ref metadata, .. } if metadata.is_hot_reloadable)
    }
}

impl AstNode for Stmt {
    fn span(&self) -> Span {
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

    fn node_id(&self) -> NodeId {
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

impl AstNode for Decl {
    fn span(&self) -> Span {
        match *self {
            Self::Function { span, .. }
            | Self::Type { span, .. }
            | Self::Import { span, .. }
            | Self::Let { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Function { id, .. }
            | Self::Type { id, .. }
            | Self::Import { id, .. }
            | Self::Let { id, .. } => id,
        }
    }

    fn abi_symbols(&self) -> alloc::vec::Vec<SymbolInfo> {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.abi_symbol.iter().cloned().collect(),
        }
    }

    fn dependencies(&self) -> alloc::vec::Vec<ModulePath> {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.dependencies.clone(),
        }
    }

    fn is_hot_reloadable(&self) -> bool {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.is_hot_reloadable,
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
        assert_eq!(
            UnaryOp::try_from(TokenType::Minus).unwrap(),
            UnaryOp::Negate
        );
        assert_eq!(UnaryOp::try_from(TokenType::Not).unwrap(), UnaryOp::Not);
        assert_eq!(
            UnaryOp::try_from(TokenType::BitNot).unwrap(),
            UnaryOp::BitNot
        );
    }
}
