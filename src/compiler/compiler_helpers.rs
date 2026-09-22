#![allow(
    clippy::pattern_type_mismatch,
    reason = "compiler helper layout scans intentionally destructure borrowed field tuples directly"
)]
extern crate alloc;
use crate::ast::{
    Decl, Expr, ImportItem, LabeledValue, LambdaBody, NodeId, Program, Stmt, TypeDef,
};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::{
    codegen_function_declaration, codegen_import_declaration, codegen_top_level_value_declaration,
};
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::errors::ParseError;
use crate::token::{Position, Span};
use crate::type_system::AdtLayoutManifestKind;
use crate::type_system::checker::TypeChecker;
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::string::String;
use inkwell::context::Context;
use inkwell::module::Module;
use std::path::Path;

use super::CompileError;

/// Converts a lambda body into a statement for function lowering.
pub fn lambda_body_to_function_body(body: &LambdaBody) -> Stmt {
    match *body {
        LambdaBody::Block(ref statements) => Stmt::Block {
            statements: statements.clone(),
            span: statements.first().zip(statements.last()).map_or_else(
                || Span::single(Position::start()),
                |(first_statement, last_statement)| {
                    Span::new(
                        first_statement.span_const().start,
                        last_statement.span_const().end,
                    )
                },
            ),
            id: NodeId(0),
        },
        LambdaBody::Expression(ref expression) => {
            let expression_span = expression.span_const();
            Stmt::Return {
                values: vec![LabeledValue {
                    label: String::new(),
                    value: *expression.clone(),
                    span: expression_span,
                    id: NodeId(0),
                }],
                span: expression_span,
                id: NodeId(0),
            }
        }
    }
}

/// Parses source code into an AST program.
pub fn parse_source_to_program(source: &str) -> Result<Program, CompileError> {
    let normalized_source = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalized_source);
    let (tokens, lex_errors) = lexer.tokenize();
    if let Some(first_lex_error) = lex_errors.errors.into_iter().next() {
        return Err(CompileError::Lex(first_lex_error));
    }

    let parser = Parser::new(tokens);
    let (program_option, parse_errors) = parser.parse();
    if let Some(first_parse_error) = parse_errors.errors.into_iter().next() {
        return Err(CompileError::Parse(first_parse_error));
    }

    let Some(program) = program_option else {
        return Err(CompileError::Parse(ParseError::InvalidSyntax {
            message: String::from("parser returned no program after successful parse"),
            span: LexError::span_from_position(Position::start(), 1),
        }));
    };

    Ok(program)
}

/// Checks if a module path is the main entry point.
pub fn is_main_module_path(project_dir: &Path, module_path: &Path) -> bool {
    let expected_main = project_dir.join("src").join("main.op");
    canonicalize_or_original_path(&expected_main) == canonicalize_or_original_path(module_path)
}

/// Canonicalizes a path or returns the original if canonicalization fails.
pub fn canonicalize_or_original_path(path: &Path) -> std::path::PathBuf {
    path.canonicalize()
        .unwrap_or_else(|_io_err| path.to_path_buf())
}

/// Validates that entry declarations only appear in the main module.
pub fn validate_entry_declarations_for_module(
    project_dir: &Path,
    module_path: &Path,
    program: &Program,
) -> Result<(), CompileError> {
    if is_main_module_path(project_dir, module_path) {
        return Ok(());
    }

    for declaration in &program.declarations {
        if let &Decl::Function { is_entry, span, .. } = declaration {
            if is_entry {
                return Err(CompileError::Type(
                    crate::type_system::errors::TypeError::EntryNotInMainModule {
                        file_path: module_path.display().to_string(),
                        span: crate::type_system::errors::TypeError::span_from_span(span),
                    },
                ));
            }
        }
    }

    Ok(())
}

/// Builds a field-index lookup table for every product and sum type in a program.
pub fn collect_program_adt_field_indices(
    program: &Program,
) -> BTreeMap<String, BTreeMap<String, u32>> {
    let mut adt_field_indices = BTreeMap::new();
    for (name, fields) in collect_program_adt_field_layouts(program) {
        let mut field_indices = BTreeMap::new();
        for (index, (field_name, _field_type)) in fields.iter().enumerate() {
            let Ok(converted_index) = u32::try_from(index) else {
                continue;
            };
            field_indices.insert(field_name.clone(), converted_index);
        }
        adt_field_indices.insert(name, field_indices);
    }
    adt_field_indices
}

/// Builds the lowered field layout for every product and sum variant in a program.
pub fn collect_program_adt_field_layouts(
    program: &Program,
) -> BTreeMap<String, Vec<(String, CoreType)>> {
    let mut adt_field_layouts = BTreeMap::new();
    for declaration in &program.declarations {
        let Decl::Type {
            ref name,
            ref type_def,
            ..
        } = *declaration
        else {
            continue;
        };
        match *type_def {
            TypeDef::Product { ref fields, .. } => {
                let mut field_layout = Vec::new();
                for field in fields {
                    let Ok(core_type) = ast_type_to_core_type(&field.type_annotation) else {
                        continue;
                    };
                    field_layout.push((field.name.clone(), core_type));
                }
                adt_field_layouts.insert(name.clone(), field_layout);
            }
            TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    let mut field_layout = Vec::new();
                    for field in &variant.fields {
                        let Ok(core_type) = ast_type_to_core_type(&field.type_annotation) else {
                            continue;
                        };
                        field_layout.push((field.name.clone(), core_type));
                    }
                    adt_field_layouts.insert(format!("{name}.{}", variant.name), field_layout);
                }
            }
            TypeDef::Alias { .. } | TypeDef::Opaque { .. } => {}
        }
    }
    adt_field_layouts
}

/// Build field-index metadata from lowered ADT field layouts.
pub fn collect_adt_field_indices_from_layouts(
    layouts: &BTreeMap<String, Vec<(String, CoreType)>>,
) -> BTreeMap<String, BTreeMap<String, u32>> {
    let mut adt_field_indices = BTreeMap::new();
    for (name, fields) in layouts {
        let mut field_indices = BTreeMap::new();
        for (index, (field_name, _field_type)) in fields.iter().enumerate() {
            let Ok(converted_index) = u32::try_from(index) else {
                continue;
            };
            field_indices.insert(field_name.clone(), converted_index);
        }
        adt_field_indices.insert(name.clone(), field_indices);
    }
    adt_field_indices
}

/// Merge field-index metadata derived from ADT field layouts without replacing local metadata.
pub fn merge_adt_field_indices_from_layouts(
    adt_field_indices: &mut BTreeMap<String, BTreeMap<String, u32>>,
    adt_field_layouts: &BTreeMap<String, Vec<(String, CoreType)>>,
) {
    for (owner, field_indices) in collect_adt_field_indices_from_layouts(adt_field_layouts) {
        adt_field_indices.entry(owner).or_insert(field_indices);
    }
}

/// Collect ADT field layouts from modules imported by the current program.
pub fn collect_imported_adt_field_layouts(
    checker: &TypeChecker,
    program: &Program,
) -> BTreeMap<String, Vec<(String, CoreType)>> {
    let mut adt_field_layouts = BTreeMap::new();
    for declaration in &program.declarations {
        let Decl::Import { source, .. } = declaration else {
            continue;
        };
        let Some(interface) = checker.module_interface(source.as_str()) else {
            continue;
        };
        for (type_name, type_declaration) in &interface.type_declarations {
            match &type_declaration.type_def {
                TypeDef::Product { fields, .. } => {
                    let mut field_layout = Vec::new();
                    for field in fields {
                        let Ok(core_type) = ast_type_to_core_type(&field.type_annotation) else {
                            continue;
                        };
                        field_layout.push((field.name.clone(), core_type));
                    }
                    adt_field_layouts.insert(type_name.clone(), field_layout);
                }
                TypeDef::Sum { variants, .. } => {
                    for variant in variants {
                        let mut field_layout = Vec::new();
                        for field in &variant.fields {
                            let Ok(core_type) = ast_type_to_core_type(&field.type_annotation)
                            else {
                                continue;
                            };
                            field_layout.push((field.name.clone(), core_type));
                        }
                        adt_field_layouts
                            .insert(format!("{type_name}.{}", variant.name), field_layout);
                    }
                }
                TypeDef::Alias { .. } | TypeDef::Opaque { .. } => {}
            }
        }
    }
    adt_field_layouts
}

/// Merge canonical ADT layouts from one discovered module interface manifest.
pub fn merge_interface_adt_field_layouts(
    interface: &crate::type_system::ModuleInterface,
    adt_field_layouts: &mut BTreeMap<String, Vec<(String, CoreType)>>,
) {
    if interface.adt_layout_manifests.is_empty() {
        merge_legacy_interface_adt_field_layouts(interface, adt_field_layouts);
        return;
    }
    for manifest in interface.adt_layout_manifests.values() {
        let type_key = manifest.type_id.layout_key();
        match &manifest.kind {
            AdtLayoutManifestKind::Product { fields } => {
                adt_field_layouts.entry(type_key).or_insert_with(|| {
                    fields
                        .iter()
                        .map(|field| (field.name.clone(), field.core_type.clone()))
                        .collect()
                });
            }
            AdtLayoutManifestKind::Sum { variants } => {
                for variant in variants {
                    let variant_key = format!("{type_key}.{}", variant.name);
                    adt_field_layouts.entry(variant_key).or_insert_with(|| {
                        variant
                            .fields
                            .iter()
                            .map(|field| (field.name.clone(), field.core_type.clone()))
                            .collect()
                    });
                }
            }
            AdtLayoutManifestKind::Alias { .. } | AdtLayoutManifestKind::Opaque { .. } => {}
        }
    }
}

/// Merge legacy type-declaration layouts for built-in interfaces not yet manifest-backed.
fn merge_legacy_interface_adt_field_layouts(
    interface: &crate::type_system::ModuleInterface,
    adt_field_layouts: &mut BTreeMap<String, Vec<(String, CoreType)>>,
) {
    for (type_name, declaration) in &interface.type_declarations {
        match &declaration.type_def {
            TypeDef::Product { fields, .. } => {
                let field_layout = fields
                    .iter()
                    .filter_map(|field| {
                        ast_type_to_core_type(&field.type_annotation)
                            .ok()
                            .map(|core_type| (field.name.clone(), core_type))
                    })
                    .collect::<Vec<_>>();
                adt_field_layouts
                    .entry(type_name.clone())
                    .or_insert(field_layout);
            }
            TypeDef::Sum { variants, .. } => {
                for variant in variants {
                    let field_layout = variant
                        .fields
                        .iter()
                        .filter_map(|field| {
                            ast_type_to_core_type(&field.type_annotation)
                                .ok()
                                .map(|core_type| (field.name.clone(), core_type))
                        })
                        .collect::<Vec<_>>();
                    adt_field_layouts
                        .entry(format!("{type_name}.{}", variant.name))
                        .or_insert(field_layout);
                }
            }
            TypeDef::Alias { .. } | TypeDef::Opaque { .. } => {}
        }
    }
}

/// Merge canonical sum variant discriminants from one discovered module interface manifest.
pub fn merge_interface_adt_variant_discriminants(
    interface: &crate::type_system::ModuleInterface,
    discriminants: &mut BTreeMap<String, i64>,
) {
    for manifest in interface.adt_layout_manifests.values() {
        let AdtLayoutManifestKind::Sum { variants } = &manifest.kind else {
            continue;
        };
        let type_key = manifest.type_id.layout_key();
        for variant in variants {
            discriminants
                .entry(format!("{type_key}.{}", variant.name))
                .or_insert(variant.discriminant);
        }
    }
}

/// Collect globally unambiguous short ADT aliases for transitive signature use.
pub fn collect_unique_adt_layout_aliases<'interface, I>(interfaces: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = &'interface crate::type_system::ModuleInterface>,
{
    let mut aliases = BTreeMap::new();
    let mut ambiguous = alloc::collections::BTreeSet::new();
    for interface in interfaces {
        for manifest in interface.adt_layout_manifests.values() {
            let type_key = manifest.type_id.layout_key();
            register_unique_alias(
                &mut aliases,
                &mut ambiguous,
                &manifest.type_id.type_name,
                &type_key,
            );
            if let AdtLayoutManifestKind::Sum { variants } = &manifest.kind {
                for variant in variants {
                    register_unique_alias(
                        &mut aliases,
                        &mut ambiguous,
                        format!("{}.{}", manifest.type_id.type_name, variant.name).as_str(),
                        format!("{type_key}.{}", variant.name).as_str(),
                    );
                }
            }
        }
    }
    for name in ambiguous {
        aliases.remove(name.as_str());
    }
    aliases
}

/// Collect import-local ADT aliases from explicit type imports in one program.
pub fn collect_imported_adt_layout_aliases(
    checker: &TypeChecker,
    program: &Program,
) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();
    for declaration in &program.declarations {
        let Decl::Import { items, source, .. } = declaration else {
            continue;
        };
        let Some(interface) = checker.module_interface(source.as_str()) else {
            continue;
        };
        for item in items {
            match item {
                ImportItem::Type { name, alias, .. } => {
                    register_imported_type_alias(&mut aliases, &interface, name, alias.as_deref());
                }
                ImportItem::Glob { .. } => {
                    for manifest in interface.adt_layout_manifests.values() {
                        register_imported_type_alias(
                            &mut aliases,
                            &interface,
                            manifest.type_id.type_name.as_str(),
                            None,
                        );
                    }
                }
                ImportItem::Named { .. } => {}
            }
        }
    }
    aliases
}

/// Register a direct import-local type alias and its variant aliases.
fn register_imported_type_alias(
    aliases: &mut BTreeMap<String, String>,
    interface: &crate::type_system::ModuleInterface,
    imported_name: &str,
    alias: Option<&str>,
) {
    let Some(manifest) = interface.adt_layout_manifest(imported_name) else {
        return;
    };
    let local_name = alias.unwrap_or(imported_name);
    let type_key = manifest.type_id.layout_key();
    aliases.insert(local_name.to_owned(), type_key.clone());
    if let AdtLayoutManifestKind::Sum { variants } = &manifest.kind {
        for variant in variants {
            aliases.insert(
                format!("{local_name}.{}", variant.name),
                format!("{type_key}.{}", variant.name),
            );
        }
    }
}

/// Register a short alias only while it remains globally unambiguous.
fn register_unique_alias(
    aliases: &mut BTreeMap<String, String>,
    ambiguous: &mut alloc::collections::BTreeSet<String>,
    local_name: &str,
    canonical_name: &str,
) {
    if ambiguous.contains(local_name) {
        return;
    }
    if let Some(existing) = aliases.get(local_name) {
        if existing != canonical_name {
            ambiguous.insert(local_name.to_owned());
        }
        return;
    }
    aliases.insert(local_name.to_owned(), canonical_name.to_owned());
}

/// Merge standard-library ADT layouts needed by generated project modules.
pub fn merge_standard_adt_field_layouts(
    adt_field_layouts: &mut BTreeMap<String, Vec<(String, CoreType)>>,
) {
    let standard_layout_checker = TypeChecker::new();
    for module_path in [
        "standard",
        "standard.system",
        "standard.terminal",
        "standard.terminal.chords",
    ] {
        if let Some(interface) = standard_layout_checker.module_interface(module_path) {
            merge_interface_adt_field_layouts(&interface, adt_field_layouts);
        }
    }
}

/// Collects the current module's public and private symbol signatures for codegen.
pub fn collect_module_symbol_signatures(
    checker: &TypeChecker,
    module_path: &str,
) -> BTreeMap<String, CoreType> {
    let mut module_signatures = BTreeMap::new();
    if let Some(interface) = checker.module_interface(module_path) {
        for (name, symbol) in &interface.exports {
            module_signatures.insert(name.clone(), symbol.core_type.clone());
        }
        for (name, symbol) in &interface.private_symbols {
            module_signatures.insert(name.clone(), symbol.core_type.clone());
        }
    }
    module_signatures
}

/// Collects type signatures of imported symbols for code generation.
#[expect(
    clippy::needless_borrowed_reference,
    reason = "borrowed declaration matching avoids the repo's pattern-type-mismatch lint"
)]
pub fn collect_imported_symbol_signatures(
    checker: &TypeChecker,
    program: &Program,
) -> BTreeMap<String, CoreType> {
    let mut imported_signatures: BTreeMap<String, CoreType> = BTreeMap::new();

    for declaration in &program.declarations {
        if let &Decl::Import {
            ref items,
            source: ref import_source,
            ..
        } = declaration
        {
            if let Some(interface) = checker.module_interface(import_source.as_str()) {
                for import_item in items {
                    match import_item {
                        &ImportItem::Named {
                            ref name,
                            ref alias,
                            ..
                        }
                        | &ImportItem::Type {
                            ref name,
                            ref alias,
                            ..
                        } => {
                            if let Some(exported_symbol) = interface.exports.get(name) {
                                let import_name = alias.as_deref().unwrap_or(name).to_owned();
                                imported_signatures
                                    .insert(import_name, exported_symbol.core_type.clone());
                            }
                        }
                        &ImportItem::Glob { .. } => {
                            for (export_name, exported_symbol) in &interface.exports {
                                imported_signatures
                                    .insert(export_name.clone(), exported_symbol.core_type.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    imported_signatures
}

/// Compiles a type-checked program into an LLVM module.
#[expect(
    clippy::too_many_arguments,
    reason = "compilation inputs are threaded through explicitly"
)]
pub fn compile_checked_program_to_module<'context>(
    context: &'context Context,
    source_path: &Path,
    source: &str,
    program: &Program,
    imported_signatures: BTreeMap<String, CoreType>,
    module_symbol_signatures: &BTreeMap<String, CoreType>,
    adt_field_indices: &BTreeMap<String, BTreeMap<String, u32>>,
    adt_field_layouts: &BTreeMap<String, Vec<(String, CoreType)>>,
    adt_layout_aliases: &BTreeMap<String, String>,
    adt_variant_discriminants: &BTreeMap<String, i64>,
    target: &crate::build_system::targets::TargetTriple,
) -> Result<Module<'context>, crate::codegen::error::CodegenError> {
    let codegen_context = CodegenContext::for_triple(context, "opalescent_module", target)
        .map_err(|error| crate::codegen::error::CodegenError::new(format!("{error:?}")))?;
    let mut env = CodegenEnv::new(true);
    env.imported_signatures = imported_signatures;
    env.current_source_path = source_path.display().to_string();
    env.current_source_text = source.replace('\t', "    ");
    env.adt_field_indices = adt_field_indices.clone();
    env.adt_field_layouts = adt_field_layouts.clone();
    env.adt_layout_aliases = adt_layout_aliases.clone();
    env.adt_variant_discriminants = adt_variant_discriminants.clone();

    for declaration in &program.declarations {
        match *declaration {
            Decl::Import { .. } => {
                codegen_import_declaration(&codegen_context, &mut env, declaration)?;
            }
            Decl::Function { .. } => {
                codegen_function_declaration(&codegen_context, &mut env, declaration)?;
            }
            Decl::Let {
                ref binding,
                initializer:
                    Expr::Lambda {
                        ref generic_params,
                        ref generic_constraints,
                        ref params,
                        ref return_types,
                        ref error_types,
                        ref body,
                        ..
                    },
                ref visibility,
                ref doc_comment,
                span,
                ..
            } => {
                let lowered_body = lambda_body_to_function_body(body);
                let lowered_declaration = Decl::Function {
                    name: binding.name.clone(),
                    generic_params: generic_params.clone(),
                    generic_constraints: generic_constraints.clone(),
                    parameters: params.clone(),
                    return_types: Some(return_types.clone()),
                    error_types: error_types.clone(),
                    body: lowered_body,
                    visibility: visibility.clone(),
                    is_entry: false,
                    modifiers: vec![],
                    doc_comment: doc_comment.clone(),
                    span,
                    id: NodeId(0),
                    metadata: crate::ast::HotReloadMetadata::for_function(),
                };

                codegen_function_declaration(&codegen_context, &mut env, &lowered_declaration)?;
            }
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ..
            } => {
                if let Some(core_type) = module_symbol_signatures.get(binding.name.as_str()) {
                    codegen_top_level_value_declaration(
                        &codegen_context,
                        &mut env,
                        binding.name.as_str(),
                        initializer,
                        visibility,
                        core_type,
                    )?;
                }
            }
            Decl::Type { .. }
            | Decl::ErrorSet { .. }
            | Decl::Namespace { .. }
            | Decl::Comment { .. } => {}
        }
    }

    Ok(codegen_context.module)
}
