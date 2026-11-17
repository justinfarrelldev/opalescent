//! Canonical, pre-optimization import-edge resolution for typed doubles.
extern crate alloc;
use super::declaration_imports::resolve_bindings;
use super::identity::{DoubleIdentity, ResolvedImportEdge};
use super::project::{LoadedModule, LoadedModuleAst, NativeTestProject};
use super::subjects::resolve_subject;
use crate::ast::testing::{DoubleDeclaration, SourceReference, TestItem, TestSupportKind};
use crate::ast::{Decl, ImportItem};
use crate::module_loader::resolve_import_path;
use crate::token::Span;
use alloc::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

/// One exact source import edge, not global symbol replacement.
#[derive(Debug, Clone)]
pub struct ResolvedDouble {
    /// Canonical double declaration identity (not the replaced function).
    pub identity: DoubleIdentity,
    /// Defining test/support module for aliases and diagnostics.
    pub module_path: PathBuf,
    /// Original declaration, preserving documentation and source spans.
    pub declaration: DoubleDeclaration,
    /// Canonical importing module and imported callable identity.
    pub edge: ResolvedImportEdge,
    /// Local names for this edge in its importing module, including aliases.
    pub import_names: Vec<String>,
}

/// Resolved declarations and module-local generated handle bindings.
#[derive(Debug, Clone)]
pub struct ResolvedDoubleSet {
    /// Distinct declarations; inactive declarations may target the same edge.
    pub declarations: BTreeMap<DoubleIdentity, ResolvedDouble>,
    /// Source aliases introduce declaration identity and handle type together.
    pub bindings: BTreeMap<PathBuf, BTreeMap<String, DoubleIdentity>>,
}

/// A localized static diagnostic, never a runtime mock/assertion failure.
#[derive(Debug, Error)]
#[error("{module}: {message}")]
pub struct DoubleResolutionError {
    /// Canonical defining source module.
    pub module: String,
    /// Original declaration/import location.
    pub span: Span,
    /// Compiler diagnostic for unsupported or unresolved boundary syntax.
    pub message: String,
}

/// Resolve every declared edge, including inactive boundaries, without executing
/// application functions or changing call behavior.
///
/// # Errors
/// Rejects missing/ambiguous callables and unsupported non-import boundaries.
pub fn resolve_doubles(
    project: &NativeTestProject,
) -> Result<ResolvedDoubleSet, DoubleResolutionError> {
    let mut declarations = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for module in project.modules.values() {
        let LoadedModuleAst::Test(ref ast) = module.ast else {
            continue;
        };
        let mut local = BTreeMap::new();
        for item in &ast.items {
            if let TestItem::Double(ref declaration) = *item {
                let identity = DoubleIdentity(module.declaration_identity(&declaration.name));
                if local
                    .insert(declaration.name.clone(), identity.clone())
                    .is_some()
                {
                    return Err(error(
                        module,
                        declaration.span,
                        "duplicate double declaration",
                    ));
                }
                let (edge, import_names) = resolve_edge(project, module, declaration)?;
                declarations.insert(
                    identity.clone(),
                    ResolvedDouble {
                        identity,
                        module_path: module.path.clone(),
                        declaration: declaration.clone(),
                        edge,
                        import_names,
                    },
                );
            }
        }
        bindings.insert(module.path.clone(), local);
    }
    resolve_bindings(project, TestSupportKind::Double, &mut bindings, error)?;
    Ok(ResolvedDoubleSet {
        declarations,
        bindings,
    })
}

/// Resolve the exact canonical edge and confirm that its imported declaration
/// is genuinely callable; spelling or runtime addresses are insufficient.
fn resolve_edge(
    project: &NativeTestProject,
    module: &LoadedModule,
    declaration: &DoubleDeclaration,
) -> Result<(ResolvedImportEdge, Vec<String>), DoubleResolutionError> {
    if declaration.function.path.len() != 1 {
        return Err(error(
            module,
            declaration.function.span,
            "double target requires a named imported declaration",
        ));
    }
    let from = resolve_import_path(&module.path, &declaration.from)
        .map_err(|diagnostic| error(module, declaration.span, &diagnostic.to_string()))?;
    let importing = resolve_import_path(&module.path, &declaration.as_used_by)
        .map_err(|diagnostic| error(module, declaration.span, &diagnostic.to_string()))?;
    let importing_module = project.modules.get(&importing).ok_or_else(|| {
        error(
            module,
            declaration.span,
            "double importing module is not a compiled source module",
        )
    })?;
    let mut import_names = Vec::new();
    let mut canonical_function = None;
    for import in ordinary_declarations(importing_module) {
        let Decl::Import {
            ref items,
            ref source,
            ..
        } = *import
        else {
            continue;
        };
        let imported_path = resolve_import_path(&importing, source)
            .map_err(|diagnostic| error(module, declaration.span, &diagnostic.to_string()))?;
        if imported_path != from {
            continue;
        }
        for item in items {
            let ImportItem::Named {
                ref name,
                ref alias,
                ..
            } = *item
            else {
                continue;
            };
            if name != &declaration.function.path[0] {
                continue;
            }
            let local_name = alias.as_ref().unwrap_or(name);
            let reference = SourceReference {
                path: vec![local_name.clone()],
                span: declaration.function.span,
            };
            let function = resolve_subject(project, &importing, &reference)
                .map_err(|diagnostic| error(module, declaration.span, &diagnostic.to_string()))?;
            if canonical_function
                .as_ref()
                .is_some_and(|previous| *previous != function)
            {
                return Err(error(
                    module,
                    declaration.span,
                    "ambiguous double import edge",
                ));
            }
            canonical_function = Some(function);
            import_names.push(local_name.clone());
        }
    }
    let imported_function = canonical_function.ok_or_else(|| error(module, declaration.span, "unsupported interception boundary: no matching direct named import edge; same-module/indirect/foreign callback replacement is not supported"))?;
    Ok((
        ResolvedImportEdge {
            importing_module: importing_module.identity.clone(),
            imported_function,
        },
        import_names,
    ))
}

/// Ordinary import declarations retain the existing application grammar.
fn ordinary_declarations(module: &LoadedModule) -> Vec<&Decl> {
    match module.ast {
        LoadedModuleAst::Application(ref ast) => ast.declarations.iter().collect(),
        LoadedModuleAst::Test(ref ast) => ast
            .items
            .iter()
            .filter_map(|item| match *item {
                TestItem::Application(ref declaration) => Some(declaration.as_ref()),
                _ => None,
            })
            .collect(),
    }
}

/// Preserve the source extent while displaying a canonical declaration module.
fn error(module: &LoadedModule, span: Span, message: &str) -> DoubleResolutionError {
    DoubleResolutionError {
        module: module.identity.clone(),
        span,
        message: message.to_owned(),
    }
}
