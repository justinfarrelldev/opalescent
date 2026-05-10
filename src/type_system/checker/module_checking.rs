extern crate alloc;

use super::super::module_resolver::ModuleInterface;
use crate::ast::{ImportItem, Visibility as AstVisibility};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{format, string::String};

impl TypeChecker {
    /// Set the canonical path for the module currently being type checked.
    pub fn set_current_module_path(&mut self, module_path: String) {
        self.current_module_path = module_path;
    }

    /// Register a complete module interface into the resolver.
    pub fn register_module_interface(&mut self, interface: ModuleInterface) {
        for (owner, fields) in &interface.adt_fields {
            self.register_adt_fields(owner.clone(), fields.clone());
        }
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

    /// Synchronize current checker ADT field registry into current module interface.
    pub(super) fn sync_current_module_adt_fields(&mut self) {
        for (owner, fields) in &self.adt_fields {
            self.module_resolver.register_adt_fields_for_module(
                &self.current_module_path,
                owner.clone(),
                fields.clone(),
            );
        }
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
        // Package imports (@scope/name) are not yet supported.
        if source.starts_with('@') {
            return Err(TypeError::PackageImportNotSupported {
                path: source.to_owned(),
                span: TypeError::span_from_span(import_span),
            });
        }

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
                    if name == source {
                        if let Some(alias_name) = alias.as_ref() {
                            self.module_resolver.validate_import_name_binding(
                                &self.current_module_path,
                                alias_name,
                                source,
                                item_span,
                            )?;
                            self.register_module_alias(alias_name, source, item_span)?;
                            continue;
                        }
                    }

                    let imported_symbol = self
                        .module_resolver
                        .resolve_symbol(source, name, item_span)?;
                    let resolved_import_name = alias.as_deref().unwrap_or(name.as_str()).to_owned();
                    let mut symbol_to_register = imported_symbol;
                    if let Some(alias_name) = alias.as_ref() {
                        self.module_resolver.validate_import_name_binding(
                            &self.current_module_path,
                            alias_name,
                            source,
                            item_span,
                        )?;
                        symbol_to_register.name.clone_from(alias_name);
                    } else {
                        self.module_resolver.validate_import_name_binding(
                            &self.current_module_path,
                            name,
                            source,
                            item_span,
                        )?;
                    }
                    self.register_imported_type_adt_fields(
                        source,
                        name,
                        resolved_import_name.as_str(),
                        &symbol_to_register.symbol_type,
                        &symbol_to_register.core_type,
                    );
                    self.symbol_table.register(symbol_to_register);
                }
                ImportItem::Glob { .. } => {
                    for symbol in self
                        .module_resolver
                        .resolve_all_exports(source, import_span)?
                    {
                        self.module_resolver.validate_import_name_binding(
                            &self.current_module_path,
                            &symbol.name,
                            source,
                            import_span,
                        )?;
                        self.symbol_table.register(symbol);
                    }
                }
            }
        }

        Ok(())
    }

    /// Register a module alias plus qualified member symbols (e.g. `m.sqrt`).
    fn register_module_alias(
        &mut self,
        alias_name: &str,
        source: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        self.symbol_table.register(SymbolInfo {
            name: alias_name.to_owned(),
            symbol_type: SymbolType::Constant,
            core_type: CoreType::Generic {
                name: source.to_owned(),
                type_args: Vec::new(),
            },
            visibility: Visibility::Private,
            source_location: span,
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });

        for mut symbol in self.module_resolver.resolve_all_exports(source, span)? {
            symbol.name = alloc::format!("{alias_name}.{}", symbol.name);
            self.symbol_table.register(symbol);
        }

        Ok(())
    }

    /// Copy ADT field schemas for an imported type into local checker metadata.
    fn register_imported_type_adt_fields(
        &mut self,
        source: &str,
        imported_name: &str,
        local_name: &str,
        symbol_type: &SymbolType,
        imported_core_type: &CoreType,
    ) {
        if symbol_type != &SymbolType::Type {
            return;
        }

        let Some(interface) = self.module_resolver.module_interface(source) else {
            return;
        };

        if let Some(fields) = interface.adt_fields.get(imported_name) {
            self.register_adt_fields(local_name.to_owned(), fields.clone());
        }

        let variant_prefix = format!("{imported_name}.");
        let mut imported_variants: Vec<String> = Vec::new();
        for (owner, fields) in &interface.adt_fields {
            if let Some(variant_suffix) = owner.strip_prefix(variant_prefix.as_str()) {
                let local_owner = format!("{local_name}.{variant_suffix}");
                imported_variants.push(local_owner.clone());
                self.register_adt_fields(local_owner.clone(), fields.clone());
                self.symbol_table.register(SymbolInfo {
                    name: local_owner,
                    symbol_type: SymbolType::Constant,
                    core_type: imported_core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: interface.exports.get(imported_name).map_or(
                        crate::token::Span::single(crate::token::Position::start()),
                        |symbol| symbol.source_location,
                    ),
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                    is_pure: false,
                });

                for field_name in fields.keys() {
                    self.symbol_table.register(SymbolInfo {
                        name: format!("{local_name}.{variant_suffix}.{field_name}"),
                        symbol_type: SymbolType::Variable,
                        core_type: fields.get(field_name).cloned().unwrap_or(CoreType::Unit),
                        visibility: Visibility::Private,
                        source_location: interface.exports.get(imported_name).map_or(
                            crate::token::Span::single(crate::token::Position::start()),
                            |symbol| symbol.source_location,
                        ),
                        is_let_binding: false,
                        is_mutable: false,
                        read_count: 0,
                        is_pure: false,
                    });
                }
            }
        }

        if !imported_variants.is_empty() {
            self.adt_variants
                .insert(local_name.to_owned(), imported_variants);
        }
    }
}
