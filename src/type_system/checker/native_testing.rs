//! Native test-artifact typing state, separate from ordinary application errors.
extern crate alloc;
mod assertions;
mod contracts;
mod doubles;
mod error_variants;
mod examples;
mod handles;
mod inference;
mod module_signatures;
mod mutation;
mod source_types;
use super::TypeChecker;
use crate::ast::testing::ManagedCallback;
use crate::ast::{AstNode, BorrowKind, Expr};
use crate::token::Span;
use crate::type_system::NativeDoubleSignature;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::{BTreeMap, BTreeSet};

/// Compiler-only metadata, never runtime resource tracking.
#[derive(Debug, Clone, Default)]
pub(super) struct NativeTestingState {
    /// Explicit artifact mode; ordinary applications never enable test operations.
    enabled: bool,
    /// Native artifact type identities, separate from test grammar/availability.
    source_types: Option<BTreeMap<String, String>>,
    /// Generated canonical handle names and their concrete state types.
    worlds: BTreeMap<String, CoreType>,
    /// Concrete signatures for attempt-local generated double handle types.
    doubles: BTreeMap<String, NativeDoubleSignature>,
    /// Source aliases mapped to generated canonical nominal testing types.
    aliases: BTreeMap<String, CoreType>,
    /// Forward-declared generated kinds, independent of state/signature layout.
    kinds: BTreeMap<String, NativeHandleKind>,
    /// Active source callback's strict typed-use contract.
    active: Option<ActiveCallback>,
}

/// Forward declaration category for a generated nominal test handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeHandleKind {
    /// Per-attempt World, whose concrete state layout is defined separately.
    World,
    /// Attempt-local double, whose application signature is defined separately.
    Double,
}

/// Source binding identities for injected Worlds in one typed callback.
#[derive(Debug, Clone)]
struct ActiveCallback {
    /// Exact function boundary; inference never applies to nested ordinary helpers.
    span: Span,
    /// Source requested `errors inferred` on this managed callback.
    infer_errors: bool,
    /// Required parameter bindings, keyed by original span rather than spelling.
    required: BTreeMap<(usize, usize), (String, Span)>,
    /// Typed identifier reads that resolve to those exact parameter bindings.
    used: BTreeSet<(usize, usize)>,
}

impl TypeChecker {
    /// Enable test-only operations/imports and the runner-owned context type.
    ///
    /// # Errors
    /// Rejects a conflicting source declaration named `TestContext` in test mode.
    pub fn enable_native_test_mode(&mut self) -> Result<(), TypeError> {
        if self.native_testing.enabled {
            return Ok(());
        }
        let span = Span::single(crate::token::Position::start());
        if self.environment().lookup_type("TestContext", span).is_ok()
            || self.symbol_table().lookup("TestContext").is_some()
        {
            return Err(native_error(
                span,
                "TestContext is supplied by the test runner and cannot be redeclared in a test artifact",
            ));
        }
        let context_type = CoreType::Generic {
            name: "__opal_testing::TestContext".to_owned(),
            type_args: Vec::new(),
        };
        self.environment_mut()
            .register_type("TestContext".to_owned(), context_type.clone());
        self.native_testing
            .aliases
            .insert("TestContext".to_owned(), context_type.clone());
        self.symbol_table_mut().register(SymbolInfo {
            name: "TestContext".to_owned(),
            symbol_type: SymbolType::Type,
            core_type: context_type,
            visibility: Visibility::Private,
            source_location: span,
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
        self.register_adt_fields(
            "__opal_testing::TestContext".to_owned(),
            BTreeMap::from([
                ("failed".to_owned(), CoreType::Boolean),
                ("case_id".to_owned(), CoreType::String),
                ("attempt".to_owned(), CoreType::Int64),
            ]),
        );
        self.native_testing.enabled = true;
        self.allow_test_only_imports = true;
        Ok(())
    }

    /// Register a generated nominal World handle exposing one concrete `.state`.
    /// The native compiler supplies the canonical name, not a source alias.
    ///
    /// # Errors
    /// Rejects type/binding collisions and inconsistent generated state layouts.
    pub fn register_native_test_world(
        &mut self,
        local_name: &str,
        nominal_name: &str,
        state: CoreType,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let core_type = self.native_declare_world_alias(local_name, nominal_name, span)?;
        self.native_define_world_state(nominal_name, state, span)?;
        Ok(core_type)
    }

    /// Type-check an ordinary callback and enforce its injected World usage.
    /// Parameter-use checks are binding-aware and happen before optimization.
    ///
    /// # Errors
    /// Returns ordinary typing/borrowing errors or a strict unused-World error.
    pub fn type_check_native_callback(
        &mut self,
        callback: &ManagedCallback,
        required_world_parameters: &[String],
    ) -> Result<CoreType, TypeError> {
        self.enable_native_test_mode()?;
        let Expr::Lambda { ref params, .. } = callback.function else {
            return Err(native_error(
                callback.function.span(),
                "framework callback must be an ordinary function literal",
            ));
        };
        let mut required = BTreeMap::new();
        for name in required_world_parameters {
            let parameter = params
                .iter()
                .find(|parameter| parameter.name == *name)
                .ok_or_else(|| {
                    native_error(callback.function.span(), "missing injected World parameter")
                })?;
            if parameter.borrow_kind == BorrowKind::Owned {
                return Err(native_error(
                    parameter.span,
                    "World parameters require ref or mutable ref",
                ));
            }
            let param_type =
                self.ast_type_to_core_type_with_generics(&parameter.param_type, &[])?;
            if !matches!(param_type, CoreType::Generic { name: ref nominal_name, .. } if self.native_testing.worlds.contains_key(nominal_name))
            {
                return Err(native_error(
                    parameter.span,
                    "injected parameter requires a nominal World handle, not its state type",
                ));
            }
            required.insert(
                (parameter.span.start.offset, parameter.span.end.offset),
                (parameter.name.clone(), parameter.span),
            );
        }
        let previous = self.native_testing.active.replace(ActiveCallback {
            span: callback.function.span(),
            infer_errors: callback.infer_errors,
            required,
            used: BTreeSet::new(),
        });
        let result = self.type_check_expr(&callback.function);
        let active = self.native_testing.active.take();
        self.native_testing.active = previous;
        let core_type = match result {
            Ok(core_type) => core_type,
            Err(error) => {
                self.clear_constraints();
                return Err(error);
            }
        };
        let substitution = self.solve_constraints()?;
        let core_type = substitution.apply(&core_type);
        if let Some(active) = active {
            for (binding, (name, span)) in active.required {
                if !active.used.contains(&binding) {
                    return Err(native_error(
                        span,
                        &format!("unused injected World parameter '{name}' in this callback"),
                    ));
                }
            }
        }
        Ok(core_type)
    }

    /// User code cannot forge or substitute generated provider handles.
    pub(super) fn validate_native_world_construction(
        &self,
        name: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        if self.native_source_type(name).is_some_and(|core_type| matches!(core_type, CoreType::Generic { name: ref nominal_name, .. } if nominal_name == "__opal_testing::TestContext")) {
            return Err(native_error(span, "the runner constructs TestContext values; user construction is not permitted"));
        }
        if self.native_source_type(name).is_some_and(|core_type| matches!(core_type, CoreType::Generic { name: ref nominal_name, .. } if self.native_testing.kinds.get(nominal_name) == Some(&NativeHandleKind::World)))
            || self.native_testing.kinds.get(name) == Some(&NativeHandleKind::World)
        {
            Err(native_error(
                span,
                "the runner constructs World handles; user construction is not permitted",
            ))
        } else {
            Ok(())
        }
    }

    /// Resolve a source signature type using the current module's ordinary type
    /// declarations/imports plus generated native test nominal handles.
    pub(crate) fn native_source_core_type(
        &self,
        source_type: &crate::ast::Type,
    ) -> Result<CoreType, TypeError> {
        self.ast_type_to_core_type_with_generics(source_type, &[])
    }

    /// Resolve generated handles and imported source type aliases only in test
    /// artifacts; ordinary application type mapping remains unchanged.
    pub(super) fn native_source_type(&self, local_name: &str) -> Option<CoreType> {
        if !self.native_testing.enabled && self.native_testing.source_types.is_none() {
            return None;
        }
        if let Some(name) = self
            .native_testing
            .source_types
            .as_ref()
            .and_then(|names| names.get(local_name))
        {
            return Some(CoreType::Generic {
                name: name.clone(),
                type_args: Vec::new(),
            });
        }
        self.native_testing
            .aliases
            .get(local_name)
            .cloned()
            .or_else(|| {
                self.symbol_table()
                    .lookup(local_name)
                    .filter(|symbol| symbol.symbol_type == SymbolType::Type)
                    .map(|symbol| symbol.core_type.clone())
            })
    }

    /// Record a typed read by declaration identity, so local shadowing cannot
    /// make an unused injected World appear used.
    pub(super) fn record_native_parameter_read(&mut self, binding_span: Span) {
        if let Some(ref mut active) = self.native_testing.active {
            let binding = (binding_span.start.offset, binding_span.end.offset);
            if active.required.contains_key(&binding) {
                active.used.insert(binding);
            }
        }
    }
}

/// Native callback contract diagnostic using the original parameter location.
fn native_error(span: Span, message: &str) -> TypeError {
    TypeError::InvalidOperation {
        operation: message.to_owned(),
        type_name: "native test callback".to_owned(),
        span: TypeError::span_from_span(span),
    }
}
