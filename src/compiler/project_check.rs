//! Project checking helpers for editor integrations.

extern crate alloc;

use super::CompileError;
use super::compiler_helpers::{is_main_module_path, validate_entry_declarations_for_module};
use crate::ast::{Decl, Program};
use crate::error::LexError;
use crate::errors::reporter::CompilationErrorReport;
use crate::lexer::Lexer;
use crate::module_loader::{
    ModuleDiscoveryError, ModuleLoader, resolve_import_path, validate_module_file_role,
};
use crate::parser::Parser;
use crate::parser::errors::ParseError;
use crate::token::Position;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::{TypeError, Warning};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::path::{Path, PathBuf};

/// Diagnostics collected for one source file during editor-oriented checking.
#[derive(Debug)]
pub struct CheckFileDiagnostics {
    /// Source path that owns this diagnostic bundle.
    pub source_path: String,
    /// Tab-normalized source used for diagnostic span conversion.
    pub normalized_source: String,
    /// Error diagnostics collected for this source file.
    pub report: CompilationErrorReport,
    /// Non-fatal warnings collected for this source file.
    pub warnings: Vec<Warning>,
}

/// Project check output produced without code generation.
#[derive(Debug, Default)]
pub struct CheckProjectOutput {
    /// Per-file diagnostics in discovery/type-check order.
    pub files: Vec<CheckFileDiagnostics>,
}

impl CheckProjectOutput {
    /// Return true when no file has error-severity diagnostics.
    #[must_use]
    pub fn success(&self) -> bool {
        self.files.iter().all(|file| file.report.is_empty())
    }
}

/// Parse source into a program while preserving all lexer/parser diagnostics.
fn parse_source_to_program_with_report(
    source: &str,
) -> Result<(Program, String), (CompilationErrorReport, String)> {
    let normalized_source = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalized_source);
    let (tokens, lex_errors) = lexer.tokenize();
    let mut report = CompilationErrorReport::new();
    report.extend_lex_errors(lex_errors.errors);
    if !report.is_empty() {
        return Err((report, normalized_source));
    }

    let (program_option, parse_errors) = Parser::new(tokens).parse();
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

    Ok((program, normalized_source))
}

/// Convert a single compile error into a compilation report.
fn compile_error_to_report(error: CompileError) -> CompilationErrorReport {
    let mut report = CompilationErrorReport::new();
    match error {
        CompileError::Report {
            report: nested_report,
            ..
        } => return nested_report,
        CompileError::Lex(error) => report.push_lex_error(error),
        CompileError::Parse(error) => report.push_parse_error(error),
        CompileError::Type(error) => report.push_type_error(error),
        CompileError::Codegen(error) => report.push_codegen_error_full(error),
        CompileError::Io(error) => report.push_codegen_error(format!("io failed: {error}")),
        CompileError::Linker { stderr, .. } => report.push_codegen_error(stderr),
    }
    report
}

/// Add an error diagnostic bundle for one module to project-check output.
fn push_report_file(
    output: &mut CheckProjectOutput,
    source_path: &Path,
    normalized_source: String,
    report: CompilationErrorReport,
) {
    output.files.push(CheckFileDiagnostics {
        source_path: source_path.display().to_string(),
        normalized_source,
        report,
        warnings: Vec::new(),
    });
}

/// Check a project without code generation, returning per-file diagnostics for editor tooling.
///
/// # Errors
/// Returns unrecoverable project-discovery or filesystem failures that cannot be attached to a
/// source file.
#[expect(
    clippy::too_many_lines,
    reason = "Project checking mirrors the front-end half of project compilation"
)]
#[expect(
    clippy::needless_borrowed_reference,
    reason = "borrowed declaration matching follows compile_project conventions"
)]
pub fn check_project_frontend(project_path: &Path) -> Result<CheckProjectOutput, CompileError> {
    let project_root = if project_path.is_absolute() {
        project_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(CompileError::Io)?
            .join(project_path)
    };
    let project_dir = project_root.as_path();
    let mut output = CheckProjectOutput::default();
    let mut module_loader = ModuleLoader::new(project_dir.to_path_buf());
    let entry_module_path = project_dir.join("src").join("main.op");
    let discovered_module_paths = match module_loader.discover_all_modules(&entry_module_path) {
        Ok(paths) => paths,
        Err(ModuleDiscoveryError::Report {
            module_path,
            report,
            normalized_source,
        }) => {
            push_report_file(&mut output, &module_path, normalized_source, report);
            return Ok(output);
        }
        Err(ModuleDiscoveryError::Type(type_error)) => return Err(CompileError::Type(type_error)),
    };

    let mut parsed_programs: BTreeMap<PathBuf, Program> = BTreeMap::new();
    let mut normalized_sources: BTreeMap<PathBuf, String> = BTreeMap::new();

    for module_path in &discovered_module_paths {
        let module_source = module_loader
            .get_module_source(module_path)
            .map_err(CompileError::Io)?;
        let (program, normalized_source) = match parse_source_to_program_with_report(&module_source)
        {
            Ok(parsed) => parsed,
            Err((report, normalized_source)) => {
                push_report_file(&mut output, module_path, normalized_source, report);
                return Ok(output);
            }
        };
        if let Err(error) =
            validate_entry_declarations_for_module(project_dir, module_path, &program)
        {
            push_report_file(
                &mut output,
                module_path,
                normalized_source,
                compile_error_to_report(error),
            );
            return Ok(output);
        }
        if let Err(role_error) = validate_module_file_role(module_path, &program) {
            let mut report = CompilationErrorReport::new();
            report.extend_type_errors(vec![role_error]);
            push_report_file(&mut output, module_path, normalized_source, report);
            return Ok(output);
        }
        parsed_programs.insert(module_path.clone(), program);
        normalized_sources.insert(module_path.clone(), normalized_source);
    }

    let mut discovered_interfaces = BTreeMap::new();
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
        let mut first_report = CompilationErrorReport::new();
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
            first_report.extend_type_errors(filtered_errors);
        }

        if !first_report.is_empty() {
            output.files.push(CheckFileDiagnostics {
                source_path: first_module_path.display().to_string(),
                normalized_source: normalized_sources
                    .get(first_module_path)
                    .cloned()
                    .unwrap_or_default(),
                report: first_report,
                warnings: first_checker.warnings().to_vec(),
            });
            return Ok(output);
        }

        output.files.push(CheckFileDiagnostics {
            source_path: first_module_path.display().to_string(),
            normalized_source: normalized_sources
                .get(first_module_path)
                .cloned()
                .unwrap_or_default(),
            report: first_report,
            warnings: first_checker.warnings().to_vec(),
        });

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
                if matches!(import_source.as_str(), "standard" | "math" | "process") {
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
        let mut report = CompilationErrorReport::new();
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
            report.extend_type_errors(filtered_errors);
        }

        if !report.is_empty() {
            output.files.push(CheckFileDiagnostics {
                source_path: module_path.display().to_string(),
                normalized_source: normalized_sources
                    .get(module_path)
                    .cloned()
                    .unwrap_or_default(),
                report,
                warnings: checker.warnings().to_vec(),
            });
            return Ok(output);
        }

        output.files.push(CheckFileDiagnostics {
            source_path: module_path.display().to_string(),
            normalized_source: normalized_sources
                .get(module_path)
                .cloned()
                .unwrap_or_default(),
            report,
            warnings: checker.warnings().to_vec(),
        });

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
    }

    Ok(output)
}
