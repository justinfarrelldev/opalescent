//! Resolve World declarations and imports to nominal provider identities.
extern crate alloc;
use super::identity::WorldIdentity;
use super::project::{LoadedModuleAst, NativeTestProject};
use super::world_graph::{WorldGraph, WorldGraphError, WorldNode};
use crate::ast::ImportItem;
use crate::ast::testing::{TestItem, TestSupportKind, WorldDeclaration};
use crate::module_loader::resolve_import_path;
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

/// An explicit World import awaiting resolution, including support re-exports.
struct PendingWorldImport {
    /// Importing source module.
    module: PathBuf,
    /// Imported source module.
    from: PathBuf,
    /// Name exported by that source module.
    imported_name: String,
    /// Local provider and generated handle name.
    local_name: String,
    /// Original import location.
    span: Span,
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
    let mut pending_imports = Vec::new();
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
            if let TestItem::SupportImport(ref import) = *item {
                if import.kind != TestSupportKind::World {
                    continue;
                }
                let from = resolve_import_path(&module.path, &import.source).map_err(|error| {
                    source_error(project, &module.path, import.span, &error.to_string())
                })?;
                for import_item in &import.items {
                    let ImportItem::Named {
                        ref name,
                        ref alias,
                        span,
                    } = *import_item
                    else {
                        return Err(source_error(
                            project,
                            &module.path,
                            import.span,
                            "World imports require explicit names",
                        ));
                    };
                    pending_imports.push(PendingWorldImport {
                        module: module.path.clone(),
                        from: from.clone(),
                        imported_name: name.clone(),
                        local_name: alias.as_ref().unwrap_or(name).clone(),
                        span,
                    });
                }
            }
        }
    }
    resolve_import_bindings(project, &mut bindings, pending_imports)?;
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

/// Resolve support aliases by fixed point; no application functions are run.
fn resolve_import_bindings(
    project: &NativeTestProject,
    bindings: &mut BTreeMap<PathBuf, BTreeMap<String, WorldIdentity>>,
    mut pending: Vec<PendingWorldImport>,
) -> Result<(), WorldResolutionError> {
    while !pending.is_empty() {
        let previous_count = pending.len();
        let mut unresolved = Vec::new();
        for import in pending {
            let identity = bindings
                .get(&import.from)
                .and_then(|source| source.get(&import.imported_name))
                .cloned();
            let Some(identity) = identity else {
                unresolved.push(import);
                continue;
            };
            let local_bindings = bindings.entry(import.module.clone()).or_default();
            if local_bindings
                .insert(import.local_name.clone(), identity)
                .is_some()
            {
                return Err(source_error(
                    project,
                    &import.module,
                    import.span,
                    &format!("ambiguous World binding '{}'", import.local_name),
                ));
            }
        }
        if unresolved.len() == previous_count {
            if let Some(import) = unresolved.first() {
                return Err(source_error(
                    project,
                    &import.module,
                    import.span,
                    &format!("unresolved imported World '{}'", import.imported_name),
                ));
            }
        }
        pending = unresolved;
    }
    Ok(())
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
