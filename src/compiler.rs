//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

use crate::ast::Decl;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::expressions::CodegenError;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::parser::errors::ParseError;
use crate::parser::Parser;
use crate::token::Position;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use alloc::string::String;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use inkwell::OptimizationLevel;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let lexer = Lexer::new(source);
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
            _ => {}
        }
    }

    Ok(codegen_context.module)
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
/// `extra_sources` allows additional C source files to be compiled and linked
/// alongside the object file (used later for `runtime/opal_runtime.c`).
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_file(
    object_path: &Path,
    output_path: &Path,
    extra_sources: &[&Path],
) -> Result<PathBuf, CompileError> {
    let mut command = Command::new("cc");
    command.arg(object_path);
    for source_path in extra_sources {
        command.arg(source_path);
    }
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
    link_object_file(&object_path, &binary_path, &[])
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
