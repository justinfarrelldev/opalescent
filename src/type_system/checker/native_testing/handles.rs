//! Generated nominal declarations precede state layouts and double signatures.
extern crate alloc;
use super::{NativeHandleKind, TypeChecker, native_error};
use crate::token::Span;
use crate::type_system::NativeDoubleSignature;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;

impl TypeChecker {
    /// Forward-declare provider identity without inventing a runtime state value.
    pub(crate) fn native_declare_world_alias(
        &mut self,
        local: &str,
        nominal: &str,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        self.native_declare_handle_alias(local, nominal, NativeHandleKind::World, span)
    }

    /// Forward-declare a concrete double identity before checking its callable.
    pub(crate) fn native_declare_double_alias(
        &mut self,
        local: &str,
        nominal: &str,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        self.native_declare_handle_alias(local, nominal, NativeHandleKind::Double, span)
    }

    /// Define an actual World layout after ordinary source types have resolved.
    pub(crate) fn native_define_world_state(
        &mut self,
        nominal: &str,
        state: CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        self.enable_native_test_mode()?;
        self.native_register_handle_kind(nominal, NativeHandleKind::World, span)?;
        if self
            .native_testing
            .worlds
            .get(nominal)
            .is_some_and(|existing| *existing != state)
        {
            return Err(native_error(
                span,
                "inconsistent state type for a canonical World provider",
            ));
        }
        self.register_adt_fields(
            nominal.to_owned(),
            BTreeMap::from([("state".to_owned(), state.clone())]),
        );
        self.native_testing.worlds.insert(nominal.to_owned(), state);
        Ok(())
    }

    /// Define the concrete precompiled application signature, without introducing
    /// a source alias into modules that only infer the handle through a World.
    pub(crate) fn native_define_double_signature(
        &mut self,
        nominal: &str,
        signature: NativeDoubleSignature,
        span: Span,
    ) -> Result<(), TypeError> {
        self.enable_native_test_mode()?;
        self.native_register_handle_kind(nominal, NativeHandleKind::Double, span)?;
        if signature.parameters.len() != signature.parameter_modes.len()
            || signature.return_types.is_empty()
        {
            return Err(native_error(
                span,
                "double signature requires complete ordered parameter/reference and result metadata",
            ));
        }
        if self
            .native_testing
            .doubles
            .get(nominal)
            .is_some_and(|existing| *existing != signature)
        {
            return Err(native_error(
                span,
                "inconsistent signature for a canonical double declaration",
            ));
        }
        self.native_testing
            .doubles
            .insert(nominal.to_owned(), signature);
        Ok(())
    }

    /// Shared alias registration is idempotent only for the exact generated
    /// identity. Ordinary source declarations cannot forge its canonical name.
    fn native_declare_handle_alias(
        &mut self,
        local: &str,
        nominal: &str,
        kind: NativeHandleKind,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        self.enable_native_test_mode()?;
        self.native_register_handle_kind(nominal, kind, span)?;
        let core_type = CoreType::Generic {
            name: nominal.to_owned(),
            type_args: Vec::new(),
        };
        if let Some(existing) = self.native_testing.aliases.get(local) {
            if *existing == core_type {
                return Ok(core_type);
            }
        }
        if self.environment().lookup_type(local, span).is_ok()
            || self.symbol_table().lookup(local).is_some()
        {
            return Err(native_error(
                span,
                "generated test handle conflicts with an existing binding",
            ));
        }
        self.environment_mut()
            .register_type(local.to_owned(), core_type.clone());
        self.symbol_table_mut().register(SymbolInfo {
            name: local.to_owned(),
            symbol_type: SymbolType::Type,
            core_type: core_type.clone(),
            visibility: Visibility::Private,
            source_location: span,
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
        self.native_testing
            .aliases
            .insert(local.to_owned(), core_type.clone());
        Ok(core_type)
    }

    /// Prevent an internal declaration identity from changing generated kinds.
    fn native_register_handle_kind(
        &mut self,
        nominal: &str,
        kind: NativeHandleKind,
        span: Span,
    ) -> Result<(), TypeError> {
        if self
            .native_testing
            .kinds
            .get(nominal)
            .is_some_and(|existing| *existing != kind)
        {
            return Err(native_error(
                span,
                "canonical test handle identity has conflicting declaration kinds",
            ));
        }
        self.native_testing.kinds.insert(nominal.to_owned(), kind);
        Ok(())
    }
}
