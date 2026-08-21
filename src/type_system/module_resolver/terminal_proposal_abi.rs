#![expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed AST proposal metadata keeps ABI validation concise"
)]
#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "private ABI validator helpers are documented by the module-level summary"
    )
)]

//! ABI/history validation for terminal proposal declaration metadata.

extern crate alloc;

mod active_variant_ids;

use super::terminal_proposal_modules::{
    TERMINAL_CHORDS_MODULE_PATH, TERMINAL_CHORDS_TYPES_PATH, TERMINAL_TESTING_MODULE_PATH,
    TERMINAL_TESTING_TYPES_PATH, TERMINAL_TYPES_MODULE_PATH, TYPED_EVENT_SESSION_TYPES_PATH,
};
use super::{ModuleAvailability, ModuleInterface, ModuleTypeDeclaration};
use crate::ast::{DeclarationAnnotation, TypeDef};
use active_variant_ids::{
    ACTIVE_CHORD_VARIANT_IDS, ACTIVE_SELECTED_VARIANT_IDS, ActiveVariantIdTable,
};
use alloc::collections::BTreeMap;
use alloc::{format, string::String};
use core::fmt;

const SELECTED_ACTIVE_TYPE_ID_COUNT: usize = 87;
const CHORD_ACTIVE_TYPE_ID_COUNT: usize = 23;

const SELECTED_ACTIVE_TYPE_ID_RANGES: &[TypeIdRange] = &[
    TypeIdRange::new(0x5400_0000_0000_0010_i64, 0x5400_0000_0000_0039_i64),
    TypeIdRange::new(0x5400_0000_0000_0040_i64, 0x5400_0000_0000_004a_i64),
    TypeIdRange::new(0x5400_0000_0000_004c_i64, 0x5400_0000_0000_0059_i64),
    TypeIdRange::new(0x5400_0000_0000_0060_i64, 0x5400_0000_0000_006d_i64),
    TypeIdRange::new(0x5400_0000_0000_0070_i64, 0x5400_0000_0000_0075_i64),
];

const SELECTED_INTENTIONAL_TYPE_ID_GAPS: &[TypeIdRange] = &[
    TypeIdRange::new(0x5400_0000_0000_004b_i64, 0x5400_0000_0000_004b_i64),
    TypeIdRange::new(0x5400_0000_0000_005a_i64, 0x5400_0000_0000_005f_i64),
    TypeIdRange::new(0x5400_0000_0000_006e_i64, 0x5400_0000_0000_006f_i64),
];

const CHORD_ACTIVE_TYPE_ID_RANGES: &[TypeIdRange] = &[TypeIdRange::new(
    0x5400_0000_0000_0100_i64,
    0x5400_0000_0000_0116_i64,
)];

const RETIRED_SELECTED_VARIANT_IDS: &[HistoricalVariantId] = &[
    HistoricalVariantId::new("TerminalOperation", "AcquireOutputTerminal", 6),
    HistoricalVariantId::new("TerminalSessionState", "Opening", 1),
    HistoricalVariantId::new("TerminalSessionState", "FailedOpenRecovery", 6),
    HistoricalVariantId::new("TerminalSessionState", "FailedCloseRecovery", 7),
    HistoricalVariantId::new("TerminalSessionStateError", "SessionOpening", 1),
    HistoricalVariantId::new("TerminalSessionStateError", "SessionFailedRecovery", 6),
    HistoricalVariantId::new("TerminalSessionRestoreError", "RecoveryOwnerMismatch", 8),
];

const UNCOMMITTED_SELECTED_VARIANT_IDS: &[HistoricalVariantId] = &[HistoricalVariantId::new(
    "TerminalSessionRestoreError",
    "GenerationExhausted",
    14,
)];

/// Active terminal/chord ABI inventory counted from declaration metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalAbiInventory {
    /// Number of active selected production type IDs.
    pub(super) selected_active_production_type_ids: usize,
    /// Number of active chord production type IDs.
    pub(super) chord_active_production_type_ids: usize,
}

/// Precise ABI validation diagnostic for terminal proposal metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TerminalAbiError {
    message: String,
}

impl TerminalAbiError {
    /// Build a validation error from a fully formatted diagnostic.
    const fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for TerminalAbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Validate terminal proposal ABI metadata before resolver registration.
///
/// # Errors
/// Returns a precise diagnostic when active declaration metadata violates the
/// selected/chord ABI inventory, retired ID, intentional gap, or test-only scope
/// authority encoded by `abi-history.md`.
pub(super) fn validate_terminal_proposal_abi(
    interfaces: &[ModuleInterface],
) -> Result<TerminalAbiInventory, TerminalAbiError> {
    let selected = interface_by_path(interfaces, TERMINAL_TYPES_MODULE_PATH)?;
    let chords = interface_by_path(interfaces, TERMINAL_CHORDS_MODULE_PATH)?;
    let testing = interface_by_path(interfaces, TERMINAL_TESTING_MODULE_PATH)?;

    let selected_active_production_type_ids = validate_production_type_ids(
        selected,
        TerminalAuthority::Selected,
        TYPED_EVENT_SESSION_TYPES_PATH,
        SELECTED_ACTIVE_TYPE_ID_RANGES,
        SELECTED_INTENTIONAL_TYPE_ID_GAPS,
        SELECTED_ACTIVE_TYPE_ID_COUNT,
    )?;
    validate_explicit_variant_ids(
        selected,
        TerminalAuthority::Selected,
        ACTIVE_SELECTED_VARIANT_IDS,
        RETIRED_SELECTED_VARIANT_IDS,
        UNCOMMITTED_SELECTED_VARIANT_IDS,
    )?;

    let chord_active_production_type_ids = validate_production_type_ids(
        chords,
        TerminalAuthority::Chord,
        TERMINAL_CHORDS_TYPES_PATH,
        CHORD_ACTIVE_TYPE_ID_RANGES,
        &[],
        CHORD_ACTIVE_TYPE_ID_COUNT,
    )?;
    validate_explicit_variant_ids(
        chords,
        TerminalAuthority::Chord,
        ACTIVE_CHORD_VARIANT_IDS,
        &[],
        &[],
    )?;

    validate_test_only_scope(testing)?;

    Ok(TerminalAbiInventory {
        selected_active_production_type_ids,
        chord_active_production_type_ids,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalAuthority {
    Selected,
    Chord,
}

impl TerminalAuthority {
    const fn module_path(self) -> &'static str {
        match self {
            Self::Selected => TERMINAL_TYPES_MODULE_PATH,
            Self::Chord => TERMINAL_CHORDS_MODULE_PATH,
        }
    }

    const fn availability(self) -> ModuleAvailability {
        match self {
            Self::Selected | Self::Chord => ModuleAvailability::FuturePublicApi,
        }
    }

    const fn inventory_label(self) -> &'static str {
        match self {
            Self::Selected => "selected terminal",
            Self::Chord => "terminal chord",
        }
    }

    const fn variant_label(self) -> &'static str {
        match self {
            Self::Selected => "selected ABI variant ID",
            Self::Chord => "chord ABI variant ID",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TypeIdRange {
    start: i64,
    end: i64,
}

impl TypeIdRange {
    const fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    const fn contains(self, type_id: i64) -> bool {
        self.start <= type_id && type_id <= self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HistoricalVariantId {
    declaration: &'static str,
    variant: &'static str,
    id: i64,
}

impl HistoricalVariantId {
    const fn new(declaration: &'static str, variant: &'static str, id: i64) -> Self {
        Self {
            declaration,
            variant,
            id,
        }
    }

    fn label(self) -> String {
        format!("{}.{}={}", self.declaration, self.variant, self.id)
    }
}

fn interface_by_path<'interfaces>(
    interfaces: &'interfaces [ModuleInterface],
    module_path: &str,
) -> Result<&'interfaces ModuleInterface, TerminalAbiError> {
    interfaces
        .iter()
        .find(|interface| interface.module_path == module_path)
        .ok_or_else(|| {
            TerminalAbiError::new(format!(
                "terminal ABI validation missing module interface {module_path}"
            ))
        })
}

fn validate_production_type_ids(
    interface: &ModuleInterface,
    authority: TerminalAuthority,
    expected_source_path: &str,
    active_ranges: &[TypeIdRange],
    intentional_gaps: &[TypeIdRange],
    expected_count: usize,
) -> Result<usize, TerminalAbiError> {
    validate_interface_scope(
        interface,
        authority.module_path(),
        authority.availability(),
        expected_source_path,
    )?;

    let mut active_ids = BTreeMap::new();
    for declaration in interface.type_declarations.values() {
        let type_id = required_abi_type_id(declaration, authority)?;
        validate_abi_evolution_metadata(declaration, authority, type_id)?;
        validate_active_type_id_authority(
            declaration,
            authority,
            type_id,
            active_ranges,
            intentional_gaps,
        )?;

        if let Some(previous_name) = active_ids.get(&type_id) {
            return Err(TerminalAbiError::new(format!(
                "duplicate {} production ABI type ID {} on declarations {} and {}",
                authority.inventory_label(),
                format_type_id(type_id),
                previous_name,
                declaration.name
            )));
        }
        active_ids.insert(type_id, declaration.name.as_str());
    }

    if active_ids.len() != expected_count {
        return Err(TerminalAbiError::new(format!(
            "{} declarations must contain exactly {expected_count} active production ABI type IDs, found {}",
            authority.inventory_label(),
            active_ids.len()
        )));
    }

    validate_expected_type_id_ranges(&active_ids, authority, active_ranges)?;
    validate_intentional_type_id_gaps(&active_ids, authority, intentional_gaps)?;

    Ok(active_ids.len())
}

fn validate_interface_scope(
    interface: &ModuleInterface,
    expected_module_path: &str,
    expected_availability: ModuleAvailability,
    expected_source_path: &str,
) -> Result<(), TerminalAbiError> {
    if interface.module_path != expected_module_path {
        return Err(TerminalAbiError::new(format!(
            "terminal ABI authority expected module {expected_module_path}, found {}",
            interface.module_path
        )));
    }

    if interface.availability != expected_availability {
        return Err(TerminalAbiError::new(format!(
            "terminal ABI authority module {} has availability {:?}, expected {:?}",
            interface.module_path, interface.availability, expected_availability
        )));
    }

    for declaration in interface.type_declarations.values() {
        if declaration.source_path != expected_source_path {
            return Err(TerminalAbiError::new(format!(
                "terminal ABI declaration {} is sourced from {}, expected active authority {}",
                declaration.name, declaration.source_path, expected_source_path
            )));
        }
    }

    Ok(())
}

fn validate_active_type_id_authority(
    declaration: &ModuleTypeDeclaration,
    authority: TerminalAuthority,
    type_id: i64,
    active_ranges: &[TypeIdRange],
    intentional_gaps: &[TypeIdRange],
) -> Result<(), TerminalAbiError> {
    if type_id_in_ranges(type_id, intentional_gaps) {
        return Err(TerminalAbiError::new(format!(
            "intentional {} ABI type-ID gap {} is allocated to declaration {}",
            authority.inventory_label(),
            format_type_id(type_id),
            declaration.name
        )));
    }

    if !type_id_in_ranges(type_id, active_ranges) {
        return Err(TerminalAbiError::new(format!(
            "{} declaration {} uses ABI type ID {} outside active production authority",
            authority.inventory_label(),
            declaration.name,
            format_type_id(type_id)
        )));
    }

    Ok(())
}

fn validate_expected_type_id_ranges(
    active_ids: &BTreeMap<i64, &str>,
    authority: TerminalAuthority,
    active_ranges: &[TypeIdRange],
) -> Result<(), TerminalAbiError> {
    for range in active_ranges {
        for expected_type_id in range.start..=range.end {
            if !active_ids.contains_key(&expected_type_id) {
                return Err(TerminalAbiError::new(format!(
                    "{} active production ABI type ID {} is missing from active declarations",
                    authority.inventory_label(),
                    format_type_id(expected_type_id)
                )));
            }
        }
    }
    Ok(())
}

fn validate_intentional_type_id_gaps(
    active_ids: &BTreeMap<i64, &str>,
    authority: TerminalAuthority,
    intentional_gaps: &[TypeIdRange],
) -> Result<(), TerminalAbiError> {
    for gap in intentional_gaps {
        for gap_type_id in gap.start..=gap.end {
            if let Some(declaration_name) = active_ids.get(&gap_type_id) {
                return Err(TerminalAbiError::new(format!(
                    "intentional {} ABI type-ID gap {} is allocated to declaration {}",
                    authority.inventory_label(),
                    format_type_id(gap_type_id),
                    declaration_name
                )));
            }
        }
    }
    Ok(())
}

fn validate_abi_evolution_metadata(
    declaration: &ModuleTypeDeclaration,
    authority: TerminalAuthority,
    type_id: i64,
) -> Result<(), TerminalAbiError> {
    if has_abi_evolution(declaration) {
        return Ok(());
    }

    Err(TerminalAbiError::new(format!(
        "{} declaration {} with ABI type ID {} must carry @abi_evolution metadata",
        authority.inventory_label(),
        declaration.name,
        format_type_id(type_id)
    )))
}

fn validate_explicit_variant_ids(
    interface: &ModuleInterface,
    authority: TerminalAuthority,
    active_variant_tables: &[ActiveVariantIdTable],
    retired_variant_ids: &[HistoricalVariantId],
    uncommitted_variant_ids: &[HistoricalVariantId],
) -> Result<(), TerminalAbiError> {
    validate_active_variant_declarations_are_sums(interface, authority, active_variant_tables)?;

    for declaration in interface.type_declarations.values() {
        let TypeDef::Sum { variants, .. } = &declaration.type_def else {
            continue;
        };

        let active_variant_table =
            active_variant_table_by_declaration(active_variant_tables, &declaration.name);
        let mut active_variant_ids = BTreeMap::new();
        let mut active_variant_names = BTreeMap::new();

        for variant in variants {
            let Some(variant_id) = variant.explicit_id else {
                return Err(TerminalAbiError::new(format!(
                    "{} variant {}.{} must declare an explicit ABI variant ID",
                    authority.variant_label(),
                    declaration.name,
                    variant.name
                )));
            };

            if variant_id <= 0 {
                return Err(TerminalAbiError::new(format!(
                    "{} variant {}.{} uses nonpositive ABI variant ID {variant_id}",
                    authority.variant_label(),
                    declaration.name,
                    variant.name
                )));
            }

            if let Some(previous_variant) = active_variant_ids.get(&variant_id) {
                return Err(TerminalAbiError::new(format!(
                    "duplicate {} {}.{}={} conflicts with {}.{}={}",
                    authority.variant_label(),
                    declaration.name,
                    variant.name,
                    variant_id,
                    declaration.name,
                    previous_variant,
                    variant_id
                )));
            }

            validate_historical_variant_status(
                declaration,
                &variant.name,
                variant_id,
                retired_variant_ids,
                uncommitted_variant_ids,
            )?;

            validate_active_variant_history(
                authority,
                declaration,
                &variant.name,
                variant_id,
                active_variant_table,
            )?;

            active_variant_ids.insert(variant_id, variant.name.as_str());
            active_variant_names.insert(variant.name.as_str(), variant_id);
        }

        validate_active_variant_history_completeness(
            authority,
            declaration,
            active_variant_table,
            &active_variant_names,
        )?;
    }

    Ok(())
}

fn validate_active_variant_declarations_are_sums(
    interface: &ModuleInterface,
    authority: TerminalAuthority,
    active_variant_tables: &[ActiveVariantIdTable],
) -> Result<(), TerminalAbiError> {
    for active_variant_table in active_variant_tables {
        let Some(declaration) = interface
            .type_declarations
            .get(active_variant_table.declaration)
        else {
            return Err(TerminalAbiError::new(format!(
                "{} declaration {} is missing, but abi-history.md records active variants for it",
                authority.inventory_label(),
                active_variant_table.declaration
            )));
        };

        if !matches!(&declaration.type_def, TypeDef::Sum { .. }) {
            return Err(TerminalAbiError::new(format!(
                "{} declaration {} must remain a sum type because abi-history.md records active variants for it",
                authority.inventory_label(),
                active_variant_table.declaration
            )));
        }
    }

    Ok(())
}

fn validate_active_variant_history(
    authority: TerminalAuthority,
    declaration: &ModuleTypeDeclaration,
    active_variant_name: &str,
    active_variant_id: i64,
    active_variant_table: Option<ActiveVariantIdTable>,
) -> Result<(), TerminalAbiError> {
    let Some(active_variant_table) = active_variant_table else {
        return Ok(());
    };

    let Some(expected_variant_id) = active_variant_table.expected_variant_id(active_variant_name)
    else {
        return Err(TerminalAbiError::new(format!(
            "{} active variant {}.{} is missing from abi-history.md active variant table",
            authority.variant_label(),
            declaration.name,
            active_variant_name
        )));
    };

    if expected_variant_id != active_variant_id {
        return Err(TerminalAbiError::new(format!(
            "{} active variant {}.{} uses ABI variant ID {}; abi-history.md records {}",
            authority.variant_label(),
            declaration.name,
            active_variant_name,
            active_variant_id,
            expected_variant_id
        )));
    }

    Ok(())
}

fn validate_active_variant_history_completeness(
    authority: TerminalAuthority,
    declaration: &ModuleTypeDeclaration,
    active_variant_table: Option<ActiveVariantIdTable>,
    active_variant_names: &BTreeMap<&str, i64>,
) -> Result<(), TerminalAbiError> {
    let Some(active_variant_table) = active_variant_table else {
        return Ok(());
    };

    for (expected_variant_name, expected_variant_id) in active_variant_table.variants {
        if !active_variant_names.contains_key(expected_variant_name) {
            return Err(TerminalAbiError::new(format!(
                "{} active variant {}.{}={} from abi-history.md is missing from active declarations",
                authority.variant_label(),
                declaration.name,
                expected_variant_name,
                expected_variant_id
            )));
        }
    }

    Ok(())
}

fn active_variant_table_by_declaration(
    active_variant_tables: &[ActiveVariantIdTable],
    declaration_name: &str,
) -> Option<ActiveVariantIdTable> {
    active_variant_tables
        .iter()
        .copied()
        .find(|table| table.declaration == declaration_name)
}

fn validate_historical_variant_status(
    declaration: &ModuleTypeDeclaration,
    active_variant_name: &str,
    active_variant_id: i64,
    retired_variant_ids: &[HistoricalVariantId],
    uncommitted_variant_ids: &[HistoricalVariantId],
) -> Result<(), TerminalAbiError> {
    if let Some(retired) =
        historical_variant_by_id(retired_variant_ids, &declaration.name, active_variant_id)
    {
        return Err(TerminalAbiError::new(format!(
            "retired selected ABI variant ID {} cannot be reused by active variant {}.{}",
            retired.label(),
            declaration.name,
            active_variant_name
        )));
    }

    if let Some(retired) =
        historical_variant_by_name(retired_variant_ids, &declaration.name, active_variant_name)
    {
        return Err(TerminalAbiError::new(format!(
            "retired selected ABI variant {} cannot reappear in active declarations",
            retired.label()
        )));
    }

    if let Some(candidate) = historical_variant_by_id(
        uncommitted_variant_ids,
        &declaration.name,
        active_variant_id,
    ) {
        return Err(TerminalAbiError::new(format!(
            "uncommitted selected ABI candidate {} must remain absent and unretired; active variant {}.{} reuses its value",
            candidate.label(),
            declaration.name,
            active_variant_name
        )));
    }

    if let Some(candidate) = historical_variant_by_name(
        uncommitted_variant_ids,
        &declaration.name,
        active_variant_name,
    ) {
        return Err(TerminalAbiError::new(format!(
            "uncommitted selected ABI candidate {} must remain absent and unretired; active variant {}.{} is present",
            candidate.label(),
            declaration.name,
            active_variant_name
        )));
    }

    Ok(())
}

fn validate_test_only_scope(interface: &ModuleInterface) -> Result<(), TerminalAbiError> {
    validate_interface_scope(
        interface,
        TERMINAL_TESTING_MODULE_PATH,
        ModuleAvailability::TestOnly,
        TERMINAL_TESTING_TYPES_PATH,
    )?;

    for declaration in interface.type_declarations.values() {
        if !has_test_only_availability(declaration) {
            return Err(TerminalAbiError::new(format!(
                "test-only terminal declaration {} must carry @availability(test_only)",
                declaration.name
            )));
        }

        if let Some(type_id) = optional_abi_type_id(declaration)? {
            return Err(TerminalAbiError::new(format!(
                "test-only terminal declaration {} declares production ABI type ID {}; terminal ABI authority is limited to {} and {} active declarations",
                declaration.name,
                format_type_id(type_id),
                TERMINAL_TYPES_MODULE_PATH,
                TERMINAL_CHORDS_MODULE_PATH
            )));
        }
    }

    Ok(())
}

fn optional_abi_type_id(
    declaration: &ModuleTypeDeclaration,
) -> Result<Option<i64>, TerminalAbiError> {
    let mut type_id = None;
    for annotation in &declaration.annotations {
        if let DeclarationAnnotation::AbiTypeId { value, .. } = annotation {
            if type_id.is_some() {
                return Err(TerminalAbiError::new(format!(
                    "declaration {} declares more than one @abi_type_id annotation",
                    declaration.name
                )));
            }
            type_id = Some(*value);
        }
    }
    Ok(type_id)
}

fn required_abi_type_id(
    declaration: &ModuleTypeDeclaration,
    authority: TerminalAuthority,
) -> Result<i64, TerminalAbiError> {
    if let Some(type_id) = optional_abi_type_id(declaration)? {
        return Ok(type_id);
    }

    Err(TerminalAbiError::new(format!(
        "{} declaration {} must declare @abi_type_id metadata",
        authority.inventory_label(),
        declaration.name
    )))
}

fn has_abi_evolution(declaration: &ModuleTypeDeclaration) -> bool {
    declaration
        .annotations
        .iter()
        .any(|annotation| matches!(annotation, DeclarationAnnotation::AbiEvolution { .. }))
}

fn has_test_only_availability(declaration: &ModuleTypeDeclaration) -> bool {
    declaration.annotations.iter().any(|annotation| {
        matches!(
            annotation,
            DeclarationAnnotation::Availability { value, .. } if value == "test_only"
        )
    })
}

fn historical_variant_by_id(
    variants: &[HistoricalVariantId],
    declaration: &str,
    id: i64,
) -> Option<HistoricalVariantId> {
    variants
        .iter()
        .copied()
        .find(|variant| variant.declaration == declaration && variant.id == id)
}

fn historical_variant_by_name(
    variants: &[HistoricalVariantId],
    declaration: &str,
    variant_name: &str,
) -> Option<HistoricalVariantId> {
    variants
        .iter()
        .copied()
        .find(|variant| variant.declaration == declaration && variant.variant == variant_name)
}

fn type_id_in_ranges(type_id: i64, ranges: &[TypeIdRange]) -> bool {
    ranges.iter().any(|range| range.contains(type_id))
}

fn format_type_id(type_id: i64) -> String {
    format!("0x{type_id:016x}")
}

#[cfg(test)]
mod tests {
    use super::super::ModuleResolver;
    use super::*;
    use crate::ast::TypeDef;
    use crate::token::{Position, Span};
    use alloc::vec::Vec;

    #[test]
    fn terminal_abi_reports_selected_and_chord_inventory_counts() {
        let interfaces = registered_terminal_interfaces();

        let inventory = validate_terminal_proposal_abi(&interfaces)
            .expect("current terminal proposal ABI metadata should validate");

        assert_eq!(
            inventory.selected_active_production_type_ids,
            SELECTED_ACTIVE_TYPE_ID_COUNT
        );
        assert_eq!(
            inventory.chord_active_production_type_ids,
            CHORD_ACTIVE_TYPE_ID_COUNT
        );
    }

    #[test]
    fn terminal_abi_rejects_reused_retired_selected_variant_id() {
        let mut interfaces = registered_terminal_interfaces();
        rewrite_variant_id(
            &mut interfaces,
            TERMINAL_TYPES_MODULE_PATH,
            "TerminalSessionRestoreError",
            "WrongSession",
            8,
        );

        let error = validate_terminal_proposal_abi(&interfaces)
            .expect_err("retired selected ABI variant ID reuse must fail");

        assert_eq!(
            error.to_string(),
            "retired selected ABI variant ID TerminalSessionRestoreError.RecoveryOwnerMismatch=8 cannot be reused by active variant TerminalSessionRestoreError.WrongSession"
        );
    }

    #[test]
    fn terminal_abi_rejects_selected_active_variant_id_mismatch() {
        let mut interfaces = registered_terminal_interfaces();
        rewrite_variant_id(
            &mut interfaces,
            TERMINAL_TYPES_MODULE_PATH,
            "TerminalSessionRestoreError",
            "WrongSession",
            16,
        );

        let error = validate_terminal_proposal_abi(&interfaces)
            .expect_err("selected active ABI variant ID mismatch must fail");

        assert_eq!(
            error.to_string(),
            "selected ABI variant ID active variant TerminalSessionRestoreError.WrongSession uses ABI variant ID 16; abi-history.md records 9"
        );
    }

    #[test]
    fn terminal_abi_rejects_selected_active_variant_declared_as_non_sum() {
        let mut interfaces = registered_terminal_interfaces();
        rewrite_restore_error_to_opaque(&mut interfaces);

        let error = validate_terminal_proposal_abi(&interfaces)
            .expect_err("selected active ABI variant declaration must remain a sum type");

        assert_eq!(
            error.to_string(),
            "selected terminal declaration TerminalSessionRestoreError must remain a sum type because abi-history.md records active variants for it"
        );
    }

    #[test]
    fn terminal_abi_rejects_chord_active_variant_id_mismatch() {
        let mut interfaces = registered_terminal_interfaces();
        rewrite_variant_id(
            &mut interfaces,
            TERMINAL_CHORDS_MODULE_PATH,
            "TerminalChordTrigger",
            "Release",
            4,
        );

        let error = validate_terminal_proposal_abi(&interfaces)
            .expect_err("chord active ABI variant ID mismatch must fail");

        assert_eq!(
            error.to_string(),
            "chord ABI variant ID active variant TerminalChordTrigger.Release uses ABI variant ID 4; abi-history.md records 3"
        );
    }

    #[test]
    fn terminal_abi_rejects_test_only_production_type_id() {
        let mut interfaces = registered_terminal_interfaces();
        add_test_only_abi_type_id(
            &mut interfaces,
            "TerminalTestAuthority",
            0x5400_0000_0000_0010_i64,
        );

        let error = validate_terminal_proposal_abi(&interfaces)
            .expect_err("test-only production ABI type ID must fail");

        assert_eq!(
            error.to_string(),
            "test-only terminal declaration TerminalTestAuthority declares production ABI type ID 0x5400000000000010; terminal ABI authority is limited to standard.terminal and standard.terminal.chords active declarations"
        );
    }

    #[test]
    fn terminal_abi_keeps_restore_generation_exhausted_candidate_uncommitted() {
        let interfaces = registered_terminal_interfaces();
        let selected = interface_by_path(&interfaces, TERMINAL_TYPES_MODULE_PATH)
            .expect("selected terminal interface should exist");
        let restore = selected
            .type_declaration("TerminalSessionRestoreError")
            .expect("restore error declaration should exist");
        let TypeDef::Sum { variants, .. } = &restore.type_def else {
            assert!(
                matches!(&restore.type_def, TypeDef::Sum { .. }),
                "restore error declaration should remain a sum type"
            );
            return;
        };

        assert!(
            variants
                .iter()
                .all(|variant| variant.name != "GenerationExhausted")
        );
        assert!(
            variants
                .iter()
                .all(|variant| variant.explicit_id != Some(14))
        );
        assert!(
            historical_variant_by_id(
                RETIRED_SELECTED_VARIANT_IDS,
                "TerminalSessionRestoreError",
                14,
            )
            .is_none()
        );
        assert!(
            historical_variant_by_id(
                UNCOMMITTED_SELECTED_VARIANT_IDS,
                "TerminalSessionRestoreError",
                14,
            )
            .is_some()
        );

        validate_terminal_proposal_abi(&interfaces)
            .expect("uncommitted candidate must remain absent from active metadata");
    }

    fn registered_terminal_interfaces() -> Vec<ModuleInterface> {
        let resolver = ModuleResolver::new();
        [
            TERMINAL_TYPES_MODULE_PATH,
            TERMINAL_CHORDS_MODULE_PATH,
            TERMINAL_TESTING_MODULE_PATH,
        ]
        .into_iter()
        .map(|module_path| {
            resolver
                .module_interface(module_path)
                .expect("terminal proposal interface should be registered")
        })
        .collect()
    }

    fn rewrite_variant_id(
        interfaces: &mut [ModuleInterface],
        module_path: &str,
        declaration_name: &str,
        variant_name: &str,
        explicit_id: i64,
    ) {
        let interface = interfaces
            .iter_mut()
            .find(|interface| interface.module_path == module_path)
            .expect("module interface should exist");
        let declaration = interface
            .type_declarations
            .get_mut(declaration_name)
            .expect("type declaration should exist");
        let TypeDef::Sum { variants, .. } = &mut declaration.type_def else {
            assert!(
                matches!(&declaration.type_def, TypeDef::Sum { .. }),
                "test fixture declaration should be a sum type"
            );
            return;
        };
        let variant = variants
            .iter_mut()
            .find(|variant| variant.name == variant_name)
            .expect("variant should exist");
        variant.explicit_id = Some(explicit_id);
    }

    fn rewrite_restore_error_to_opaque(interfaces: &mut [ModuleInterface]) {
        let interface = interfaces
            .iter_mut()
            .find(|interface| interface.module_path == TERMINAL_TYPES_MODULE_PATH)
            .expect("selected terminal interface should exist");
        let declaration = interface
            .type_declarations
            .get_mut("TerminalSessionRestoreError")
            .expect("restore error declaration should exist");
        declaration.type_def = TypeDef::Opaque {
            span: Span::single(Position::start()),
        };
    }

    fn add_test_only_abi_type_id(
        interfaces: &mut [ModuleInterface],
        declaration_name: &str,
        type_id: i64,
    ) {
        let interface = interfaces
            .iter_mut()
            .find(|interface| interface.module_path == TERMINAL_TESTING_MODULE_PATH)
            .expect("test-only module interface should exist");
        let declaration = interface
            .type_declarations
            .get_mut(declaration_name)
            .expect("test-only declaration should exist");
        declaration
            .annotations
            .push(DeclarationAnnotation::AbiTypeId {
                value: type_id,
                span: Span::single(Position::start()),
            });
    }
}
