//! Shared identity/handle alias resolution for explicit World and double imports.
extern crate alloc;
use super::project::{LoadedModule, LoadedModuleAst, NativeTestProject};
use crate::ast::ImportItem;
use crate::ast::testing::{TestItem, TestSupportKind};
use crate::module_loader::resolve_import_path;
use crate::token::Span;
use alloc::collections::BTreeMap;
use std::path::PathBuf;

/// One named declaration import awaiting a canonical identity (including re-exports).
struct PendingImport {
    /// Importing module.
    module: PathBuf,
    /// Imported module.
    from: PathBuf,
    /// Name in the imported module.
    name: String,
    /// Local alias introducing provider identity and handle together.
    alias: String,
    /// Original import location.
    span: Span,
}

/// Resolve explicit declaration imports to a fixed point, preserving local
/// declaration collisions and never executing application functions.
pub(super) fn resolve_bindings<Identity: Clone, Error>(
    project: &NativeTestProject,
    kind: TestSupportKind,
    bindings: &mut BTreeMap<PathBuf, BTreeMap<String, Identity>>,
    make_error: impl Fn(&LoadedModule, Span, &str) -> Error,
) -> Result<(), Error> {
    let label = match kind {
        TestSupportKind::World => "World",
        TestSupportKind::Double => "double",
    };
    let mut pending = Vec::new();
    for module in project.modules.values() {
        let LoadedModuleAst::Test(ref ast) = module.ast else {
            continue;
        };
        for item in &ast.items {
            let TestItem::SupportImport(ref import) = *item else {
                continue;
            };
            if import.kind != kind {
                continue;
            }
            let from = resolve_import_path(&module.path, &import.source)
                .map_err(|diagnostic| make_error(module, import.span, &diagnostic.to_string()))?;
            for import_item in &import.items {
                let ImportItem::Named {
                    ref name,
                    ref alias,
                    span,
                } = *import_item
                else {
                    return Err(make_error(
                        module,
                        import.span,
                        &format!("{label} imports require explicit names"),
                    ));
                };
                pending.push(PendingImport {
                    module: module.path.clone(),
                    from: from.clone(),
                    name: name.clone(),
                    alias: alias.as_ref().unwrap_or(name).clone(),
                    span,
                });
            }
        }
    }
    while !pending.is_empty() {
        let previous_count = pending.len();
        let mut unresolved = Vec::new();
        for import in pending {
            let identity = bindings
                .get(&import.from)
                .and_then(|source| source.get(&import.name))
                .cloned();
            let Some(identity) = identity else {
                unresolved.push(import);
                continue;
            };
            if bindings
                .entry(import.module.clone())
                .or_default()
                .insert(import.alias.clone(), identity)
                .is_some()
            {
                return Err(make_error(
                    &project.modules[&import.module],
                    import.span,
                    &format!("ambiguous {label} binding '{}'", import.alias),
                ));
            }
        }
        if unresolved.len() == previous_count {
            if let Some(import) = unresolved.first() {
                return Err(make_error(
                    &project.modules[&import.module],
                    import.span,
                    &format!("unresolved imported {label} '{}'", import.name),
                ));
            }
        }
        pending = unresolved;
    }
    Ok(())
}
