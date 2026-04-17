//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

use crate::ast::{Decl, Expr, LabeledValue, LambdaBody, NodeId, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::expressions::CodegenError;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::parser::errors::ParseError;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use alloc::string::String;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use inkwell::OptimizationLevel;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Embedded C runtime source used during native linking.
const RUNTIME_SOURCE: &str = include_str!("../runtime/opal_runtime.c");

/// Temporary runtime source file materialized for the system C compiler.
struct RuntimeTempFile {
    /// Path to the generated temporary C runtime source file.
    path: PathBuf,
}

impl RuntimeTempFile {
    /// Create a uniquely named temporary runtime source file.
    fn create() -> Result<Self, CompileError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| CompileError::Io(std::io::Error::other(error)))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "opal_runtime_{}_{}.c",
            std::process::id(),
            timestamp
        ));
        std::fs::write(&path, RUNTIME_SOURCE).map_err(CompileError::Io)?;
        Ok(Self { path })
    }

    /// Borrow the filesystem path for this temporary runtime source file.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "PathBuf deref to Path is not const on stable"
    )]
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RuntimeTempFile {
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.path));
    }
}

/// Error type spanning every stage of compiler orchestration.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// Lexing stage returned a lexical analysis error.
    #[error("lexing failed")]
    Lex(LexError),
    /// Parsing stage returned a syntax analysis error.
    #[error("parsing failed")]
    Parse(ParseError),
    /// Type checking stage returned a semantic type error.
    #[error("type checking failed")]
    Type(TypeError),
    /// Code generation stage returned an LLVM lowering error.
    #[error("code generation failed: {0}")]
    Codegen(CodegenError),
    /// Filesystem interaction failed while preparing outputs.
    #[error("io failed: {0}")]
    Io(std::io::Error),
    /// Native linker process failed to produce an executable.
    #[error("linker invocation failed: {stderr}")]
    Linker {
        /// Captured stderr from the linker process.
        stderr: String,
    },
}

/// Compile source text into an LLVM module using shared context lifetime.
///
/// # Errors
/// Returns `CompileError` when lexing, parsing, type-checking, or codegen fails.
pub fn compile_to_module<'context>(
    context: &'context Context,
    source: &str,
) -> Result<Module<'context>, CompileError> {
    let normalized_source = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalized_source);
    let (tokens, lex_errors) = lexer.tokenize();
    if let Some(error) = lex_errors.errors.into_iter().next() {
        return Err(CompileError::Lex(error));
    }

    let parser = Parser::new(tokens);
    let (program_option, parse_errors) = parser.parse();
    if let Some(error) = parse_errors.errors.into_iter().next() {
        return Err(CompileError::Parse(error));
    }

    let Some(program) = program_option else {
        return Err(CompileError::Parse(ParseError::InvalidSyntax {
            message: String::from("parser returned no program after successful parse"),
            span: LexError::span_from_position(Position::start(), 1),
        }));
    };

    let mut checker = TypeChecker::new();
    if let Err(type_errors) = checker.type_check_program(&program) {
        if let Some(first_error) = type_errors.into_iter().next() {
            return Err(CompileError::Type(first_error));
        }
        return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
            reason: String::from("type checker returned empty error set"),
            span: TypeError::unknown_span(),
        }));
    }

    let codegen_context = CodegenContext::new(context, "opalescent_module");
    let mut env = CodegenEnv::new(true);

    for declaration in &program.declarations {
        match *declaration {
            Decl::Import { .. } => {
                codegen_import_declaration(&codegen_context, &mut env, declaration)
                    .map_err(CompileError::Codegen)?;
            }
            Decl::Function { .. } => {
                codegen_function_declaration(&codegen_context, &mut env, declaration)
                    .map_err(CompileError::Codegen)?;
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

                codegen_function_declaration(&codegen_context, &mut env, &lowered_declaration)
                    .map_err(CompileError::Codegen)?;
            }
            Decl::Let { .. } | Decl::Type { .. } | Decl::Comment { .. } => {}
        }
    }

    Ok(codegen_context.module)
}

/// Lower a lambda body into a function-compatible statement body.
fn lambda_body_to_function_body(body: &LambdaBody) -> Stmt {
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

/// Emit LLVM module as an object file at `path`.
///
/// # Errors
/// Returns `CodegenError` if LLVM target initialization or object emission fails.
pub fn emit_object_file(module: &Module<'_>, path: &Path) -> Result<(), CodegenError> {
    Target::initialize_native(&InitializationConfig::default()).map_err(|error| {
        CodegenError::new(format!(
            "failed to initialize native LLVM target support: {error}"
        ))
    })?;

    let triple = module.get_triple();
    let target = Target::from_triple(&triple)
        .map_err(|error| CodegenError::new(format!("failed to resolve LLVM target: {error}")))?;

    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "failed to create LLVM target machine for object emission",
            ))
        })?;

    target_machine
        .write_to_file(module, FileType::Object, path)
        .map_err(|error| CodegenError::new(format!("failed to emit object file: {error}")))
}

/// Link an object file into an executable binary.
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_file(object_path: &Path, output_path: &Path) -> Result<PathBuf, CompileError> {
    let runtime_temp_file = RuntimeTempFile::create()?;

    let mut command = Command::new("cc");
    command.arg(object_path);
    command.arg(runtime_temp_file.path());
    if cfg!(target_os = "linux") {
        command.arg("-no-pie");
    }
    command.arg("-o").arg(output_path);

    let output = command.output().map_err(CompileError::Io)?;
    if output.status.success() {
        return Ok(output_path.to_path_buf());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Err(CompileError::Linker { stderr })
}

/// Compile Opalescent source to a native binary.
///
/// Creates `program.o` and `program` inside `output_dir`.
///
/// # Errors
/// Returns `CompileError` at any pipeline stage.
pub fn compile_program(source: &str, output_dir: &Path) -> Result<PathBuf, CompileError> {
    std::fs::create_dir_all(output_dir).map_err(CompileError::Io)?;

    let context = Context::create();
    let module = compile_to_module(&context, source)?;

    let object_path = output_dir.join("program.o");
    let binary_path = output_dir.join("program");

    emit_object_file(&module, &object_path).map_err(CompileError::Codegen)?;
    link_object_file(&object_path, &binary_path)
}

#[cfg(test)]
mod tests {
    use super::{compile_to_module, CompileError};
    use inkwell::context::Context;

    /// Valid source should compile and produce a verifiable module.
    #[test]
    fn compile_to_module_valid_void_program() {
        let context = Context::create();
        let source = "entry main = f(): void => { return void }";
        let result = compile_to_module(&context, source);

        assert!(result.is_ok(), "valid source should compile into a module");

        if let Ok(module) = result {
            let verification = module.verify();
            assert!(
                verification.is_ok(),
                "generated module should pass LLVM verification"
            );
            assert!(
                module.get_function("main").is_some(),
                "entry function codegen should emit a C ABI main wrapper"
            );
        }
    }

    /// Invalid characters should fail during lexical analysis.
    #[test]
    fn compile_to_module_lex_error() {
        let context = Context::create();
        let source = "entry main = f(): void => { let x = @@@invalid }";
        let result = compile_to_module(&context, source);

        assert!(
            matches!(result, Err(CompileError::Lex(_))),
            "invalid tokens should surface as CompileError::Lex"
        );
    }

    /// Type mismatch should fail after parse but before codegen.
    #[test]
    fn compile_to_module_type_error() {
        let context = Context::create();
        let source = "entry main = f(): void => { return 1 }";
        let result = compile_to_module(&context, source);

        assert!(
            matches!(result, Err(CompileError::Type(_))),
            "semantic mismatches should surface as CompileError::Type"
        );
    }
}
