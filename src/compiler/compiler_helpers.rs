extern crate alloc;
use crate::ast::{Decl, Expr, ImportItem, LabeledValue, LambdaBody, NodeId, Program, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::errors::ParseError;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
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

/// Collects type signatures of imported symbols for code generation.
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
            if let Some(interface) = checker.module_interface(import_source) {
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
pub fn compile_checked_program_to_module<'context>(
    context: &'context Context,
    program: &Program,
    imported_signatures: BTreeMap<String, CoreType>,
) -> Result<Module<'context>, crate::codegen::error::CodegenError> {
    let codegen_context = CodegenContext::new(context, "opalescent_module");
    let mut env = CodegenEnv::new(true);
    env.imported_signatures = imported_signatures;

    for declaration in &program.declarations {
        match *declaration {
            Decl::Import { ref source, .. } => {
                if matches!(source.as_str(), "standard" | "math") {
                    codegen_import_declaration(&codegen_context, &mut env, declaration)?;
                }
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
            Decl::Let { .. } | Decl::Type { .. } | Decl::Comment { .. } => {}
        }
    }

    Ok(codegen_context.module)
}
