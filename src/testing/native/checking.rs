//! Native test source orchestration through the ordinary compiler type checker.
extern crate alloc;
mod callbacks;
mod module_order;
use super::cases::{ResolvedSuite, resolve_cases};
use super::identity::WorldIdentity;
use super::project::{LoadedModule, LoadedModuleAst, NativeLoadError, NativeTestProject};
use super::world_resolution::{ResolvedWorldSet, WorldResolutionError, resolve_worlds};
use crate::ast::testing::TestItem;
use crate::ast::{Decl, Expr, Program};
use crate::token::Span;
use crate::type_system::ModuleInterface;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A module whose ordinary declarations and managed callbacks have been checked.
pub struct CheckedNativeModule {
    /// Normal source declarations, retained for native application lowering.
    pub program: Program,
    /// Authoritative type checker, including normal expression metadata/layouts.
    pub checker: TypeChecker,
    /// Checked callback signatures by original source boundary.
    pub callbacks: BTreeMap<(usize, usize), CoreType>,
}

/// Fully checked source suite. Native ABI instrumentation/lowering is separate.
pub struct CheckedNativeProject {
    /// Canonical providers and validated dependency graph.
    pub worlds: ResolvedWorldSet,
    /// Stable independent cases and shared source bodies.
    pub suite: ResolvedSuite,
    /// Module type-checking results in canonical source paths.
    pub modules: BTreeMap<PathBuf, CheckedNativeModule>,
    /// Concrete state type for every canonical provider.
    pub world_states: BTreeMap<WorldIdentity, CoreType>,
    /// Generated nominal handle type for every canonical provider.
    pub world_types: BTreeMap<WorldIdentity, CoreType>,
}

/// Compiler failure; it never represents a failed test execution attempt.
#[derive(Debug, Error)]
pub enum NativeCheckError {
    /// Original source discovery failure.
    #[error(transparent)]
    Load(#[from] NativeLoadError),
    /// Canonical provider resolution/graph failure.
    #[error(transparent)]
    Worlds(#[from] WorldResolutionError),
    /// Ordinary compiler diagnostics with their defining source module.
    #[error("test compilation failed in {path:?}: {errors:?}")]
    Type {
        /// Defining source module.
        path: PathBuf,
        /// Collected ordinary compiler diagnostics with original source spans.
        errors: Vec<TypeError>,
    },
}

/// Check sources, callbacks, lifecycle contracts, examples and associations.
/// No application function, setup, hook or test callback executes here.
///
/// # Errors
/// Returns source/module/World resolution or ordinary static compiler diagnostics.
pub fn check_project(
    project: &NativeTestProject,
) -> Result<CheckedNativeProject, NativeCheckError> {
    let worlds = resolve_worlds(project)?;
    let suite =
        resolve_cases(project, &worlds).map_err(|error| type_error(&project.root, error))?;
    let order = module_order::resolve(project)?;
    let mut modules = BTreeMap::new();
    let mut interfaces: BTreeMap<PathBuf, ModuleInterface> = BTreeMap::new();
    let mut world_states = BTreeMap::new();
    let mut world_types = BTreeMap::new();
    for path in order {
        let module = &project.modules[&path];
        let program = ordinary_program(module);
        let mut checker = TypeChecker::new();
        checker.set_current_module_path(path.display().to_string());
        for interface in interfaces.values() {
            checker.register_module_interface(interface.clone());
        }
        register_import_interfaces(&mut checker, &path, &program, &interfaces)?;
        let test_mode = matches!(module.ast, LoadedModuleAst::Test(_));
        if test_mode {
            checker
                .enable_native_test_mode()
                .map_err(|error| type_error(&path, error))?;
        }
        let (prerequisites, values) = ordinary_checking_phases(&program);
        check_ordinary_program(&mut checker, &prerequisites, &path)?;
        if test_mode {
            register_world_types(
                module,
                &mut checker,
                &worlds,
                &mut world_states,
                &mut world_types,
            )?;
        }
        check_ordinary_program(&mut checker, &values, &path)?;
        let mut callback_signatures = BTreeMap::new();
        if let LoadedModuleAst::Test(ref ast) = module.ast {
            for item in &ast.items {
                if let TestItem::Double(ref declaration) = *item {
                    return Err(type_error(
                        &path,
                        invalid(
                            declaration.span,
                            "double declaration requires resolved typed test-build interception",
                        ),
                    ));
                }
            }
            callbacks::check_module(
                module,
                &mut checker,
                &worlds,
                &world_states,
                &world_types,
                &suite,
                &mut callback_signatures,
            )?;
        }
        if let Some(interface) = checker.module_interface(&path.display().to_string()) {
            interfaces.insert(path.clone(), interface);
        }
        modules.insert(
            path,
            CheckedNativeModule {
                program,
                checker,
                callbacks: callback_signatures,
            },
        );
    }
    Ok(CheckedNativeProject {
        worlds,
        suite,
        modules,
        world_states,
        world_types,
    })
}

/// Retain ordinary source declarations without source rewriting.
fn ordinary_program(module: &LoadedModule) -> Program {
    match module.ast {
        LoadedModuleAst::Application(ref program) => program.clone(),
        LoadedModuleAst::Test(ref ast) => Program {
            declarations: ast
                .items
                .iter()
                .filter_map(|item| match *item {
                    TestItem::Application(ref declaration) => Some(*declaration.clone()),
                    _ => None,
                })
                .collect(),
            span: ast.span,
            id: crate::ast::NodeId(0),
        },
    }
}

/// Imports and ordinary type declarations must precede generated handle
/// registration; helpers are checked afterward so their World signatures work.
fn ordinary_checking_phases(program: &Program) -> (Program, Program) {
    let (prerequisites, values): (Vec<_>, Vec<_>) =
        program
            .declarations
            .iter()
            .cloned()
            .partition(|declaration| {
                matches!(
                    *declaration,
                    Decl::Import { .. }
                        | Decl::Type { .. }
                        | Decl::ErrorSet { .. }
                        | Decl::Namespace { .. }
                )
            });
    (
        Program {
            declarations: prerequisites,
            span: program.span,
            id: program.id,
        },
        Program {
            declarations: values,
            span: program.span,
            id: program.id,
        },
    )
}

/// Register source-keyed views of actual dependency interfaces, as in the normal
/// project compiler, while retaining canonical defining identities separately.
fn register_import_interfaces(
    checker: &mut TypeChecker,
    path: &Path,
    program: &Program,
    interfaces: &BTreeMap<PathBuf, ModuleInterface>,
) -> Result<(), NativeCheckError> {
    for declaration in &program.declarations {
        if let Decl::Import { ref source, .. } = *declaration {
            let resolved = crate::module_loader::resolve_import_path(path, source)
                .map_err(|error| type_error(path, error))?;
            if let Some(interface) = interfaces.get(&resolved) {
                let mut source_view = interface.clone();
                source_view.module_path.clone_from(source);
                checker.register_module_interface(source_view);
            }
        }
    }
    Ok(())
}

/// Test/support modules and ordinary dependencies need no application entry.
fn check_ordinary_program(
    checker: &mut TypeChecker,
    program: &Program,
    path: &Path,
) -> Result<(), NativeCheckError> {
    if let Err(errors) = checker.type_check_program(program) {
        let errors: Vec<_> = errors
            .into_iter()
            .filter(|error| !matches!(*error, TypeError::MissingEntryPoint { .. }))
            .collect();
        if !errors.is_empty() {
            return Err(NativeCheckError::Type {
                path: path.to_owned(),
                errors,
            });
        }
    }
    Ok(())
}

/// Register all visible provider aliases with one canonical nominal handle each.
fn register_world_types(
    module: &LoadedModule,
    checker: &mut TypeChecker,
    worlds: &ResolvedWorldSet,
    states: &mut BTreeMap<WorldIdentity, CoreType>,
    types: &mut BTreeMap<WorldIdentity, CoreType>,
) -> Result<(), NativeCheckError> {
    for world in worlds
        .declarations
        .values()
        .filter(|world| world.module_path == module.path)
    {
        let Expr::Lambda {
            ref return_types, ..
        } = world.declaration.setup.function
        else {
            return Err(type_error(
                &module.path,
                invalid(
                    world.declaration.span,
                    "World setup requires an ordinary typed function",
                ),
            ));
        };
        if return_types.len() != 1 {
            return Err(type_error(
                &module.path,
                invalid(
                    world.declaration.span,
                    "World setup returns exactly one concrete state value, not labeled multiple returns",
                ),
            ));
        }
        let state = checker
            .native_source_core_type(&return_types[0])
            .map_err(|error| type_error(&module.path, error))?;
        if state == CoreType::Unit {
            return Err(type_error(
                &module.path,
                invalid(
                    world.declaration.span,
                    "World setup must return a concrete state value",
                ),
            ));
        }
        states.insert(world.identity.clone(), state);
    }
    if let Some(bindings) = worlds.bindings.get(&module.path) {
        for (local_name, identity) in bindings {
            let state = states.get(identity).ok_or_else(|| {
                type_error(
                    &module.path,
                    invalid(
                        module_span(module),
                        "World state type is not available from its defining module",
                    ),
                )
            })?;
            let canonical_name = format!(
                "__opal_world_{}:{}{}:{}",
                identity.0.module.len(),
                identity.0.module,
                identity.0.name.len(),
                identity.0.name
            );
            let handle = checker
                .register_native_test_world(
                    local_name,
                    &canonical_name,
                    state.clone(),
                    worlds.declarations[identity].declaration.span,
                )
                .map_err(|error| type_error(&module.path, error))?;
            types.insert(identity.clone(), handle);
        }
    }
    Ok(())
}

/// Original module extent for static diagnostics.
const fn module_span(module: &LoadedModule) -> Span {
    match module.ast {
        LoadedModuleAst::Application(ref program) => program.span,
        LoadedModuleAst::Test(ref ast) => ast.span,
    }
}

/// Attach the source module to a normal compiler type diagnostic.
fn type_error(path: &Path, error: TypeError) -> NativeCheckError {
    NativeCheckError::Type {
        path: path.to_owned(),
        errors: vec![error],
    }
}

/// Static native-artifact contract diagnostic.
fn invalid(span: Span, message: &str) -> TypeError {
    TypeError::InvalidOperation {
        operation: message.to_owned(),
        type_name: "native test artifact".to_owned(),
        span: TypeError::span_from_span(span),
    }
}
