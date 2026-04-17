//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

use crate::ast::{Decl, Expr, LabeledValue, LambdaBody, NodeId, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::errors::reporter::{CompilationErrorReport, CompilerError};
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
const RUNTIME_SOURCE: &str = concat!(
    include_str!("../runtime/opal_error.c"),
    "\n",
    include_str!("../runtime/opal_io.c"),
    "\n",
    include_str!("../runtime/opal_print.c"),
    "\n",
    include_str!("../runtime/opal_rng.c"),
    "\n",
    include_str!("../runtime/opal_parse.c"),
    "\n",
    include_str!("../runtime/opal_string.c"),
);

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
    /// Front-end compilation returned one or more diagnostics.
    #[error("front-end compilation failed")]
    Report {
        /// Collected diagnostics across compiler phases.
        report: CompilationErrorReport,
        /// Tab-normalized source used for diagnostics.
        normalized_source: String,
    },
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
/// Returns a multi-error report plus normalized source when any stage fails.
pub fn compile_to_module<'context>(
    context: &'context Context,
    source: &str,
) -> Result<Module<'context>, (CompilationErrorReport, String)> {
    let normalized_source = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalized_source);
    let (tokens, lex_errors) = lexer.tokenize();
    let mut report = CompilationErrorReport::new();
    report.extend_lex_errors(lex_errors.errors);
    if !report.is_empty() {
        return Err((report, normalized_source));
    }

    let parser = Parser::new(tokens);
    let (program_option, parse_errors) = parser.parse();
    report.extend_parse_errors(parse_errors.errors);
    if !report.is_empty() {
        return Err((report, normalized_source));
    }

    let Some(program) = program_option else {
        report.push_parse_error(ParseError::InvalidSyntax {
            message: String::from("parser returned no program after successful parse"),
            span: LexError::span_from_position(Position::start(), 1),
        });
        return Err((report, normalized_source));
    };

    let mut checker = TypeChecker::new();
    if let Err(type_errors) = checker.type_check_program(&program) {
        report.extend_type_errors(type_errors);
        if report.is_empty() {
            report.push_type_error(TypeError::ConstraintSolvingFailed {
                reason: String::from("type checker returned empty error set"),
                span: TypeError::unknown_span(),
            });
        }
        return Err((report, normalized_source));
    }

    let codegen_context = CodegenContext::new(context, "opalescent_module");
    let mut env = CodegenEnv::new(true);

    for declaration in &program.declarations {
        match *declaration {
            Decl::Import { .. } => {
                codegen_import_declaration(&codegen_context, &mut env, declaration).map_err(
                    |error| {
                        let mut codegen_report = CompilationErrorReport::new();
                        codegen_report.push_codegen_error(error.message);
                        (codegen_report, normalized_source.clone())
                    },
                )?;
            }
            Decl::Function { .. } => {
                codegen_function_declaration(&codegen_context, &mut env, declaration).map_err(
                    |error| {
                        let mut codegen_report = CompilationErrorReport::new();
                        codegen_report.push_codegen_error(error.message);
                        (codegen_report, normalized_source.clone())
                    },
                )?;
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
                    .map_err(|error| {
                        let mut codegen_report = CompilationErrorReport::new();
                        codegen_report.push_codegen_error(error.message);
                        (codegen_report, normalized_source.clone())
                    })?;
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

/// Build a platform-appropriate linker [`Command`] for the given object and output paths.
///
/// Platform behaviour:
/// - **Linux**: `cc -no-pie <obj> <runtime> -o <out>` — PIE relocation workaround required
/// - **macOS**: `cc <obj> <runtime> -o <out>` — `-no-pie` not needed and may be unsupported
/// - **Windows (MSVC)**: `link.exe /OUT:<out> <obj> <runtime>` — MSVC linker syntax
/// - **Windows (other)**: `gcc <obj> <runtime> -o <out>` — MinGW / Cygwin fallback
///
/// The `target_os` parameter accepts the same values as [`std::env::consts::OS`].
#[must_use]
pub fn build_linker_command(
    target_os: &str,
    object_path: &Path,
    runtime_path: &Path,
    output_path: &Path,
) -> Command {
    match target_os {
        "windows" => {
            // Try MSVC link.exe first; fall back to MinGW gcc if unavailable.
            if std::process::Command::new("link.exe")
                .arg("/?")
                .output()
                .is_ok()
            {
                let mut cmd = Command::new("link.exe");
                cmd.arg(format!("/OUT:{}", output_path.display()));
                cmd.arg(object_path);
                cmd.arg(runtime_path);
                cmd
            } else {
                let mut cmd = Command::new("gcc");
                cmd.arg(object_path);
                cmd.arg(runtime_path);
                cmd.arg("-o").arg(output_path);
                cmd
            }
        }
        "linux" => {
            let mut cmd = Command::new("cc");
            cmd.arg(object_path);
            cmd.arg(runtime_path);
            cmd.arg("-no-pie");
            cmd.arg("-o").arg(output_path);
            cmd
        }
        _ => {
            // macOS and other Unix-like platforms.
            let mut cmd = Command::new("cc");
            cmd.arg(object_path);
            cmd.arg(runtime_path);
            cmd.arg("-o").arg(output_path);
            cmd
        }
    }
}

/// Link an object file into an executable binary.
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_file(object_path: &Path, output_path: &Path) -> Result<PathBuf, CompileError> {
    let runtime_temp_file = RuntimeTempFile::create()?;

    let mut command = build_linker_command(
        std::env::consts::OS,
        object_path,
        runtime_temp_file.path(),
        output_path,
    );

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
    let module = match compile_to_module(&context, source) {
        Ok(module) => module,
        Err((report, normalized_source)) => {
            if report.len() == 1 {
                if let Some(&(_, CompilerError::Codegen(ref codegen_error))) =
                    report.entries().first()
                {
                    return Err(CompileError::Codegen(codegen_error.clone()));
                }
            }

            return Err(CompileError::Report {
                report,
                normalized_source,
            });
        }
    };

    let object_path = output_dir.join("program.o");
    let binary_path = output_dir.join("program");

    emit_object_file(&module, &object_path).map_err(CompileError::Codegen)?;
    link_object_file(&object_path, &binary_path)
}

#[cfg(test)]
mod tests {
    use super::{build_linker_command, compile_to_module};
    use crate::errors::reporter::CompilerError;
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
        let source = "entry main = f(): void => {\n\tlet x = @@@invalid\n}";
        let result = compile_to_module(&context, source);
        assert!(
            result.is_err(),
            "invalid tokens should surface as lexer diagnostics"
        );

        let Err((report, normalized_source)) = result else {
            return;
        };
        assert!(
            !report.is_empty(),
            "lexer diagnostics report should not be empty"
        );
        assert!(
            report
                .entries()
                .iter()
                .any(|entry| matches!(entry, &(_, CompilerError::Lexer(_)))),
            "invalid tokens should surface as lexer entries in CompilationErrorReport"
        );
        assert_eq!(
            normalized_source,
            source.replace('\t', "    "),
            "error payload should return the tab-normalized source"
        );
    }

    /// Type mismatch should fail after parse but before codegen.
    #[test]
    fn compile_to_module_type_error() {
        let context = Context::create();
        let source = "entry main = f(): void => { return 1 }";
        let result = compile_to_module(&context, source);
        assert!(
            result.is_err(),
            "semantic mismatches should fail compilation"
        );

        let Err((report, _source)) = result else {
            return;
        };
        assert!(
            report
                .entries()
                .iter()
                .any(|entry| matches!(entry, &(_, CompilerError::TypeChecker(_)))),
            "semantic mismatches should surface as type-checker entries in CompilationErrorReport"
        );
    }

    #[test]
    fn compile_to_module_collects_multiple_type_errors() {
        let context = Context::create();
        let source = "let bad_type = f(): int32 => { return true }\nlet bad_symbol = f(): int32 => { return missing_symbol }\nentry main = f(): void => { return void }";
        let result = compile_to_module(&context, source);
        assert!(
            result.is_err(),
            "source with multiple semantic issues should fail compilation"
        );

        let Err((report, _source)) = result else {
            return;
        };

        assert!(
            report.len() >= 2,
            "expected multiple diagnostics, got {}",
            report.len()
        );

        let type_mismatch_present = report.entries().iter().any(|entry| {
            matches!(
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(
                        crate::type_system::errors::TypeError::TypeMismatch { .. }
                    )
                )
            )
        });
        assert!(
            type_mismatch_present,
            "report should include a type mismatch diagnostic"
        );

        let symbol_not_found_present = report.entries().iter().any(|entry| {
            matches!(
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(
                        crate::type_system::errors::TypeError::SymbolNotFound { .. }
                    )
                )
            )
        });
        assert!(
            symbol_not_found_present,
            "report should include a symbol-not-found diagnostic"
        );
    }

    #[test]
    fn build_linker_command_linux_includes_no_pie() {
        let obj = std::path::Path::new("/tmp/prog.o");
        let rt = std::path::Path::new("/tmp/runtime.o");
        let out = std::path::Path::new("/tmp/prog");
        let cmd = build_linker_command("linux", obj, rt, out);
        let has_no_pie = cmd.get_args().any(|a| a.to_string_lossy() == "-no-pie");
        assert!(has_no_pie, "linux linker command must include -no-pie");
        assert_eq!(cmd.get_program(), "cc");
    }

    #[test]
    fn build_linker_command_macos_omits_no_pie() {
        let obj = std::path::Path::new("/tmp/prog.o");
        let rt = std::path::Path::new("/tmp/runtime.o");
        let out = std::path::Path::new("/tmp/prog");
        let cmd = build_linker_command("macos", obj, rt, out);
        let has_no_pie = cmd.get_args().any(|a| a.to_string_lossy() == "-no-pie");
        assert!(!has_no_pie, "macos linker command must NOT include -no-pie");
        assert_eq!(cmd.get_program(), "cc");
    }

    #[test]
    fn build_linker_command_windows_uses_appropriate_linker() {
        let obj = std::path::Path::new("C:\\tmp\\prog.obj");
        let rt = std::path::Path::new("C:\\tmp\\runtime.obj");
        let out = std::path::Path::new("C:\\tmp\\prog.exe");
        let cmd = build_linker_command("windows", obj, rt, out);
        let program = cmd.get_program().to_string_lossy();
        assert!(
            program == "link.exe" || program == "gcc",
            "windows linker must be link.exe or gcc, got: {program}"
        );
    }
}
