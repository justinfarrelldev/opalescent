//! Canonical source nominal identities for a native test artifact.
//! This does not enable test grammar or test-only imports in application modules.
extern crate alloc;
use super::TypeChecker;
use crate::ast::{Decl, Program, Type, TypeDef};
use crate::token::{Position, Span};
use crate::type_system::ModuleInterface;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use core::fmt::Write;

/// Not a legal source identifier; hex module bytes keep variant separators clear.
const SOURCE_TYPE_PREFIX: &str = "__opal_type::";

impl TypeChecker {
    /// Whether this checker participates in canonical native artifact typing.
    pub(in crate::type_system::checker) const fn native_source_type_identities_enabled(
        &self,
    ) -> bool {
        self.native_testing.source_types.is_some()
    }

    /// Resolve all ordinary annotation sites consistently inside test artifacts;
    /// preserve the production compiler's existing mapping outside artifact mode.
    pub(in crate::type_system::checker) fn native_annotation_type(
        &self,
        annotation: &Type,
    ) -> Result<CoreType, TypeError> {
        if self.native_source_type_identities_enabled() {
            self.ast_type_to_core_type_with_generics(annotation, &[])
        } else {
            crate::type_system::type_mapping::ast_type_to_core_type(annotation)
                .map_err(TypeError::from)
        }
    }

    /// Test-only imports may re-export the genuine context, but a same-spelled
    /// application type/value must not replace the runner's implicit contract.
    pub(in crate::type_system::checker) fn validate_native_import_binding(
        &self,
        local: &str,
        imported: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        if self.native_testing.enabled
            && local == "TestContext"
            && self
                .environment
                .lookup_type("TestContext", span)
                .is_ok_and(|expected| expected != imported)
        {
            Err(super::native_error(
                span,
                "TestContext is supplied by the test runner; import an application type using a different alias",
            ))
        } else {
            Ok(())
        }
    }

    /// Define declaration identities before resolving source signatures/fields.
    /// No implicit testing names or availability permissions are installed here.
    pub(crate) fn native_enable_source_type_identities(&mut self, program: &Program, module: &str) {
        let mut encoded = String::new();
        for byte in module.as_bytes() {
            write!(encoded, "{byte:02x}")
                .expect("hexadecimal formatting into a String cannot fail");
        }
        let names = program
            .declarations
            .iter()
            .filter_map(|declaration| {
                if let Decl::Type { ref name, .. } = *declaration {
                    Some((
                        name.clone(),
                        alloc::format!("{SOURCE_TYPE_PREFIX}{encoded}::{name}"),
                    ))
                } else {
                    None
                }
            })
            .collect();
        self.native_testing.source_types = Some(names);
    }

    /// Resolve the owning declaration rather than an import alias's spelling.
    pub(in crate::type_system::checker) fn native_nominal_type_name(&self, source: &str) -> String {
        self.native_testing
            .source_types
            .as_ref()
            .and_then(|names| names.get(source))
            .cloned()
            .unwrap_or_else(|| source.to_owned())
    }

    /// Make canonical schemas available without removing source alias schemas.
    pub(crate) fn native_sync_source_type_metadata(&mut self) {
        let names = self.native_testing.source_types.clone().unwrap_or_default();
        for (local, canonical) in names {
            if let Some(fields) = self.adt_fields.get(&local).cloned() {
                self.register_adt_fields(canonical.clone(), fields);
            }
            if let Some(parameters) = self.adt_generic_params.get(&local).cloned() {
                self.register_adt_generic_params(canonical.clone(), parameters);
            }
            if let Some(variants) = self.adt_variants.get(&local).cloned() {
                let mut qualified = Vec::new();
                for variant in variants {
                    if let Some(suffix) = variant.strip_prefix(&alloc::format!("{local}.")) {
                        let owner = alloc::format!("{canonical}.{suffix}");
                        if let Some(fields) = self.adt_fields.get(&variant).cloned() {
                            self.register_adt_fields(owner.clone(), fields);
                        }
                        qualified.push(owner);
                    }
                }
                self.adt_variants.insert(canonical.clone(), qualified);
            }
            self.native_register_canonical_type(&canonical);
        }
        self.sync_current_module_adt_fields();
    }

    /// Imported canonical schemas are ambient compiler metadata, not ambient
    /// source declarations. Source aliases are installed only by real imports.
    pub(crate) fn native_register_source_interface(&mut self, interface: ModuleInterface) {
        for (owner, fields) in &interface.adt_fields {
            if owner.starts_with(SOURCE_TYPE_PREFIX) {
                self.register_adt_fields(owner.clone(), fields.clone());
            }
        }
        for declaration in interface.type_declarations.values() {
            let symbol = interface
                .exports
                .get(&declaration.name)
                .or_else(|| interface.private_symbols.get(&declaration.name));
            if let Some(SymbolInfo {
                core_type:
                    CoreType::Generic {
                        name: ref canonical,
                        ..
                    },
                ..
            }) = symbol.cloned()
            {
                if canonical.starts_with(SOURCE_TYPE_PREFIX) {
                    self.native_register_canonical_type(canonical);
                    if let TypeDef::Sum { ref variants, .. } = declaration.type_def {
                        self.adt_variants.insert(
                            canonical.clone(),
                            variants
                                .iter()
                                .map(|variant| alloc::format!("{canonical}.{}", variant.name))
                                .collect(),
                        );
                    }
                }
            }
        }
        self.module_resolver.register_module_interface(interface);
    }

    /// Internal type symbols cannot be named in source, but allow canonical
    /// error-set leaves inferred/imported from another compiler interface.
    fn native_register_canonical_type(&mut self, canonical: &str) {
        self.symbol_table.register(SymbolInfo {
            name: canonical.to_owned(),
            symbol_type: SymbolType::Type,
            core_type: CoreType::Generic {
                name: canonical.to_owned(),
                type_args: Vec::new(),
            },
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
    }

    /// Concrete constructors retain canonical identity through named imports.
    pub(in crate::type_system::checker) fn native_constructor_owner(&self, source: &str) -> String {
        match self.native_source_type(source) {
            Some(CoreType::Generic { name, .. }) => name,
            _ => source.to_owned(),
        }
    }

    /// Return a nominal application's error family through a genuine type or
    /// variant alias, not a same-spelled local value or another declaration.
    pub(in crate::type_system::checker) fn native_error_leaf_name(
        &self,
        source: &str,
    ) -> Option<String> {
        self.native_testing.source_types.as_ref()?;
        let core = self
            .symbol_table
            .lookup(source)
            .and_then(|symbol| {
                let genuine_variant = self
                    .adt_variants
                    .values()
                    .any(|variants| variants.iter().any(|variant| variant == source));
                (symbol.symbol_type == SymbolType::Type || genuine_variant)
                    .then_some(&symbol.core_type)
            })
            .or_else(|| {
                self.environment
                    .lookup_type(source, Span::single(Position::start()))
                    .ok()
            });
        if let Some(&CoreType::Generic { ref name, .. }) = core {
            if name != "Error" && (name.ends_with("Error") || name.contains("Error.")) {
                return Some(name.clone());
            }
        }
        None
    }
}
