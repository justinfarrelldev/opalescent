//! Stable case expansion and resolved scoped metadata, before artifact validation.
//!
//! Resolution is not proof that callbacks type-check. Only the native compiler
//! may promote this source-backed suite into an executable artifact.
extern crate alloc;
mod metadata;
mod world_needs;
use super::identity::{CaseIdentity, FunctionIdentity, WorldIdentity};
use super::project::{LoadedModule, LoadedModuleAst, NativeTestProject};
use super::world_resolution::ResolvedWorldSet;
use crate::ast::testing::{
    ExampleDeclaration, HookDeclaration, ManagedCallback, TestItem, TestKind,
};
use crate::ast::{Documentation, Type};
use crate::token::Span;
use crate::type_system::errors::TypeError;
use alloc::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// A source hook scope, entered independently during every attempt.
#[derive(Debug, Clone)]
pub struct HookScope {
    /// Empty for file hooks; otherwise complete effective scenario ancestry.
    pub scenarios: Vec<String>,
    /// Source-ordered hooks; after-hooks execute in reverse order.
    pub hooks: Vec<HookDeclaration>,
}

/// Why one case requires a provider. Multiple reasons do not create instances.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum WorldUseOrigin {
    /// Explicit ordered body injection.
    Injected,
    /// Explicit activation-only requirement.
    Activated,
    /// An explicit nominal parameter of a scoped hook.
    Hook {
        /// Hook scope ancestry.
        scenarios: Vec<String>,
        /// Source declaration position within this scope's hook list.
        index: usize,
    },
    /// Explicit dependency of another initialized provider.
    Dependency {
        /// Dependent provider, retaining diamond graph provenance.
        dependent: WorldIdentity,
    },
}

/// Shared source body and fully resolved association/lifecycle metadata.
#[derive(Debug, Clone)]
pub struct ResolvedTestBody {
    /// Defining filesystem module, used for compiler diagnostics and imports.
    pub module_path: PathBuf,
    /// One typed source callback shared by all examples, not compiled per case.
    pub callback: ManagedCallback,
    /// Declared example schema, pending ordinary type/schema validation.
    pub example_type: Option<Type>,
    /// Ordered nominal body injections.
    pub injected_worlds: Vec<WorldIdentity>,
    /// Effect-only provider requirements, with no artificial body parameters.
    pub activated_worlds: Vec<WorldIdentity>,
    /// Canonical function associations, deduplicated within the body.
    pub subjects: Vec<FunctionIdentity>,
    /// Effective inherited or overridden category.
    pub kind: TestKind,
    /// Complete scheduler-only key set for this body.
    pub exclusion_keys: Vec<String>,
    /// Outermost-to-innermost hook scopes, preserving source order.
    pub hook_scopes: Vec<HookScope>,
    /// Deterministic per-attempt initialization order, including hook needs.
    pub initialization_order: Vec<WorldIdentity>,
    /// Distinct direct/hook/activation/transitive request origins.
    pub world_origins: BTreeMap<WorldIdentity, BTreeSet<WorldUseOrigin>>,
    /// Scenario and test documentation, outermost first.
    pub documentation: Vec<Documentation>,
}

/// An independently selectable, schedulable case; not an execution attempt.
#[derive(Debug, Clone)]
pub struct ResolvedCase {
    /// Complete structured identity, independent of example position.
    pub identity: CaseIdentity,
    /// Shared body key; its example component is always absent.
    pub body_identity: CaseIdentity,
    /// Fresh typed data constructed for each attempt after schema validation.
    pub example: Option<ExampleDeclaration>,
}

/// Source-backed case expansion. It must still pass native artifact validation.
#[derive(Debug, Clone, Default)]
pub struct ResolvedSuite {
    /// One body per test declaration, independent of its number of examples.
    pub bodies: BTreeMap<CaseIdentity, ResolvedTestBody>,
    /// One independent case per unparameterized test or named example.
    pub cases: Vec<ResolvedCase>,
}

/// Effective replace-not-append metadata inherited through scenario scopes.
#[derive(Debug, Clone)]
struct EffectiveMetadata {
    /// Canonical function associations.
    subjects: Vec<FunctionIdentity>,
    /// Ordered body providers.
    worlds: Vec<WorldIdentity>,
    /// Effect-only requirements.
    activated: Vec<WorldIdentity>,
    /// Category defaults to unit.
    kind: TestKind,
    /// Inherited scheduler requirements.
    exclusion_keys: Vec<String>,
}

impl Default for EffectiveMetadata {
    fn default() -> Self {
        Self {
            subjects: Vec::new(),
            worlds: Vec::new(),
            activated: Vec::new(),
            kind: TestKind::Unit,
            exclusion_keys: Vec::new(),
        }
    }
}

/// Resolve independent cases in selected test roots, without invoking callbacks.
///
/// # Errors
/// Rejects ambiguous names, invalid subjects/subscriptions and conflicting metadata.
pub fn resolve_cases(
    project: &NativeTestProject,
    worlds: &ResolvedWorldSet,
) -> Result<ResolvedSuite, TypeError> {
    let mut suite = ResolvedSuite::default();
    let mut selected_modules = BTreeSet::new();
    for path in &project.test_files {
        if !selected_modules.insert(path) {
            continue;
        }
        let module = &project.modules[path];
        let LoadedModuleAst::Test(ref ast) = module.ast else {
            continue;
        };
        if ast
            .items
            .iter()
            .any(|item| matches!(*item, TestItem::Metadata(_)))
        {
            return Err(case_error(
                ast.span,
                "test metadata belongs in a scenario or test",
            ));
        }
        let scope = hook_scope(&ast.items, &[]);
        let mut resolver = CaseResolver {
            project,
            worlds,
            module,
            suite: &mut suite,
        };
        resolver.resolve_scope(
            &ast.items,
            &[],
            &EffectiveMetadata::default(),
            &[scope],
            &[],
        )?;
    }
    Ok(suite)
}

/// Source context for recursive scenario expansion, never a scheduling group.
struct CaseResolver<'project, 'suite> {
    /// Read-only loaded source graph.
    project: &'project NativeTestProject,
    /// Resolved nominal provider graph.
    worlds: &'project ResolvedWorldSet,
    /// Current source module.
    module: &'project LoadedModule,
    /// Shared body registry and independent case collection.
    suite: &'suite mut ResolvedSuite,
}

impl CaseResolver<'_, '_> {
    /// Expand one scope with inherited, already resolved metadata.
    fn resolve_scope(
        &mut self,
        items: &[TestItem],
        ancestry: &[String],
        inherited: &EffectiveMetadata,
        hook_scopes: &[HookScope],
        documentation: &[Documentation],
    ) -> Result<(), TypeError> {
        let mut sibling_names = BTreeSet::new();
        for item in items {
            match *item {
                TestItem::Scenario(ref scenario) => {
                    let metadata = metadata::apply(
                        self.project,
                        self.module,
                        self.worlds,
                        inherited,
                        scenario
                            .items
                            .iter()
                            .filter_map(|scenario_item| match *scenario_item {
                                TestItem::Metadata(ref clause) => Some(clause),
                                _ => None,
                            }),
                    )?;
                    let name = scenario.title.clone().or_else(|| (metadata.subjects.len() == 1).then(|| metadata.subjects[0].0.name.clone())).ok_or_else(|| case_error(scenario.span, "scenario requires an explicit or unambiguous subject-derived title"))?;
                    check_sibling_name(&mut sibling_names, &name, scenario.span)?;
                    let mut ancestry = ancestry.to_vec();
                    ancestry.push(name);
                    let mut scopes = hook_scopes.to_vec();
                    scopes.push(hook_scope(&scenario.items, &ancestry));
                    let mut docs = documentation.to_vec();
                    docs.extend(scenario.documentation.clone());
                    self.resolve_scope(&scenario.items, &ancestry, &metadata, &scopes, &docs)?;
                }
                TestItem::Test(ref test) => {
                    check_sibling_name(&mut sibling_names, &test.title, test.span)?;
                    let metadata = metadata::apply(
                        self.project,
                        self.module,
                        self.worlds,
                        inherited,
                        test.metadata.iter(),
                    )?;
                    let identity = CaseIdentity {
                        module: self.module.identity.clone(),
                        scenarios: ancestry.to_vec(),
                        test: test.title.clone(),
                        example: None,
                    };
                    let mut docs = documentation.to_vec();
                    docs.extend(test.documentation.clone());
                    let (initialization_order, world_origins) = world_needs::resolve(
                        self.worlds,
                        &self.module.path,
                        &metadata,
                        hook_scopes,
                        test.span,
                    )?;
                    self.suite.bodies.insert(
                        identity.clone(),
                        ResolvedTestBody {
                            module_path: self.module.path.clone(),
                            callback: test.callback.clone(),
                            example_type: test
                                .examples
                                .as_ref()
                                .map(|group| group.record_type.clone()),
                            injected_worlds: metadata.worlds,
                            activated_worlds: metadata.activated,
                            subjects: metadata.subjects,
                            kind: metadata.kind,
                            exclusion_keys: metadata.exclusion_keys,
                            hook_scopes: hook_scopes.to_vec(),
                            initialization_order,
                            world_origins,
                            documentation: docs,
                        },
                    );
                    if let Some(ref group) = test.examples {
                        let mut names = BTreeSet::new();
                        for example in &group.examples {
                            check_sibling_name(&mut names, &example.title, example.span)?;
                            let mut case_identity = identity.clone();
                            case_identity.example = Some(example.title.clone());
                            self.suite.cases.push(ResolvedCase {
                                identity: case_identity,
                                body_identity: identity.clone(),
                                example: Some(example.clone()),
                            });
                        }
                    } else {
                        self.suite.cases.push(ResolvedCase {
                            identity: identity.clone(),
                            body_identity: identity,
                            example: None,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Source-ordered hooks local to one explicit lifecycle scope.
fn hook_scope(items: &[TestItem], ancestry: &[String]) -> HookScope {
    HookScope {
        scenarios: ancestry.to_vec(),
        hooks: items
            .iter()
            .filter_map(|item| match *item {
                TestItem::Hook(ref hook) => Some(hook.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// Duplicate effective names are errors, never silently suffixed with ordinals.
fn check_sibling_name(
    names: &mut BTreeSet<String>,
    name: &str,
    span: Span,
) -> Result<(), TypeError> {
    if names.insert(name.to_owned()) {
        Ok(())
    } else {
        Err(case_error(
            span,
            &format!("duplicate effective declaration name '{name}'"),
        ))
    }
}

/// A static case-resolution diagnostic at the original declaration location.
fn case_error(span: Span, message: &str) -> TypeError {
    TypeError::InvalidOperation {
        operation: message.to_owned(),
        type_name: "test declaration".to_owned(),
        span: TypeError::span_from_span(span),
    }
}
