extern crate alloc;

use super::{ErrorSetDeclarationInfo, TypeChecker};
use crate::{
    ast::{ErrorSetMember, Type, Visibility as AstVisibility},
    token::Span,
    type_system::{
        error_families::{stdlib_error_core_type, stdlib_error_families, stdlib_error_family},
        errors::{TypeError, Warning},
        symbol_table::SymbolType,
        type_mapping::ast_type_to_core_type,
        types::CoreType,
    },
};
use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec,
    vec::Vec,
};

impl TypeChecker {
    /// Register a raw local named error-set declaration before signatures are resolved.
    pub(super) fn register_error_set_declaration_raw(
        &mut self,
        name: String,
        members: Vec<ErrorSetMember>,
        visibility: AstVisibility,
        span: Span,
    ) -> Option<TypeError> {
        if members.is_empty() {
            return Some(TypeError::EmptyErrorSet {
                set_name: name,
                span: TypeError::span_from_span(span),
            });
        }

        let mut seen = BTreeSet::new();
        for member in &members {
            if !seen.insert(member.name.clone()) {
                return Some(TypeError::DuplicateErrorSetMember {
                    set_name: name,
                    member_name: member.name.clone(),
                    span: TypeError::span_from_span(member.span),
                });
            }
        }

        self.environment.register_type(
            name.clone(),
            CoreType::Generic {
                name: name.clone(),
                type_args: Vec::new(),
            },
        );
        self.error_sets.insert(
            name.clone(),
            ErrorSetDeclarationInfo {
                name,
                members,
                visibility,
                span,
            },
        );
        None
    }

    /// Return true when `name` resolves as a named error set.
    pub(super) fn is_error_set_name(&self, name: &str) -> bool {
        self.error_sets.contains_key(name) || stdlib_error_family(name).is_some()
    }

    /// Return the expanded leaf members for a visible named error set.
    #[must_use]
    pub fn expanded_error_set_members(&self, name: &str) -> Option<Vec<String>> {
        if !self.is_error_set_name(name) {
            return None;
        }
        self.expand_error_name_to_leaf_names(
            name,
            Span::single(crate::token::Position::start()),
            &mut Vec::new(),
        )
        .ok()
    }

    /// Return true when `name` is a known leaf error type rather than a named set.
    fn is_error_leaf_name(&self, name: &str, span: Span) -> bool {
        if name == "Error" || self.is_error_set_name(name) {
            return false;
        }
        if stdlib_error_families()
            .iter()
            .any(|family| family.members.contains(&name))
        {
            return true;
        }
        if self.environment.lookup_type(name, span).is_ok() {
            return Self::type_name_can_be_error_leaf(name);
        }
        self.symbol_table.lookup(name).is_some_and(|symbol| {
            symbol.symbol_type == SymbolType::Type && Self::type_name_can_be_error_leaf(name)
        })
    }

    /// Return whether a nominal type name can be propagated as a leaf error.
    fn type_name_can_be_error_leaf(name: &str) -> bool {
        name != "Error" && (name.ends_with("Error") || name.contains("Error."))
    }

    /// Expand one source-level error name into canonical leaf names.
    fn expand_error_name_to_leaf_names(
        &self,
        name: &str,
        span: Span,
        stack: &mut Vec<String>,
    ) -> Result<Vec<String>, TypeError> {
        if let Some(family) = stdlib_error_family(name) {
            return Ok(family
                .members
                .iter()
                .map(|member| (*member).to_owned())
                .collect());
        }

        if let Some(info) = self.error_sets.get(name) {
            if stack.iter().any(|entry| entry == name) {
                let mut cycle = stack.clone();
                cycle.push(name.to_owned());
                return Err(TypeError::ErrorSetCycle {
                    cycle: cycle.join(" -> "),
                    span: TypeError::span_from_span(info.span),
                });
            }

            stack.push(name.to_owned());
            let mut leaves = BTreeSet::new();
            for member in &info.members {
                if self.is_error_set_name(member.name.as_str()) {
                    for leaf in self.expand_error_name_to_leaf_names(
                        member.name.as_str(),
                        member.span,
                        stack,
                    )? {
                        leaves.insert(leaf);
                    }
                    continue;
                }

                if self.is_error_leaf_name(member.name.as_str(), member.span) {
                    leaves.insert(member.name.clone());
                    continue;
                }

                let member_is_visible_type =
                    self.environment.lookup_type(&member.name, span).is_ok()
                        || self
                            .symbol_table
                            .lookup(&member.name)
                            .is_some_and(|symbol| symbol.symbol_type == SymbolType::Type);
                if member_is_visible_type {
                    return Err(TypeError::NonErrorSetMember {
                        set_name: info.name.clone(),
                        member_name: member.name.clone(),
                        span: TypeError::span_from_span(member.span),
                    });
                }

                return Err(TypeError::UnknownErrorSetMember {
                    set_name: info.name.clone(),
                    member_name: member.name.clone(),
                    span: TypeError::span_from_span(member.span),
                });
            }
            stack.pop();
            return Ok(leaves.into_iter().collect());
        }

        if self.is_error_leaf_name(name, span) {
            return Ok(vec![name.to_owned()]);
        }

        Err(TypeError::UndeclaredErrorType {
            name: name.to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Resolve error names into canonical leaf [`CoreType`]s or emit a named-set diagnostic.
    pub(super) fn resolve_error_types(
        &self,
        error_names: &[String],
        span: Span,
    ) -> Result<Vec<CoreType>, TypeError> {
        let mut resolved_leaf_names = BTreeSet::new();
        for name in error_names {
            for leaf_name in self.expand_error_name_to_leaf_names(name, span, &mut Vec::new())? {
                resolved_leaf_names.insert(leaf_name);
            }
        }
        Ok(resolved_leaf_names
            .into_iter()
            .map(|name| stdlib_error_core_type(name.as_str()))
            .collect())
    }
    /// Begin collecting body-derived escaping error leaves for an active function/lambda.
    pub(super) fn begin_escaping_error_collection(&mut self) {
        self.escaping_error_stack.push(BTreeSet::new());
    }

    /// Finish collecting body-derived escaping error leaves for an active function/lambda.
    pub(super) fn end_escaping_error_collection(&mut self) -> BTreeSet<String> {
        self.escaping_error_stack.pop().unwrap_or_default()
    }

    /// Record error leaves that can escape through a checked propagation path.
    pub(super) fn record_escaping_error_types(&mut self, error_types: &[CoreType]) {
        let Some(active_errors) = self.escaping_error_stack.last_mut() else {
            return;
        };
        for error_type in error_types {
            active_errors.insert(error_type.to_string());
        }
    }

    /// Resolve one AST error annotation, expanding named sets while preserving generic error vars.
    pub(super) fn resolve_error_annotation_type_with_generics(
        &self,
        ast_type: &Type,
        generic_bindings: &[(String, CoreType)],
    ) -> Result<Vec<CoreType>, TypeError> {
        if let &Type::Basic { ref name, span } = ast_type {
            if let Some(core_type) = generic_bindings
                .iter()
                .find_map(|binding| (&binding.0 == name).then_some(&binding.1))
            {
                return Ok(vec![core_type.clone()]);
            }
            return self.resolve_error_types(&[name.clone()], span);
        }

        Ok(vec![
            ast_type_to_core_type(ast_type).map_err(TypeError::from)?,
        ])
    }

    /// Return true when a source spelling should be linted as an error set rather than a leaf.
    fn is_warning_error_set_name(&self, name: &str) -> bool {
        if self.error_sets.contains_key(name) {
            return true;
        }
        stdlib_error_family(name).is_some_and(|family| {
            !(family.members.len() == 1 && family.members.first() == Some(&name))
        })
    }

    /// Return source-name expansion for a declaration warning, if available.
    fn warning_error_name_expansion(&self, name: &str, span: Span) -> Option<BTreeSet<String>> {
        self.expand_error_name_to_leaf_names(name, span, &mut Vec::new())
            .ok()
            .map(|members| members.into_iter().collect())
    }

    /// Find the preferred visible named set with exactly `members` as expansion.
    fn preferred_exact_error_set_name_for_members(
        &self,
        members: &BTreeSet<String>,
        span: Span,
    ) -> Option<String> {
        if members.len() <= 1 {
            return None;
        }

        let local_match = self
            .error_sets
            .values()
            .filter_map(|info| {
                let expanded = self.warning_error_name_expansion(info.name.as_str(), span)?;
                (expanded == *members).then(|| info.name.clone())
            })
            .min();
        if local_match.is_some() {
            return local_match;
        }

        stdlib_error_families()
            .iter()
            .filter(|family| {
                family.warning_eligible
                    && family.members.len() == members.len()
                    && family
                        .members
                        .iter()
                        .all(|member| members.contains(*member))
            })
            .min_by_key(|family| (family.specificity_rank, family.members.len(), family.name))
            .map(|family| family.name.to_owned())
    }

    /// Emit named-error-set lint warnings for a source `errors` clause.
    #[expect(
        clippy::too_many_lines,
        reason = "error-clause linting coordinates several related warning shapes"
    )]
    pub(super) fn warn_for_error_clause(
        &mut self,
        source_error_names: &[String],
        expanded_error_types: &[CoreType],
        escaping_error_names: Option<&BTreeSet<String>>,
        span: Span,
    ) {
        let source_sets = source_error_names
            .iter()
            .filter_map(|name| {
                if !self.is_warning_error_set_name(name) {
                    return None;
                }
                self.warning_error_name_expansion(name, span)
                    .map(|members| (name.clone(), members))
            })
            .collect::<Vec<_>>();
        let source_leaves = source_error_names
            .iter()
            .filter(|name| !self.is_warning_error_set_name(name))
            .cloned()
            .collect::<Vec<_>>();

        let expanded_names = expanded_error_types
            .iter()
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>();

        if source_sets.is_empty() && source_error_names.len() > 1 {
            if let Some(set_name) =
                self.preferred_exact_error_set_name_for_members(&expanded_names, span)
            {
                let mut seen_source_leaves = BTreeSet::new();
                let replaceable_errors = source_leaves
                    .iter()
                    .filter(|name| seen_source_leaves.insert((*name).clone()))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                self.push_warning(Warning::ReplaceableErrorList {
                    family_name: set_name,
                    replaceable_errors,
                    span: TypeError::span_from_span(span),
                    suppression_annotation: None,
                });
            }
        }

        for leaf in &source_leaves {
            for &(ref set_name, ref set_members) in &source_sets {
                if set_members.contains(leaf) {
                    self.push_warning(Warning::ErrorSetRedundantMember {
                        redundant_member: leaf.clone(),
                        covering_set: set_name.clone(),
                        span: TypeError::span_from_span(span),
                        suppression_annotation: None,
                    });
                    break;
                }
            }
        }

        for (left_index, &(ref left_name, ref left_members)) in source_sets.iter().enumerate() {
            for &(ref right_name, ref right_members) in
                source_sets.iter().skip(left_index.saturating_add(1))
            {
                if left_members.is_subset(right_members) {
                    self.push_warning(Warning::ErrorSetOverlap {
                        narrower_set: left_name.clone(),
                        broader_set: right_name.clone(),
                        span: TypeError::span_from_span(span),
                        suppression_annotation: None,
                    });
                } else if right_members.is_subset(left_members) {
                    self.push_warning(Warning::ErrorSetOverlap {
                        narrower_set: right_name.clone(),
                        broader_set: left_name.clone(),
                        span: TypeError::span_from_span(span),
                        suppression_annotation: None,
                    });
                }
            }
        }

        let Some(escaping_names) = escaping_error_names else {
            return;
        };
        if escaping_names.is_empty() {
            return;
        }

        for (set_name, set_members) in source_sets {
            if escaping_names.is_superset(&set_members) || escaping_names.is_disjoint(&set_members)
            {
                continue;
            }
            let unused = set_members
                .difference(escaping_names)
                .cloned()
                .collect::<Vec<_>>();
            if unused.is_empty() {
                continue;
            }
            let suggestion = self
                .preferred_exact_error_set_name_for_members(escaping_names, span)
                .unwrap_or_else(|| {
                    escaping_names
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                });
            self.push_warning(Warning::ErrorSetUnusedMembers {
                set_name,
                unused_members: unused.join(", "),
                suggested_errors: suggestion,
                span: TypeError::span_from_span(span),
                suppression_annotation: None,
            });
        }
    }
}
