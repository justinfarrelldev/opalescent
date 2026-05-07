//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

/// Helper functions for compiler pipeline orchestration.
mod compiler_helpers;

use crate::ast::{Decl, Expr, NodeId, Program};
use crate::build_system::targets::{TargetTriple, executable_filename, object_file_extension};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::{codegen_function_declaration, codegen_import_declaration};
use crate::error::LexError;
use crate::errors::reporter::{CompilationErrorReport, CompilerError};
use crate::lexer::Lexer;
use crate::module_loader::{
    ModuleLoader, is_types_file, resolve_import_path, validate_module_file_role,
};
use crate::parser::Parser;
use crate::parser::errors::ParseError;
use crate::token::Position;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::{collections::BTreeMap, vec::Vec};
use compiler_helpers::{
    collect_imported_symbol_signatures, compile_checked_program_to_module, is_main_module_path,
    lambda_body_to_function_body, parse_source_to_program, validate_entry_declarations_for_module,
};
use inkwell::context::Context;
use inkwell::module::Module;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Build the structured linker error used when Linux→MSVC xwin paths are missing.
#[cfg(not(windows))]
fn missing_xwin_env_error() -> CompileError {
    CompileError::Linker {
        stderr: String::from(
            "missing XWIN_CACHE or OPAL_XWIN_SYSROOT for Linux→MSVC cross-compilation (point to xwin splat directory)",
        ),
    }
}

/// Embedded C runtime source used during native linking.
const RUNTIME_SOURCE: &str = concat!(
    "#define OPAL_RUNTIME_H\n#define OPAL_RC_H\n",
    include_str!("../runtime/opal_runtime.c"),
    "\n#undef OPAL_RC_H\n#undef OPAL_RUNTIME_H\n",
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
    "\n",
    include_str!("../runtime/opal_bytes.c"),
    "\n",
    include_str!("../runtime/opal_rc.c"),
    "\n",
    include_str!("../runtime/opal_fs.c"),
);

/// Embedded C runtime headers used during native linking.
const OPAL_PORTABILITY_H: &[u8] = include_bytes!("../runtime/opal_portability.h");
/// Embedded reference-counting runtime header.
const OPAL_RC_H: &[u8] = include_bytes!("../runtime/opal_rc.h");
/// Embedded public runtime API header.
const OPAL_RUNTIME_H: &[u8] = include_bytes!("../runtime/opal_runtime.h");
/// Embedded filesystem runtime error discriminant header.
const OPAL_FS_ERRORS_H: &[u8] = include_bytes!("../runtime/opal_fs_errors.h");

/// Temporary runtime source file materialized for the system C compiler.
struct RuntimeTempFile {
    /// Path to the temporary directory containing the runtime source and headers.
    dir: PathBuf,
    /// Path to the generated temporary C runtime source file.
    source_file: PathBuf,
}

impl RuntimeTempFile {
    /// Create a uniquely named temporary directory with runtime source and headers.
    fn create() -> Result<Self, CompileError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| CompileError::Io(std::io::Error::other(error)))?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "opal_runtime_{}_{}/",
            std::process::id(),
            timestamp
        ));
        std::fs::create_dir_all(&dir).map_err(CompileError::Io)?;

        // Write the C runtime source file
        let source_file = dir.join("opal_runtime.c");
        std::fs::write(&source_file, RUNTIME_SOURCE).map_err(CompileError::Io)?;

        // Write the header files
        std::fs::write(dir.join("opal_portability.h"), OPAL_PORTABILITY_H)
            .map_err(CompileError::Io)?;
        std::fs::write(dir.join("opal_rc.h"), OPAL_RC_H).map_err(CompileError::Io)?;
        std::fs::write(dir.join("opal_runtime.h"), OPAL_RUNTIME_H).map_err(CompileError::Io)?;
        std::fs::write(dir.join("opal_fs_errors.h"), OPAL_FS_ERRORS_H).map_err(CompileError::Io)?;

        Ok(Self { dir, source_file })
    }

    /// Borrow the filesystem path for this temporary runtime source file.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "PathBuf deref to Path is not const on stable"
    )]
    fn path(&self) -> &Path {
        &self.source_file
    }

    /// Borrow the filesystem path for the temporary directory containing headers.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "PathBuf deref to Path is not const on stable"
    )]
    fn include_dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for RuntimeTempFile {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.dir));
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

/// Compile source text into an LLVM module using an explicit target triple.
///
/// # Errors
/// Returns a multi-error report plus normalized source when any stage fails.
#[expect(
    clippy::too_many_lines,
    reason = "Complex compilation pipeline with multiple stages"
)]
pub fn compile_to_module_for_target<'context>(
    context: &'context Context,
    source_path: &Path,
    source: &str,
    target: &TargetTriple,
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

    if let Err(role_error) = validate_module_file_role(source_path, &program) {
        report.extend_type_errors(vec![role_error]);
        return Err((report, normalized_source));
    }

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

    let codegen_context = CodegenContext::for_triple(context, "opalescent_module", target)
        .map_err(|error| {
            let mut codegen_report = CompilationErrorReport::new();
            codegen_report.push_codegen_error_full(CodegenError::new(format!("{error:?}")));
            (codegen_report, normalized_source.clone())
        })?;
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

/// Compile source text into an LLVM module using the host target triple.
///
/// # Errors
/// Returns a multi-error report plus normalized source when any stage fails.
pub fn compile_to_module<'context>(
    context: &'context Context,
    source_path: &Path,
    source: &str,
) -> Result<Module<'context>, (CompilationErrorReport, String)> {
    compile_to_module_for_target(context, source_path, source, &TargetTriple::host())
}

/// Emits an LLVM module to an object file for the specified target.
pub fn emit_object_file(
    module: &Module<'_>,
    path: &std::path::Path,
    target: &TargetTriple,
) -> Result<(), CodegenError> {
    use inkwell::targets::{InitializationConfig, Target};

    Target::initialize_all(&InitializationConfig::default());

    let llvm_triple = target.to_llvm_string();
    let triple = inkwell::targets::TargetTriple::create(&llvm_triple);
    let target_machine = Target::from_triple(&triple)
        .map_err(|error| CodegenError::new(format!("failed to resolve LLVM target: {error}")))?
        .create_target_machine(
            &triple,
            "generic",
            "",
            inkwell::OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "failed to create LLVM target machine for object emission",
            ))
        })?;

    target_machine
        .write_to_file(module, inkwell::targets::FileType::Object, path)
        .map_err(|error| CodegenError::new(format!("failed to emit object file: {error}")))
}

/// Compile the runtime C source file to an object file for MSVC targets.
///
/// On Windows, uses `cl.exe`; on Linux (cross-compile), uses `clang-cl`.
/// The compiled `.obj` file is placed in the same directory as the source.
///
/// # Errors
/// Returns `CompileError` if the C compiler fails or is not found.
fn compile_runtime_c_to_obj(
    runtime_c_path: &Path,
    include_dir: &Path,
) -> Result<PathBuf, CompileError> {
    let obj_path = runtime_c_path.with_extension("obj");

    let cc_bin = std::env::var("OPAL_MSVC_CC").unwrap_or_else(|_| {
        #[cfg(windows)]
        {
            "cl.exe".to_owned()
        }
        #[cfg(not(windows))]
        {
            "clang-cl".to_owned()
        }
    });

    let mut cmd = std::process::Command::new(&cc_bin);
    cmd.arg("/c");
    cmd.arg(format!("/I{}", include_dir.display()));
    cmd.arg(runtime_c_path);
    cmd.arg(format!("/Fo{}", obj_path.display()));

    #[cfg(not(windows))]
    {
        if let Ok(cflags) = std::env::var("CFLAGS_x86_64_pc_windows_msvc") {
            for flag in cflags.split_whitespace() {
                cmd.arg(flag);
            }
        }
    }

    let output = cmd.output().map_err(CompileError::Io)?;
    if output.status.success() {
        return Ok(obj_path);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Err(CompileError::Linker {
        stderr: format!("failed to compile runtime C to .obj: {stderr}"),
    })
}

/// Build a platform-appropriate linker [`Command`] for the given object and output paths.
///
/// Platform behaviour:
/// - **Linux**: `cc -no-pie <objs...> <runtime> -o <out>` — PIE relocation workaround required
/// - **macOS**: `cc <objs...> <runtime> -o <out>` — `-no-pie` not needed and may be unsupported
/// - **Windows (MSVC)**: `link.exe /OUT:<out> <objs...> <runtime>` — MSVC linker syntax
/// - **Windows (other)**: `gcc <objs...> <runtime> -o <out>` — MinGW / Cygwin fallback
///
/// The `target` parameter specifies the build target triple.
#[must_use]
pub fn build_linker_command(
    target: &TargetTriple,
    object_paths: &[PathBuf],
    runtime_path: &Path,
    output_path: &Path,
    include_dir: &Path,
) -> Command {
    let mut linker_cmd = crate::build_system::LinkerCommand::new(target, output_path.to_path_buf());
    for obj_path in object_paths {
        linker_cmd = linker_cmd.with_input(obj_path.clone());
    }
    linker_cmd
        .with_runtime(runtime_path.to_path_buf())
        .with_include_dir(include_dir.to_path_buf())
        .build()
}

/// Link multiple object files into an executable binary.
///
/// # Errors
/// Returns `CompileError` if the linker process fails or produces errors.
pub fn link_object_files(
    object_paths: &[PathBuf],
    output_path: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    #[cfg(not(windows))]
    if target.is_windows_msvc()
        && std::env::var("XWIN_CACHE").is_err()
        && std::env::var("OPAL_XWIN_SYSROOT").is_err()
    {
        return Err(missing_xwin_env_error());
    }

    let runtime_temp_file = RuntimeTempFile::create()?;

    // For MSVC targets, compile the runtime .c to .obj first
    let runtime_path = if target.is_windows_msvc() {
        compile_runtime_c_to_obj(runtime_temp_file.path(), runtime_temp_file.include_dir())?
    } else {
        runtime_temp_file.path().to_path_buf()
    };

    let mut command = build_linker_command(
        target,
        object_paths,
        &runtime_path,
        output_path,
        runtime_temp_file.include_dir(),
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
pub fn link_object_file(
    object_path: &Path,
    output_path: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    link_object_files(&[object_path.to_path_buf()], output_path, target)
}

/// Compile Opalescent source to a native binary.
///
/// Creates `program.o` and `program` inside `output_dir`.
///
/// # Errors
/// Returns `CompileError` at any pipeline stage.
pub fn compile_program(
    source_path: &Path,
    source: &str,
    output_dir: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
    std::fs::create_dir_all(output_dir).map_err(CompileError::Io)?;

    let context = Context::create();
    let module = match compile_to_module_for_target(&context, source_path, source, target) {
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

    let object_ext = object_file_extension(target);
    let object_path = output_dir.join(format!("program{object_ext}"));
    let binary_name = executable_filename("program", target);
    let binary_path = output_dir.join(binary_name);

    emit_object_file(&module, &object_path, target).map_err(CompileError::Codegen)?;
    link_object_file(&object_path, &binary_path, target)
}

/// Compile Opalescent source to a native binary using the host target triple.
///
/// # Deprecated
/// Prefer `compile_program` with an explicit `target` parameter.
/// This shim exists for backward compatibility.
///
/// # Errors
/// Returns `CompileError` at any pipeline stage.
pub fn compile_program_host(
    source_path: &Path,
    source: &str,
    output_dir: &Path,
) -> Result<PathBuf, CompileError> {
    let target = TargetTriple {
        arch: if cfg!(target_arch = "aarch64") {
            crate::build_system::targets::Architecture::Aarch64
        } else {
            crate::build_system::targets::Architecture::X86_64
        },
        platform: if cfg!(target_os = "windows") {
            crate::build_system::targets::Platform::Windows
        } else if cfg!(target_os = "macos") {
            crate::build_system::targets::Platform::MacOs
        } else {
            crate::build_system::targets::Platform::Linux
        },
        env: if cfg!(target_env = "msvc") {
            Some(crate::build_system::targets::TripleEnv::Msvc)
        } else if cfg!(target_env = "musl") {
            Some(crate::build_system::targets::TripleEnv::Musl)
        } else {
            Some(crate::build_system::targets::TripleEnv::Gnu)
        },
    };
    compile_program(source_path, source, output_dir, &target)
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
#[expect(
    clippy::needless_borrowed_reference,
    reason = "borrowed declaration matching avoids the repo's pattern-type-mismatch lint"
)]
pub fn compile_project(
    project_dir: &Path,
    output_dir: &Path,
    target: &TargetTriple,
) -> Result<PathBuf, CompileError> {
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
        if let Err(role_error) = validate_module_file_role(module_path, &program) {
            let mut report = CompilationErrorReport::new();
            report.extend_type_errors(vec![role_error]);
            return Err(CompileError::Report {
                report,
                normalized_source: module_source.replace('\t', "    "),
            });
        }
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

                let resolved_path_result = resolve_import_path(module_path, import_source.as_str());
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
        if is_types_file(module_path) {
            continue;
        }

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
        let llvm_module =
            compile_checked_program_to_module(&context, program, imported_signatures, target)
                .map_err(CompileError::Codegen)?;

        let object_ext = object_file_extension(target);
        let object_path = output_dir.join(format!("module_{index}{object_ext}"));
        emit_object_file(&llvm_module, &object_path, target).map_err(CompileError::Codegen)?;
        object_paths.push(object_path);
    }

    let binary_name = executable_filename("program", target);
    let binary_path = output_dir.join(binary_name);
    link_object_files(&object_paths, &binary_path, target)
}

#[cfg(test)]
mod tests;
