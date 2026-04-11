extern crate alloc;

use super::super::module_resolver::ModuleInterface;
use crate::ast::{ImportItem, Visibility as AstVisibility};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::SymbolInfo;

use alloc::string::String;

impl TypeChecker {
    /// Set the canonical path for the module currently being type checked.
    pub fn set_current_module_path(&mut self, module_path: String) {
        self.current_module_path = module_path;
    }

    /// Register a complete module interface into the resolver.
    pub fn register_module_interface(&mut self, interface: ModuleInterface) {
        self.module_resolver.register_module_interface(interface);
    }

    /// Fetch a cloned module interface for inspection and tests.
    pub fn module_interface(&self, module_path: &str) -> Option<ModuleInterface> {
        self.module_resolver.module_interface(module_path)
    }

    /// Register an explicit dependency edge between two modules.
    pub fn register_module_dependency(&mut self, module: &str, dependency: &str) {
        self.module_resolver.register_dependency(module, dependency);
    }

    /// Register one symbol in the current module interface using AST visibility.
    ///
    /// # Errors
    /// Returns duplicate-export errors when a `public` symbol name is reused.
    pub fn register_current_module_symbol(
        &mut self,
        symbol: SymbolInfo,
        visibility: &AstVisibility,
    ) -> Result<(), TypeError> {
        let symbol_visibility = match *visibility {
            AstVisibility::Public => crate::type_system::symbol_table::Visibility::Public,
            AstVisibility::Private => crate::type_system::symbol_table::Visibility::Private,
        };

        let mut module_symbol = symbol;
        module_symbol.visibility = symbol_visibility;
        self.module_resolver
            .register_symbol_for_module(&self.current_module_path, module_symbol)?;
        Ok(())
    }

    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Import item matching borrows from slice entries"
    )]
    /// Resolve and register imported symbols from `source`.
    ///
    /// # Errors
    /// Returns unresolved-import, private-access, missing-symbol, or cycle diagnostics.
    pub(super) fn register_import_declaration(
        &mut self,
        items: &[ImportItem],
        source: &str,
        import_span: Span,
    ) -> Result<(), TypeError> {
        self.module_resolver
            .register_dependency(&self.current_module_path, source);
        self.module_resolver
            .validate_no_cycles_from(&self.current_module_path, import_span)?;

        for item in items {
            match item {
                &ImportItem::Named {
                    ref name,
                    ref alias,
                    span: item_span,
                }
                | &ImportItem::Type {
                    ref name,
                    ref alias,
                    span: item_span,
                } => {
                    let imported_symbol = self
                        .module_resolver
                        .resolve_symbol(source, name, item_span)?;
                    let mut symbol_to_register = imported_symbol;
                    if let Some(alias_name) = alias.as_ref() {
                        symbol_to_register.name.clone_from(alias_name);
                    }
                    self.symbol_table.register(symbol_to_register);
                }
                ImportItem::Glob { .. } => {
                    for symbol in self
                        .module_resolver
                        .resolve_all_exports(source, import_span)?
                    {
                        self.symbol_table.register(symbol);
                    }
                }
            }
        }

        Ok(())
    }
}
