//! Type System Core for Opalescent Language
//!
//! This module provides the core type checking, type inference, and type safety
//! validation for the Opalescent programming language. It ensures static type safety
//! while providing helpful error messages and supporting advanced features like
//! generics and algebraic data types.
//!
//! ## Phase Integration
//!
//! This module is used by:
//! - **Phase 1**: Foundation for parser type annotations and AST type validation
//! - **Phase 2**: Function and variable type checking, type inference for lambdas and let bindings
//! - **Phase 3**: ADT validation, pattern matching, and generic type instantiation
//! - **Phase 4**: Cross-module type checking and import validation
//! - **Phase 5**: Type information for LLVM code generation
//! - **Phase 6**: ABI signature generation for hot reload compatibility checking
//!
//! ## Current Status & Future Enhancements
//!
//! ### Error Categories
//!
//! - [`TypeError::TypeNotFound`]: Type reference not in scope
//! - [`TypeError::TypeMismatch`]: Incompatible types in expression
//! - [`TypeError::InvalidOperation`]: Operation not supported for type
//! - [`TypeError::UnificationFailed`]: Type inference failure
//! - [`TypeError::OccursCheckFailed`]: Infinite type detected
//! - [`TypeError::ConstraintSolvingFailed`]: Constraint system failure
//!
//! ## Ownership Strategy
//!
//! - `lookup_type`: Returns reference (type environment owns the type)
//! - `ast_type_to_core_type`: Returns owned value (creates new `CoreType`)
//! - `unify`: Returns owned `Substitution` (creates new mapping)
//! - `fresh_type_var`: Returns owned `CoreType::Variable` (creates new type variable)
//!
//! ## Examples
//!
//! ### Basic Type Checking
//!
//! ```rust,ignore
//! use opalescent::type_system::{TypeChecker, CoreType};
//!
//! let checker = TypeChecker::new();
//! assert!(checker.environment().has_type("int32"));
//! assert!(checker.types_compatible(&CoreType::Int32, &CoreType::Int32));
//! ```
//!
//! ### Type Unification
//!
//! ```rust,ignore
//! let mut checker = TypeChecker::new();
//! let var = checker.fresh_type_var("x".to_owned())?;
//! let subst = checker.unify(&var, &CoreType::Int32)?;
//! ```
//!
//! ## Testing
//!
//! The module includes comprehensive unit tests covering:
//! - Type environment operations
//! - AST to `CoreType` conversion
//! - Type unification algorithm
//! - Occurs check validation
//! - Error message formatting
//! - ADT type validation
//! - Pattern matching type checking

#![expect(
    dead_code,
    reason = "Type system is foundational infrastructure being built incrementally"
)]

extern crate alloc;

use crate::ast::AstNode;
use crate::ast::{
    BinaryOp, Decl, Expr, LambdaBody, LetBinding, LiteralValue, Parameter, Program, Stmt,
    StringPart, Type, UnaryOp, Visibility as AstVisibility,
};
use crate::token::Span;
use alloc::{collections::BTreeMap, fmt, string::String, vec::Vec};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Represents type variables used in type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    /// Unique identifier for this type variable
    pub id: usize,
    /// Human-readable name for debugging
    pub name: String,
}

impl TypeVar {
    /// Create a new type variable with the given id and name for debugging context.
    pub const fn new(id: usize, name: String) -> Self {
        Self { id, name }
    }
}

/// Represents the core types supported by the Opalescent language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreType {
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit unsigned integer
    UInt16,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit floating point
    Float32,
    /// 64-bit floating point
    Float64,
    /// Unicode string
    String,
    /// Boolean type
    Boolean,
    /// Unit type (empty value)
    Unit,
    /// Type variable for inference
    Variable(TypeVar),
    /// Array type with element type
    Array(Box<CoreType>),
    /// Function type with parameter types and return type
    Function {
        /// Parameter types
        parameters: Vec<CoreType>,
        /// Return type
        return_type: Box<CoreType>,
    },
    /// Generic type with name and type arguments
    Generic {
        /// Name of the generic type
        name: String,
        /// Type arguments
        type_args: Vec<CoreType>,
    },
}

impl fmt::Display for CoreType {
    /// Format `CoreType` for user-friendly error messages
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Int8 => write!(f, "int8"),
            Self::Int16 => write!(f, "int16"),
            Self::Int32 => write!(f, "int32"),
            Self::Int64 => write!(f, "int64"),
            Self::UInt8 => write!(f, "uint8"),
            Self::UInt16 => write!(f, "uint16"),
            Self::UInt32 => write!(f, "uint32"),
            Self::UInt64 => write!(f, "uint64"),
            Self::Float32 => write!(f, "float32"),
            Self::Float64 => write!(f, "float64"),
            Self::String => write!(f, "string"),
            Self::Boolean => write!(f, "boolean"),
            Self::Unit => write!(f, "unit"),
            Self::Variable(ref var) => write!(f, "{}", var.name.as_str()),
            Self::Array(ref element_type) => write!(f, "[{element_type}]"),
            Self::Function {
                ref parameters,
                ref return_type,
            } => {
                write!(f, "(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Self::Generic {
                ref name,
                ref type_args,
            } => {
                write!(f, "{name}")?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
    }
}

/// Classification of numeric types used for cast and operation validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
    /// Signed integer family (int8, int16, int32, int64).
    SignedInt,
    /// Unsigned integer family (uint8, uint16, uint32, uint64).
    UnsignedInt,
    /// Floating point family (float32, float64).
    Float,
}

/// Type checking errors that can occur during type analysis
#[derive(Error, Debug, Clone, PartialEq, Eq, Diagnostic)]
pub enum TypeError {
    /// Type was not found in the current scope
    #[error("Type '{type_name}' not found")]
    #[diagnostic(
        code(opalescent::type_system::type_not_found),
        help("Check that the type is defined or imported in this scope")
    )]
    TypeNotFound {
        /// Name of the type that was not found
        type_name: String,
        #[label("undefined type")]
        /// Source span highlighting where the type was referenced
        span: SourceSpan,
    },

    /// Symbol (variable/function) was not found in the current scope
    #[error("Symbol '{name}' not found in this scope")]
    #[diagnostic(
        code(opalescent::type_system::symbol_not_found),
        help("Ensure the symbol is declared before use or imported from the correct module")
    )]
    SymbolNotFound {
        /// Name of the missing symbol
        name: String,
        #[label("undefined symbol")]
        /// Location where the symbol was referenced
        span: SourceSpan,
    },

    /// Types do not match in an expression
    #[error("Type mismatch: expected '{expected}', found '{found}'")]
    #[diagnostic(
        code(opalescent::type_system::type_mismatch),
        help(
            "Consider using an explicit cast if this conversion is intentional, or change one of the types to match"
        )
    )]
    TypeMismatch {
        /// Expected type name
        expected: String,
        /// Actually found type name
        found: String,
        #[label("type '{found}' found here")]
        /// Source span highlighting where the incompatible type was found
        found_span: SourceSpan,
        #[label("type '{expected}' expected here")]
        /// Source span highlighting where the expected type was required
        expected_span: Option<SourceSpan>,
    },

    /// Invalid type operation
    #[error("Invalid operation '{operation}' for type '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::invalid_operation),
        help(
            "This operation is not supported for this type. Check the language reference for valid operations"
        )
    )]
    InvalidOperation {
        /// Operation that was attempted
        operation: String,
        /// Name of the type the operation was attempted on
        type_name: String,
        #[label("invalid operation here")]
        /// Source span highlighting where the invalid operation was attempted
        span: SourceSpan,
    },

    /// Generic type parameter not found
    #[error("Generic type parameter '{param_name}' not found")]
    #[diagnostic(
        code(opalescent::type_system::generic_parameter_not_found),
        help("Check that the generic parameter is declared in the type or function signature")
    )]
    GenericParameterNotFound {
        /// Name of the generic parameter that was not found
        param_name: String,
        #[label("undefined generic parameter")]
        /// Source span highlighting where the parameter was referenced
        span: SourceSpan,
    },

    /// Unification failed between two types
    #[error("Cannot unify types '{left}' and '{right}'")]
    #[diagnostic(
        code(opalescent::type_system::unification_failed),
        help(
            "These types are incompatible. Consider using an explicit cast or changing one of the types"
        )
    )]
    UnificationFailed {
        /// Left type in the unification
        left: String,
        /// Right type in the unification
        right: String,
        #[label("type '{left}' found here")]
        /// Source span highlighting the left type location
        left_span: SourceSpan,
        #[label("type '{right}' expected here")]
        /// Source span highlighting the right type location
        right_span: SourceSpan,
    },

    /// Occurs check failed (infinite type)
    #[error("Occurs check failed: type variable '{var_name}' occurs in '{type_name}'")]
    #[diagnostic(
        code(opalescent::type_system::occurs_check_failed),
        help(
            "This would create an infinite type. Check for recursive type definitions or incorrect type constraints"
        )
    )]
    OccursCheckFailed {
        /// Name of the type variable
        var_name: String,
        /// Name of the type it occurs in
        type_name: String,
        #[label("infinite type would be created here")]
        /// Source span highlighting where the occurs check failed
        span: SourceSpan,
    },

    /// Constraint solving failed
    #[error("Constraint solving failed: {reason}")]
    #[diagnostic(
        code(opalescent::type_system::constraint_solving_failed),
        help(
            "The type constraints for this expression could not be satisfied. Review the types involved"
        )
    )]
    ConstraintSolvingFailed {
        /// Reason for the failure
        reason: String,
        #[label("constraint violation")]
        /// Source span highlighting where the constraint failed
        span: SourceSpan,
    },

    /// Type variable ID overflow occurred
    #[error("Type variable ID overflow - too many type variables generated")]
    #[diagnostic(
        code(opalescent::type_system::type_variable_overflow),
        help(
            "This is an internal compiler error. The program has generated too many type variables"
        )
    )]
    TypeVariableOverflow {
        #[label("overflow occurred during type inference here")]
        /// Source span highlighting where the overflow occurred
        span: SourceSpan,
    },

    /// Feature not yet implemented
    #[error("Feature not yet implemented: {feature}")]
    #[diagnostic(
        code(opalescent::type_system::not_implemented),
        help(
            "This feature is planned but not yet available. Check the project roadmap for implementation status"
        )
    )]
    NotImplementedYet {
        /// Description of the feature not yet implemented
        feature: String,
        #[label("unimplemented feature used here")]
        /// Source span highlighting where the unimplemented feature was used
        span: SourceSpan,
    },
}

impl TypeError {
    /// Convert AST Span to miette `SourceSpan`
    ///
    /// This utility method provides consistent conversion from the compiler's internal
    /// [`Span`] type to miette's [`SourceSpan`] for error reporting.
    pub fn span_from_span(span: Span) -> SourceSpan {
        let start: usize = span.start.offset;
        let len = span.end.offset.saturating_sub(span.start.offset);
        SourceSpan::new(start.into(), len)
    }

    /// Create a default/unknown source span for errors without location information
    ///
    /// Used as a temporary measure for code that doesn't yet track source locations.
    /// All code should eventually be updated to provide actual spans.
    pub fn unknown_span() -> SourceSpan {
        SourceSpan::new(0.into(), 0)
    }
}

/// Type constraints used in constraint-based type inference
///
/// These constraints are collected during AST traversal and then solved
/// to determine concrete types for type variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    /// Two types must be equal
    Equality(CoreType, CoreType),
    /// A type must have a specific field with a given type
    HasField(CoreType, String, CoreType),
    /// A type must be callable with specific argument and return types
    Callable(CoreType, Vec<CoreType>, CoreType),
}

/// Memory layout information for a type
///
/// Required for Phase 6 hot reload ABI compatibility checking.
/// This ensures that types maintain binary compatibility across reloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLayout {
    /// Size of the type in bytes
    pub size: usize,
    /// Alignment requirement in bytes
    pub align: usize,
}

impl CoreType {
    /// Get the memory layout for this type
    ///
    /// This is critical for Phase 6 hot reload ABI compatibility checking.
    /// Types with different memory layouts cannot be hot-swapped safely.
    ///
    /// # Returns
    ///
    /// The memory layout (size and alignment) for this type.
    ///
    /// # Note
    ///
    /// Currently returns placeholder values. These should be updated to match
    /// the actual LLVM backend layout in Phase 5.
    pub const fn memory_layout(&self) -> MemoryLayout {
        match *self {
            Self::Int8 | Self::UInt8 | Self::Boolean => MemoryLayout { size: 1, align: 1 },
            Self::Int16 | Self::UInt16 => MemoryLayout { size: 2, align: 2 },
            Self::Int32 | Self::UInt32 | Self::Float32 => MemoryLayout { size: 4, align: 4 },
            Self::Int64 | Self::UInt64 | Self::Float64 => MemoryLayout { size: 8, align: 8 },
            Self::Unit | Self::Variable(_) => MemoryLayout { size: 0, align: 1 },
            // Pointers (strings, arrays, functions) are pointer-sized
            Self::String | Self::Array(_) | Self::Function { .. } | Self::Generic { .. } => {
                MemoryLayout {
                    size: core::mem::size_of::<usize>(),
                    align: core::mem::align_of::<usize>(),
                }
            }
        }
    }
}

/// Symbol visibility for exported items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Private to the module
    Private,
    /// Exported with `public` keyword
    Public,
    /// Entry point function
    Entry,
}

/// Type of symbol for ABI signature generation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolType {
    /// Function symbol
    Function,
    /// Type definition symbol
    Type,
    /// Variable or constant symbol
    Variable,
    /// Constant symbol
    Constant,
}

/// Symbol information for hot reload ABI signature generation
///
/// Required for Phase 6 hot reload. Tracks exported symbols and their
/// type signatures for ABI compatibility checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Symbol name
    pub name: String,
    /// Type of symbol
    pub symbol_type: SymbolType,
    /// Type signature of the symbol
    pub core_type: CoreType,
    /// Visibility (private, public, entry)
    pub visibility: Visibility,
    /// Source location for error reporting
    pub source_location: Span,
}

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

/// Represents a single scope in the symbol table
#[derive(Debug, Clone)]
struct Scope {
    /// Parent scope ID (None for global scope)
    parent: Option<ScopeId>,
    /// Symbols defined in this scope
    symbols: BTreeMap<String, SymbolInfo>,
}

/// Symbol table for tracking symbols in scope
///
/// Required for Phase 2 type checking and Phase 6 hot reload.
/// Manages symbol visibility and resolution across nested scopes.
///
/// ## Scope Management
///
/// The symbol table supports nested scopes for:
/// - Global scope (module-level definitions)
/// - Function parameter scopes
/// - Block scopes (if, while, for, etc.)
/// - Lambda expression scopes
///
/// Scopes are organized hierarchically, with child scopes able to
/// access symbols from parent scopes but not vice versa.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Stack of all scopes
    scopes: Vec<Scope>,
    /// Currently active scope
    current_scope: ScopeId,
}

impl SymbolTable {
    /// Create a new symbol table with a global scope
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                parent: None,
                symbols: BTreeMap::new(),
            }],
            current_scope: ScopeId(0),
        }
    }

    /// Enter a new nested scope
    ///
    /// Creates a new child scope and makes it the current scope.
    /// Returns the ID of the newly created scope.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut table = SymbolTable::new();
    /// let function_scope = table.enter_scope();
    /// // Register function parameters in this scope
    /// table.exit_scope();
    /// ```
    pub fn enter_scope(&mut self) -> ScopeId {
        let scope_id = ScopeId(self.scopes.len());
        self.scopes.push(Scope {
            parent: Some(self.current_scope),
            symbols: BTreeMap::new(),
        });
        self.current_scope = scope_id;
        scope_id
    }

    /// Exit the current scope, returning to the parent scope
    ///
    /// # Panics
    ///
    /// Panics if attempting to exit the global scope (scope 0).
    /// This is a programming error that should never happen in correct usage.
    pub fn exit_scope(&mut self) {
        assert!(
            self.current_scope.0 != 0,
            "Cannot exit global scope - this is a bug"
        );
        if let Some(parent) = self.scopes[self.current_scope.0].parent {
            self.current_scope = parent;
        }
    }

    /// Get the current scope ID
    pub const fn current_scope_id(&self) -> ScopeId {
        self.current_scope
    }

    /// Register a symbol in the current scope
    ///
    /// If a symbol with the same name already exists in the current scope,
    /// it will be shadowed by this new definition.
    pub fn register(&mut self, symbol: SymbolInfo) {
        self.scopes[self.current_scope.0]
            .symbols
            .insert(symbol.name.clone(), symbol);
    }

    /// Register a symbol in a specific scope
    ///
    /// This is useful when you need to register symbols in a scope
    /// other than the current one (e.g., pre-populating a function scope).
    pub fn register_in_scope(&mut self, scope_id: ScopeId, symbol: SymbolInfo) {
        self.scopes[scope_id.0]
            .symbols
            .insert(symbol.name.clone(), symbol);
    }

    /// Look up a symbol by name in the current scope and parent scopes
    ///
    /// Searches upward through the scope hierarchy, returning the first
    /// matching symbol found. This implements lexical scoping rules.
    ///
    /// # Returns
    ///
    /// - `Some(&SymbolInfo)` if the symbol is found in the current or any parent scope
    /// - `None` if the symbol is not found in any accessible scope
    pub fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
        let mut current = Some(self.current_scope);
        while let Some(scope_id) = current {
            let scope = &self.scopes[scope_id.0];
            if let Some(symbol) = scope.symbols.get(name) {
                return Some(symbol);
            }
            current = scope.parent;
        }
        None
    }

    /// Look up a symbol only in the current scope (no parent lookup)
    ///
    /// This is useful for checking if a symbol is defined locally,
    /// as opposed to inherited from a parent scope.
    pub fn lookup_local(&self, name: &str) -> Option<&SymbolInfo> {
        self.scopes[self.current_scope.0].symbols.get(name)
    }

    /// Get all exported symbols from the global scope (public and entry)
    ///
    /// This is used for Phase 6 hot reload ABI signature generation.
    /// Only symbols in the global scope (scope 0) are considered for export.
    pub fn exported_symbols(&self) -> Vec<&SymbolInfo> {
        self.scopes[0]
            .symbols
            .values()
            .filter(|s| matches!(s.visibility, Visibility::Public | Visibility::Entry))
            .collect()
    }

    /// Check if a symbol exists in the current scope or any parent scope
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Check if a symbol exists in the current scope only
    pub fn contains_local(&self, name: &str) -> bool {
        self.lookup_local(name).is_some()
    }

    /// Get all symbols in the current scope (for debugging/testing)
    pub fn current_scope_symbols(&self) -> Vec<&SymbolInfo> {
        self.scopes[self.current_scope.0].symbols.values().collect()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a substitution from type variables to types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    /// Map from type variable IDs to their substituted types
    mappings: BTreeMap<usize, CoreType>,
}

impl Substitution {
    /// Create an empty substitution
    #[expect(clippy::missing_const_for_fn, reason = "BTreeMap::new() is not const")]
    pub fn empty() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Create a substitution with a single mapping
    pub fn single(var_id: usize, type_value: CoreType) -> Self {
        let mut mappings = BTreeMap::new();
        mappings.insert(var_id, type_value);
        Self { mappings }
    }

    /// Apply this substitution to a type
    pub fn apply(&self, core_type: &CoreType) -> CoreType {
        match *core_type {
            CoreType::Variable(ref var) => self
                .mappings
                .get(&var.id)
                .map_or_else(|| core_type.clone(), |substituted| self.apply(substituted)),
            CoreType::Array(ref element_type) => {
                CoreType::Array(Box::new(self.apply(element_type)))
            }
            CoreType::Function {
                ref parameters,
                ref return_type,
            } => CoreType::Function {
                parameters: parameters.iter().map(|p| self.apply(p)).collect(),
                return_type: Box::new(self.apply(return_type)),
            },
            CoreType::Generic {
                ref name,
                ref type_args,
            } => CoreType::Generic {
                name: name.clone(),
                type_args: type_args.iter().map(|arg| self.apply(arg)).collect(),
            },
            // Primitive types don't contain type variables
            _ => core_type.clone(),
        }
    }

    /// Compose this substitution with another (self after other)
    pub fn compose(self, other: &Self) -> Self {
        let mut result_mappings = BTreeMap::new();

        // Apply self to all mappings in other
        for (var_id, type_value) in &other.mappings {
            result_mappings.insert(*var_id, self.apply(type_value));
        }

        // Add mappings from self that are not in other
        for (var_id, type_value) in self.mappings {
            result_mappings.entry(var_id).or_insert(type_value);
        }

        Self {
            mappings: result_mappings,
        }
    }

    /// Check if this substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Get the mappings for testing
    pub const fn mappings(&self) -> &BTreeMap<usize, CoreType> {
        &self.mappings
    }
}

/// Environment for tracking types and their definitions
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Map of type names to their definitions
    types: BTreeMap<String, CoreType>,
    /// Map of generic type parameters to their constraints
    generic_params: BTreeMap<String, Vec<String>>,
}

impl TypeEnvironment {
    /// Create a new type environment with built-in types
    pub fn new() -> Self {
        let mut env = Self {
            types: BTreeMap::new(),
            generic_params: BTreeMap::new(),
        };

        // Register built-in types
        env.register_builtin_types();
        env
    }

    /// Register all built-in core types
    fn register_builtin_types(&mut self) {
        self.types.insert("int8".to_owned(), CoreType::Int8);
        self.types.insert("int16".to_owned(), CoreType::Int16);
        self.types.insert("int32".to_owned(), CoreType::Int32);
        self.types.insert("int64".to_owned(), CoreType::Int64);
        self.types.insert("uint8".to_owned(), CoreType::UInt8);
        self.types.insert("uint16".to_owned(), CoreType::UInt16);
        self.types.insert("uint32".to_owned(), CoreType::UInt32);
        self.types.insert("uint64".to_owned(), CoreType::UInt64);
        self.types.insert("float32".to_owned(), CoreType::Float32);
        self.types.insert("float64".to_owned(), CoreType::Float64);
        self.types.insert("string".to_owned(), CoreType::String);
        self.types.insert("boolean".to_owned(), CoreType::Boolean);
        self.types.insert("unit".to_owned(), CoreType::Unit);
    }

    /// Look up a type by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the type to look up
    /// * `span` - Source location where the type was referenced (for error reporting)
    pub fn lookup_type(&self, name: &str, span: Span) -> Result<&CoreType, TypeError> {
        self.types.get(name).ok_or_else(|| TypeError::TypeNotFound {
            type_name: name.to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Register a new type in the environment
    pub fn register_type(&mut self, name: String, core_type: CoreType) {
        self.types.insert(name, core_type);
    }

    /// Check if a type exists in the environment
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    /// Get all registered type names
    pub fn get_type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.types.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic type checker for validating types
#[derive(Debug)]
pub struct TypeChecker {
    /// Current type environment
    environment: TypeEnvironment,
    /// Counter for generating fresh type variables
    next_var_id: usize,
    /// Symbol table for tracking symbols in scope (Phase 2 and Phase 6)
    symbol_table: SymbolTable,
    /// Collected type constraints for inference (Phase 2)
    constraints: Vec<TypeConstraint>,
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        Self {
            environment: TypeEnvironment::new(),
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
        }
    }

    /// Create a type checker with a specific environment
    pub fn with_environment(environment: TypeEnvironment) -> Self {
        Self {
            environment,
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
        }
    }

    /// Get a reference to the current environment
    pub const fn environment(&self) -> &TypeEnvironment {
        &self.environment
    }

    /// Get a mutable reference to the current environment
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Cannot have const fn with mutable reference"
    )]
    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.environment
    }

    /// Get a reference to the symbol table
    pub const fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub const fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Register a symbol for ABI signature generation (Phase 6)
    pub fn register_symbol(&mut self, symbol: SymbolInfo) {
        self.symbol_table.register(symbol);
    }

    /// Add a type constraint for inference (Phase 2)
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    /// Get all collected constraints
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec deref coercion to slice is not allowed in const fn"
    )]
    pub fn constraints(&self) -> &[TypeConstraint] {
        &self.constraints
    }

    /// Clear all collected constraints
    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }

    /// Solve all collected constraints (Phase 2 - not yet implemented)
    ///
    /// This will be the main entry point for constraint-based type inference.
    /// It should unify all constraints and return a substitution that satisfies them all.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if constraints cannot be satisfied.
    pub fn solve_constraints(&mut self) -> Result<Substitution, TypeError> {
        let pending_constraints = core::mem::take(&mut self.constraints);
        let mut substitution = Substitution::empty();

        for constraint in pending_constraints {
            match constraint {
                TypeConstraint::Equality(left, right) => {
                    let left_applied = substitution.apply(&left);
                    let right_applied = substitution.apply(&right);
                    let new_substitution = self.unify(&left_applied, &right_applied)?;
                    substitution = new_substitution.compose(&substitution);
                }
                TypeConstraint::HasField(_, field, _) => {
                    return Err(TypeError::NotImplementedYet {
                        feature: format!("has-field constraint solving for field '{field}'"),
                        span: TypeError::unknown_span(),
                    });
                }
                TypeConstraint::Callable(_, _, _) => {
                    return Err(TypeError::NotImplementedYet {
                        feature: "callable constraint solving".to_owned(),
                        span: TypeError::unknown_span(),
                    });
                }
            }
        }

        Ok(substitution)
    }

    /// Generate a fresh type variable
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the type variable
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    pub fn fresh_type_var(&mut self, name: String, span: Span) -> Result<CoreType, TypeError> {
        let var = TypeVar::new(self.next_var_id, name);
        self.next_var_id =
            self.next_var_id
                .checked_add(1)
                .ok_or_else(|| TypeError::TypeVariableOverflow {
                    span: TypeError::span_from_span(span),
                })?;
        Ok(CoreType::Variable(var))
    }

    /// Generate a fresh type variable with an auto-generated name
    ///
    /// # Arguments
    ///
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    pub fn fresh_type_var_auto(&mut self, span: Span) -> Result<CoreType, TypeError> {
        self.fresh_type_var(format!("t{}", self.next_var_id), span)
    }

    /// Convert an AST Type to a `CoreType` for validation and instantiation
    /// Supports generics, arrays, and function types.
    pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, span } => match name.as_str() {
                "int8" => Ok(CoreType::Int8),
                "int16" => Ok(CoreType::Int16),
                "int32" => Ok(CoreType::Int32),
                "int64" => Ok(CoreType::Int64),
                "uint8" => Ok(CoreType::UInt8),
                "uint16" => Ok(CoreType::UInt16),
                "uint32" => Ok(CoreType::UInt32),
                "uint64" => Ok(CoreType::UInt64),
                "float32" => Ok(CoreType::Float32),
                "float64" => Ok(CoreType::Float64),
                "string" => Ok(CoreType::String),
                "boolean" => Ok(CoreType::Boolean),
                "unit" => Ok(CoreType::Unit),
                _ => Err(TypeError::TypeNotFound {
                    type_name: name.clone(),
                    span: TypeError::span_from_span(span),
                }),
            },
            Type::Array {
                ref element_type, ..
            } => {
                let elem_core = Self::ast_type_to_core_type(element_type.as_ref())?;
                Ok(CoreType::Array(Box::new(elem_core)))
            }
            Type::Function {
                ref parameters,
                ref return_type,
                ..
            } => {
                let mut param_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    param_types.push(Self::ast_type_to_core_type(param)?);
                }
                let ret_type = Self::ast_type_to_core_type(return_type.as_ref())?;
                Ok(CoreType::Function {
                    parameters: param_types,
                    return_type: Box::new(ret_type),
                })
            }
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let mut core_args = Vec::with_capacity(type_args.len());
                for arg in type_args {
                    core_args.push(Self::ast_type_to_core_type(arg)?);
                }
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: core_args,
                })
            }
        }
    }

    /// Validate algebraic data type definitions against the known type environment to ensure all
    /// referenced field and variant types are resolvable.
    pub fn validate_adt_type(&self, type_def: &crate::ast::TypeDef) -> Result<(), TypeError> {
        match *type_def {
            crate::ast::TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                        self.validate_type_name(&field.name, &core_field_type, field.span)?;
                    }
                }
                Ok(())
            }
            crate::ast::TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                    self.validate_type_name(&field.name, &core_field_type, field.span)?;
                }
                Ok(())
            }
            crate::ast::TypeDef::Alias {
                ref target_type, ..
            } => {
                let _: CoreType = Self::ast_type_to_core_type(target_type)?;
                Ok(())
            }
        }
    }
    /// Type check a pattern match expression
    /// Ensures all patterns and arms are type compatible
    pub fn type_check_pattern_match(
        &self,
        matched_type: &CoreType,
        patterns: &[CoreType],
        arm_types: &[CoreType],
    ) -> Result<(), TypeError> {
        // Each pattern must be compatible with matched_type
        for pat in patterns {
            if !self.types_compatible(matched_type, pat) {
                return Err(TypeError::TypeMismatch {
                    expected: matched_type.to_string(),
                    found: pat.to_string(),
                    found_span: TypeError::unknown_span(),
                    expected_span: None,
                });
            }
        }
        // All arm types must be compatible with each other
        if let Some(first) = arm_types.first() {
            for arm in arm_types {
                if !self.types_compatible(first, arm) {
                    return Err(TypeError::TypeMismatch {
                        expected: first.to_string(),
                        found: arm.to_string(),
                        found_span: TypeError::unknown_span(),
                        expected_span: None,
                    });
                }
            }
        }
        Ok(())
    }

    /// Check if two core types are structurally compatible (including nested types)
    ///
    /// This method performs deep structural comparison for complex types like
    /// arrays, functions, and generics, ensuring all nested components are compatible.
    /// For simple equality checking, use the `==` operator on `CoreType` directly.
    #[expect(
        clippy::only_used_in_recursion,
        reason = "self parameter needed for structural recursion"
    )]
    pub fn types_compatible(&self, left: &CoreType, right: &CoreType) -> bool {
        match (left, right) {
            // All primitive types
            (&CoreType::Int8, &CoreType::Int8)
            | (&CoreType::Int16, &CoreType::Int16)
            | (&CoreType::Int32, &CoreType::Int32)
            | (&CoreType::Int64, &CoreType::Int64)
            | (&CoreType::UInt8, &CoreType::UInt8)
            | (&CoreType::UInt16, &CoreType::UInt16)
            | (&CoreType::UInt32, &CoreType::UInt32)
            | (&CoreType::UInt64, &CoreType::UInt64)
            | (&CoreType::Float32, &CoreType::Float32)
            | (&CoreType::Float64, &CoreType::Float64)
            | (&CoreType::String, &CoreType::String)
            | (&CoreType::Boolean, &CoreType::Boolean)
            | (&CoreType::Unit, &CoreType::Unit) => true,

            // Variables are equal if they have the same ID
            (&CoreType::Variable(ref left_var), &CoreType::Variable(ref right_var)) => {
                left_var.id == right_var.id
            }

            // Arrays are compatible if element types are compatible
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.types_compatible(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions are compatible if parameters and return types are compatible
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                left_params.len() == right_params.len()
                    && left_params
                        .iter()
                        .zip(right_params.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
                    && self.types_compatible(left_ret.as_ref(), right_ret.as_ref())
            }

            // Generic types are compatible if names and type arguments are compatible
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                left_name == right_name
                    && left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .zip(right_args.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
            }

            // Different types are not compatible
            _ => false,
        }
    }

    /// Type check an expression and return its [`CoreType`]
    ///
    /// # Errors
    /// Returns `TypeError` variants when expression typing fails.
    pub fn type_check_expr(&mut self, expr: &Expr) -> Result<CoreType, TypeError> {
        match *expr {
            Expr::Literal { ref value, .. } => Ok(Self::literal_to_core_type(value)),
            Expr::Identifier { ref name, span, .. } => self.resolve_identifier(name, span),
            Expr::Parenthesized { ref expr, .. } => self.type_check_expr(expr),
            Expr::Binary {
                ref left,
                ref operator,
                ref right,
                span,
                ..
            } => self.type_check_binary_expr(left.as_ref(), operator, right.as_ref(), span),
            Expr::Unary {
                ref operator,
                ref operand,
                span,
                ..
            } => self.type_check_unary_expr(operator, operand.as_ref(), span),
            Expr::Call {
                ref callee,
                ref args,
                span,
                ..
            } => self.type_check_call_expr(callee.as_ref(), args.as_slice(), span),
            Expr::Index {
                ref object,
                ref index,
                span,
                ..
            } => self.type_check_index_expr(object.as_ref(), index.as_ref(), span),
            Expr::Member { span, .. } => Err(TypeError::NotImplementedYet {
                feature: "member access type checking".to_owned(),
                span: TypeError::span_from_span(span),
            }),
            Expr::Cast {
                ref expr,
                ref target_type,
                span,
                ..
            } => self.type_check_cast_expr(expr.as_ref(), target_type, span),
            Expr::TypeOf { ref expr, .. } => {
                self.type_check_expr(expr.as_ref())?;
                Ok(CoreType::String)
            }
            Expr::StringInterpolation {
                ref parts, span, ..
            } => {
                self.type_check_string_interpolation(parts.as_slice(), span)?;
                Ok(CoreType::String)
            }
            Expr::Array {
                ref elements, span, ..
            } => self.type_check_array_expr(elements.as_slice(), span),
            Expr::Lambda {
                ref generic_params,
                ref params,
                ref return_type,
                ref body,
                span,
                ..
            } => self.type_check_lambda_expr(
                generic_params.as_deref(),
                params.as_slice(),
                return_type,
                body,
                span,
            ),
        }
    }

    /// Determine the canonical [`CoreType`] for a literal value.
    const fn literal_to_core_type(value: &LiteralValue) -> CoreType {
        match *value {
            LiteralValue::Integer(_) => CoreType::Int64,
            LiteralValue::Float(_) => CoreType::Float64,
            LiteralValue::String(_) => CoreType::String,
            LiteralValue::Boolean(_) => CoreType::Boolean,
            LiteralValue::Void => CoreType::Unit,
        }
    }

    /// Resolve an identifier to its registered core type or emit a symbol error.
    fn resolve_identifier(&self, name: &str, span: Span) -> Result<CoreType, TypeError> {
        self.symbol_table()
            .lookup(name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::SymbolNotFound {
                name: name.to_owned(),
                span: TypeError::span_from_span(span),
            })
    }

    /// Categorize a core type into a numeric family when applicable.
    const fn classify_numeric(core_type: &CoreType) -> Option<NumericKind> {
        match *core_type {
            CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64 => {
                Some(NumericKind::SignedInt)
            }
            CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64 => {
                Some(NumericKind::UnsignedInt)
            }
            CoreType::Float32 | CoreType::Float64 => Some(NumericKind::Float),
            _ => None,
        }
    }

    /// Check whether the provided type belongs to any numeric family.
    const fn is_numeric_type(core_type: &CoreType) -> bool {
        Self::classify_numeric(core_type).is_some()
    }

    /// Check whether the provided type is an integer (signed or unsigned).
    const fn is_integer_type(core_type: &CoreType) -> bool {
        matches!(
            Self::classify_numeric(core_type),
            Some(NumericKind::SignedInt | NumericKind::UnsignedInt)
        )
    }

    /// Check whether the provided type is a floating point primitive.
    const fn is_float_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::Float32 | &CoreType::Float64)
    }

    /// Check whether the provided type is the boolean primitive.
    const fn is_boolean_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::Boolean)
    }

    /// Check whether the provided type is the string primitive.
    const fn is_string_type(core_type: &CoreType) -> bool {
        matches!(core_type, &CoreType::String)
    }

    /// Construct a type error describing an invalid operation on a type.
    fn invalid_operation_error(operation: &str, core_type: &CoreType, span: Span) -> TypeError {
        TypeError::InvalidOperation {
            operation: operation.to_owned(),
            type_name: core_type.to_string(),
            span: TypeError::span_from_span(span),
        }
    }

    /// Construct a type mismatch diagnostic with consistent formatting.
    fn type_mismatch_error(
        expected: &CoreType,
        expected_span: Option<Span>,
        found: &CoreType,
        found_span: Span,
    ) -> TypeError {
        TypeError::TypeMismatch {
            expected: expected.to_string(),
            found: found.to_string(),
            found_span: TypeError::span_from_span(found_span),
            expected_span: expected_span.map(TypeError::span_from_span),
        }
    }

    /// Attempt to coerce a literal expression's type to match an expected core type.
    fn coerce_literal_to_expected(
        expected: &CoreType,
        expr: &Expr,
        actual: &CoreType,
    ) -> Option<CoreType> {
        match *expr {
            Expr::Literal { ref value, .. } => match *value {
                LiteralValue::Integer(_) => (Self::is_integer_type(expected)
                    && Self::is_integer_type(actual))
                .then(|| expected.clone()),
                LiteralValue::Float(_) => (Self::is_float_type(expected)
                    && Self::is_float_type(actual))
                .then(|| expected.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Ensure that two resolved operand types are identical, capturing precise source spans
    /// for a subsequent diagnostic when they differ.
    fn ensure_same_type(
        expected: &CoreType,
        expected_span: Span,
        actual: &CoreType,
        actual_span: Span,
    ) -> Result<(), TypeError> {
        if expected == actual {
            Ok(())
        } else {
            Err(Self::type_mismatch_error(
                expected,
                Some(expected_span),
                actual,
                actual_span,
            ))
        }
    }

    /// Validate that a core type belongs to one of the numeric families prior to a numeric
    /// operation, preserving architectural guarantees about arithmetic safety.
    fn ensure_numeric_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_numeric_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Ensure that the provided type is an integer (signed or unsigned) before executing an
    /// integer-only operation, preventing silent lossy conversions.
    fn ensure_integer_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_integer_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Guard boolean-only operations so that only strict `boolean` operands are permitted,
    /// preserving logical semantics for control-flow constructs.
    fn ensure_boolean_type(
        core_type: &CoreType,
        span: Span,
        operation: &str,
    ) -> Result<(), TypeError> {
        if Self::is_boolean_type(core_type) {
            Ok(())
        } else {
            Err(Self::invalid_operation_error(operation, core_type, span))
        }
    }

    /// Provide a human-readable description for a binary operator, feeding into diagnostics
    /// and future telemetry without repeating strings across the code base.
    const fn binary_operation_name(operator: &BinaryOp) -> &'static str {
        match *operator {
            BinaryOp::Add => "addition",
            BinaryOp::Subtract => "subtraction",
            BinaryOp::Multiply => "multiplication",
            BinaryOp::Divide => "division",
            BinaryOp::Modulo => "modulo",
            BinaryOp::Power => "exponentiation",
            BinaryOp::Equal => "equality comparison",
            BinaryOp::NotEqual => "inequality comparison",
            BinaryOp::Less => "less-than comparison",
            BinaryOp::LessEqual => "less-or-equal comparison",
            BinaryOp::Greater => "greater-than comparison",
            BinaryOp::GreaterEqual => "greater-or-equal comparison",
            BinaryOp::Is => "identity comparison",
            BinaryOp::IsNot => "negative identity comparison",
            BinaryOp::And => "logical and",
            BinaryOp::Or => "logical or",
            BinaryOp::Xor => "logical xor",
            BinaryOp::BitAnd => "bitwise and",
            BinaryOp::BitOr => "bitwise or",
            BinaryOp::BitXor => "bitwise xor",
            BinaryOp::BitShiftLeft => "left shift",
            BinaryOp::BitShiftRight => "right shift",
            BinaryOp::BitUnsignedShiftRight => "unsigned right shift",
            BinaryOp::Assign => "assignment",
        }
    }

    /// Provide a human-readable description for unary operators so that diagnostics can
    /// reference intent rather than symbolic tokens alone.
    const fn unary_operation_name(operator: &UnaryOp) -> &'static str {
        match *operator {
            UnaryOp::Negate => "numeric negation",
            UnaryOp::Not => "logical not",
            UnaryOp::BitNot => "bitwise not",
            UnaryOp::Plus => "unary plus",
        }
    }

    /// Determine the numeric family and bit width for cast validation, enabling widening rules
    /// that mirror the language specification while keeping the data in a const context.
    const fn numeric_bit_width(core_type: &CoreType) -> Option<(NumericKind, u8)> {
        match *core_type {
            CoreType::Int8 => Some((NumericKind::SignedInt, 8)),
            CoreType::Int16 => Some((NumericKind::SignedInt, 16)),
            CoreType::Int32 => Some((NumericKind::SignedInt, 32)),
            CoreType::Int64 => Some((NumericKind::SignedInt, 64)),
            CoreType::UInt8 => Some((NumericKind::UnsignedInt, 8)),
            CoreType::UInt16 => Some((NumericKind::UnsignedInt, 16)),
            CoreType::UInt32 => Some((NumericKind::UnsignedInt, 32)),
            CoreType::UInt64 => Some((NumericKind::UnsignedInt, 64)),
            CoreType::Float32 => Some((NumericKind::Float, 32)),
            CoreType::Float64 => Some((NumericKind::Float, 64)),
            _ => None,
        }
    }

    /// Determine whether an implicit cast between numeric types is permitted under the
    /// language's widening rules. This intentionally excludes narrowing conversions and
    /// mixed-family casts unless explicitly sanctioned by the specification.
    fn is_cast_allowed(from: &CoreType, to: &CoreType) -> bool {
        if from == to {
            return true;
        }
        match (Self::numeric_bit_width(from), Self::numeric_bit_width(to)) {
            (
                Some((NumericKind::SignedInt, from_bits)),
                Some((NumericKind::SignedInt, to_bits)),
            )
            | (
                Some((NumericKind::UnsignedInt, from_bits)),
                Some((NumericKind::UnsignedInt, to_bits)),
            )
            | (Some((NumericKind::Float, from_bits)), Some((NumericKind::Float, to_bits))) => {
                from_bits <= to_bits
            }
            (
                Some((NumericKind::SignedInt | NumericKind::UnsignedInt, _)),
                Some((NumericKind::Float, _)),
            ) => true,
            _ => false,
        }
    }

    /// Type check a binary expression, enforcing operand compatibility, recording inference
    /// constraints, and returning the resulting core type for subsequent analysis.
    fn type_check_binary_expr(
        &mut self,
        left: &Expr,
        operator: &BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let left_type = self.type_check_expr(left)?;
        let right_type = self.type_check_expr(right)?;
        let op_name = Self::binary_operation_name(operator);

        match *operator {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Power => {
                if Self::is_string_type(&left_type) && Self::is_string_type(&right_type) {
                    return Ok(CoreType::String);
                }
                Self::ensure_numeric_type(&left_type, left.span(), op_name)?;
                Self::ensure_numeric_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::Modulo => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Is | BinaryOp::IsNot => {
                if !self.types_compatible(&left_type, &right_type) {
                    return Err(Self::type_mismatch_error(
                        &left_type,
                        Some(left.span()),
                        &right_type,
                        right.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(CoreType::Boolean)
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                Self::ensure_numeric_type(&left_type, left.span(), op_name)?;
                Self::ensure_numeric_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
                Self::ensure_boolean_type(&left_type, left.span(), op_name)?;
                Self::ensure_boolean_type(&right_type, right.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::Equality(left_type, right_type));
                Ok(result_type)
            }
            BinaryOp::BitShiftLeft | BinaryOp::BitShiftRight | BinaryOp::BitUnsignedShiftRight => {
                Self::ensure_integer_type(&left_type, left.span(), op_name)?;
                Self::ensure_integer_type(&right_type, right.span(), op_name)?;
                Ok(left_type)
            }
            BinaryOp::Assign => Err(Self::invalid_operation_error(op_name, &left_type, span)),
        }
    }

    /// Type check a unary expression, returning the deduced result type while enforcing the
    /// operator's domain constraints.
    fn type_check_unary_expr(
        &mut self,
        operator: &UnaryOp,
        operand: &Expr,
        _span: Span,
    ) -> Result<CoreType, TypeError> {
        let operand_type = self.type_check_expr(operand)?;
        let op_name = Self::unary_operation_name(operator);
        match *operator {
            UnaryOp::Negate | UnaryOp::Plus => {
                Self::ensure_numeric_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
            UnaryOp::Not => {
                Self::ensure_boolean_type(&operand_type, operand.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            UnaryOp::BitNot => {
                Self::ensure_integer_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
        }
    }

    /// Validate a function call, ensuring arity matches, arguments conform to parameter types,
    /// and recording equality constraints for the inference engine.
    fn type_check_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let callee_type = self.type_check_expr(callee)?;
        match callee_type {
            CoreType::Function {
                parameters,
                return_type,
            } => {
                if parameters.len() != args.len() {
                    return Err(TypeError::InvalidOperation {
                        operation: format!(
                            "function call expected {} arguments but received {}",
                            parameters.len(),
                            args.len()
                        ),
                        type_name: "function".to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                }

                for (index, arg_expr) in args.iter().enumerate() {
                    let param_type = parameters[index].clone();
                    let arg_type = self.type_check_expr(arg_expr)?;
                    let reconciled_type = if self.types_compatible(&param_type, &arg_type) {
                        arg_type
                    } else if let Some(adjusted) =
                        Self::coerce_literal_to_expected(&param_type, arg_expr, &arg_type)
                    {
                        adjusted
                    } else {
                        return Err(Self::type_mismatch_error(
                            &param_type,
                            None,
                            &arg_type,
                            arg_expr.span(),
                        ));
                    };
                    self.add_constraint(TypeConstraint::Equality(
                        param_type.clone(),
                        reconciled_type,
                    ));
                }

                Ok(*return_type)
            }
            other => Err(Self::invalid_operation_error("function call", &other, span)),
        }
    }

    /// Type check an array indexing operation, confirming integer indices and yielding the
    /// element type for subsequent evaluation.
    fn type_check_index_expr(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let object_type = self.type_check_expr(object)?;
        let index_type = self.type_check_expr(index)?;
        Self::ensure_integer_type(&index_type, index.span(), "indexing")?;
        match object_type {
            CoreType::Array(element_type) => Ok(*element_type),
            other => Err(Self::invalid_operation_error("indexing", &other, span)),
        }
    }

    /// Type check an explicit cast expression, leveraging the numeric widening rules to
    /// determine whether the conversion is permitted.
    fn type_check_cast_expr(
        &mut self,
        expr: &Expr,
        target_type: &Type,
        _span: Span,
    ) -> Result<CoreType, TypeError> {
        let source_type = self.type_check_expr(expr)?;
        let target_core_type = Self::ast_type_to_core_type(target_type)?;
        if Self::is_cast_allowed(&source_type, &target_core_type) {
            Ok(target_core_type)
        } else {
            Err(Self::type_mismatch_error(
                &target_core_type,
                Some(target_type.span()),
                &source_type,
                expr.span(),
            ))
        }
    }

    /// Validate each interpolated expression, ensuring only display-safe primitives appear
    /// inside a string literal interpolation sequence.
    fn type_check_string_interpolation(
        &mut self,
        parts: &[StringPart],
        _span: Span,
    ) -> Result<(), TypeError> {
        for part in parts {
            if let StringPart::Expression(ref expr) = *part {
                let expr_type = self.type_check_expr(expr)?;
                if !(Self::is_numeric_type(&expr_type)
                    || Self::is_boolean_type(&expr_type)
                    || Self::is_string_type(&expr_type))
                {
                    return Err(Self::invalid_operation_error(
                        "string interpolation",
                        &expr_type,
                        expr.span(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Type check an array literal, deriving a unified element type and generating equality
    /// constraints between each element and the inferred element type.
    fn type_check_array_expr(
        &mut self,
        elements: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let mut element_type: Option<CoreType> = None;
        for element in elements {
            let element_core_type = self.type_check_expr(element)?;
            if let Some(existing_type) = element_type.as_ref() {
                if !self.types_compatible(existing_type, &element_core_type) {
                    return Err(Self::type_mismatch_error(
                        existing_type,
                        Some(element.span()),
                        &element_core_type,
                        element.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::Equality(
                    existing_type.clone(),
                    element_core_type,
                ));
            } else {
                element_type = Some(element_core_type);
            }
        }

        let resolved = match element_type {
            Some(core_type) => core_type,
            None => self.fresh_type_var_auto(span)?,
        };

        Ok(CoreType::Array(Box::new(resolved)))
    }

    /// Type check a lambda expression by establishing a scoped environment for its parameters and body.
    fn type_check_lambda_expr(
        &mut self,
        generic_params: Option<&[String]>,
        parameters: &[Parameter],
        return_type: &Type,
        body: &LambdaBody,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if let Some(params) = generic_params {
            if !params.is_empty() {
                return Err(TypeError::NotImplementedYet {
                    feature: "generic lambda type checking".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = Self::ast_type_to_core_type(return_type)?;
        let return_span = return_type.span();

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            match *body {
                LambdaBody::Expression(ref expr) => {
                    let expr_type = checker.type_check_expr(expr)?;
                    if !checker.types_compatible(&return_core, &expr_type) {
                        return Err(Self::type_mismatch_error(
                            &return_core,
                            Some(return_span),
                            &expr_type,
                            expr.span(),
                        ));
                    }
                    checker
                        .add_constraint(TypeConstraint::Equality(return_core.clone(), expr_type));
                    Ok(())
                }
                LambdaBody::Block(ref statements) => {
                    checker.type_check_statements(statements, Some(&return_core))
                }
            }
        })?;

        Ok(CoreType::Function {
            parameters: parameter_types,
            return_type: Box::new(return_core),
        })
    }

    /// Execute a closure within a fresh lexical scope, ensuring the scope is
    /// entered and exited even when the closure returns early.
    fn within_new_scope<F, R>(&mut self, action: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.symbol_table.enter_scope();
        let result = action(self);
        self.symbol_table.exit_scope();
        result
    }

    /// Type check a slice of statements while propagating the expected return
    /// type for the enclosing function or lambda.
    fn type_check_statements(
        &mut self,
        statements: &[Stmt],
        expected_return: Option<&CoreType>,
    ) -> Result<(), TypeError> {
        for statement in statements {
            self.type_check_stmt_with_return(statement, expected_return)?;
        }
        Ok(())
    }

    /// Type check a single statement, validating it within the context of an
    /// optional expected return type.
    fn type_check_stmt_with_return(
        &mut self,
        stmt: &Stmt,
        expected_return: Option<&CoreType>,
    ) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Let {
                ref binding,
                ref initializer,
                ..
            } => self.type_check_let_statement(binding, initializer.as_ref()),
            Stmt::Assignment {
                ref target,
                ref value,
                span,
                ..
            } => self.type_check_assignment(target, value, span),
            Stmt::Return {
                ref value, span, ..
            } => self.type_check_return(value.as_ref(), expected_return, span),
            Stmt::Expression { ref expr, .. } => {
                self.type_check_expr(expr)?;
                Ok(())
            }
            Stmt::Block { ref statements, .. } => self.within_new_scope(|checker| {
                checker.type_check_statements(statements, expected_return)
            }),
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                Self::ensure_boolean_type(&condition_type, condition.span(), "if condition")?;
                self.within_new_scope(|checker| {
                    checker.type_check_stmt_with_return(then_branch.as_ref(), expected_return)
                })?;
                if let Some(else_branch_stmt) = else_branch.as_deref() {
                    self.within_new_scope(|checker| {
                        checker.type_check_stmt_with_return(else_branch_stmt, expected_return)
                    })?;
                }
                Ok(())
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                span,
                ..
            } => {
                let iterable_type = self.type_check_expr(iterable)?;
                match iterable_type {
                    CoreType::Array(element_type) => {
                        let element_core = *element_type;
                        let variable_name = variable.clone();
                        self.within_new_scope(move |checker| {
                            checker.symbol_table.register(SymbolInfo {
                                name: variable_name.clone(),
                                symbol_type: SymbolType::Variable,
                                core_type: element_core,
                                visibility: Visibility::Private,
                                source_location: span,
                            });
                            checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                        })
                    }
                    _ => Err(Self::invalid_operation_error(
                        "for loop iteration",
                        &iterable_type,
                        span,
                    )),
                }
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                Self::ensure_boolean_type(&condition_type, condition.span(), "while condition")?;
                self.within_new_scope(|checker| {
                    checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                })
            }
            Stmt::Loop { ref body, .. } => self.within_new_scope(|checker| {
                checker.type_check_stmt_with_return(body.as_ref(), expected_return)
            }),
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
        }
    }

    /// Validate a `let` statement by resolving optional type annotations,
    /// initializer compatibility, and registering the binding in the current
    /// scope.
    fn type_check_let_statement(
        &mut self,
        binding: &LetBinding,
        initializer: Option<&Expr>,
    ) -> Result<(), TypeError> {
        let annotated_type = binding
            .type_annotation
            .as_ref()
            .map(Self::ast_type_to_core_type)
            .transpose()?;

        let initializer_info = match initializer {
            Some(expr) => Some((self.type_check_expr(expr)?, expr)),
            None => None,
        };

        let final_type = match (annotated_type, initializer_info) {
            (Some(expected), Some((actual, expr))) => {
                let reconciled = if self.types_compatible(&expected, &actual) {
                    actual
                } else if let Some(adjusted) =
                    Self::coerce_literal_to_expected(&expected, expr, &actual)
                {
                    adjusted
                } else {
                    return Err(Self::type_mismatch_error(
                        &expected,
                        binding.type_annotation.as_ref().map(Type::span),
                        &actual,
                        expr.span(),
                    ));
                };
                self.add_constraint(TypeConstraint::Equality(expected.clone(), reconciled));
                expected
            }
            (Some(expected), None) => expected,
            (None, Some((actual, _))) => actual,
            (None, None) => {
                return Err(TypeError::ConstraintSolvingFailed {
                    reason: format!(
                        "Cannot infer type for binding '{}' without annotation or initializer",
                        binding.name
                    ),
                    span: TypeError::span_from_span(binding.span),
                });
            }
        };

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };

        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: final_type,
            visibility: Visibility::Private,
            source_location: binding.span,
        });

        Ok(())
    }

    /// Ensure an assignment statement has a valid target and a value that is
    /// type compatible with that target.
    fn type_check_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        let target_type = self.type_check_expr(target)?;
        let value_type = self.type_check_expr(value)?;
        let reconciled_value_type = if self.types_compatible(&target_type, &value_type) {
            value_type
        } else if let Some(adjusted) =
            Self::coerce_literal_to_expected(&target_type, value, &value_type)
        {
            adjusted
        } else {
            return Err(Self::type_mismatch_error(
                &target_type,
                Some(target.span()),
                &value_type,
                value.span(),
            ));
        };
        let validity = match *target {
            Expr::Identifier { .. } | Expr::Member { .. } | Expr::Index { .. } => Ok(()),
            _ => Err(Self::invalid_operation_error(
                "assignment target",
                &target_type,
                span,
            )),
        };

        if validity.is_ok() {
            self.add_constraint(TypeConstraint::Equality(target_type, reconciled_value_type));
        }

        validity
    }

    /// Validate a return statement against the function's expected return type,
    /// guaranteeing both presence and compatibility.
    fn type_check_return(
        &mut self,
        value: Option<&Expr>,
        expected_return: Option<&CoreType>,
        span: Span,
    ) -> Result<(), TypeError> {
        let expected = expected_return.ok_or_else(|| TypeError::InvalidOperation {
            operation: "return outside of function".to_owned(),
            type_name: "<unknown>".to_owned(),
            span: TypeError::span_from_span(span),
        })?;

        match value {
            Some(expr) => {
                let value_type = self.type_check_expr(expr)?;
                let reconciled_type = if self.types_compatible(expected, &value_type) {
                    value_type
                } else if let Some(adjusted) =
                    Self::coerce_literal_to_expected(expected, expr, &value_type)
                {
                    adjusted
                } else {
                    return Err(Self::type_mismatch_error(
                        expected,
                        None,
                        &value_type,
                        expr.span(),
                    ));
                };
                self.add_constraint(TypeConstraint::Equality(expected.clone(), reconciled_type));
                Ok(())
            }
            None => {
                if matches!(expected, &CoreType::Unit) {
                    Ok(())
                } else {
                    Err(Self::type_mismatch_error(
                        expected,
                        None,
                        &CoreType::Unit,
                        span,
                    ))
                }
            }
        }
    }

    /// Type check a statement and update the symbol table as needed.
    ///
    /// # Errors
    /// Returns `TypeError` variants when statement typing fails.
    pub fn type_check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        self.type_check_stmt_with_return(stmt, None)
    }

    /// Convert AST-level visibility into the internal representation, accounting for entry points.
    const fn convert_visibility(visibility: &AstVisibility, is_entry: bool) -> Visibility {
        if is_entry {
            Visibility::Entry
        } else {
            match *visibility {
                AstVisibility::Public => Visibility::Public,
                AstVisibility::Private => Visibility::Private,
            }
        }
    }

    /// Register a declaration's symbol signature prior to body checking so forward references succeed.
    fn register_declaration_signature(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match decl {
            &Decl::Function {
                ref name,
                ref parameters,
                ref return_type,
                ref visibility,
                is_entry,
                ref span,
                ..
            } => {
                let mut parameter_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
                }

                let return_core = return_type
                    .as_ref()
                    .map(Self::ast_type_to_core_type)
                    .transpose()?
                    .unwrap_or(CoreType::Unit);

                let function_type = CoreType::Function {
                    parameters: parameter_types,
                    return_type: Box::new(return_core),
                };

                let visibility = Self::convert_visibility(visibility, is_entry);
                self.symbol_table.register(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Function,
                    core_type: function_type,
                    visibility,
                    source_location: *span,
                });
                Ok(())
            }
            &Decl::Let {
                ref binding,
                ref visibility,
                ..
            } => {
                if let Some(annotation) = binding.type_annotation.as_ref() {
                    let annotated_type = Self::ast_type_to_core_type(annotation)?;
                    let symbol_type = if binding.is_mutable {
                        SymbolType::Variable
                    } else {
                        SymbolType::Constant
                    };
                    let visibility = Self::convert_visibility(visibility, false);
                    self.symbol_table.register(SymbolInfo {
                        name: binding.name.clone(),
                        symbol_type,
                        core_type: annotated_type,
                        visibility,
                        source_location: binding.span,
                    });
                }
                Ok(())
            }
            &Decl::Type { .. } | &Decl::Import { .. } => Ok(()),
        }
    }

    /// Type check a top-level declaration and update symbol/type environments accordingly.
    fn type_check_declaration(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match *decl {
            Decl::Function {
                ref parameters,
                ref return_type,
                ref body,
                ..
            } => self.type_check_function_declaration(
                parameters.as_slice(),
                return_type.as_ref(),
                body,
            ),
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ..
            } => self.type_check_let_declaration(binding, initializer, visibility),
            Decl::Type { ref type_def, .. } => self.validate_adt_type(type_def),
            Decl::Import { .. } => {
                // Phase 4 will introduce full import validation; until then we simply acknowledge the declaration.
                Ok(())
            }
        }
    }

    /// Type check a function body within a dedicated parameter scope, enforcing return compatibility.
    fn type_check_function_declaration(
        &mut self,
        parameters: &[Parameter],
        return_type: Option<&Type>,
        body: &Stmt,
    ) -> Result<(), TypeError> {
        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = return_type
            .map(Self::ast_type_to_core_type)
            .transpose()?
            .unwrap_or(CoreType::Unit);

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            checker.type_check_stmt_with_return(body, Some(&return_core))
        })
    }

    /// Type check a module-level let declaration and ensure the registered symbol honors visibility.
    fn type_check_let_declaration(
        &mut self,
        binding: &LetBinding,
        initializer: &Expr,
        visibility: &AstVisibility,
    ) -> Result<(), TypeError> {
        self.type_check_let_statement(binding, Some(initializer))?;

        let inferred_type = self
            .symbol_table()
            .lookup(&binding.name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "Binding '{}' failed to register during top-level let processing",
                    binding.name
                ),
                span: TypeError::span_from_span(binding.span),
            })?;

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };
        let visibility = Self::convert_visibility(visibility, false);
        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: inferred_type,
            visibility,
            source_location: binding.span,
        });

        Ok(())
    }

    /// Type check an entire program, collecting all discovered errors.
    pub fn type_check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.clear_constraints();

        let mut errors: Vec<TypeError> = Vec::new();
        let mut skipped_decls: Vec<usize> = Vec::new();

        for decl in &program.declarations {
            if let Err(error) = self.register_declaration_signature(decl) {
                skipped_decls.push(decl.node_id().0);
                errors.push(error);
            }
        }

        for decl in &program.declarations {
            if skipped_decls.contains(&decl.node_id().0) {
                continue;
            }

            if let Err(error) = self.type_check_declaration(decl) {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            if let Err(error) = self.solve_constraints() {
                errors.push(error);
            }
        } else {
            self.clear_constraints();
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate that a type name is valid for the given core type
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the type to validate
    /// * `core_type` - The core type definition
    /// * `span` - Source location of the type definition (for error reporting)
    pub fn validate_type_name(
        &self,
        name: &str,
        core_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Ok(existing_type) = self.environment.lookup_type(name, span) {
            if existing_type != core_type {
                return Err(TypeError::TypeMismatch {
                    expected: existing_type.to_string(),
                    found: core_type.to_string(),
                    found_span: TypeError::span_from_span(span),
                    expected_span: None,
                });
            }
        }
        Ok(())
    }

    /// Unify two types, returning a substitution that makes them equal
    pub fn unify(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        self.unify_impl(left, right)
    }

    /// Internal implementation of unification algorithm
    fn unify_impl(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        match (left, right) {
            // Same primitive types unify with empty substitution
            (l, r) if self.types_compatible(l, r) => Ok(Substitution::empty()),

            // Variable unifies with any type (with occurs check)
            (&CoreType::Variable(ref var), other) | (other, &CoreType::Variable(ref var)) => {
                if Self::occurs_check(var.id, other) {
                    Err(TypeError::OccursCheckFailed {
                        var_name: var.name.clone(),
                        type_name: other.to_string(),
                        span: TypeError::unknown_span(),
                    })
                } else {
                    Ok(Substitution::single(var.id, other.clone()))
                }
            }

            // Arrays unify if their element types unify
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.unify_impl(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions unify if parameters and return types unify
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all parameters
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    let left_applied = combined_subst.apply(left_param);
                    let right_applied = combined_subst.apply(right_param);
                    let param_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&param_subst);
                }

                // Unify return types
                let left_ret_applied = combined_subst.apply(left_ret.as_ref());
                let right_ret_applied = combined_subst.apply(right_ret.as_ref());
                let ret_subst = self.unify_impl(&left_ret_applied, &right_ret_applied)?;
                combined_subst = combined_subst.compose(&ret_subst);

                Ok(combined_subst)
            }

            // Generic types unify if names match and type arguments unify
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all type arguments
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    let left_applied = combined_subst.apply(left_arg);
                    let right_applied = combined_subst.apply(right_arg);
                    let arg_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&arg_subst);
                }

                Ok(combined_subst)
            }

            // Different types cannot be unified
            _ => Err(TypeError::UnificationFailed {
                left: left.to_string(),
                right: right.to_string(),
                left_span: TypeError::unknown_span(),
                right_span: TypeError::unknown_span(),
            }),
        }
    }

    /// Check if a type variable occurs in a type (prevents infinite types)
    /// Uses iterative approach to avoid stack overflow with deeply nested types
    fn occurs_check(var_id: usize, initial_type: &CoreType) -> bool {
        let mut work_queue = vec![initial_type];

        while let Some(current_type) = work_queue.pop() {
            match *current_type {
                CoreType::Variable(ref var) => {
                    if var.id == var_id {
                        return true;
                    }
                }
                CoreType::Array(ref element_type) => {
                    work_queue.push(element_type.as_ref());
                }
                CoreType::Function {
                    parameters: ref params,
                    return_type: ref ret_type,
                } => {
                    work_queue.push(ret_type.as_ref());
                    work_queue.extend(params.iter());
                }
                CoreType::Generic {
                    type_args: ref args,
                    ..
                } => {
                    work_queue.extend(args.iter());
                }
                // Primitive types don't contain variables - skip them
                _ => {}
            }
        }

        false
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        Decl, Expr, Field, HotReloadMetadata, LetBinding, LiteralValue, NodeId, Parameter, Program,
        Stmt, StringPart, Type, TypeDef, Variant, Visibility as AstVisibility,
    };
    use crate::token::{Position, Span};

    // Test constants for semantic meaning instead of magic numbers
    const TEST_VAR_ID: usize = 0;
    const ANOTHER_TEST_VAR_ID: usize = 1;
    const THIRD_TEST_VAR_ID: usize = 42;

    // Helper function to create test spans
    fn test_span() -> Span {
        Span::single(Position::start())
    }

    fn node_id(id: usize) -> NodeId {
        NodeId(id)
    }

    fn literal_expr(value: LiteralValue, id: usize) -> Expr {
        Expr::Literal {
            value,
            span: test_span(),
            id: node_id(id),
        }
    }

    fn identifier_expr(name: &str, id: usize) -> Expr {
        Expr::Identifier {
            name: name.to_owned(),
            span: test_span(),
            id: node_id(id),
        }
    }

    fn int_type(name: &str) -> Type {
        Type::Basic {
            name: name.to_owned(),
            span: test_span(),
        }
    }

    fn create_program(declarations: Vec<Decl>) -> Program {
        Program {
            declarations,
            span: test_span(),
            id: node_id(900_000),
        }
    }

    fn make_parameter(name: &str, ty: Type) -> Parameter {
        Parameter {
            name: name.to_owned(),
            param_type: ty,
            span: test_span(),
        }
    }

    fn make_function_decl(
        name: &str,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Stmt,
        id: usize,
    ) -> Decl {
        Decl::Function {
            name: name.to_owned(),
            parameters: params,
            return_type,
            body,
            visibility: AstVisibility::Private,
            is_entry: false,
            doc_comment: None,
            span: test_span(),
            id: node_id(id),
            metadata: HotReloadMetadata::for_function(),
        }
    }

    fn make_let_decl(name: &str, annotation: Option<Type>, initializer: Expr, id: usize) -> Decl {
        let next_id = id.checked_add(1).unwrap_or(id);
        Decl::Let {
            binding: LetBinding {
                name: name.to_owned(),
                type_annotation: annotation,
                is_mutable: false,
                span: test_span(),
                id: node_id(id),
            },
            initializer,
            visibility: AstVisibility::Private,
            doc_comment: None,
            span: test_span(),
            id: node_id(next_id),
            metadata: HotReloadMetadata::for_let_declaration(),
        }
    }

    fn return_stmt(value: Expr, id: usize) -> Stmt {
        Stmt::Return {
            value: Some(value),
            span: test_span(),
            id: node_id(id),
        }
    }

    #[test]
    fn test_generic_type_instantiation() {
        let span = test_span();
        let ast_type = Type::Generic {
            name: "Result".to_owned(),
            type_args: vec![
                Type::Basic {
                    name: "int32".to_owned(),
                    span,
                },
                Type::Basic {
                    name: "string".to_owned(),
                    span,
                },
            ],
            span,
        };
        let core_type = TypeChecker::ast_type_to_core_type(&ast_type).unwrap();
        if let CoreType::Generic { name, type_args } = core_type {
            assert_eq!(name, "Result");
            assert_eq!(type_args.len(), 2);
            assert_eq!(type_args[0], CoreType::Int32);
            assert_eq!(type_args[1], CoreType::String);
        } else {
            unreachable!("Expected CoreType::Generic");
        }
    }

    #[test]
    fn test_adt_type_validation_sum() {
        let span = Span::single(Position::start());
        let variant = Variant {
            name: "Some".to_owned(),
            fields: vec![Field {
                name: "value".to_owned(),
                type_annotation: Type::Basic {
                    name: "int32".to_owned(),
                    span,
                },
                span,
            }],
            span,
        };
        let type_def = TypeDef::Sum {
            variants: vec![variant],
            span,
        };
        let checker = TypeChecker::new();
        assert!(checker.validate_adt_type(&type_def).is_ok());
    }

    #[test]
    fn test_adt_type_validation_product() {
        let span = Span::single(Position::start());
        let field = Field {
            name: "count".to_owned(),
            type_annotation: Type::Basic {
                name: "int32".to_owned(),
                span,
            },
            span,
        };
        let type_def = TypeDef::Product {
            fields: vec![field],
            span,
        };
        let checker = TypeChecker::new();
        assert!(checker.validate_adt_type(&type_def).is_ok());
    }

    #[test]
    fn test_pattern_match_type_check() {
        let checker = TypeChecker::new();
        let matched_type = CoreType::Int32;
        let patterns = vec![CoreType::Int32, CoreType::Int32];
        let arm_types = vec![CoreType::String, CoreType::String];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &patterns, &arm_types)
                .is_ok()
        );

        // Incompatible pattern
        let bad_patterns = vec![CoreType::String];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &bad_patterns, &arm_types)
                .is_err()
        );

        // Incompatible arm types
        let bad_arms = vec![CoreType::String, CoreType::Int32];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &patterns, &bad_arms)
                .is_err()
        );
    }

    #[test]
    fn test_type_environment_creation() {
        let env = TypeEnvironment::new();
        // Test basic types
        assert!(env.has_type("int32"));
        assert!(env.has_type("string"));
        assert!(env.has_type("boolean"));

        // Test extended integer types
        assert!(env.has_type("int8"));
        assert!(env.has_type("int16"));
        assert!(env.has_type("int64"));
        assert!(env.has_type("uint8"));
        assert!(env.has_type("uint16"));
        assert!(env.has_type("uint32"));
        assert!(env.has_type("uint64"));

        // Test floating point types
        assert!(env.has_type("float32"));
        assert!(env.has_type("float64"));

        // Test that non-existent types are not found
        assert!(!env.has_type("nonexistent"));
        assert!(!env.has_type("char"));
        assert!(!env.has_type("i32"));
    }

    #[test]
    fn test_type_environment_lookup() {
        let env = TypeEnvironment::new();
        let span = test_span();

        // Test basic types
        assert_eq!(env.lookup_type("int32", span).unwrap(), &CoreType::Int32);
        assert_eq!(env.lookup_type("string", span).unwrap(), &CoreType::String);
        assert_eq!(
            env.lookup_type("boolean", span).unwrap(),
            &CoreType::Boolean
        );

        // Test extended integer types
        assert_eq!(env.lookup_type("int8", span).unwrap(), &CoreType::Int8);
        assert_eq!(env.lookup_type("int16", span).unwrap(), &CoreType::Int16);
        assert_eq!(env.lookup_type("int64", span).unwrap(), &CoreType::Int64);
        assert_eq!(env.lookup_type("uint8", span).unwrap(), &CoreType::UInt8);
        assert_eq!(env.lookup_type("uint16", span).unwrap(), &CoreType::UInt16);
        assert_eq!(env.lookup_type("uint32", span).unwrap(), &CoreType::UInt32);
        assert_eq!(env.lookup_type("uint64", span).unwrap(), &CoreType::UInt64);

        // Test floating point types
        assert_eq!(
            env.lookup_type("float32", span).unwrap(),
            &CoreType::Float32
        );
        assert_eq!(
            env.lookup_type("float64", span).unwrap(),
            &CoreType::Float64
        );

        // Test unit type
        assert_eq!(env.lookup_type("unit", span).unwrap(), &CoreType::Unit);

        // Test non-existent type
        assert!(env.lookup_type("nonexistent", span).is_err());
    }

    #[test]
    fn test_type_environment_register() {
        let mut env = TypeEnvironment::new();
        let span = test_span();

        assert!(!env.has_type("custom"));
        env.register_type("custom".to_owned(), CoreType::Int32);
        assert!(env.has_type("custom"));
        assert_eq!(env.lookup_type("custom", span).unwrap(), &CoreType::Int32);
    }

    #[test]
    fn test_type_checker_creation() {
        let checker = TypeChecker::new();
        assert!(checker.environment().has_type("int32"));
        assert!(checker.environment().has_type("string"));
    }

    #[test]
    fn test_ast_type_to_core_type() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        let int32_type = Type::Basic {
            name: "int32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int32_type).unwrap(),
            CoreType::Int32
        );

        let string_type = Type::Basic {
            name: "string".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&string_type).unwrap(),
            CoreType::String
        );

        let invalid_type = Type::Basic {
            name: "nonexistent".to_owned(),
            span,
        };
        assert!(TypeChecker::ast_type_to_core_type(&invalid_type).is_err());
    }

    #[test]
    fn test_ast_type_to_core_type_extended_integers() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        // Test all integer types
        let int8_type = Type::Basic {
            name: "int8".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int8_type).unwrap(),
            CoreType::Int8
        );

        let int16_type = Type::Basic {
            name: "int16".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int16_type).unwrap(),
            CoreType::Int16
        );

        let uint8_type = Type::Basic {
            name: "uint8".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint8_type).unwrap(),
            CoreType::UInt8
        );

        let uint16_type = Type::Basic {
            name: "uint16".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint16_type).unwrap(),
            CoreType::UInt16
        );

        let uint32_type = Type::Basic {
            name: "uint32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint32_type).unwrap(),
            CoreType::UInt32
        );

        let uint64_type = Type::Basic {
            name: "uint64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint64_type).unwrap(),
            CoreType::UInt64
        );

        let int64_type = Type::Basic {
            name: "int64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int64_type).unwrap(),
            CoreType::Int64
        );
    }

    #[test]
    fn test_ast_type_to_core_type_float_types() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        let float32_type = Type::Basic {
            name: "float32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&float32_type).unwrap(),
            CoreType::Float32
        );

        let float64_type = Type::Basic {
            name: "float64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&float64_type).unwrap(),
            CoreType::Float64
        );
    }

    #[test]
    fn test_ast_type_to_core_type_complex_types() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        // Test that complex types now succeed
        let array_type = Type::Array {
            element_type: Box::new(Type::Basic {
                name: "int32".to_owned(),
                span,
            }),
            span,
        };
        let array_result = TypeChecker::ast_type_to_core_type(&array_type);
        assert!(array_result.is_ok());
        assert_eq!(
            array_result.unwrap(),
            CoreType::Array(Box::new(CoreType::Int32))
        );

        let function_type = Type::Function {
            parameters: vec![],
            return_type: Box::new(Type::Basic {
                name: "unit".to_owned(),
                span,
            }),
            span,
        };
        let function_result = TypeChecker::ast_type_to_core_type(&function_type);
        assert!(function_result.is_ok());
        assert_eq!(
            function_result.unwrap(),
            CoreType::Function {
                parameters: vec![],
                return_type: Box::new(CoreType::Unit),
            }
        );

        let generic_type = Type::Generic {
            name: "Array".to_owned(),
            type_args: vec![Type::Basic {
                name: "int32".to_owned(),
                span,
            }],
            span,
        };
        let generic_result = TypeChecker::ast_type_to_core_type(&generic_type);
        assert!(generic_result.is_ok());
        assert_eq!(
            generic_result.unwrap(),
            CoreType::Generic {
                name: "Array".to_owned(),
                type_args: vec![CoreType::Int32],
            }
        );
    }

    #[test]
    fn test_types_compatible() {
        let checker = TypeChecker::new();
        assert!(checker.types_compatible(&CoreType::Int32, &CoreType::Int32));
        assert!(checker.types_compatible(&CoreType::String, &CoreType::String));
        assert!(!checker.types_compatible(&CoreType::Int32, &CoreType::String));
        assert!(!checker.types_compatible(&CoreType::Boolean, &CoreType::Float32));
    }

    #[test]
    fn test_validate_type_name() {
        let checker = TypeChecker::new();
        let span = test_span();

        // Valid type name for existing type
        assert!(
            checker
                .validate_type_name("int32", &CoreType::Int32, span)
                .is_ok()
        );

        // Invalid type name for different type
        assert!(
            checker
                .validate_type_name("int32", &CoreType::String, span)
                .is_err()
        );

        // New type name should be valid
        assert!(
            checker
                .validate_type_name("custom", &CoreType::Int32, span)
                .is_ok()
        );
    }

    #[test]
    fn test_core_type_equality() {
        assert_eq!(CoreType::Int32, CoreType::Int32);
        assert_ne!(CoreType::Int32, CoreType::Int64);
        assert_ne!(CoreType::String, CoreType::Boolean);
    }

    #[test]
    fn test_type_error_messages() {
        let not_found = TypeError::TypeNotFound {
            type_name: "test".to_owned(),
            span: TypeError::unknown_span(),
        };
        assert!(not_found.to_string().contains("Type 'test' not found"));

        let mismatch = TypeError::TypeMismatch {
            expected: "int32".to_owned(),
            found: "string".to_owned(),
            found_span: TypeError::unknown_span(),
            expected_span: None,
        };
        assert!(mismatch.to_string().contains("Type mismatch"));
        assert!(mismatch.to_string().contains("expected 'int32'"));
        assert!(mismatch.to_string().contains("found 'string'"));
    }

    #[test]
    fn test_environment_get_type_names() {
        let env = TypeEnvironment::new();
        let type_names = env.get_type_names();

        assert!(type_names.iter().any(|name| name == "int8"));
        assert!(type_names.iter().any(|name| name == "int16"));
        assert!(type_names.iter().any(|name| name == "int32"));
        assert!(type_names.iter().any(|name| name == "int64"));
        assert!(type_names.iter().any(|name| name == "uint8"));
        assert!(type_names.iter().any(|name| name == "uint16"));
        assert!(type_names.iter().any(|name| name == "uint32"));
        assert!(type_names.iter().any(|name| name == "uint64"));
        assert!(type_names.iter().any(|name| name == "float32"));
        assert!(type_names.iter().any(|name| name == "float64"));
        assert!(type_names.iter().any(|name| name == "string"));
        assert!(type_names.iter().any(|name| name == "boolean"));
        assert!(type_names.iter().any(|name| name == "unit"));

        // Ensure we have the minimum expected built-in types
        assert!(
            type_names.len() >= 13,
            "Expected at least 13 built-in types, found {}",
            type_names.len()
        );

        // Ensure names are sorted
        let mut sorted_names = type_names.clone();
        sorted_names.sort();
        assert_eq!(
            type_names, sorted_names,
            "Type names should be returned in sorted order"
        );
    }

    #[test]
    fn test_type_var_creation() {
        let var = TypeVar::new(THIRD_TEST_VAR_ID, "test_var".to_owned());
        assert_eq!(var.id, THIRD_TEST_VAR_ID);
        assert_eq!(var.name, "test_var");
    }

    #[test]
    fn test_substitution_empty() {
        let subst = Substitution::empty();
        assert!(subst.is_empty());
        assert_eq!(subst.mappings().len(), 0);
    }

    #[test]
    fn test_substitution_single() {
        let var_id = 0;
        let core_type = CoreType::Int32;
        let subst = Substitution::single(var_id, core_type.clone());

        assert!(!subst.is_empty());
        assert_eq!(subst.mappings().len(), 1);
        assert_eq!(subst.mappings().get(&var_id), Some(&core_type));
    }

    #[test]
    fn test_substitution_apply_primitive() {
        let subst = Substitution::empty();
        let int_type = CoreType::Int32;

        // Applying substitution to primitive type should return the same type
        assert_eq!(subst.apply(&int_type), int_type);
    }

    #[test]
    fn test_substitution_apply_variable() {
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let int_type = CoreType::Int32;

        // Apply substitution that maps the variable to int32
        let subst = Substitution::single(var.id, int_type.clone());
        assert_eq!(subst.apply(&var_type), int_type);

        // Apply empty substitution should return the variable unchanged
        let empty_subst = Substitution::empty();
        assert_eq!(empty_subst.apply(&var_type), var_type);
    }

    #[test]
    fn test_substitution_apply_array() {
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var_type = CoreType::Array(Box::new(var_type));

        let subst = Substitution::single(var.id, CoreType::Int32);
        let expected = CoreType::Array(Box::new(CoreType::Int32));

        assert_eq!(subst.apply(&array_var_type), expected);
    }

    #[test]
    fn test_substitution_apply_function() {
        let var1 = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var2 = TypeVar::new(ANOTHER_TEST_VAR_ID, "y".to_owned());
        let var1_type = CoreType::Variable(var1.clone());
        let var2_type = CoreType::Variable(var2.clone());

        let function_type = CoreType::Function {
            parameters: vec![var1_type],
            return_type: Box::new(var2_type),
        };

        let mut mappings = BTreeMap::new();
        mappings.insert(var1.id, CoreType::Int32);
        mappings.insert(var2.id, CoreType::String);
        let subst = Substitution { mappings };

        let expected = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };

        assert_eq!(subst.apply(&function_type), expected);
    }

    #[test]
    fn test_substitution_compose() {
        // s1 maps x -> int32
        let s1 = Substitution::single(TEST_VAR_ID, CoreType::Int32);

        // s2 maps y -> x (which should become int32 after composition)
        let var_x = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let s2 = Substitution::single(ANOTHER_TEST_VAR_ID, CoreType::Variable(var_x));

        // Compose s1 after s2: s1(s2(...))
        let composed = s1.compose(&s2);

        // Should have mapping for y -> int32 and x -> int32
        assert_eq!(composed.mappings().len(), 2);
        assert_eq!(
            composed.mappings().get(&TEST_VAR_ID),
            Some(&CoreType::Int32)
        );
        assert_eq!(
            composed.mappings().get(&ANOTHER_TEST_VAR_ID),
            Some(&CoreType::Int32)
        );
    }

    #[test]
    fn test_fresh_type_var_generation() {
        let mut checker = TypeChecker::new();
        let span = test_span();

        let var1 = checker
            .fresh_type_var("test".to_owned(), span)
            .expect("Should generate fresh type var");
        let var2 = checker
            .fresh_type_var_auto(span)
            .expect("Should generate fresh type var");

        // Should generate different variables
        assert_ne!(var1, var2);

        // Check they are variables
        assert!(matches!(var1, CoreType::Variable(_)));
        assert!(matches!(var2, CoreType::Variable(_)));
    }

    #[test]
    fn test_unify_identical_primitives() {
        let checker = TypeChecker::new();

        let int_result = checker.unify(&CoreType::Int32, &CoreType::Int32);
        assert!(int_result.is_ok());
        assert!(int_result.unwrap().is_empty());

        let string_result = checker.unify(&CoreType::String, &CoreType::String);
        assert!(string_result.is_ok());
        assert!(string_result.unwrap().is_empty());
    }

    #[test]
    fn test_unify_different_primitives() {
        let checker = TypeChecker::new();

        let mismatch_result = checker.unify(&CoreType::Int32, &CoreType::String);
        assert!(mismatch_result.is_err());

        if let Err(TypeError::UnificationFailed { left, right, .. }) = mismatch_result {
            assert!(left.contains("int32"));
            assert!(right.contains("string"));
        } else {
            unreachable!("Expected UnificationFailed error");
        }
    }

    #[test]
    fn test_unify_variable_with_type() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let int_type = CoreType::Int32;

        let result = checker.unify(&var_type, &int_type);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert!(!subst.is_empty());
        assert_eq!(subst.mappings().get(&var.id), Some(&int_type));
    }

    #[test]
    fn test_unify_variable_with_variable() {
        let checker = TypeChecker::new();
        let var1 = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var2 = TypeVar::new(ANOTHER_TEST_VAR_ID, "y".to_owned());
        let var1_type = CoreType::Variable(var1.clone());
        let var2_type = CoreType::Variable(var2.clone());

        let result = checker.unify(&var1_type, &var2_type);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert!(!subst.is_empty());
        // One variable should be mapped to the other
        assert!(subst.mappings().contains_key(&var1.id) || subst.mappings().contains_key(&var2.id));
    }

    #[test]
    fn test_unify_arrays() {
        let checker = TypeChecker::new();
        let array_int = CoreType::Array(Box::new(CoreType::Int32));
        let array_string = CoreType::Array(Box::new(CoreType::String));

        // Arrays with same element type should unify
        let same_result = checker.unify(&array_int, &array_int);
        assert!(same_result.is_ok());
        assert!(same_result.unwrap().is_empty());

        // Arrays with different element types should not unify
        let different_result = checker.unify(&array_int, &array_string);
        assert!(different_result.is_err());
    }

    #[test]
    fn test_unify_arrays_with_variables() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var = CoreType::Array(Box::new(var_type));
        let array_int = CoreType::Array(Box::new(CoreType::Int32));

        let result = checker.unify(&array_var, &array_int);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert_eq!(subst.mappings().get(&var.id), Some(&CoreType::Int32));
    }

    #[test]
    fn test_unify_functions() {
        let checker = TypeChecker::new();
        let func1 = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };
        let func2 = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };
        let func3 = CoreType::Function {
            parameters: vec![CoreType::String],
            return_type: Box::new(CoreType::Int32),
        };

        // Identical functions should unify
        let same_result = checker.unify(&func1, &func2);
        assert!(same_result.is_ok());
        assert!(same_result.unwrap().is_empty());

        // Different functions should not unify
        let different_result = checker.unify(&func1, &func3);
        assert!(different_result.is_err());
    }

    #[test]
    fn test_occurs_check() {
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());

        // Variable should occur in itself
        assert!(TypeChecker::occurs_check(var.id, &var_type));

        // Variable should occur in array containing it
        let array_var = CoreType::Array(Box::new(var_type));
        assert!(TypeChecker::occurs_check(var.id, &array_var));

        // Variable should not occur in different type
        assert!(!TypeChecker::occurs_check(var.id, &CoreType::Int32));

        // Variable should not occur in array of different type
        let array_int = CoreType::Array(Box::new(CoreType::Int32));
        assert!(!TypeChecker::occurs_check(var.id, &array_int));
    }

    #[test]
    fn test_occurs_check_prevents_infinite_types() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var = CoreType::Array(Box::new(var_type));

        // Trying to unify x with Array<x> should fail
        let infinite_result = checker.unify(&CoreType::Variable(var.clone()), &array_var);
        assert!(infinite_result.is_err());

        if let Err(TypeError::OccursCheckFailed {
            var_name,
            type_name,
            ..
        }) = infinite_result
        {
            assert_eq!(var_name, var.name);
            assert!(type_name.contains('[') && type_name.contains('x'));
        } else {
            unreachable!("Expected OccursCheckFailed error");
        }
    }

    #[test]
    fn test_symbol_table_scope_management() {
        let mut table = SymbolTable::new();

        // Register in global scope
        table.register(SymbolInfo {
            name: "global_var".to_owned(),
            symbol_type: SymbolType::Variable,
            core_type: CoreType::Int32,
            visibility: Visibility::Public,
            source_location: Span::single(Position::start()),
        });

        // Should find global variable
        assert!(table.contains("global_var"));
        assert!(table.lookup("global_var").is_some());

        // Enter function scope
        let func_scope = table.enter_scope();
        assert_ne!(func_scope, ScopeId(0));

        // Register parameter in function scope
        table.register(SymbolInfo {
            name: "param".to_owned(),
            symbol_type: SymbolType::Variable,
            core_type: CoreType::String,
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
        });

        // Should find both global and local
        assert!(
            table.contains("global_var"),
            "Should find global variable from nested scope"
        );
        assert!(table.contains("param"), "Should find local parameter");
        assert!(
            table.lookup_local("param").is_some(),
            "Should find param in current scope"
        );
        assert!(
            table.lookup_local("global_var").is_none(),
            "Should not find global_var in current scope only"
        );

        // Enter block scope
        let block_scope = table.enter_scope();
        assert_ne!(block_scope, func_scope);

        // Register local variable in block
        table.register(SymbolInfo {
            name: "local_var".to_owned(),
            symbol_type: SymbolType::Variable,
            core_type: CoreType::Boolean,
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
        });

        // Should find all three variables
        assert!(table.contains("global_var"));
        assert!(table.contains("param"));
        assert!(table.contains("local_var"));

        // Exit block scope
        table.exit_scope();

        // Should still find global and param, but not local_var
        assert!(table.contains("global_var"));
        assert!(table.contains("param"));
        assert!(
            !table.contains("local_var"),
            "local_var should not be accessible after exiting scope"
        );

        // Exit function scope
        table.exit_scope();

        // Should only find global
        assert!(table.contains("global_var"));
        assert!(
            !table.contains("param"),
            "param should not be accessible after exiting function scope"
        );
        assert!(!table.contains("local_var"));
    }

    #[test]
    fn test_symbol_table_shadowing() {
        let mut table = SymbolTable::new();

        // Register in global scope
        table.register(SymbolInfo {
            name: "x".to_owned(),
            symbol_type: SymbolType::Variable,
            core_type: CoreType::Int32,
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
        });

        let global_x = table.lookup("x").unwrap();
        assert_eq!(global_x.core_type, CoreType::Int32);

        // Enter scope and shadow x
        table.enter_scope();
        table.register(SymbolInfo {
            name: "x".to_owned(),
            symbol_type: SymbolType::Variable,
            core_type: CoreType::String,
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
        });

        // Should find shadowed version
        let shadowed_x = table.lookup("x").unwrap();
        assert_eq!(
            shadowed_x.core_type,
            CoreType::String,
            "Should find shadowed version"
        );

        // Exit scope
        table.exit_scope();

        // Should find original version again
        let original_x = table.lookup("x").unwrap();
        assert_eq!(
            original_x.core_type,
            CoreType::Int32,
            "Should find original version after exiting scope"
        );
    }

    #[test]
    fn test_symbol_table_exported_symbols() {
        let mut table = SymbolTable::new();

        // Register public symbol in global scope
        table.register(SymbolInfo {
            name: "public_func".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: CoreType::Function {
                parameters: vec![],
                return_type: Box::new(CoreType::Unit),
            },
            visibility: Visibility::Public,
            source_location: Span::single(Position::start()),
        });

        // Register entry point in global scope
        table.register(SymbolInfo {
            name: "main".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: CoreType::Function {
                parameters: vec![],
                return_type: Box::new(CoreType::Unit),
            },
            visibility: Visibility::Entry,
            source_location: Span::single(Position::start()),
        });

        // Register private symbol in global scope
        table.register(SymbolInfo {
            name: "private_func".to_owned(),
            symbol_type: SymbolType::Function,
            core_type: CoreType::Function {
                parameters: vec![],
                return_type: Box::new(CoreType::Unit),
            },
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
        });

        let exported = table.exported_symbols();
        assert_eq!(exported.len(), 2, "Should have 2 exported symbols");
        assert!(exported.iter().any(|s| s.name == "public_func"));
        assert!(exported.iter().any(|s| s.name == "main"));
        assert!(!exported.iter().any(|s| s.name == "private_func"));
    }

    #[test]
    fn test_type_check_literal_expression() {
        let mut checker = TypeChecker::new();
        let expr = literal_expr(LiteralValue::Integer(42), 10_000);
        let ty = checker
            .type_check_expr(&expr)
            .expect("literal expressions should type check");
        assert_eq!(ty, CoreType::Int64, "integer literals default to int64");
    }

    #[test]
    fn test_type_check_array_literal_with_consistent_elements() {
        let mut checker = TypeChecker::new();
        let array_expr = Expr::Array {
            elements: vec![
                literal_expr(LiteralValue::Integer(1), 20_000),
                literal_expr(LiteralValue::Integer(2), 20_001),
            ],
            span: test_span(),
            id: node_id(20_002),
        };

        let ty = checker
            .type_check_expr(&array_expr)
            .expect("consistent element types should infer array type");

        assert_eq!(ty, CoreType::Array(Box::new(CoreType::Int64)));
    }

    #[test]
    fn test_type_check_array_literal_detects_mismatched_elements() {
        let mut checker = TypeChecker::new();
        let array_expr = Expr::Array {
            elements: vec![
                literal_expr(LiteralValue::Integer(1), 20_010),
                literal_expr(LiteralValue::String("oops".to_owned()), 20_011),
            ],
            span: test_span(),
            id: node_id(20_012),
        };

        let result = checker.type_check_expr(&array_expr);
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "array literals must enforce uniform element types"
        );
    }

    #[test]
    fn test_type_check_string_interpolation_rejects_non_displayable_expression() {
        let mut checker = TypeChecker::new();
        let array_expr = Expr::Array {
            elements: vec![literal_expr(LiteralValue::Integer(7), 20_020)],
            span: test_span(),
            id: node_id(20_021),
        };
        let interpolation = Expr::StringInterpolation {
            parts: vec![
                StringPart::Literal("value: ".to_owned()),
                StringPart::Expression(array_expr),
            ],
            span: test_span(),
            id: node_id(20_022),
        };

        let result = checker.type_check_expr(&interpolation);
        assert!(
            matches!(result, Err(TypeError::InvalidOperation { .. })),
            "string interpolation should reject non-displayable values"
        );
    }

    #[test]
    fn test_type_check_let_statement_registers_symbol() {
        let mut checker = TypeChecker::new();
        let binding = LetBinding {
            name: "value".to_owned(),
            type_annotation: Some(Type::Basic {
                name: "int64".to_owned(),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: node_id(10_100),
        };

        let stmt = Stmt::Let {
            binding,
            initializer: Some(literal_expr(LiteralValue::Integer(1), 10_101)),
            span: test_span(),
            id: node_id(10_102),
        };

        checker
            .type_check_stmt(&stmt)
            .expect("let with matching initializer should type check");

        let symbol = checker
            .symbol_table()
            .lookup("value")
            .expect("binding should be registered");
        assert_eq!(symbol.core_type, CoreType::Int64);
    }

    #[test]
    fn test_type_check_assignment_type_mismatch() {
        let mut checker = TypeChecker::new();
        let binding = LetBinding {
            name: "value".to_owned(),
            type_annotation: Some(Type::Basic {
                name: "int64".to_owned(),
                span: test_span(),
            }),
            is_mutable: true,
            span: test_span(),
            id: node_id(10_110),
        };

        let let_stmt = Stmt::Let {
            binding,
            initializer: Some(literal_expr(LiteralValue::Integer(10), 10_111)),
            span: test_span(),
            id: node_id(10_112),
        };

        checker
            .type_check_stmt(&let_stmt)
            .expect("initial declaration should succeed");

        let assignment = Stmt::Assignment {
            target: identifier_expr("value", 10_113),
            value: literal_expr(LiteralValue::String("oops".to_owned()), 10_114),
            span: test_span(),
            id: node_id(10_115),
        };

        let result = checker.type_check_stmt(&assignment);
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "assignment should fail due to mismatched types"
        );
    }

    #[test]
    fn test_type_check_for_loop_registers_loop_variable_in_body_scope() {
        let mut checker = TypeChecker::new();
        let for_stmt = Stmt::For {
            variable: "item".to_owned(),
            iterable: Expr::Array {
                elements: vec![literal_expr(LiteralValue::Integer(1), 20_100)],
                span: test_span(),
                id: node_id(20_101),
            },
            body: Box::new(Stmt::Expression {
                expr: identifier_expr("item", 20_102),
                span: test_span(),
                id: node_id(20_103),
            }),
            span: test_span(),
            id: node_id(20_104),
        };

        checker
            .type_check_stmt(&for_stmt)
            .expect("for loop over array should type check");

        assert!(
            checker.symbol_table().lookup("item").is_none(),
            "loop variable should not escape its scope"
        );
    }

    #[test]
    fn test_type_check_for_loop_requires_iterable_array() {
        let mut checker = TypeChecker::new();
        let for_stmt = Stmt::For {
            variable: "value".to_owned(),
            iterable: literal_expr(LiteralValue::Integer(1), 20_110),
            body: Box::new(Stmt::Expression {
                expr: literal_expr(LiteralValue::Void, 20_111),
                span: test_span(),
                id: node_id(20_112),
            }),
            span: test_span(),
            id: node_id(20_113),
        };

        let result = checker.type_check_stmt(&for_stmt);
        assert!(
            matches!(result, Err(TypeError::InvalidOperation { .. })),
            "for loop should reject non-iterable types"
        );
    }

    #[test]
    fn test_type_check_return_enforces_expected_type() {
        let mut checker = TypeChecker::new();
        let return_stmt = Stmt::Return {
            value: Some(literal_expr(LiteralValue::String("bad".to_owned()), 20_120)),
            span: test_span(),
            id: node_id(20_121),
        };

        let expected = CoreType::Int32;
        let result = checker.type_check_stmt_with_return(&return_stmt, Some(&expected));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "return statements must match expected return type"
        );
    }

    #[test]
    fn test_type_check_if_requires_boolean_condition() {
        let mut checker = TypeChecker::new();
        let condition = literal_expr(LiteralValue::Integer(1), 10_120);
        let then_branch = Stmt::Expression {
            expr: literal_expr(LiteralValue::Void, 10_121),
            span: test_span(),
            id: node_id(10_122),
        };

        let if_stmt = Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: None,
            span: test_span(),
            id: node_id(10_123),
        };

        let result = checker.type_check_stmt(&if_stmt);
        assert!(
            matches!(result, Err(TypeError::InvalidOperation { .. })),
            "non-boolean conditions must be rejected"
        );
    }

    #[test]
    fn test_type_check_program_collects_errors() {
        let mut checker = TypeChecker::new();
        let decl = Decl::Function {
            name: "bad".to_owned(),
            parameters: vec![Parameter {
                name: "x".to_owned(),
                param_type: Type::Basic {
                    name: "int32".to_owned(),
                    span: test_span(),
                },
                span: test_span(),
            }],
            return_type: Some(Type::Basic {
                name: "int32".to_owned(),
                span: test_span(),
            }),
            body: Stmt::Return {
                value: Some(literal_expr(LiteralValue::Boolean(true), 10_200)),
                span: test_span(),
                id: node_id(10_201),
            },
            visibility: AstVisibility::Private,
            is_entry: false,
            doc_comment: None,
            span: test_span(),
            id: node_id(10_202),
            metadata: HotReloadMetadata::for_function(),
        };

        let program = Program {
            declarations: vec![decl],
            span: test_span(),
            id: node_id(10_203),
        };

        let result = checker.type_check_program(&program);
        assert!(result.is_err(), "program should fail type checking");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1, "exactly one type mismatch expected");
        assert!(matches!(errors[0], TypeError::TypeMismatch { .. }));
    }

    #[test]
    #[should_panic(expected = "Cannot exit global scope")]
    fn test_symbol_table_cannot_exit_global_scope() {
        let mut table = SymbolTable::new();
        table.exit_scope(); // Should panic
    }

    #[test]
    fn test_type_check_program_handles_forward_function_reference() {
        let mut checker = TypeChecker::new();

        let call_expr = Expr::Call {
            callee: Box::new(identifier_expr("future_fn", 30_000)),
            args: vec![],
            span: test_span(),
            id: node_id(30_001),
        };

        let let_decl = make_let_decl("value", Some(int_type("int32")), call_expr, 30_010);

        let fn_return = literal_expr(LiteralValue::Integer(42), 30_020);
        let function_body = return_stmt(fn_return, 30_021);
        let fn_decl = make_function_decl(
            "future_fn",
            vec![],
            Some(int_type("int32")),
            function_body,
            30_030,
        );

        let program = create_program(vec![let_decl, fn_decl]);
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "forward references should resolve once declarations are registered"
        );
    }

    #[test]
    fn test_type_check_program_accumulates_multiple_errors() {
        let mut checker = TypeChecker::new();

        let first_fn = make_function_decl(
            "bad_one",
            vec![make_parameter("x", int_type("int32"))],
            Some(int_type("int32")),
            return_stmt(literal_expr(LiteralValue::Boolean(true), 31_100), 31_101),
            31_102,
        );

        let second_fn = make_function_decl(
            "second_bad",
            vec![],
            Some(int_type("boolean")),
            return_stmt(literal_expr(LiteralValue::Integer(5), 31_110), 31_111),
            31_112,
        );

        let program = create_program(vec![first_fn, second_fn]);
        let result = checker.type_check_program(&program);
        assert!(result.is_err(), "program should report collected errors");
        let errors = result.unwrap_err();
        assert!(
            errors.len() >= 2,
            "expected at least two independent errors"
        );
    }

    #[test]
    fn test_type_check_program_reports_let_type_mismatch() {
        let mut checker = TypeChecker::new();

        let let_decl = make_let_decl(
            "value",
            Some(int_type("int32")),
            literal_expr(LiteralValue::Boolean(true), 31_200),
            31_201,
        );

        let program = create_program(vec![let_decl]);
        let result = checker.type_check_program(&program);
        assert!(result.is_err(), "mismatched let annotations must fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|err| matches!(*err, TypeError::TypeMismatch { .. })),
            "expected a type mismatch error for the let declaration"
        );
    }

    #[test]
    fn test_lambda_expression_body_type_checking() {
        let mut checker = TypeChecker::new();
        let lambda = Expr::Lambda {
            generic_params: None,
            params: vec![make_parameter("x", int_type("int32"))],
            return_type: int_type("int32"),
            body: crate::ast::LambdaBody::Expression(Box::new(identifier_expr("x", 32_000))),
            captured_variables: vec![],
            metadata: HotReloadMetadata::for_expression(),
            span: test_span(),
            id: node_id(32_001),
        };

        let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
        assert!(
            result.is_ok(),
            "lambda expression should type check successfully"
        );
        let core_type = result.unwrap();
        if let CoreType::Function {
            parameters,
            return_type,
        } = core_type
        {
            assert_eq!(parameters, vec![CoreType::Int32]);
            assert_eq!(*return_type, CoreType::Int32);
        } else {
            unreachable!("lambda should yield a function type");
        }
    }

    #[test]
    fn test_lambda_block_body_type_checking() {
        let mut checker = TypeChecker::new();
        let return_stmt = Stmt::Return {
            value: Some(identifier_expr("x", 32_100)),
            span: test_span(),
            id: node_id(32_101),
        };
        let body = Stmt::Block {
            statements: vec![return_stmt],
            span: test_span(),
            id: node_id(32_102),
        };
        let lambda = Expr::Lambda {
            generic_params: None,
            params: vec![make_parameter("x", int_type("int32"))],
            return_type: int_type("int32"),
            body: crate::ast::LambdaBody::Block(vec![body]),
            captured_variables: vec![],
            metadata: HotReloadMetadata::for_expression(),
            span: test_span(),
            id: node_id(32_103),
        };

        let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
        assert!(
            result.is_ok(),
            "lambda block body should type check successfully"
        );
        let core_type = result.unwrap();
        if let CoreType::Function {
            parameters,
            return_type,
        } = core_type
        {
            assert_eq!(parameters, vec![CoreType::Int32]);
            assert_eq!(*return_type, CoreType::Int32);
        } else {
            unreachable!("lambda should yield a function type");
        }
    }

    #[test]
    fn test_lambda_return_type_mismatch_is_reported() {
        let mut checker = TypeChecker::new();
        let lambda = Expr::Lambda {
            generic_params: None,
            params: vec![make_parameter("x", int_type("int32"))],
            return_type: int_type("int32"),
            body: crate::ast::LambdaBody::Expression(Box::new(literal_expr(
                LiteralValue::Boolean(true),
                32_200,
            ))),
            captured_variables: vec![],
            metadata: HotReloadMetadata::for_expression(),
            span: test_span(),
            id: node_id(32_201),
        };

        let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "lambda returning the wrong type should fail"
        );
    }

    #[test]
    fn test_solve_constraints_unifies_equalities() {
        let mut checker = TypeChecker::new();
        let span = test_span();
        let var_a = checker
            .fresh_type_var_auto(span)
            .expect("should create type variable");
        let var_b = checker
            .fresh_type_var_auto(span)
            .expect("should create type variable");

        checker.add_constraint(TypeConstraint::Equality(var_a.clone(), CoreType::Int32));
        checker.add_constraint(TypeConstraint::Equality(var_b.clone(), var_a.clone()));

        let subst = checker
            .solve_constraints()
            .expect("constraints should solve successfully");

        assert_eq!(subst.apply(&var_a), CoreType::Int32);
        assert_eq!(subst.apply(&var_b), CoreType::Int32);
    }

    #[test]
    fn test_solve_constraints_detects_conflicts() {
        let mut checker = TypeChecker::new();
        checker.add_constraint(TypeConstraint::Equality(CoreType::Int32, CoreType::String));
        let result = checker.solve_constraints();
        assert!(
            matches!(result, Err(TypeError::UnificationFailed { .. })),
            "conflicting constraints must fail"
        );
    }

    #[test]
    fn test_solve_constraints_composes_substitutions() {
        let mut checker = TypeChecker::new();
        let span = test_span();
        let var_a = checker
            .fresh_type_var_auto(span)
            .expect("should create type variable");
        let var_b = checker
            .fresh_type_var_auto(span)
            .expect("should create type variable");
        let var_c = checker
            .fresh_type_var_auto(span)
            .expect("should create type variable");

        checker.add_constraint(TypeConstraint::Equality(var_a.clone(), CoreType::Int32));
        checker.add_constraint(TypeConstraint::Equality(var_b.clone(), var_a.clone()));
        checker.add_constraint(TypeConstraint::Equality(var_c.clone(), CoreType::Boolean));

        let subst = checker
            .solve_constraints()
            .expect("constraints should compose correctly");

        assert_eq!(subst.apply(&var_a), CoreType::Int32);
        assert_eq!(subst.apply(&var_b), CoreType::Int32);
        assert_eq!(subst.apply(&var_c), CoreType::Boolean);
    }
}
