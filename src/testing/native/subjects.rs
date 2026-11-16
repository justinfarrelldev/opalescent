//! Resolve subject metadata to genuine callable declaration identities.
extern crate alloc;
use super::identity::{DeclarationIdentity, FunctionIdentity};
use super::project::{LoadedModule, LoadedModuleAst, NativeTestProject};
use crate::ast::testing::{SourceReference, TestItem};
use crate::ast::{Decl, Expr, ImportItem, Visibility};
use crate::module_loader::resolve_import_path;
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use std::path::Path;

/// Resolve canonical declaration identity, never a call, alias or runtime address.
///
/// # Errors
/// Rejects missing, ambiguous, private, non-callable and unsupported references.
pub fn resolve_subject(
    project: &NativeTestProject,
    module_path: &Path,
    reference: &SourceReference,
) -> Result<FunctionIdentity, TypeError> {
    let module = project
        .modules
        .get(module_path)
        .ok_or_else(|| subject_error(reference.span, "subject module is not loaded"))?;
    let Some(first) = reference.path.first() else {
        return Err(subject_error(reference.span, "empty function reference"));
    };
    let declarations = application_declarations(module);
    let mut matches = Vec::new();
    for declaration in declarations {
        if reference.path.len() == 1 && declaration_name(declaration) == Some(first.as_str()) {
            matches.push(callable_identity(
                module,
                declaration,
                false,
                reference.span,
            ));
        }
        if let Decl::Import {
            ref items,
            ref source,
            ..
        } = *declaration
        {
            for item in items {
                let ImportItem::Named {
                    ref name,
                    ref alias,
                    ..
                } = *item
                else {
                    continue;
                };
                let local_name = alias.as_deref().unwrap_or(name);
                if local_name != first {
                    continue;
                }
                let imported_name = if reference.path.len() == 1 {
                    name.as_str()
                } else if reference.path.len() == 2 && name == source {
                    reference.path[1].as_str()
                } else {
                    return Err(subject_error(
                        reference.span,
                        "unsupported qualified function reference",
                    ));
                };
                matches.push(resolve_imported_callable(
                    project,
                    &module.path,
                    source,
                    imported_name,
                    reference.span,
                ));
            }
        }
    }
    if matches.len() > 1 {
        return Err(subject_error(
            reference.span,
            "ambiguous function subject reference",
        ));
    }
    if let Some(result) = matches.pop() {
        return result;
    }
    // Preserve the existing compiler's implicit standard builtins, but never
    // invent a global function merely because its name exists in a module.
    if reference.path.len() == 1
        && TypeChecker::new()
            .symbol_table()
            .lookup(first)
            .is_some_and(|symbol| matches!(symbol.core_type, CoreType::Function { .. }))
    {
        return resolve_standard_callable("standard", first, reference.span);
    }
    Err(subject_error(
        reference.span,
        &format!("unresolved function subject '{}'", reference.path.join(".")),
    ))
}

/// Ordinary source declarations in either grammar mode.
fn application_declarations(module: &LoadedModule) -> Vec<&Decl> {
    match module.ast {
        LoadedModuleAst::Application(ref program) => program.declarations.iter().collect(),
        LoadedModuleAst::Test(ref module_ast) => module_ast
            .items
            .iter()
            .filter_map(|item| match *item {
                TestItem::Application(ref declaration) => Some(declaration.as_ref()),
                _ => None,
            })
            .collect(),
    }
}

/// Declaration binding name; imports are resolved separately.
fn declaration_name(declaration: &Decl) -> Option<&str> {
    match *declaration {
        Decl::Function { ref name, .. }
        | Decl::Type { ref name, .. }
        | Decl::ErrorSet { ref name, .. } => Some(name.as_str()),
        Decl::Let { ref binding, .. } => Some(binding.name.as_str()),
        Decl::Import { .. } | Decl::Namespace { .. } | Decl::Comment { .. } => None,
    }
}

/// Validate a genuine source callable instead of accepting arbitrary function values.
fn callable_identity(
    module: &LoadedModule,
    declaration: &Decl,
    require_public: bool,
    span: Span,
) -> Result<FunctionIdentity, TypeError> {
    let (name, public) = match *declaration {
        Decl::Function {
            ref name,
            ref visibility,
            is_entry,
            ..
        } => (name.as_str(), *visibility == Visibility::Public || is_entry),
        Decl::Let {
            ref binding,
            initializer: Expr::Lambda { .. },
            ref visibility,
            ..
        } => (binding.name.as_str(), *visibility == Visibility::Public),
        _ => {
            return Err(subject_error(
                span,
                "subject must reference a genuine callable declaration",
            ));
        }
    };
    if require_public && !public {
        return Err(subject_error(
            span,
            "function subject is private to its defining module",
        ));
    }
    Ok(FunctionIdentity(module.declaration_identity(name)))
}

/// Resolve one genuine callable through the exact source import and alias.
fn resolve_imported_callable(
    project: &NativeTestProject,
    importing_module: &Path,
    source: &str,
    name: &str,
    span: Span,
) -> Result<FunctionIdentity, TypeError> {
    let path = resolve_import_path(importing_module, source)?;
    if path.starts_with("__stdlib__") {
        return resolve_standard_callable(source, name, span);
    }
    let module = project
        .modules
        .get(&path)
        .ok_or_else(|| subject_error(span, "imported subject module is not loaded"))?;
    let declarations: Vec<_> = application_declarations(module)
        .into_iter()
        .filter(|declaration| declaration_name(declaration) == Some(name))
        .collect();
    if declarations.len() != 1 {
        return Err(subject_error(
            span,
            &format!("missing or ambiguous imported function '{name}'"),
        ));
    }
    callable_identity(module, declarations[0], true, span)
}

/// Use existing registered standard module declarations, signatures and privacy.
fn resolve_standard_callable(
    module: &str,
    name: &str,
    span: Span,
) -> Result<FunctionIdentity, TypeError> {
    let interface = TypeChecker::new()
        .module_interface(module)
        .ok_or_else(|| subject_error(span, "unknown standard subject module"))?;
    let symbol = interface
        .exports
        .get(name)
        .ok_or_else(|| subject_error(span, "missing or private standard function subject"))?;
    if !matches!(symbol.core_type, CoreType::Function { .. }) {
        return Err(subject_error(
            span,
            "standard subject is not a callable declaration",
        ));
    }
    Ok(FunctionIdentity(DeclarationIdentity {
        module: module.to_owned(),
        name: symbol.name.clone(),
    }))
}

/// Localized metadata diagnostic, not a claim about application behavior.
fn subject_error(span: Span, message: &str) -> TypeError {
    TypeError::InvalidOperation {
        operation: message.to_owned(),
        type_name: "test subject".to_owned(),
        span: TypeError::span_from_span(span),
    }
}
