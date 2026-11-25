//! Native test source orchestration through the ordinary compiler type checker.
extern crate alloc;
mod activation;
mod callbacks;
mod declarations;
mod double_signatures;
mod module_order;
use super::cases::{ResolvedSuite, resolve_cases};
use super::doubles::{DoubleResolutionError, ResolvedDoubleSet, resolve_doubles};
use super::identity::{DoubleIdentity, WorldIdentity};
use super::project::{LoadedModule, LoadedModuleAst, NativeLoadError, NativeTestProject};
use super::world_resolution::{ResolvedWorldSet, WorldResolutionError, resolve_worlds};
use crate::ast::testing::TestItem;
use crate::ast::{Decl, Program};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use crate::type_system::{ModuleInterface, NativeDoubleSignature};
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
    /// Canonical direct import edges, independent of activation.
    pub doubles: ResolvedDoubleSet,
    /// Concrete callable signatures for precompiled double dispatchers.
    pub double_signatures: BTreeMap<DoubleIdentity, NativeDoubleSignature>,
    /// Stable independent cases and shared source bodies.
    pub suite: ResolvedSuite,
    /// Module type-checking results in canonical source paths.
    pub modules: BTreeMap<PathBuf, CheckedNativeModule>,
    /// Concrete state type for every canonical provider.
    pub world_states: BTreeMap<WorldIdentity, CoreType>,
    /// Generated nominal handle type for every canonical provider.
    pub world_types: BTreeMap<WorldIdentity, CoreType>,
    /// Generated nominal double types, not aliases for the replaced functions.
    pub double_types: BTreeMap<DoubleIdentity, CoreType>,
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
    /// Invalid or unsupported source import-edge declaration.
    #[error(transparent)]
    Doubles(#[from] DoubleResolutionError),
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
/// Returns source/module/World/edge resolution or ordinary compiler diagnostics.
pub fn check_project(
    project: &NativeTestProject,
) -> Result<CheckedNativeProject, NativeCheckError> {
    let worlds = resolve_worlds(project)?;
    let doubles = resolve_doubles(project)?;
    let suite =
        resolve_cases(project, &worlds).map_err(|error| type_error(&project.root, error))?;
    let order = module_order::resolve(project)?;
    let mut compilation = ModuleCompilation::default();
    // Ordinary application dependencies cannot import test-only modules, so
    // they can be checked first without changing their normal language mode.
    for path in order
        .iter()
        .filter(|path| matches!(project.modules[*path].ast, LoadedModuleAst::Application(_)))
    {
        compilation.prepare_application(
            &project.modules[path],
            path.starts_with(project.root.join("tests")),
        )?;
    }
    for path in order
        .iter()
        .filter(|path| matches!(project.modules[*path].ast, LoadedModuleAst::Test(_)))
    {
        compilation.prepare_test(&project.modules[path], &worlds, &doubles)?;
    }
    let double_signatures = double_signatures::resolve(project, &doubles, &compilation.interfaces)?;
    let world_states = declarations::state_types(&worlds, &compilation.modules)?;
    compilation.finish_tests(
        project,
        &worlds,
        &doubles,
        &suite,
        &world_states,
        &double_signatures,
    )?;
    activation::check(&suite, &worlds, &doubles)?;
    Ok(CheckedNativeProject {
        worlds,
        doubles,
        double_signatures,
        suite,
        modules: compilation.modules,
        world_states,
        world_types: compilation.world_types,
        double_types: compilation.double_types,
    })
}

/// Mutable compiler state used only while constructing one checked source suite.
#[derive(Default)]
struct ModuleCompilation {
    /// Ordinary checked interfaces or provisional test-helper signatures.
    interfaces: BTreeMap<PathBuf, ModuleInterface>,
    /// Prepared module checkers, retaining original source declarations.
    modules: BTreeMap<PathBuf, CheckedNativeModule>,
    /// Test helpers whose bodies wait for state/double contracts to be defined.
    pending_bodies: BTreeMap<PathBuf, Program>,
    /// Canonical World types, shared across source aliases.
    world_types: BTreeMap<WorldIdentity, CoreType>,
    /// Canonical double types, shared across source aliases.
    double_types: BTreeMap<DoubleIdentity, CoreType>,
}

impl ModuleCompilation {
    /// Prepare a fresh checker with normal dependency interfaces and imports.
    fn checker(
        &self,
        module: &LoadedModule,
        program: &Program,
    ) -> Result<TypeChecker, NativeCheckError> {
        let mut checker = TypeChecker::new();
        checker.set_current_module_path(module.path.display().to_string());
        for interface in self.interfaces.values() {
            checker.native_register_source_interface(interface.clone());
        }
        register_import_interfaces(&mut checker, &module.path, program, &self.interfaces)?;
        checker.native_enable_source_type_identities(program, &module.identity);
        Ok(checker)
    }

    /// Application sources retain ordinary grammar/availability and receive
    /// canonical artifact type identities without enabling test operations.
    fn prepare_application(
        &mut self,
        module: &LoadedModule,
        test_support: bool,
    ) -> Result<(), NativeCheckError> {
        let program = ordinary_program(module);
        let mut checker = self.checker(module, &program)?;
        if test_support {
            checker.enable_test_only_imports();
        }
        let (prerequisites, values) = ordinary_checking_phases(&program);
        check_ordinary_program(&mut checker, &prerequisites, &module.path)?;
        checker.native_sync_source_type_metadata();
        check_ordinary_program(&mut checker, &values, &module.path)?;
        self.interfaces.insert(
            module.path.clone(),
            checked_interface(&checker, &module.path)?,
        );
        self.modules.insert(
            module.path.clone(),
            CheckedNativeModule {
                program,
                checker,
                callbacks: BTreeMap::new(),
            },
        );
        Ok(())
    }

    /// Declare identities and publish signatures before type-checking helpers
    /// that configure doubles or access generated World state fields.
    fn prepare_test(
        &mut self,
        module: &LoadedModule,
        worlds: &ResolvedWorldSet,
        doubles: &ResolvedDoubleSet,
    ) -> Result<(), NativeCheckError> {
        let program = ordinary_program(module);
        let mut checker = self.checker(module, &program)?;
        checker
            .enable_native_test_mode()
            .map_err(|error| type_error(&module.path, error))?;
        let (prerequisites, values) = ordinary_checking_phases(&program);
        check_ordinary_program(&mut checker, &prerequisites, &module.path)?;
        checker.native_sync_source_type_metadata();
        declarations::declare_aliases(
            module,
            &mut checker,
            worlds,
            doubles,
            &mut self.world_types,
            &mut self.double_types,
        )?;
        let (bodies, interface) =
            checker
                .native_prepare_test_values(&values)
                .map_err(|errors| NativeCheckError::Type {
                    path: module.path.clone(),
                    errors,
                })?;
        self.interfaces.insert(module.path.clone(), interface);
        self.pending_bodies.insert(module.path.clone(), bodies);
        self.modules.insert(
            module.path.clone(),
            CheckedNativeModule {
                program,
                checker,
                callbacks: BTreeMap::new(),
            },
        );
        Ok(())
    }

    /// Define immutable compiler contracts globally, then check each actual
    /// source body once. No nominal alias is invented in an importing module.
    fn finish_tests(
        &mut self,
        project: &NativeTestProject,
        worlds: &ResolvedWorldSet,
        doubles: &ResolvedDoubleSet,
        suite: &ResolvedSuite,
        states: &BTreeMap<WorldIdentity, CoreType>,
        signatures: &BTreeMap<DoubleIdentity, NativeDoubleSignature>,
    ) -> Result<(), NativeCheckError> {
        for path in self.pending_bodies.keys() {
            let checker = &mut self
                .modules
                .get_mut(path)
                .ok_or_else(|| {
                    type_error(
                        path,
                        invalid(
                            module_span(&project.modules[path]),
                            "native module checker was not prepared",
                        ),
                    )
                })?
                .checker;
            for (identity, state) in states {
                checker
                    .native_define_world_state(
                        &declarations::world_name(identity),
                        state.clone(),
                        worlds.declarations[identity].declaration.span,
                    )
                    .map_err(|error| type_error(path, error))?;
            }
            for (identity, signature) in signatures {
                checker
                    .native_define_double_signature(
                        &declarations::double_name(identity),
                        signature.clone(),
                        doubles.declarations[identity].declaration.span,
                    )
                    .map_err(|error| type_error(path, error))?;
            }
        }
        for (path, bodies) in &self.pending_bodies {
            let checked = self.modules.get_mut(path).ok_or_else(|| {
                type_error(
                    path,
                    invalid(
                        module_span(&project.modules[path]),
                        "native module checker was not prepared",
                    ),
                )
            })?;
            register_import_interfaces(
                &mut checked.checker,
                path,
                &checked.program,
                &self.interfaces,
            )?;
            checked
                .checker
                .native_check_test_value_bodies(bodies)
                .map_err(|errors| NativeCheckError::Type {
                    path: path.clone(),
                    errors,
                })?;
            callbacks::check_module(
                &project.modules[path],
                &mut checked.checker,
                worlds,
                states,
                &self.world_types,
                suite,
                &mut checked.callbacks,
            )?;
            self.interfaces
                .insert(path.clone(), checked_interface(&checked.checker, path)?);
        }
        Ok(())
    }
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

/// Source imports and type declarations precede generated nominal declarations.
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

/// Import source-keyed views of checked or provisional dependency interfaces.
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
                checker.native_register_source_interface(source_view);
            }
        }
    }
    Ok(())
}

/// Dependencies and support modules do not need application entry points.
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

/// A checked module must expose its actual compiler interface, even if empty.
fn checked_interface(
    checker: &TypeChecker,
    path: &Path,
) -> Result<ModuleInterface, NativeCheckError> {
    checker
        .module_interface(&path.display().to_string())
        .ok_or_else(|| {
            type_error(
                path,
                invalid(
                    Span::single(crate::token::Position::start()),
                    "checked native module has no compiler interface",
                ),
            )
        })
}

/// Original module extent for static diagnostics.
const fn module_span(module: &LoadedModule) -> Span {
    match module.ast {
        LoadedModuleAst::Application(ref program) => program.span,
        LoadedModuleAst::Test(ref ast) => ast.span,
    }
}

/// Attach a defining source module to a normal compiler type diagnostic.
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
