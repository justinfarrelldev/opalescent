#![expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed AST proposal declarations keeps metadata extraction concise"
)]

extern crate alloc;

use super::terminal_proposal_symbols::register_terminal_proposal_symbols;
use super::{ModuleAvailability, ModuleInterface, ModuleResolver, ModuleTypeDeclaration};
use crate::ast::{Decl, Program, TypeDef, Visibility as AstVisibility};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::{format, string::String, vec::Vec};

/// Internal module path for selected terminal-session proposal type declarations.
pub(super) const TERMINAL_TYPES_MODULE_PATH: &str = "standard.terminal";
/// Internal module path for selected terminal chord proposal type declarations.
pub(super) const TERMINAL_CHORDS_MODULE_PATH: &str = "standard.terminal.chords";
/// Test-only module path for deterministic terminal testing declarations.
pub(super) const TERMINAL_TESTING_MODULE_PATH: &str = "standard.testing.terminal";

/// Exact source path for selected terminal-session declarations.
pub(super) const TYPED_EVENT_SESSION_TYPES_PATH: &str =
    "stdlib-proposals/terminal-session-input/typed-event-session/typed_event_session.types.op";
/// Exact source path for selected terminal chord declarations.
pub(super) const TERMINAL_CHORDS_TYPES_PATH: &str =
    "stdlib-proposals/terminal-session-input/terminal_chords.types.op";
/// Exact source path for test-only terminal declarations.
pub(super) const TERMINAL_TESTING_TYPES_PATH: &str =
    "stdlib-proposals/terminal-session-input/terminal_testing.types.op";

/// Authoritative selected terminal-session declaration source.
const TYPED_EVENT_SESSION_TYPES_SOURCE: &str = include_str!(
    "../../../stdlib-proposals/terminal-session-input/typed-event-session/typed_event_session.types.op"
);
/// Authoritative terminal chord declaration source.
const TERMINAL_CHORDS_TYPES_SOURCE: &str =
    include_str!("../../../stdlib-proposals/terminal-session-input/terminal_chords.types.op");
/// Authoritative test-only terminal declaration source.
const TERMINAL_TESTING_TYPES_SOURCE: &str =
    include_str!("../../../stdlib-proposals/terminal-session-input/terminal_testing.types.op");

/// Register proposal declaration interfaces without opening production terminal imports.
pub(super) fn register_terminal_proposal_modules(resolver: &mut ModuleResolver) {
    let interfaces = terminal_proposal_interfaces();
    let validation = super::terminal_proposal_abi::validate_terminal_proposal_abi(&interfaces);
    if let Err(error) = &validation {
        let error_message = format!("{error}");
        assert!(
            error_message.is_empty(),
            "terminal proposal ABI validation failed: {error_message}"
        );
    }
    let Ok(inventory) = validation else {
        return;
    };
    assert_eq!(
        inventory.selected_active_production_type_ids, 87,
        "selected terminal ABI validation must report exactly 87 active production type IDs"
    );
    assert_eq!(
        inventory.chord_active_production_type_ids, 23,
        "terminal chord ABI validation must report exactly 23 active production type IDs"
    );

    for interface in interfaces {
        resolver.register_module_interface(interface);
    }
}

/// Build all terminal proposal interfaces from authoritative embedded sources.
fn terminal_proposal_interfaces() -> [ModuleInterface; 3] {
    [
        proposal_interface_from_source(
            TERMINAL_TYPES_MODULE_PATH,
            ModuleAvailability::FuturePublicApi,
            TYPED_EVENT_SESSION_TYPES_PATH,
            TYPED_EVENT_SESSION_TYPES_SOURCE,
        ),
        proposal_interface_from_source(
            TERMINAL_CHORDS_MODULE_PATH,
            ModuleAvailability::FuturePublicApi,
            TERMINAL_CHORDS_TYPES_PATH,
            TERMINAL_CHORDS_TYPES_SOURCE,
        ),
        proposal_interface_from_source(
            TERMINAL_TESTING_MODULE_PATH,
            ModuleAvailability::TestOnly,
            TERMINAL_TESTING_TYPES_PATH,
            TERMINAL_TESTING_TYPES_SOURCE,
        ),
    ]
}

/// Parse one authoritative proposal source into a gated module interface.
fn proposal_interface_from_source(
    module_path: &str,
    availability: ModuleAvailability,
    source_path: &str,
    source: &str,
) -> ModuleInterface {
    let program = parse_authoritative_types_source(source_path, source);
    let mut interface = ModuleInterface::with_availability(String::from(module_path), availability);

    for declaration in &program.declarations {
        assert!(
            matches!(
                declaration,
                Decl::Namespace { .. }
                    | Decl::Type { .. }
                    | Decl::Import { .. }
                    | Decl::Comment { .. }
            ),
            "{source_path} must contain only type, import, namespace, and comment declarations"
        );

        match declaration {
            Decl::Namespace { path, .. } => interface.register_namespace(path.clone()),
            Decl::Type {
                name,
                type_def,
                annotations,
                form,
                visibility,
                span,
                ..
            } => {
                register_type_symbol(&mut interface, name, visibility, *span);
                register_adt_fields(&mut interface, name, type_def);
                interface.register_type_declaration(ModuleTypeDeclaration {
                    name: name.clone(),
                    source_path: String::from(source_path),
                    annotations: annotations.clone(),
                    form: *form,
                    type_def: type_def.clone(),
                    visibility: visibility.clone(),
                    span: *span,
                });
            }
            Decl::Import { .. }
            | Decl::Comment { .. }
            | Decl::Function { .. }
            | Decl::Let { .. } => {}
        }
    }

    register_terminal_proposal_symbols(&mut interface);
    interface
}

/// Parse an embedded `.types.op` source and fail fast if it no longer parses.
fn parse_authoritative_types_source(source_path: &str, source: &str) -> Program {
    let normalized_source = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalized_source);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "authoritative terminal declaration file {source_path} must lex without errors: {:?}",
        lex_errors.errors
    );

    let parser = Parser::new(tokens);
    let (program, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "authoritative terminal declaration file {source_path} must parse without errors: {:?}",
        parse_errors.errors
    );

    program.unwrap_or_else(|| Program {
        declarations: Vec::new(),
        span: Span::single(Position::start()),
        id: crate::ast::NodeId(0),
    })
}

/// Register a type declaration as an importable module type symbol.
fn register_type_symbol(
    interface: &mut ModuleInterface,
    name: &str,
    visibility: &AstVisibility,
    span: Span,
) {
    let register_result = interface.register_symbol(SymbolInfo {
        name: String::from(name),
        symbol_type: SymbolType::Type,
        core_type: generic_type(name),
        visibility: module_visibility(visibility),
        source_location: span,
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    assert!(
        register_result.is_ok(),
        "duplicate public terminal proposal type export in {}: {name}",
        interface.module_path
    );
}

/// Register product and sum-variant fields alongside retained declaration metadata.
fn register_adt_fields(interface: &mut ModuleInterface, name: &str, type_def: &TypeDef) {
    match type_def {
        TypeDef::Product { fields, .. } => {
            interface
                .adt_fields
                .insert(String::from(name), fields_to_core_map(fields));
        }
        TypeDef::Sum { variants, .. } => {
            for variant in variants {
                interface.adt_fields.insert(
                    format!("{name}.{}", variant.name),
                    fields_to_core_map(&variant.fields),
                );
            }
        }
        TypeDef::Alias { .. } | TypeDef::Opaque { .. } => {}
    }
}

/// Convert parsed AST fields into resolver field metadata.
fn fields_to_core_map(fields: &[crate::ast::Field]) -> BTreeMap<String, CoreType> {
    let mut field_map = BTreeMap::new();
    for field in fields {
        let field_type = ast_type_to_core_type(&field.type_annotation)
            .expect("proposal field types must convert to CoreType");
        field_map.insert(field.name.clone(), field_type);
    }
    field_map
}

/// Convert AST visibility into module-resolver symbol visibility.
const fn module_visibility(visibility: &AstVisibility) -> Visibility {
    match visibility {
        AstVisibility::Public => Visibility::Public,
        AstVisibility::Private => Visibility::Private,
    }
}

/// Build a nominal generic core type for proposal declarations.
fn generic_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: String::from(name),
        type_args: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{DeclarationAnnotation, TypeDeclarationForm};

    #[test]
    fn loads_selected_terminal_declarations_from_authoritative_file() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_TYPES_MODULE_PATH)
            .expect("selected terminal proposal interface should be registered");

        assert_eq!(interface.availability, ModuleAvailability::FuturePublicApi);
        assert!(interface.exports.contains_key("TerminalSession"));

        let declaration = interface
            .type_declaration("TerminalSession")
            .expect("TerminalSession metadata should be retained");
        assert_eq!(declaration.source_path, TYPED_EVENT_SESSION_TYPES_PATH);
        assert_eq!(
            declaration.form,
            TypeDeclarationForm::CompilerRegisteredAffineResource
        );
        assert!(matches!(declaration.type_def, TypeDef::Opaque { .. }));
        assert!(declaration.annotations.iter().any(|annotation| matches!(
            annotation,
            DeclarationAnnotation::AbiTypeId { value, .. }
                if *value == 0x5400_0000_0000_002e_i64
        )));
        assert!(declaration.annotations.iter().any(|annotation| matches!(
            annotation,
            DeclarationAnnotation::ConstructorVisibility { value, .. } if value == "runtime"
        )));
    }

    #[test]
    fn loads_chord_declarations_with_variant_ids_and_fields() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_CHORDS_MODULE_PATH)
            .expect("chord proposal interface should be registered");

        assert_eq!(interface.availability, ModuleAvailability::FuturePublicApi);

        let router = interface
            .type_declaration("TerminalChordRouter")
            .expect("TerminalChordRouter metadata should be retained");
        assert_eq!(router.source_path, TERMINAL_CHORDS_TYPES_PATH);
        assert_eq!(
            router.form,
            TypeDeclarationForm::CompilerRegisteredAffineResource
        );
        assert!(router.annotations.iter().any(|annotation| matches!(
            annotation,
            DeclarationAnnotation::AbiTypeId { value, .. }
                if *value == 0x5400_0000_0000_0113_i64
        )));

        let output = interface
            .type_declaration("TerminalChordRouterOutput")
            .expect("TerminalChordRouterOutput metadata should be retained");
        let TypeDef::Sum { variants, .. } = &output.type_def else {
            assert!(
                matches!(output.type_def, TypeDef::Sum { .. }),
                "TerminalChordRouterOutput should remain a sum type"
            );
            return;
        };
        let pending = variants
            .iter()
            .find(|variant| variant.name == "Pending")
            .expect("Pending variant should be retained");
        assert_eq!(pending.explicit_id, Some(1));
        assert_eq!(pending.fields[0].name, "deadline");
        assert!(
            interface
                .adt_fields
                .contains_key("TerminalChordRouterOutput.Pending")
        );
    }

    #[test]
    fn loads_test_only_terminal_declarations_with_namespace_and_without_abi_ids() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_TESTING_MODULE_PATH)
            .expect("test-only terminal proposal interface should be registered");

        assert_eq!(interface.availability, ModuleAvailability::TestOnly);
        assert_eq!(
            interface.namespaces,
            vec![vec![
                String::from("standard"),
                String::from("testing"),
                String::from("terminal"),
            ]]
        );

        let authority = interface
            .type_declaration("TerminalTestAuthority")
            .expect("TerminalTestAuthority metadata should be retained");
        assert_eq!(authority.source_path, TERMINAL_TESTING_TYPES_PATH);
        assert_eq!(authority.form, TypeDeclarationForm::OpaqueImmutable);
        assert!(authority.annotations.iter().any(|annotation| matches!(
            annotation,
            DeclarationAnnotation::Availability { value, .. } if value == "test_only"
        )));
        assert!(authority.annotations.iter().any(|annotation| matches!(
            annotation,
            DeclarationAnnotation::ConstructorVisibility { value, .. }
                if value == "test_runner"
        )));
        assert!(
            !authority
                .annotations
                .iter()
                .any(|annotation| matches!(annotation, DeclarationAnnotation::AbiTypeId { .. }))
        );
    }

    #[test]
    fn registers_all_core_prerequisite_function_signatures() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface("standard.system")
            .expect("core prerequisite proposal interface should be registered");

        assert_eq!(interface.availability, ModuleAvailability::FuturePublicApi);
        assert_interface_function_signatures(
            &interface,
            super::super::terminal_proposal_symbols::CORE_PREREQUISITE_FUNCTIONS,
        );
    }

    #[test]
    fn registers_all_selected_terminal_function_signatures() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_TYPES_MODULE_PATH)
            .expect("selected terminal proposal interface should be registered");

        assert_interface_function_signatures(
            &interface,
            super::super::terminal_proposal_symbols::SELECTED_TERMINAL_FUNCTIONS,
        );
    }

    #[test]
    fn registers_all_chord_function_signatures() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_CHORDS_MODULE_PATH)
            .expect("chord proposal interface should be registered");

        assert_interface_function_signatures(
            &interface,
            super::super::terminal_proposal_symbols::TERMINAL_CHORD_FUNCTIONS,
        );
    }

    #[test]
    fn registers_all_test_only_terminal_function_signatures() {
        let resolver = ModuleResolver::new();
        let interface = resolver
            .module_interface(TERMINAL_TESTING_MODULE_PATH)
            .expect("test-only terminal proposal interface should be registered");

        assert_interface_function_signatures(
            &interface,
            super::super::terminal_proposal_symbols::TERMINAL_TESTING_FUNCTIONS,
        );
    }

    fn assert_interface_function_signatures(
        interface: &ModuleInterface,
        specs: &[super::super::terminal_proposal_symbols::TerminalApiFunctionSpec],
    ) {
        let function_export_count = interface
            .exports
            .values()
            .filter(|symbol| symbol.symbol_type == SymbolType::Function)
            .count();
        assert_eq!(
            function_export_count,
            specs.len(),
            "{} should export exactly the proposal function table",
            interface.module_path
        );

        for spec in specs {
            let Some(symbol) = interface.exports.get(spec.name) else {
                assert!(
                    interface.exports.contains_key(spec.name),
                    "{} should export proposal function {}",
                    interface.module_path,
                    spec.name
                );
                continue;
            };
            let CoreType::Function {
                parameters,
                return_types,
                error_types,
                ..
            } = &symbol.core_type
            else {
                assert!(
                    matches!(symbol.core_type, CoreType::Function { .. }),
                    "{} should be a function symbol",
                    spec.name
                );
                continue;
            };
            assert_eq!(
                parameters,
                &spec
                    .parameters
                    .iter()
                    .copied()
                    .map(expected_core_type)
                    .collect::<Vec<_>>(),
                "{} parameter signature changed",
                spec.name
            );
            assert_eq!(
                return_types,
                &vec![expected_core_type(spec.return_type)],
                "{} return signature changed",
                spec.name
            );
            assert_eq!(
                error_types,
                &spec
                    .errors
                    .iter()
                    .map(|error| expected_nominal_type(error))
                    .collect::<Vec<_>>(),
                "{} error signature changed",
                spec.name
            );
        }
    }

    fn expected_core_type(
        type_ref: super::super::terminal_proposal_symbols::ApiTypeRef,
    ) -> CoreType {
        match type_ref {
            super::super::terminal_proposal_symbols::ApiTypeRef::Named(name) => {
                expected_named_core_type(name)
            }
            super::super::terminal_proposal_symbols::ApiTypeRef::Array(element) => {
                CoreType::Array(Box::new(expected_named_core_type(element)))
            }
        }
    }

    fn expected_named_core_type(name: &str) -> CoreType {
        match name {
            "int8" => CoreType::Int8,
            "int16" => CoreType::Int16,
            "int32" => CoreType::Int32,
            "int64" => CoreType::Int64,
            "uint8" => CoreType::UInt8,
            "uint16" => CoreType::UInt16,
            "uint32" => CoreType::UInt32,
            "uint64" => CoreType::UInt64,
            "float32" => CoreType::Float32,
            "float64" => CoreType::Float64,
            "string" => CoreType::String,
            "boolean" => CoreType::Boolean,
            "void" => CoreType::Unit,
            other => expected_nominal_type(other),
        }
    }

    fn expected_nominal_type(name: &str) -> CoreType {
        CoreType::Generic {
            name: String::from(name),
            type_args: Vec::new(),
        }
    }
}
