//! Replace-not-append resolution for scoped test metadata.
extern crate alloc;
use super::{EffectiveMetadata, case_error};
use crate::ast::testing::{SourceReference, TestMetadata};
use crate::testing::native::identity::WorldIdentity;
use crate::testing::native::project::{LoadedModule, NativeTestProject};
use crate::testing::native::subjects::resolve_subject;
use crate::testing::native::world_resolution::ResolvedWorldSet;
use crate::type_system::errors::TypeError;
use alloc::collections::BTreeSet;

/// Apply local clauses once, resolving identities before checking duplicates.
pub(super) fn apply<'clause>(
    project: &NativeTestProject,
    module: &LoadedModule,
    worlds: &ResolvedWorldSet,
    inherited: &EffectiveMetadata,
    clauses: impl Iterator<Item = &'clause TestMetadata>,
) -> Result<EffectiveMetadata, TypeError> {
    let mut effective = inherited.clone();
    let mut declared = BTreeSet::new();
    for clause in clauses {
        let (key, span) = match *clause {
            TestMetadata::Subjects { span, .. } => ("subjects", span),
            TestMetadata::Worlds { span, .. } => ("worlds", span),
            TestMetadata::ActivateWorlds { span, .. } => ("activate worlds", span),
            TestMetadata::Kind { span, .. } => ("kind", span),
            TestMetadata::Exclusive { span, .. } => ("requires exclusive access to", span),
        };
        if !declared.insert(key) {
            return Err(case_error(
                span,
                &format!("duplicate or conflicting '{key}' metadata"),
            ));
        }
        match *clause {
            TestMetadata::Subjects { ref references, .. } => {
                effective.subjects.clear();
                for reference in references {
                    let subject = resolve_subject(project, &module.path, reference)?;
                    if !effective.subjects.contains(&subject) {
                        effective.subjects.push(subject);
                    }
                }
            }
            TestMetadata::Worlds { ref references, .. } => {
                effective.worlds = resolve_world_references(worlds, module, references)?;
            }
            TestMetadata::ActivateWorlds { ref references, .. } => {
                effective.activated = resolve_world_references(worlds, module, references)?;
            }
            TestMetadata::Kind { kind, .. } => effective.kind = kind,
            TestMetadata::Exclusive { ref keys, .. } => {
                for exclusion_key in keys {
                    if !effective.exclusion_keys.contains(exclusion_key) {
                        effective.exclusion_keys.push(exclusion_key.clone());
                    }
                }
            }
        }
    }
    if effective
        .worlds
        .iter()
        .any(|world| effective.activated.contains(world))
    {
        return Err(case_error(
            module_span(module),
            "a World cannot appear in both effective injection and activation lists",
        ));
    }
    Ok(effective)
}

/// Resolve ordered providers and reject duplicates, including imported aliases.
fn resolve_world_references(
    worlds: &ResolvedWorldSet,
    module: &LoadedModule,
    references: &[SourceReference],
) -> Result<Vec<WorldIdentity>, TypeError> {
    let bindings = worlds.bindings.get(&module.path);
    let mut resolved = Vec::new();
    for reference in references {
        let name = reference.path.join(".");
        let identity = bindings
            .and_then(|bindings| bindings.get(&name))
            .ok_or_else(|| case_error(reference.span, &format!("unresolved World '{name}'")))?;
        if resolved.contains(identity) {
            return Err(case_error(
                reference.span,
                "duplicate canonical World subscription",
            ));
        }
        resolved.push(identity.clone());
    }
    Ok(resolved)
}

/// Source extent for inherited-list conflict diagnostics.
const fn module_span(module: &LoadedModule) -> crate::token::Span {
    match module.ast {
        crate::testing::native::project::LoadedModuleAst::Test(ref ast) => ast.span,
        crate::testing::native::project::LoadedModuleAst::Application(ref ast) => ast.span,
    }
}
