extern crate alloc;

use super::super::module_resolver::{
    AdtFieldManifest, AdtLayoutManifest, AdtLayoutManifestKind, AdtTypeId, AdtVariantManifest,
    ModuleAvailability, ModuleErrorSetDeclaration, ModuleInterface, ModuleTypeDeclaration,
};
use crate::ast::{
    DeclarationAnnotation, ErrorSetMember, ImportItem, TypeDeclarationForm, TypeDef,
    Visibility as AstVisibility,
};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::{format, string::String, vec::Vec};

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
        let symbol_name = module_symbol.name.clone();
        module_symbol.visibility = symbol_visibility;
        if matches!(
            module_symbol.visibility,
            Visibility::Public | Visibility::Entry
        ) {
            self.validate_production_core_type_surface(
                "module export",
                symbol_name.as_str(),
                &module_symbol.core_type,
                module_symbol.source_location,
            )?;
        }
        self.module_resolver
            .register_symbol_for_module(&self.current_module_path, module_symbol)?;
        if let Some(mut interface) = self
            .module_resolver
            .module_interface(&self.current_module_path)
        {
            if let Some(labels) = self.function_return_labels(symbol_name.as_str()) {
                interface.register_function_return_labels(symbol_name.clone(), labels.to_vec());
            }
            if let Some(borrow_kinds) = self.function_borrow_kinds_for_symbol(symbol_name.as_str())
            {
                interface.register_function_borrow_kinds(symbol_name, borrow_kinds.to_vec());
            }
            self.module_resolver.register_module_interface(interface);
        }
        Ok(())
    }

    /// Register parsed type metadata and public layout manifests for the current module.
    pub(super) fn register_current_module_type_declaration(
        &mut self,
        name: String,
        annotations: Vec<DeclarationAnnotation>,
        form: TypeDeclarationForm,
        type_def: &TypeDef,
        visibility: &AstVisibility,
        span: Span,
    ) {
        let mut interface = self
            .module_resolver
            .module_interface(&self.current_module_path)
            .unwrap_or_else(|| ModuleInterface::new(self.current_module_path.clone()));
        let declaration = ModuleTypeDeclaration {
            name: name.clone(),
            source_path: self.current_module_path.clone(),
            annotations,
            form,
            type_def: type_def.clone(),
            visibility: visibility.clone(),
            span,
        };
        interface.register_type_declaration(declaration);
        if *visibility == AstVisibility::Public {
            if let Some(manifest) = self.build_adt_layout_manifest(name, type_def) {
                interface.register_adt_layout_manifest(manifest);
            }
        }
        self.module_resolver.register_module_interface(interface);
    }

    /// Register named error-set metadata for the current module interface.
    pub(super) fn register_current_module_error_set_declaration(
        &mut self,
        name: String,
        members: &[ErrorSetMember],
        expanded_members: Vec<String>,
        visibility: &AstVisibility,
        span: Span,
    ) {
        let mut interface = self
            .module_resolver
            .module_interface(&self.current_module_path)
            .unwrap_or_else(|| ModuleInterface::new(self.current_module_path.clone()));
        interface.register_error_set_declaration(ModuleErrorSetDeclaration {
            name,
            source_path: self.current_module_path.clone(),
            members: members.iter().map(|member| member.name.clone()).collect(),
            expanded_members,
            visibility: visibility.clone(),
            span,
        });
        self.module_resolver.register_module_interface(interface);
    }

    /// Build a public ADT layout manifest from a checked source type definition.
    fn build_adt_layout_manifest(
        &self,
        type_name: String,
        type_def: &TypeDef,
    ) -> Option<AdtLayoutManifest> {
        let type_id = AdtTypeId::new(self.current_module_path.clone(), type_name);
        let kind = match *type_def {
            TypeDef::Product { ref fields, .. } => AdtLayoutManifestKind::Product {
                fields: fields
                    .iter()
                    .map(Self::field_manifest)
                    .collect::<Option<Vec<_>>>()?,
            },
            TypeDef::Sum { ref variants, .. } => AdtLayoutManifestKind::Sum {
                variants: variants
                    .iter()
                    .enumerate()
                    .map(|(index, variant)| {
                        let discriminant =
                            variant.explicit_id.unwrap_or(i64::try_from(index).ok()?);
                        let fields = variant
                            .fields
                            .iter()
                            .map(Self::field_manifest)
                            .collect::<Option<Vec<_>>>()?;
                        Some(AdtVariantManifest {
                            name: variant.name.clone(),
                            discriminant,
                            propertyless: fields.is_empty(),
                            fields,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            },
            TypeDef::Alias {
                ref target_type, ..
            } => AdtLayoutManifestKind::Alias {
                target: ast_type_to_core_type(target_type).ok()?,
            },
            TypeDef::Opaque { .. } => AdtLayoutManifestKind::Opaque {
                layout_public: false,
            },
        };
        let layout_hash = Self::adt_layout_hash(&type_id, &kind);
        Some(AdtLayoutManifest {
            type_id,
            kind,
            layout_hash,
        })
    }

    /// Convert one AST field into manifest field metadata.
    fn field_manifest(field: &crate::ast::Field) -> Option<AdtFieldManifest> {
        let core_type = ast_type_to_core_type(&field.type_annotation).ok()?;
        Some(AdtFieldManifest {
            name: field.name.clone(),
            requires_drop: Self::core_type_requires_drop(&core_type),
            core_type,
            is_public: true,
        })
    }

    /// Return whether a core type owns RC-managed children.
    fn core_type_requires_drop(core_type: &CoreType) -> bool {
        match *core_type {
            CoreType::String | CoreType::Array(_) | CoreType::Generic { .. } => true,
            CoreType::Function {
                ref return_types,
                ref parameters,
                ref error_types,
                ref generic_params,
            } => {
                parameters.iter().any(Self::core_type_requires_drop)
                    || return_types.iter().any(Self::core_type_requires_drop)
                    || error_types.iter().any(Self::core_type_requires_drop)
                    || generic_params
                        .iter()
                        .any(|param| param.constraints.iter().any(Self::core_type_requires_drop))
            }
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
            | CoreType::Boolean
            | CoreType::Unit
            | CoreType::Variable(_) => false,
        }
    }

    /// Deterministically hash the public layout contract with FNV-1a.
    fn adt_layout_hash(type_id: &AdtTypeId, kind: &AdtLayoutManifestKind) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        Self::hash_text(&mut hash, type_id.module_path.as_str());
        Self::hash_text(&mut hash, type_id.type_name.as_str());
        match *kind {
            AdtLayoutManifestKind::Product { ref fields } => {
                Self::hash_text(&mut hash, "product");
                for field in fields {
                    Self::hash_field(&mut hash, field);
                }
            }
            AdtLayoutManifestKind::Sum { ref variants } => {
                Self::hash_text(&mut hash, "sum");
                for variant in variants {
                    Self::hash_text(&mut hash, variant.name.as_str());
                    Self::hash_text(&mut hash, variant.discriminant.to_string().as_str());
                    for field in &variant.fields {
                        Self::hash_field(&mut hash, field);
                    }
                }
            }
            AdtLayoutManifestKind::Alias { ref target } => {
                Self::hash_text(&mut hash, "alias");
                Self::hash_text(&mut hash, target.to_string().as_str());
            }
            AdtLayoutManifestKind::Opaque { layout_public } => {
                Self::hash_text(&mut hash, "opaque");
                Self::hash_text(&mut hash, layout_public.to_string().as_str());
            }
        }
        hash
    }

    /// Hash one manifest field into an FNV-1a state.
    fn hash_field(hash: &mut u64, field: &AdtFieldManifest) {
        Self::hash_text(hash, field.name.as_str());
        Self::hash_text(hash, field.core_type.to_string().as_str());
        Self::hash_text(hash, field.is_public.to_string().as_str());
        Self::hash_text(hash, field.requires_drop.to_string().as_str());
    }

    /// Hash text bytes into an FNV-1a state with a separator.
    fn hash_text(hash: &mut u64, text: &str) {
        for byte in text.as_bytes().iter().copied().chain([0xff]) {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
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

    /// Validate availability metadata for a registered module before resolving imported symbols.
    fn validate_module_import_availability(
        &self,
        items: &[ImportItem],
        source: &str,
        import_span: Span,
    ) -> Result<(), TypeError> {
        if let Some(interface) = self.module_resolver.module_interface(source) {
            let is_error_inspector_exception =
                matches!(interface.availability, ModuleAvailability::FuturePublicApi)
                    && Self::items_are_implemented_error_inspector_imports(items, source);
            let terminal_public_api_prerequisites_missing =
                crate::type_system::is_terminal_proposal_module_path(source)
                    && !self.terminal_public_api_prerequisites_are_satisfied()
                    && !is_error_inspector_exception
                    && (!matches!(interface.availability, ModuleAvailability::TestOnly)
                        || self.allow_test_only_imports);
            if terminal_public_api_prerequisites_missing {
                return Err(TypeError::ModuleUnavailable {
                    module: source.to_owned(),
                    reason: self.terminal_public_api_prerequisite_reason(),
                    help: self.terminal_public_api_prerequisite_help(),
                    span: TypeError::span_from_span(import_span),
                });
            }
            if !interface.availability.is_import_allowed(
                self.allow_test_only_imports,
                self.terminal_public_api_prerequisites_are_satisfied(),
            ) && !is_error_inspector_exception
            {
                return Err(TypeError::ModuleUnavailable {
                    module: source.to_owned(),
                    reason: interface.availability.rejection_reason().to_owned(),
                    help: interface.availability.rejection_help().to_owned(),
                    span: TypeError::span_from_span(import_span),
                });
            }
        }
        Ok(())
    }

    /// Return whether every imported item is an implemented Task 16 error inspector.
    fn items_are_implemented_error_inspector_imports(items: &[ImportItem], source: &str) -> bool {
        !items.is_empty()
            && items.iter().all(|item| match *item {
                ImportItem::Named { ref name, .. } => {
                    crate::type_system::is_terminal_proposal_implemented_error_inspector_import(
                        source, name,
                    )
                }
                ImportItem::Type { .. } | ImportItem::Glob { .. } => false,
            })
    }

    /// Resolve and register imported symbols from `source`.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Import item matching borrows from slice entries"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "Import resolution keeps named and glob registration in one flow"
    )]
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

        self.validate_module_import_availability(items, source, import_span)?;

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
                    self.register_imported_error_set(
                        source,
                        name,
                        resolved_import_name.as_str(),
                        item_span,
                    )?;
                    if let Some(interface) = self.module_resolver.module_interface(source) {
                        if let Some(labels) = interface.function_return_labels(name) {
                            self.register_function_return_labels_for_symbol(
                                resolved_import_name.clone(),
                                labels.to_vec(),
                            );
                        }
                        if let Some(borrow_kinds) = interface.function_borrow_kinds(name) {
                            self.register_function_borrow_modes_for_symbol(
                                resolved_import_name,
                                borrow_kinds,
                            );
                        }
                    }
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
                        if let Some(interface) = self.module_resolver.module_interface(source) {
                            if let Some(labels) =
                                interface.function_return_labels(symbol.name.as_str())
                            {
                                self.register_function_return_labels_for_symbol(
                                    symbol.name.clone(),
                                    labels.to_vec(),
                                );
                            }
                            if let Some(borrow_kinds) =
                                interface.function_borrow_kinds(symbol.name.as_str())
                            {
                                self.register_function_borrow_modes_for_symbol(
                                    symbol.name.clone(),
                                    borrow_kinds,
                                );
                            }
                        }
                        let symbol_name = symbol.name.clone();
                        self.register_imported_type_adt_fields(
                            source,
                            symbol_name.as_str(),
                            symbol_name.as_str(),
                            &symbol.symbol_type,
                            &symbol.core_type,
                        );
                        self.register_imported_error_set(
                            source,
                            symbol_name.as_str(),
                            symbol_name.as_str(),
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

        let interface = self.module_resolver.module_interface(source);
        for mut symbol in self.module_resolver.resolve_all_exports(source, span)? {
            let original_name = symbol.name.clone();
            symbol.name = alloc::format!("{alias_name}.{}", symbol.name);
            if let Some(interface_ref) = interface.as_ref() {
                if let Some(labels) = interface_ref.function_return_labels(original_name.as_str()) {
                    self.register_function_return_labels_for_symbol(
                        symbol.name.clone(),
                        labels.to_vec(),
                    );
                }
                if let Some(borrow_kinds) =
                    interface_ref.function_borrow_kinds(original_name.as_str())
                {
                    self.register_function_borrow_modes_for_symbol(
                        symbol.name.clone(),
                        borrow_kinds,
                    );
                }
            }
            self.symbol_table.register(symbol);
        }

        Ok(())
    }

    /// Copy an imported named error set into the local checker registry.
    fn register_imported_error_set(
        &mut self,
        source: &str,
        imported_name: &str,
        local_name: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        let Some(interface) = self.module_resolver.module_interface(source) else {
            return Ok(());
        };
        let Some(declaration) = interface.error_set_declaration(imported_name) else {
            return Ok(());
        };

        let members = declaration
            .expanded_members
            .iter()
            .map(|member_name| ErrorSetMember {
                name: member_name.clone(),
                span,
            })
            .collect::<Vec<_>>();
        if let Some(error) = self.register_error_set_declaration_raw(
            local_name.to_owned(),
            members,
            AstVisibility::Private,
            span,
        ) {
            return Err(error);
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

        self.register_imported_constructor_visibility(source, imported_name, local_name);
        if interface
            .type_declaration(imported_name)
            .is_some_and(|declaration| {
                declaration.form == TypeDeclarationForm::CompilerRegisteredAffineResource
            })
        {
            self.register_affine_resource_type(local_name.to_owned());
        }

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
