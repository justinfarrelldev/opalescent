extern crate alloc;

use crate::token::{Position, Span};
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::{format, string::String, vec::Vec};

/// Export/import view for one module path.
#[derive(Debug, Clone)]
pub struct ModuleInterface {
    /// Symbols visible to importers.
    pub exports: BTreeMap<String, SymbolInfo>,
    /// Symbols declared in-module but not exportable.
    pub private_symbols: BTreeMap<String, SymbolInfo>,
    /// Canonical module identifier.
    pub module_path: String,
}

impl ModuleInterface {
    /// Build an empty interface for `module_path`.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "BTreeMap::new is not const in current toolchain"
    )]
    #[must_use]
    pub fn new(module_path: String) -> Self {
        Self {
            exports: BTreeMap::new(),
            private_symbols: BTreeMap::new(),
            module_path,
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

    /// Register a complete module interface.
    pub fn register_module_interface(&mut self, interface: ModuleInterface) {
        self.modules
            .insert(interface.module_path.clone(), interface);
    }

    /// Get a cloned interface for inspection and tests.
    pub fn module_interface(&self, module_path: &str) -> Option<ModuleInterface> {
        self.modules.get(module_path).cloned()
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

    /// Register built-in module interfaces used by imports.
    fn register_standard_modules(&mut self) {
        self.register_standard_module();
        self.register_math_module();
    }

    /// Register `standard` built-in module symbols.
    fn register_standard_module(&mut self) {
        let mut interface = ModuleInterface::new(String::from("standard"));
        let standard_symbols = [
            (
                String::from("print"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Variable(crate::type_system::types::TypeVar::new(
                        0,
                        String::from("T"),
                    ))],
                    return_types: vec![CoreType::Unit],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("println"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Unit],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("take_input"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: Vec::new(),
                    return_types: vec![CoreType::String],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int32"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
            (
                String::from("string_to_int64"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
                SymbolType::Function,
            ),
        ];

        for (name, core_type, symbol_type) in standard_symbols {
            let register_result = interface.register_symbol(Self::module_symbol(
                name,
                symbol_type,
                core_type,
                Visibility::Public,
            ));
            if register_result.is_err() {
                return;
            }
        }
        self.register_module_interface(interface);
    }

    /// Register `math` built-in module symbols.
    fn register_math_module(&mut self) {
        let mut interface = ModuleInterface::new(String::from("math"));
        let math_symbols = [
            (
                String::from("random_int32"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int64, CoreType::Int64],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
            ),
            (
                String::from("random_int64"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int64, CoreType::Int64],
                    return_types: vec![CoreType::Int64],
                    error_types: Vec::new(),
                },
            ),
            (
                String::from("sqrt"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Float64],
                    return_types: vec![CoreType::Float64],
                    error_types: Vec::new(),
                },
            ),
            (
                String::from("abs"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Int32],
                    return_types: vec![CoreType::Int32],
                    error_types: Vec::new(),
                },
            ),
            (
                String::from("sin"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Float64],
                    return_types: vec![CoreType::Float64],
                    error_types: Vec::new(),
                },
            ),
            (
                String::from("cos"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::Float64],
                    return_types: vec![CoreType::Float64],
                    error_types: Vec::new(),
                },
            ),
        ];

        for (name, core_type) in math_symbols {
            let register_result = interface.register_symbol(Self::module_symbol(
                name,
                SymbolType::Function,
                core_type,
                Visibility::Public,
            ));
            if register_result.is_err() {
                return;
            }
        }
        self.register_module_interface(interface);
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
        }
    }
}

impl Default for ModuleResolver {
    /// Default resolver delegates to [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}
