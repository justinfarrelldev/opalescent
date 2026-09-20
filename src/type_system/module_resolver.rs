extern crate alloc;

use crate::ast::{
    BorrowKind, DeclarationAnnotation, TypeDeclarationForm, TypeDef, Visibility as AstVisibility,
};
use crate::token::{Position, Span};
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::{format, string::String, vec::Vec};

/// Built-in module registration helpers shared by resolver initialization.
mod standard_modules;
/// Core I/O and bytes symbol declarations for the `standard` module.
mod standard_symbols_core_io_and_bytes;
/// Filesystem operation symbol declarations for the `standard` module.
mod standard_symbols_filesystem_operations;
/// Filesystem type and error symbol declarations for the `standard` module.
mod standard_symbols_filesystem_types_and_errors;
/// Process-module symbol declarations.
mod standard_symbols_process;
/// Terminal-session proposal ABI/history validation helpers.
mod terminal_proposal_abi;
/// Terminal-session proposal function borrow metadata.
mod terminal_proposal_borrows;
/// Implemented Task 16 error-inspector inventory.
mod terminal_proposal_error_inspectors;
/// Terminal-session proposal declaration interfaces.
mod terminal_proposal_modules;
/// Proposal symbols that already have real runtime/codegen lowering.
mod terminal_proposal_runtime_ready;
/// Terminal-session proposal function signature inventory.
mod terminal_proposal_symbols;

/// Return whether a module path belongs to the selected terminal proposal inventory.
#[must_use]
pub(super) fn is_terminal_proposal_module_path(module_path: &str) -> bool {
    terminal_proposal_symbols::contains_module(module_path)
}

/// Return whether a module/symbol pair belongs to the gated terminal proposal surface.
#[must_use]
pub(super) fn is_terminal_proposal_codegen_gated_import(
    module_path: &str,
    symbol_name: &str,
) -> bool {
    terminal_proposal_symbols::contains_function(module_path, symbol_name)
        && !terminal_proposal_runtime_ready::contains_import(module_path, symbol_name)
}

/// Return whether a module/symbol pair is an implemented Task 16 error inspector.
#[must_use]
pub(super) fn is_terminal_proposal_implemented_error_inspector_import(
    module_path: &str,
    symbol_name: &str,
) -> bool {
    terminal_proposal_error_inspectors::contains_implemented_error_inspector(
        module_path,
        symbol_name,
    )
}

/// Return whether a runtime symbol name belongs to any gated terminal proposal surface.
#[must_use]
pub(super) fn is_terminal_proposal_codegen_gated_runtime_name(symbol_name: &str) -> bool {
    terminal_proposal_symbols::contains_function_name(symbol_name)
        && !terminal_proposal_runtime_ready::contains_runtime_name(symbol_name)
}

/// Return whether a selected terminal proposal type name uses product-constructor syntax.
#[must_use]
pub(super) fn is_terminal_proposal_constructible_product_type(type_name: &str) -> bool {
    if !type_name.starts_with("Terminal") {
        return false;
    }

    terminal_proposal_type_declaration(type_name)
        .is_some_and(|declaration| matches!(declaration.type_def, TypeDef::Product { .. }))
}

/// Resolve the authoritative selected terminal proposal ABI tag for one variant.
#[must_use]
pub(super) fn terminal_proposal_variant_id(type_name: &str, variant_name: &str) -> Option<i64> {
    if !type_name.starts_with("Terminal") {
        return None;
    }

    let declaration = terminal_proposal_type_declaration(type_name)?;
    let TypeDef::Sum { ref variants, .. } = declaration.type_def else {
        return None;
    };
    variants
        .iter()
        .find(|variant| variant.name == variant_name)
        .and_then(|variant| variant.explicit_id)
}

/// Resolve a terminal proposal type declaration by name from the selected terminal modules.
fn terminal_proposal_type_declaration(type_name: &str) -> Option<ModuleTypeDeclaration> {
    let resolver = ModuleResolver::new();
    [
        terminal_proposal_modules::TERMINAL_TYPES_MODULE_PATH,
        terminal_proposal_modules::TERMINAL_CHORDS_MODULE_PATH,
        terminal_proposal_modules::TERMINAL_TESTING_MODULE_PATH,
    ]
    .iter()
    .find_map(|module_path| {
        resolver
            .module_interface(module_path)
            .and_then(|interface| interface.type_declaration(type_name).cloned())
    })
}

/// Import availability for a registered module interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleAvailability {
    /// Importable from ordinary production and test compilations.
    Always,
    /// Importable only when the checker is explicitly running in test mode.
    TestOnly,
    /// Parsed and retained for source-of-truth metadata, but not importable yet.
    FuturePublicApi,
}

impl ModuleAvailability {
    /// Return true when this availability is importable under the current checker mode.
    #[must_use]
    pub const fn is_import_allowed(
        self,
        allow_test_only: bool,
        allow_future_public_api: bool,
    ) -> bool {
        match self {
            Self::Always => true,
            Self::TestOnly => allow_test_only,
            Self::FuturePublicApi => allow_future_public_api,
        }
    }

    /// Human-readable reason used in diagnostics when an import is rejected.
    #[must_use]
    pub const fn rejection_reason(self) -> &'static str {
        match self {
            Self::Always => "module is available",
            Self::TestOnly => {
                "module is test-only and may only be imported by test-runner compilations"
            }
            Self::FuturePublicApi => {
                "terminal public API prerequisite validation has not completed"
            }
        }
    }

    /// Human-readable help used in diagnostics when an import is rejected.
    #[must_use]
    pub const fn rejection_help(self) -> &'static str {
        match self {
            Self::Always => "No availability action is required.",
            Self::TestOnly => {
                "Remove this production import or run the checker in test-only terminal mode."
            }
            Self::FuturePublicApi => {
                "Enable the selected terminal public API prerequisite set before importing this module."
            }
        }
    }
}

/// Parsed type-declaration metadata retained for module-interface consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTypeDeclaration {
    /// Type name exported or declared by this interface.
    pub name: String,
    /// Exact repository-relative declaration source path.
    pub source_path: String,
    /// Parsed proposal annotations attached to this declaration.
    pub annotations: Vec<DeclarationAnnotation>,
    /// Parsed proposal-specific declaration form.
    pub form: TypeDeclarationForm,
    /// Parsed type definition, including constraints, variant IDs, and fields.
    pub type_def: TypeDef,
    /// Source declaration visibility.
    pub visibility: AstVisibility,
    /// Source span inside the authoritative declaration file.
    pub span: Span,
}

/// Export/import view for one module path.
#[derive(Debug, Clone)]
pub struct ModuleInterface {
    /// Symbols visible to importers.
    pub exports: BTreeMap<String, SymbolInfo>,
    /// Symbols declared in-module but not exportable.
    pub private_symbols: BTreeMap<String, SymbolInfo>,
    /// Canonical module identifier.
    pub module_path: String,
    /// ADT field layouts keyed by nominal owner name.
    pub adt_fields: BTreeMap<String, BTreeMap<String, CoreType>>,
    /// Ordered function return-label metadata keyed by exported/local symbol name.
    pub function_return_labels: BTreeMap<String, Vec<String>>,
    /// Ordered function parameter borrow metadata keyed by exported/local symbol name.
    pub function_borrow_kinds: BTreeMap<String, Vec<BorrowKind>>,
    /// Availability gate applied before imports from this interface are resolved.
    pub availability: ModuleAvailability,
    /// Namespace declarations parsed from the authoritative module source.
    pub namespaces: Vec<Vec<String>>,
    /// Parsed type declaration metadata keyed by declared type name.
    pub type_declarations: BTreeMap<String, ModuleTypeDeclaration>,
}

impl ModuleInterface {
    /// Build an empty interface for `module_path`.
    #[must_use]
    pub fn new(module_path: String) -> Self {
        Self::with_availability(module_path, ModuleAvailability::Always)
    }

    /// Build an empty interface with an explicit import availability gate.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "BTreeMap::new is not const in current toolchain"
    )]
    #[must_use]
    pub fn with_availability(module_path: String, availability: ModuleAvailability) -> Self {
        Self {
            exports: BTreeMap::new(),
            private_symbols: BTreeMap::new(),
            module_path,
            adt_fields: BTreeMap::new(),
            function_return_labels: BTreeMap::new(),
            function_borrow_kinds: BTreeMap::new(),
            availability,
            namespaces: Vec::new(),
            type_declarations: BTreeMap::new(),
        }
    }

    /// Insert a symbol into export or private buckets.
    ///
    /// # Errors
    /// Returns an error when a public export name is duplicated.
    pub fn register_symbol(&mut self, symbol: SymbolInfo) -> Result<(), TypeError> {
        match symbol.visibility {
            Visibility::Public | Visibility::Entry => {
                if self.exports.contains_key(&symbol.name) {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: format!(
                            "duplicate public export '{}' in module '{}'",
                            symbol.name, self.module_path
                        ),
                        span: TypeError::span_from_span(symbol.source_location),
                    });
                }
                self.exports.insert(symbol.name.clone(), symbol);
            }
            Visibility::Private => {
                self.private_symbols.insert(symbol.name.clone(), symbol);
            }
        }
        Ok(())
    }

    /// Register ordered return-label metadata for one symbol name.
    pub fn register_function_return_labels(&mut self, symbol_name: String, labels: Vec<String>) {
        self.function_return_labels.insert(symbol_name, labels);
    }

    /// Read ordered return-label metadata for one symbol name.
    pub fn function_return_labels(&self, symbol_name: &str) -> Option<&[String]> {
        self.function_return_labels
            .get(symbol_name)
            .map(Vec::as_slice)
    }

    /// Register ordered parameter borrow metadata for one symbol name.
    pub fn register_function_borrow_kinds(
        &mut self,
        symbol_name: String,
        borrow_kinds: Vec<BorrowKind>,
    ) {
        self.function_borrow_kinds.insert(symbol_name, borrow_kinds);
    }

    /// Read ordered parameter borrow metadata for one symbol name.
    pub fn function_borrow_kinds(&self, symbol_name: &str) -> Option<&[BorrowKind]> {
        self.function_borrow_kinds
            .get(symbol_name)
            .map(Vec::as_slice)
    }

    /// Register one namespace declaration parsed from the authoritative source.
    pub fn register_namespace(&mut self, namespace: Vec<String>) {
        self.namespaces.push(namespace);
    }

    /// Register parsed type-declaration metadata for interface consumers.
    pub fn register_type_declaration(&mut self, declaration: ModuleTypeDeclaration) {
        self.type_declarations
            .insert(declaration.name.clone(), declaration);
    }

    /// Read parsed type-declaration metadata for one type name.
    pub fn type_declaration(&self, type_name: &str) -> Option<&ModuleTypeDeclaration> {
        self.type_declarations.get(type_name)
    }
}

/// Resolver for imports and module dependency validation.
#[derive(Debug, Clone)]
pub struct ModuleResolver {
    /// Known interfaces keyed by module path.
    modules: BTreeMap<String, ModuleInterface>,
    /// Directed import graph: module -> imported modules.
    dependency_graph: BTreeMap<String, Vec<String>>,
    /// Imported local bindings per module for conflict detection.
    import_name_bindings: BTreeMap<String, BTreeMap<String, String>>,
}

impl ModuleResolver {
    /// Construct resolver with preloaded standard interfaces.
    #[must_use]
    pub fn new() -> Self {
        let mut resolver = Self {
            modules: BTreeMap::new(),
            dependency_graph: BTreeMap::new(),
            import_name_bindings: BTreeMap::new(),
        };
        resolver.register_standard_modules();
        resolver
    }

    /// Register built-in module interfaces used by import resolution.
    fn register_standard_modules(&mut self) {
        standard_modules::register_standard_modules(self);
    }

    /// Register a complete module interface.
    pub fn register_module_interface(&mut self, interface: ModuleInterface) {
        self.modules
            .insert(interface.module_path.clone(), interface);
    }

    /// Register ADT field metadata for one owner in a specific module.
    pub fn register_adt_fields_for_module(
        &mut self,
        module_path: &str,
        owner: String,
        fields: BTreeMap<String, CoreType>,
    ) {
        let interface = self
            .modules
            .entry(module_path.to_owned())
            .or_insert_with(|| ModuleInterface::new(module_path.to_owned()));
        interface.adt_fields.insert(owner, fields);
    }

    /// Get a cloned interface for inspection and tests.
    pub fn module_interface(&self, module_path: &str) -> Option<ModuleInterface> {
        self.modules.get(module_path).cloned()
    }

    /// Return the test-only module that declares `type_name`, if any.
    pub fn test_only_type_source(&self, type_name: &str) -> Option<&str> {
        self.modules.values().find_map(|interface| {
            (interface.availability == ModuleAvailability::TestOnly
                && interface.type_declarations.contains_key(type_name))
            .then_some(interface.module_path.as_str())
        })
    }

    /// Return the first test-only type reached by a core type signature.
    pub fn core_type_test_only_reference(&self, core_type: &CoreType) -> Option<(String, String)> {
        match *core_type {
            CoreType::Array(ref element_type) => self.core_type_test_only_reference(element_type),
            CoreType::Function {
                ref generic_params,
                ref parameters,
                ref return_types,
                ref error_types,
            } => generic_params
                .iter()
                .flat_map(|parameter| parameter.constraints.iter())
                .chain(parameters.iter())
                .chain(return_types.iter())
                .chain(error_types.iter())
                .find_map(|inner_type| self.core_type_test_only_reference(inner_type)),
            CoreType::Generic {
                ref name,
                ref type_args,
            } => self.test_only_type_source(name).map_or_else(
                || {
                    type_args
                        .iter()
                        .find_map(|type_arg| self.core_type_test_only_reference(type_arg))
                },
                |module_path| Some((name.clone(), module_path.to_owned())),
            ),
            CoreType::Int8
            | CoreType::Int16
            | CoreType::Int32
            | CoreType::Int64
            | CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
            | CoreType::Float32
            | CoreType::Float64
            | CoreType::String
            | CoreType::Boolean
            | CoreType::Unit
            | CoreType::Variable(_) => None,
        }
    }

    /// Register one symbol for a module.
    ///
    /// # Errors
    /// Returns duplicate-export errors from interface validation.
    pub fn register_symbol_for_module(
        &mut self,
        module_path: &str,
        symbol: SymbolInfo,
    ) -> Result<(), TypeError> {
        let interface = self
            .modules
            .entry(module_path.to_owned())
            .or_insert_with(|| ModuleInterface::new(module_path.to_owned()));
        interface.register_symbol(symbol)
    }

    /// Generate and register a module interface from provided symbols.
    ///
    /// # Errors
    /// Returns duplicate-export errors from interface validation.
    pub fn generate_module_interface(
        &mut self,
        module_path: &str,
        symbols: &[SymbolInfo],
    ) -> Result<(), TypeError> {
        let mut interface = ModuleInterface::new(module_path.to_owned());
        for symbol in symbols {
            interface.register_symbol(symbol.clone())?;
        }
        self.register_module_interface(interface);
        Ok(())
    }

    /// Register an import edge `module -> dependency`.
    pub fn register_dependency(&mut self, module: &str, dependency: &str) {
        let dependencies = self.dependency_graph.entry(module.to_owned()).or_default();
        if !dependencies.iter().any(|entry| entry == dependency) {
            dependencies.push(dependency.to_owned());
        }
    }

    /// Validate and record one imported local binding in a module.
    ///
    /// # Errors
    /// Returns `TypeError::ImportNameConflict` when the same local name was
    /// already introduced from a different module.
    pub fn validate_import_name_binding(
        &mut self,
        module_path: &str,
        local_name: &str,
        source_module: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        let module_bindings = self
            .import_name_bindings
            .entry(module_path.to_owned())
            .or_default();

        if let Some(first_module) = module_bindings.get(local_name) {
            if first_module != source_module {
                return Err(TypeError::ImportNameConflict {
                    name: local_name.to_owned(),
                    first_module: first_module.clone(),
                    second_module: source_module.to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
            return Ok(());
        }

        module_bindings.insert(local_name.to_owned(), source_module.to_owned());
        Ok(())
    }

    /// Resolve a named symbol import from a source module.
    ///
    /// # Errors
    /// Returns unresolved-import, private-access, or missing-symbol errors.
    pub fn resolve_symbol(
        &self,
        source: &str,
        symbol_name: &str,
        span: Span,
    ) -> Result<SymbolInfo, TypeError> {
        let interface = self
            .modules
            .get(source)
            .ok_or_else(|| TypeError::UnresolvedImport {
                path: source.to_owned(),
                span: TypeError::span_from_span(span),
            })?;

        if let Some(symbol) = interface.exports.get(symbol_name) {
            return Ok(symbol.clone());
        }

        if interface.private_symbols.contains_key(symbol_name) {
            return Err(TypeError::PrivateSymbolAccess {
                symbol: symbol_name.to_owned(),
                module: source.to_owned(),
                span: TypeError::span_from_span(span),
            });
        }

        Err(TypeError::SymbolNotFound {
            name: format!("{source}.{symbol_name}"),
            suggestion: None,
            span: TypeError::span_from_span(span),
        })
    }

    /// Resolve all exported symbols from `source`.
    ///
    /// # Errors
    /// Returns unresolved-import errors when source is unknown.
    pub fn resolve_all_exports(
        &self,
        source: &str,
        span: Span,
    ) -> Result<Vec<SymbolInfo>, TypeError> {
        let interface = self
            .modules
            .get(source)
            .ok_or_else(|| TypeError::UnresolvedImport {
                path: source.to_owned(),
                span: TypeError::span_from_span(span),
            })?;
        Ok(interface.exports.values().cloned().collect())
    }

    /// Detect dependency cycles reachable from `module`.
    ///
    /// # Errors
    /// Returns `TypeError::CircularDependency` with cycle path.
    pub fn validate_no_cycles_from(&self, module: &str, span: Span) -> Result<(), TypeError> {
        let mut visiting: Vec<String> = Vec::new();
        let mut visited: BTreeMap<String, bool> = BTreeMap::new();
        self.dfs_cycle(module, &mut visiting, &mut visited, span)
    }

    /// Depth-first traversal used by cycle detection.
    ///
    /// # Errors
    /// Returns cycle diagnostics with the detected path.
    fn dfs_cycle(
        &self,
        module: &str,
        visiting: &mut Vec<String>,
        visited: &mut BTreeMap<String, bool>,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Some(index) = visiting.iter().position(|entry| entry == module) {
            let mut cycle = visiting[index..].to_vec();
            cycle.push(module.to_owned());
            return Err(TypeError::CircularDependency {
                cycle,
                span: TypeError::span_from_span(span),
            });
        }

        if visited.get(module).copied().unwrap_or(false) {
            return Ok(());
        }

        visiting.push(module.to_owned());
        if let Some(dependencies) = self.dependency_graph.get(module) {
            for dependency in dependencies {
                self.dfs_cycle(dependency, visiting, visited, span)?;
            }
        }
        visiting.pop();
        visited.insert(module.to_owned(), true);
        Ok(())
    }

    /// Build a symbol record anchored to synthetic built-in source span.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Constructing SymbolInfo is runtime-oriented and const is not required"
    )]
    fn module_symbol(
        name: String,
        symbol_type: SymbolType,
        core_type: CoreType,
        visibility: Visibility,
    ) -> SymbolInfo {
        SymbolInfo {
            name,
            symbol_type,
            core_type,
            visibility,
            source_location: Span::single(Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        }
    }
}

impl Default for ModuleResolver {
    /// Default resolver delegates to [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}
