//! Resolve World declarations and imports to nominal provider identities.
extern crate alloc;
use super::declaration_imports::resolve_bindings;
use super::identity::WorldIdentity;
use super::project::{LoadedModuleAst, NativeTestProject};
use super::world_graph::{WorldGraph, WorldGraphError, WorldNode};
use crate::ast::testing::{TestItem, TestSupportKind, WorldDeclaration};
use crate::token::Span;
use alloc::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

/// Canonical provider and its original typed source callback declarations.
#[derive(Debug, Clone)]
pub struct ResolvedWorld {
    /// Generated handle/provider identity, never its state type identity.
    pub identity: WorldIdentity,
    /// Defining module for callback type checking and code generation.
    pub module_path: PathBuf,
    /// Original source declaration, retaining documentation and spans.
    pub declaration: WorldDeclaration,
}

/// Resolved bindings and the static graph shared by all case descriptors.
#[derive(Debug, Clone)]
pub struct ResolvedWorldSet {
    /// One entry per canonical provider declaration.
    pub declarations: BTreeMap<WorldIdentity, ResolvedWorld>,
    /// Module-local names (including aliases) mapped to canonical providers.
    pub bindings: BTreeMap<PathBuf, BTreeMap<String, WorldIdentity>>,
    /// Validated graph in deterministic source/declaration order.
    pub graph: WorldGraph,
}

/// A static source-resolution failure, distinct from an attempt failure.
#[derive(Debug, Error)]
pub enum WorldResolutionError {
    /// Original declaration/import location and actionable compiler diagnostic.
    #[error("{module}: {message}")]
    Source {
        /// Canonical project-relative source module.
        module: String,
        /// Original source extent.
        span: Span,
        /// Static validation diagnostic.
        message: String,
    },
    /// Canonical dependency graph failure, including complete cycle paths.
    #[error(transparent)]
    Graph(#[from] WorldGraphError),
}

/// Resolve providers/aliases first, then validate all explicit dependencies.
///
/// # Errors
/// Rejects ambiguous/missing providers, duplicate dependency aliases and cycles.
pub fn resolve_worlds(
    project: &NativeTestProject,
) -> Result<ResolvedWorldSet, WorldResolutionError> {
    let mut declarations = BTreeMap::new();
    let mut bindings: BTreeMap<PathBuf, BTreeMap<String, WorldIdentity>> = BTreeMap::new();
    let mut declaration_order = Vec::new();
    for module in project.modules.values() {
        let LoadedModuleAst::Test(ref ast) = module.ast else {
            continue;
        };
        let module_bindings = bindings.entry(module.path.clone()).or_default();
        for item in &ast.items {
            if let TestItem::World(ref declaration) = *item {
                let identity = WorldIdentity(module.declaration_identity(&declaration.name));
                if module_bindings
                    .insert(declaration.name.clone(), identity.clone())
                    .is_some()
                {
                    return Err(source_error(
                        project,
                        &module.path,
                        declaration.span,
                        "duplicate World declaration",
                    ));
                }
                declaration_order.push(identity.clone());
                declarations.insert(
                    identity.clone(),
                    ResolvedWorld {
                        identity,
                        module_path: module.path.clone(),
                        declaration: declaration.clone(),
                    },
                );
            }
        }
    }
    resolve_bindings(
        project,
        TestSupportKind::World,
        &mut bindings,
        |module, span, message| source_error(project, &module.path, span, message),
    )?;
    let mut nodes = Vec::new();
    for identity in declaration_order {
        let world = &declarations[&identity];
        let local_bindings = &bindings[&world.module_path];
        let mut dependencies = Vec::new();
        for reference in &world.declaration.dependencies {
            let name = reference.path.join(".");
            let dependency = local_bindings.get(&name).ok_or_else(|| {
                source_error(
                    project,
                    &world.module_path,
                    reference.span,
                    &format!("unresolved World '{name}'"),
                )
            })?;
            dependencies.push(dependency.clone());
        }
        nodes.push(WorldNode {
            identity,
            dependencies,
        });
    }
    let graph = WorldGraph::new(nodes)?;
    Ok(ResolvedWorldSet {
        declarations,
        bindings,
        graph,
    })
}

/// Preserve source-level diagnostic locations while reporting canonical modules.
fn source_error(
    project: &NativeTestProject,
    path: &PathBuf,
    span: Span,
    message: &str,
) -> WorldResolutionError {
    let module = project.modules.get(path).map_or_else(
        || path.display().to_string(),
        |module| module.identity.clone(),
    );
    WorldResolutionError::Source {
        module,
        span,
        message: message.to_owned(),
    }
}
