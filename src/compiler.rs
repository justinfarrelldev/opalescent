//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

use crate::ast::{Decl, Expr, ImportItem, LabeledValue, LambdaBody, NodeId, Program, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::errors::reporter::{CompilationErrorReport, CompilerError};
use crate::lexer::Lexer;
use crate::module_loader::{resolve_import_path, ModuleLoader};
use crate::parser::errors::ParseError;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::{collections::BTreeMap, vec::Vec};
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
    #[error("type checking failed: {0}")]
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
                        codegen_report.push_codegen_error_full(error);
                        (codegen_report, normalized_source.clone())
                    },
                )?;
            }
            Decl::Function { .. } => {
                codegen_function_declaration(&codegen_context, &mut env, declaration).map_err(
                    |error| {
                        let mut codegen_report = CompilationErrorReport::new();
                        codegen_report.push_codegen_error_full(error);
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
                        codegen_report.push_codegen_error_full(error);
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

/// Parse source text into an AST program using the compiler lexer/parser pipeline.
///
/// # Errors
/// Returns `CompileError::Lex` or `CompileError::Parse` when front-end parsing fails.
fn parse_source_to_program(source: &str) -> Result<Program, CompileError> {
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

/// Return true when this module path is `src/main.op` relative to project root.
fn is_main_module_path(project_dir: &Path, module_path: &Path) -> bool {
    let expected_main = project_dir.join("src").join("main.op");
    canonicalize_or_original_path(&expected_main) == canonicalize_or_original_path(module_path)
}

/// Canonicalize a path, or return the original path when canonicalization fails.
fn canonicalize_or_original_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_io_err| path.to_path_buf())
}

/// Validate that `entry` appears only in `src/main.op`.
///
/// # Errors
/// Returns `TypeError::EntryNotInMainModule` when `entry` is used in a non-main module.
fn validate_entry_declarations_for_module(
    project_dir: &Path,
    module_path: &Path,
    program: &Program,
) -> Result<(), CompileError> {
    if is_main_module_path(project_dir, module_path) {
        return Ok(());
    }

    for declaration in &program.declarations {
        if let &Decl::Function {
            is_entry,
            span,
            ..
        } = declaration
        {
            if is_entry {
                return Err(CompileError::Type(TypeError::EntryNotInMainModule {
                    file_path: module_path.display().to_string(),
                    span: TypeError::span_from_span(span),
                }));
            }
        }
    }

    Ok(())
}

/// Collect imported symbol signatures for codegen from checker-resolved module interfaces.
fn collect_imported_symbol_signatures(
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

/// Lower a previously type-checked program to an LLVM module.
///
/// # Errors
/// Returns `CodegenError` when LLVM lowering fails.
fn compile_checked_program_to_module<'context>(
    context: &'context Context,
    program: &Program,
    imported_signatures: BTreeMap<String, CoreType>,
) -> Result<Module<'context>, CodegenError> {
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
/// - **Linux**: `cc -no-pie <objs...> <runtime> -o <out>` — PIE relocation workaround required
/// - **macOS**: `cc <objs...> <runtime> -o <out>` — `-no-pie` not needed and may be unsupported
/// - **Windows (MSVC)**: `link.exe /OUT:<out> <objs...> <runtime>` — MSVC linker syntax
/// - **Windows (other)**: `gcc <objs...> <runtime> -o <out>` — MinGW / Cygwin fallback
///
/// The `target_os` parameter accepts the same values as [`std::env::consts::OS`].
#[must_use]
pub fn build_linker_command(
    target_os: &str,
    object_paths: &[PathBuf],
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
                for obj_path in object_paths {
                    cmd.arg(obj_path);
                }
                cmd.arg(runtime_path);
                cmd
            } else {
                let mut cmd = Command::new("gcc");
                for obj_path in object_paths {
                    cmd.arg(obj_path);
                }
                cmd.arg(runtime_path);
                cmd.arg("-o").arg(output_path);
                cmd
            }
        }
        "linux" => {
            let mut cmd = Command::new("cc");
            for obj_path in object_paths {
                cmd.arg(obj_path);
            }
            cmd.arg(runtime_path);
            cmd.arg("-no-pie");
            cmd.arg("-o").arg(output_path);
            cmd
        }
        _ => {
            // macOS and other Unix-like platforms.
            let mut cmd = Command::new("cc");
            for obj_path in object_paths {
                cmd.arg(obj_path);
            }
            cmd.arg(runtime_path);
            cmd.arg("-o").arg(output_path);
            cmd
        }
    }
}

/// Link multiple object files into an executable binary.
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_files(object_paths: &[PathBuf], output_path: &Path) -> Result<PathBuf, CompileError> {
    let runtime_temp_file = RuntimeTempFile::create()?;

    let mut command = build_linker_command(
        std::env::consts::OS,
        object_paths,
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

/// Link an object file into an executable binary.
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_file(object_path: &Path, output_path: &Path) -> Result<PathBuf, CompileError> {
    link_object_files(&[object_path.to_path_buf()], output_path)
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

/// Compile a full Opalescent project rooted at `project_dir` into `output_dir/program`.
///
/// The compilation pipeline discovers all source modules reachable from `src/main.op`,
/// validates entry-point placement, type-checks modules in dependency order with
/// accumulated module interfaces, emits one object file per source module, and links
/// all objects into a single executable.
///
/// # Errors
/// Returns `CompileError` when discovery, parsing, validation, type-checking, codegen,
/// object emission, or linking fails.
#[expect(
    clippy::too_many_lines,
    reason = "Project compilation orchestrates discovery, parsing, typing, codegen, and linking in one flow"
)]
pub fn compile_project(project_dir: &Path, output_dir: &Path) -> Result<PathBuf, CompileError> {
    std::fs::create_dir_all(output_dir).map_err(CompileError::Io)?;

    let mut module_loader = ModuleLoader::new(project_dir.to_path_buf());
    let entry_module_path = project_dir.join("src").join("main.op");
    let discovered_module_paths = module_loader
        .discover_all_modules(&entry_module_path)
        .map_err(CompileError::Type)?;

    let mut parsed_programs: BTreeMap<PathBuf, Program> = BTreeMap::new();
    for module_path in &discovered_module_paths {
        let module_source = module_loader
            .get_module_source(module_path)
            .map_err(CompileError::Io)?;
        let program = parse_source_to_program(&module_source)?;
        validate_entry_declarations_for_module(project_dir, module_path, &program)?;
        parsed_programs.insert(module_path.clone(), program);
    }

    let mut discovered_interfaces = BTreeMap::new();
    let mut imported_signatures_by_module: BTreeMap<PathBuf, BTreeMap<String, CoreType>> =
        BTreeMap::new();

    if let Some(first_module_path) = discovered_module_paths.first() {
        let Some(first_program) = parsed_programs.get(first_module_path) else {
            return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "internal error: parsed program missing for '{}'",
                    first_module_path.display()
                ),
                span: TypeError::unknown_span(),
            }));
        };

        let mut first_checker = TypeChecker::new();
        first_checker.set_current_module_path(first_module_path.display().to_string());
        let first_type_check_result = first_checker.type_check_program(first_program);
        if let Err(type_errors) = first_type_check_result {
            let first_module_is_main = is_main_module_path(project_dir, first_module_path);
            let filtered_errors: Vec<TypeError> = if first_module_is_main {
                type_errors
            } else {
                type_errors
                    .into_iter()
                    .filter(|type_error| {
                        !matches!(type_error, &TypeError::MissingEntryPoint { .. })
                    })
                    .collect()
            };

            if let Some(first_error) = filtered_errors.into_iter().next() {
                return Err(CompileError::Type(first_error));
            }
        }

        let first_module_key = first_module_path.display().to_string();
        let Some(first_module_interface) = first_checker.module_interface(&first_module_key) else {
            return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "internal error: module interface missing for '{}'",
                    first_module_path.display()
                ),
                span: TypeError::unknown_span(),
            }));
        };
        discovered_interfaces.insert(first_module_path.clone(), first_module_interface);

        let first_imported_signatures =
            collect_imported_symbol_signatures(&first_checker, first_program);
        imported_signatures_by_module.insert(first_module_path.clone(), first_imported_signatures);
    }

    for module_path in discovered_module_paths.iter().skip(1) {
        let Some(program) = parsed_programs.get(module_path) else {
            return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "internal error: parsed program missing for '{}'",
                    module_path.display()
                ),
                span: TypeError::unknown_span(),
            }));
        };

        let mut checker = TypeChecker::new();
        checker.set_current_module_path(module_path.display().to_string());
        for discovered_interface in discovered_interfaces.values() {
            checker.register_module_interface(discovered_interface.clone());
        }

        for declaration in &program.declarations {
            if let &Decl::Import {
                source: ref import_source,
                ..
            } = declaration
            {
                if matches!(import_source.as_str(), "standard" | "math") {
                    continue;
                }

                let resolved_path_result = resolve_import_path(module_path, import_source);
                let Ok(resolved_path) = resolved_path_result else {
                    continue;
                };
                if let Some(discovered_interface) = discovered_interfaces.get(&resolved_path) {
                    let mut source_keyed_interface = discovered_interface.clone();
                    source_keyed_interface.module_path.clone_from(import_source);
                    checker.register_module_interface(source_keyed_interface);
                }
            }
        }

        let type_check_result = checker.type_check_program(program);
        if let Err(type_errors) = type_check_result {
            let module_is_main = is_main_module_path(project_dir, module_path);
            let filtered_errors: Vec<TypeError> = if module_is_main {
                type_errors
            } else {
                type_errors
                    .into_iter()
                    .filter(|type_error| {
                        !matches!(type_error, &TypeError::MissingEntryPoint { .. })
                    })
                    .collect()
            };

            if let Some(first_error) = filtered_errors.into_iter().next() {
                return Err(CompileError::Type(first_error));
            }
        }

        let module_path_key = module_path.display().to_string();
        let Some(module_interface) = checker.module_interface(&module_path_key) else {
            return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "internal error: module interface missing for '{}'",
                    module_path.display()
                ),
                span: TypeError::unknown_span(),
            }));
        };
        discovered_interfaces.insert(module_path.clone(), module_interface);

        let imported_signatures = collect_imported_symbol_signatures(&checker, program);
        imported_signatures_by_module.insert(module_path.clone(), imported_signatures);
    }

    let mut object_paths: Vec<PathBuf> = Vec::new();
    for (index, module_path) in discovered_module_paths.iter().enumerate() {
        let Some(program) = parsed_programs.get(module_path) else {
            return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "internal error: missing parsed program for '{}' during codegen",
                    module_path.display()
                ),
                span: TypeError::unknown_span(),
            }));
        };
        let imported_signatures = imported_signatures_by_module
            .get(module_path)
            .cloned()
            .unwrap_or_default();

        let context = Context::create();
        let llvm_module = compile_checked_program_to_module(&context, program, imported_signatures)
            .map_err(CompileError::Codegen)?;

        let object_path = output_dir.join(format!("module_{index}.o"));
        emit_object_file(&llvm_module, &object_path).map_err(CompileError::Codegen)?;
        object_paths.push(object_path);
    }

    let binary_path = output_dir.join("program");
    link_object_files(&object_paths, &binary_path)
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
        let source = "##\n  Description: Entry test program with valid docs for compilation\n##\nentry main = f(): void => { return void }";
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
        let source = "##\n  Description: Entry lexical error sample with valid docs\n##\nentry main = f(): void => {\n\tlet x = @@@invalid\n}";
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
        let source = "##\n  Description: Entry type mismatch sample with valid docs\n##\nentry main = f(): void => { return 1 }";
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
        let source = "let bad_type = f(): int32 => { return true }\nlet bad_symbol = f(): int32 => { return missing_symbol }\n##\n  Description: Entry multi-error sample with valid docs\n##\nentry main = f(): void => { return void }";
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
        let cmd = build_linker_command("linux", &[obj.to_path_buf()], rt, out);
        let has_no_pie = cmd.get_args().any(|a| a.to_string_lossy() == "-no-pie");
        assert!(has_no_pie, "linux linker command must include -no-pie");
        assert_eq!(cmd.get_program(), "cc");
    }

    #[test]
    fn build_linker_command_macos_omits_no_pie() {
        let obj = std::path::Path::new("/tmp/prog.o");
        let rt = std::path::Path::new("/tmp/runtime.o");
        let out = std::path::Path::new("/tmp/prog");
        let cmd = build_linker_command("macos", &[obj.to_path_buf()], rt, out);
        let has_no_pie = cmd.get_args().any(|a| a.to_string_lossy() == "-no-pie");
        assert!(!has_no_pie, "macos linker command must NOT include -no-pie");
        assert_eq!(cmd.get_program(), "cc");
    }

    #[test]
    fn build_linker_command_windows_uses_appropriate_linker() {
        let obj = std::path::Path::new("C:\\tmp\\prog.obj");
        let rt = std::path::Path::new("C:\\tmp\\runtime.obj");
        let out = std::path::Path::new("C:\\tmp\\prog.exe");
        let cmd = build_linker_command("windows", &[obj.to_path_buf()], rt, out);
        let program = cmd.get_program().to_string_lossy();
        assert!(
            program == "link.exe" || program == "gcc",
            "windows linker must be link.exe or gcc, got: {program}"
        );
    }
}
