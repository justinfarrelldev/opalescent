//! Private transactional affine aggregate metadata shared by checker and codegen.

extern crate alloc;

use crate::type_system::types::CoreType;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// One member obligation controlled by a transactional aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineAggregateSlotSpec {
    /// Declared slot or variant member name.
    pub name: String,
    /// Nominal member resource type stored in this slot.
    pub type_name: String,
    /// Whether this slot owns cleanup authority when initialized.
    pub owns_obligation: bool,
}

impl AffineAggregateSlotSpec {
    /// Build a cleanup-owning slot specification.
    #[must_use]
    pub fn owning(name: &str, type_name: &str) -> Self {
        Self {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            owns_obligation: true,
        }
    }

    /// Build a value-only slot specification.
    #[must_use]
    pub fn value(name: &str, type_name: &str) -> Self {
        Self {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            owns_obligation: false,
        }
    }
}

/// One compiler-authorized same-owner layout transition helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineAggregateTransitionSpec {
    /// Infallible helper that commits this transition.
    pub operation: String,
    /// Complete old-layout obligation keys.
    pub old_layout: Vec<String>,
    /// Complete new-layout obligation keys.
    pub new_layout: Vec<String>,
}

impl AffineAggregateTransitionSpec {
    /// Build a transition specification.
    #[must_use]
    pub fn new(operation: &str, old_layout: &[&str], new_layout: &[&str]) -> Self {
        Self {
            operation: operation.to_owned(),
            old_layout: old_layout.iter().map(|slot| (*slot).to_owned()).collect(),
            new_layout: new_layout.iter().map(|slot| (*slot).to_owned()).collect(),
        }
    }
}

/// A fallible operation whose successful retarget must be followed by one commit helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineAggregateRetargetRule {
    /// Fallible retarget operation that changes a member host relationship.
    pub operation: String,
    /// Required immediately-following infallible commit operation.
    pub commit_operation: String,
}

impl AffineAggregateRetargetRule {
    /// Build a retarget/commit pair.
    #[must_use]
    pub fn new(operation: &str, commit_operation: &str) -> Self {
        Self {
            operation: operation.to_owned(),
            commit_operation: commit_operation.to_owned(),
        }
    }
}

/// Opt-in transactional aggregate declaration metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineAggregateSpec {
    /// Nominal aggregate owner type.
    pub type_name: String,
    /// Named constructor that may form the aggregate owner.
    pub constructor: String,
    /// Sole cleanup operation for the sealed aggregate owner.
    pub cleanup_operation: String,
    /// Member slots that can own provisional obligations.
    pub slots: Vec<AffineAggregateSlotSpec>,
    /// Deterministic sealed cleanup order for owning slots.
    pub cleanup_order: Vec<String>,
    /// Declared same-owner layout transition helpers.
    pub transitions: Vec<AffineAggregateTransitionSpec>,
    /// Retarget calls whose success requires immediate commit.
    pub retarget_rules: Vec<AffineAggregateRetargetRule>,
    /// Private helper name prefixes that may perform declared transitions.
    pub transition_prefixes: Vec<String>,
}

impl AffineAggregateSpec {
    /// Return whether `slot_name` is a declared member slot.
    #[must_use]
    pub fn has_slot(&self, slot_name: &str) -> bool {
        self.slots.iter().any(|slot| slot.name == slot_name)
    }

    /// Return the nominal resource type for a member slot.
    #[must_use]
    pub fn slot_type(&self, slot_name: &str) -> Option<&str> {
        self.slots
            .iter()
            .find(|slot| slot.name == slot_name)
            .map(|slot| slot.type_name.as_str())
    }

    /// Return whether `operation` is a declared transition helper.
    #[must_use]
    pub fn has_transition_operation(&self, operation: &str) -> bool {
        self.transitions
            .iter()
            .any(|transition| transition.operation == operation)
    }

    /// Find a retarget rule by fallible operation name.
    #[must_use]
    pub fn retarget_rule(&self, operation: &str) -> Option<&AffineAggregateRetargetRule> {
        self.retarget_rules
            .iter()
            .find(|rule| rule.operation == operation)
    }

    /// Return whether a helper name is reserved for aggregate transitions.
    #[must_use]
    pub fn helper_name_is_reserved_transition(&self, operation: &str) -> bool {
        self.transition_prefixes
            .iter()
            .any(|prefix| operation.starts_with(prefix))
            && (operation.contains("commit") || operation.contains("swap"))
    }
}

/// Build the private `EditorChordRuntime` aggregate specification.
#[must_use]
pub fn editor_chord_runtime_spec() -> AffineAggregateSpec {
    AffineAggregateSpec {
        type_name: "EditorChordRuntime".to_owned(),
        constructor: "editor_chord_runtime_new".to_owned(),
        cleanup_operation: "editor_chord_runtime_close".to_owned(),
        slots: vec![
            AffineAggregateSlotSpec::value("binding_map", "EditorBindingMap"),
            AffineAggregateSlotSpec::owning("router", "TerminalChordRouter"),
            AffineAggregateSlotSpec::owning("prefix_timer", "MonotonicTimer"),
            AffineAggregateSlotSpec::owning("active_registration", "SystemOwnedWaitRegistration"),
            AffineAggregateSlotSpec::owning("spare_timer", "MonotonicTimer"),
            AffineAggregateSlotSpec::owning("spare_registration", "SystemOwnedWaitRegistration"),
            AffineAggregateSlotSpec::value("timer_recovery_slot", "EditorChordTimerRecoverySlot"),
            AffineAggregateSlotSpec::value("capacity_recovery", "EditorChordCapacityRecovery"),
        ],
        cleanup_order: vec![
            "router".to_owned(),
            "active_registration".to_owned(),
            "spare_registration".to_owned(),
            "spare_timer".to_owned(),
            "prefix_timer".to_owned(),
        ],
        transitions: vec![
            AffineAggregateTransitionSpec::new(
                "editor_chord_timer_recovery_slot_swap_pairs_after_exhaustion",
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
            ),
            AffineAggregateTransitionSpec::new(
                "editor_chord_timer_recovery_slot_prepare_candidate_sync",
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
            ),
            AffineAggregateTransitionSpec::new(
                "editor_chord_timer_recovery_slot_commit_retargeted_spare",
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
                &[
                    "prefix_timer",
                    "active_registration",
                    "spare_timer",
                    "spare_registration",
                ],
            ),
        ],
        retarget_rules: vec![AffineAggregateRetargetRule::new(
            "system_owned_wait_registration_retarget",
            "editor_chord_timer_recovery_slot_commit_retargeted_spare",
        )],
        transition_prefixes: vec!["editor_chord_timer_recovery_slot_".to_owned()],
    }
}

/// Build the focused compiler test aggregate specification.
#[cfg(test)]
#[must_use]
pub fn terminal_aggregate_fixture_spec() -> AffineAggregateSpec {
    AffineAggregateSpec {
        type_name: "TerminalAggregateFixture".to_owned(),
        constructor: "terminal_aggregate_fixture_new".to_owned(),
        cleanup_operation: "terminal_aggregate_fixture_close".to_owned(),
        slots: vec![
            AffineAggregateSlotSpec::owning("first", "TerminalAggregateMember"),
            AffineAggregateSlotSpec::owning("second", "TerminalAggregateMember"),
        ],
        cleanup_order: vec!["second".to_owned(), "first".to_owned()],
        transitions: vec![AffineAggregateTransitionSpec::new(
            "terminal_aggregate_commit_candidate",
            &["first", "second"],
            &["first", "second"],
        )],
        retarget_rules: vec![AffineAggregateRetargetRule::new(
            "terminal_aggregate_retarget_member",
            "terminal_aggregate_commit_candidate",
        )],
        transition_prefixes: vec!["terminal_aggregate_".to_owned()],
    }
}

/// Return the cleanup operation for a compiler-known affine resource.
#[must_use]
pub fn cleanup_operation_for_resource(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "SystemWaitSet" => Some("system_wait_set_drop"),
        "SystemOwnedWaitRegistration" => Some("system_owned_wait_registration_drop"),
        "ProcessControlSource" => Some("process_control_source_drop"),
        "MonotonicTimer" => Some("monotonic_timer_drop"),
        "CancellationSource" => Some("cancellation_source_drop"),
        "TerminalChordRouter" => Some("terminal_chord_router_drop"),
        "TerminalSession" => Some("terminal_session_close_sync"),
        "EditorChordRuntime" => Some("editor_chord_runtime_close"),
        #[cfg(test)]
        "TerminalAggregateFixture" => Some("terminal_aggregate_fixture_close"),
        #[cfg(test)]
        "TerminalAggregateMember" => Some("terminal_aggregate_member_drop"),
        _ => None,
    }
}

/// Return all built-in aggregate specs enabled by the terminal proposal gate.
#[must_use]
pub fn terminal_affine_aggregate_specs() -> Vec<AffineAggregateSpec> {
    #[cfg(test)]
    {
        let mut specs = vec![editor_chord_runtime_spec()];
        specs.push(terminal_aggregate_fixture_spec());
        specs
    }
    #[cfg(not(test))]
    {
        vec![editor_chord_runtime_spec()]
    }
}

/// Validate one aggregate spec before registering it with a checker.
pub fn validate_affine_aggregate_spec(spec: &AffineAggregateSpec) -> Result<(), String> {
    if spec.type_name.is_empty() || spec.constructor.is_empty() || spec.cleanup_operation.is_empty()
    {
        return Err("transactional affine aggregate declaration is incomplete".to_owned());
    }

    let mut slot_names = BTreeSet::new();
    let mut owning_slots = BTreeMap::new();
    for slot in &spec.slots {
        if !slot_names.insert(slot.name.clone()) {
            return Err(format!(
                "duplicate obligation slot '{}' in transactional affine aggregate '{}'",
                slot.name, spec.type_name
            ));
        }
        if slot.owns_obligation {
            owning_slots.insert(slot.name.clone(), slot.type_name.clone());
            if cleanup_operation_for_resource(slot.type_name.as_str()).is_none() {
                return Err(format!(
                    "missing cleanup registration for aggregate member '{}' of type '{}'",
                    slot.name, slot.type_name
                ));
            }
        }
    }

    validate_cleanup_order(spec, &owning_slots)?;
    validate_transitions(spec, &owning_slots)?;
    Ok(())
}

/// Validate exact sealed cleanup coverage.
fn validate_cleanup_order(
    spec: &AffineAggregateSpec,
    owning_slots: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut seen_cleanup = BTreeSet::new();
    for slot_name in &spec.cleanup_order {
        if !seen_cleanup.insert(slot_name.clone()) {
            return Err(format!(
                "duplicate obligation cleanup '{}' in transactional affine aggregate '{}'",
                slot_name, spec.type_name
            ));
        }
        if !owning_slots.contains_key(slot_name) {
            return Err(format!(
                "cleanup order for transactional affine aggregate '{}' names non-owning slot '{}'",
                spec.type_name, slot_name
            ));
        }
    }
    for slot_name in owning_slots.keys() {
        if !seen_cleanup.contains(slot_name) {
            return Err(format!(
                "missing cleanup for aggregate member '{}' in transactional affine aggregate '{}'",
                slot_name, spec.type_name
            ));
        }
    }
    Ok(())
}

/// Validate transition layouts use each obligation at most once and only declared slots.
fn validate_transitions(
    spec: &AffineAggregateSpec,
    owning_slots: &BTreeMap<String, String>,
) -> Result<(), String> {
    for transition in &spec.transitions {
        validate_transition_layout(
            spec,
            transition.operation.as_str(),
            &transition.old_layout,
            owning_slots,
        )?;
        validate_transition_layout(
            spec,
            transition.operation.as_str(),
            &transition.new_layout,
            owning_slots,
        )?;
    }
    for rule in &spec.retarget_rules {
        if !spec.has_transition_operation(rule.commit_operation.as_str()) {
            return Err(format!(
                "retarget operation '{}' on transactional affine aggregate '{}' names undeclared commit '{}'",
                rule.operation, spec.type_name, rule.commit_operation
            ));
        }
    }
    Ok(())
}

/// Validate one transition side.
fn validate_transition_layout(
    spec: &AffineAggregateSpec,
    operation: &str,
    layout: &[String],
    owning_slots: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for slot_name in layout {
        if !seen.insert(slot_name.clone()) {
            return Err(format!(
                "duplicate obligation '{}' in declared aggregate transition '{}' for '{}'",
                slot_name, operation, spec.type_name
            ));
        }
        if !owning_slots.contains_key(slot_name) {
            return Err(format!(
                "declared aggregate transition '{}' for '{}' references unknown obligation '{}'",
                operation, spec.type_name, slot_name
            ));
        }
    }
    Ok(())
}

/// Build a nominal core type for synthetic aggregate tests and proposal gates.
#[must_use]
pub fn nominal_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
