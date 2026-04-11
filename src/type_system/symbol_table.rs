//! Symbol table for tracking symbols and their scopes

extern crate alloc;

use super::types::CoreType;
use crate::token::Span;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::{string::String, vec, vec::Vec};

/// Information about the current function context for error handling checks.
#[derive(Debug, Clone)]
struct FunctionContext {
    /// The error types declared by the function.
    error_types: Vec<CoreType>,
    /// The span of the function's signature, for error reporting.
    span: Span,
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
    /// Whether this symbol was declared from a `let` binding.
    pub is_let_binding: bool,
    /// Whether this symbol allows reassignment.
    pub is_mutable: bool,
    /// Number of read usages observed during type checking.
    pub read_count: usize,
}

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub(crate) usize);

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
    /// Stack of function contexts for tracking error declarations.
    function_context_stack: Vec<FunctionContext>,
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
            function_context_stack: Vec::new(),
        }
    }

    /// Pushes a new function context onto the stack when entering a function body.
    pub fn enter_function(&mut self, error_types: Vec<CoreType>, span: Span) {
        self.function_context_stack
            .push(FunctionContext { error_types, span });
    }

    /// Pops the current function context from the stack when leaving a function body.
    pub fn exit_function(&mut self) {
        self.function_context_stack.pop();
    }

    /// Returns the error types declared by the current function, if any.
    pub fn current_function_error_types(&self) -> Option<&[CoreType]> {
        self.function_context_stack
            .last()
            .map(|ctx| ctx.error_types.as_slice())
    }

    /// Returns the span of the current function's signature, if available.
    pub fn current_function_span(&self) -> Option<Span> {
        self.function_context_stack.last().map(|ctx| ctx.span)
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

    /// Look up a symbol mutably in the current scope and parent scopes.
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut SymbolInfo> {
        let mut current = Some(self.current_scope);
        while let Some(scope_id) = current {
            let parent = self.scopes[scope_id.0].parent;
            if self.scopes[scope_id.0].symbols.contains_key(name) {
                return self.scopes[scope_id.0].symbols.get_mut(name);
            }
            current = parent;
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

    /// Collect all `let` bindings that were never read.
    pub fn unused_let_bindings(&self) -> Vec<&SymbolInfo> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.symbols.values())
            .filter(|symbol| {
                symbol.is_let_binding && symbol.read_count == 0 && !symbol.name.starts_with('_')
            })
            .collect()
    }

    /// Collect all symbol names visible from the current scope.
    pub fn visible_symbol_names(&self) -> Vec<String> {
        let mut current = Some(self.current_scope);
        let mut unique_names: BTreeSet<String> = BTreeSet::new();

        while let Some(scope_id) = current {
            for name in self.scopes[scope_id.0].symbols.keys() {
                unique_names.insert(name.clone());
            }
            current = self.scopes[scope_id.0].parent;
        }

        unique_names.into_iter().collect()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
