#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;

#[test]
fn immutability_compile_error() {
    let temp_dir = unique_probe_target_dir("immutability-compile-error");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "immutability target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/immutability/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "immutability source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );
        if binary_result.is_ok() {
            return Err(
                "immutability source should fail to compile (assignment to immutable variable), but compilation succeeded".to_owned()
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "immutability target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "immutability source should be rejected at compile time: {failure_message}"
    );
}

#[test]
fn no_doc_comments_fails_to_compile() {
    let temp_dir = unique_probe_target_dir("no-doc-comments");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "no-doc-comments target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/no-doc-comments/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "no-doc-comments source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "no-doc-comments source should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let CompileError::Report { report, .. } = compile_error else {
            return Err(format!(
                "no-doc-comments should fail in type-check report, got different error: {compile_error}"
            ));
        };

        let has_expected_error = report.entries().iter().any(|entry| {
            matches!(
                &entry.1,
                &CompilerError::TypeChecker(
                    TypeError::MissingDocComment { .. } | TypeError::DocCommentTooShort { .. }
                )
            )
        });

        if !has_expected_error {
            return Err(
                "no-doc-comments should emit MissingDocComment or DocCommentTooShort".to_owned(),
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "no-doc-comments target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "no-doc-comments source should be rejected for missing/short docs: {failure_message}"
    );
}

#[test]
fn multiple_entry_fails_to_compile() {
    let temp_dir = unique_probe_target_dir("multiple-entry");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "multiple-entry target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/multiple-entry/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "multiple-entry source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "multiple-entry source should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let CompileError::Report { report, .. } = compile_error else {
            return Err(format!(
                "multiple-entry should fail in type-check report, got different error: {compile_error}"
            ));
        };

        let has_expected_error = report.entries().iter().any(|entry| {
            matches!(
                &entry.1,
                &CompilerError::TypeChecker(TypeError::DuplicateEntryPoint { .. })
            )
        });

        if !has_expected_error {
            return Err("multiple-entry should emit DuplicateEntryPoint error".to_owned());
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "multiple-entry target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "multiple-entry source should be rejected for duplicate entry points: {failure_message}"
    );
}

#[test]
fn type_declaration_in_regular_file_is_rejected() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/type-in-regular-file-fail");
    let temp_dir = unique_probe_target_dir("type-in-regular-file-fail");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "type-in-regular-file-fail target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "type-in-regular-file-fail project should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let CompileError::Report { report, .. } = compile_error else {
            return Err(format!(
                "type-in-regular-file-fail should fail with CompileError::Report, got: {compile_error}"
            ));
        };

        let has_expected_error = report.entries().iter().any(|entry| {
            matches!(
                &entry.1,
                &CompilerError::TypeChecker(TypeError::TypeDeclarationOutsideTypesFile { .. })
            )
        });

        if !has_expected_error {
            return Err(
                "type-in-regular-file-fail should emit TypeDeclarationOutsideTypesFile".to_owned(),
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "type-in-regular-file-fail target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "type-in-regular-file-fail source should be rejected with TypeDeclarationOutsideTypesFile: {failure_message}"
    );
}

#[test]
fn value_declaration_in_types_file_is_rejected() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/value-in-types-file-fail");
    let temp_dir = unique_probe_target_dir("value-in-types-file-fail");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "value-in-types-file-fail target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err(
                    "value-in-types-file-fail project should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let CompileError::Report { report, .. } = compile_error else {
            return Err(format!(
                "value-in-types-file-fail should fail with CompileError::Report, got: {compile_error}"
            ));
        };

        let has_expected_error = report.entries().iter().any(|entry| {
            matches!(
                &entry.1,
                &CompilerError::TypeChecker(TypeError::NonTypeDeclarationInTypesFile { .. })
            )
        });

        if !has_expected_error {
            return Err(
                "value-in-types-file-fail should emit NonTypeDeclarationInTypesFile".to_owned(),
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "value-in-types-file-fail target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "value-in-types-file-fail source should be rejected with NonTypeDeclarationInTypesFile: {failure_message}"
    );
}

#[test]
fn ref_basic_fails_to_compile() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/ref-basic");
    let temp_dir = unique_probe_target_dir("ref-basic");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "ref-basic target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        if binary_result.is_ok() {
            return Err(
                "ref-basic project should fail to compile, but compilation succeeded".to_owned(),
            );
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "ref-basic target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "ref-basic should be rejected at compile time: {failure_message}"
    );
}

#[test]
fn mutable_ref_fails_to_compile() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/mutable-ref");
    let temp_dir = unique_probe_target_dir("mutable-ref");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "mutable-ref target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        if binary_result.is_ok() {
            return Err(
                "mutable-ref project should fail to compile, but compilation succeeded".to_owned(),
            );
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "mutable-ref target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "mutable-ref should be rejected at compile time: {failure_message}"
    );
}

#[test]
fn ambiguous_guard_if_project_fails_with_miette_help() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/ambiguous-guard-if");
    let temp_dir = unique_probe_target_dir("ambiguous-guard-if");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "ambiguous-guard-if target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = project_dir.join("src/main.op");
        let source_result = fs::read_to_string(&source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "ambiguous-guard-if source file should be readable: {error}"
                ));
            }
        };

        let binary_result = compile_program_for_tests(
            &source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );
        let compile_failure = match binary_result {
            Ok(_path) => {
                return Err(
                    "ambiguous-guard-if source should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        validate_ambiguous_guard_if_failure(compile_failure)
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "ambiguous-guard-if target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "ambiguous-guard-if should fail with GuardAmbiguousIfElse and focused miette help: {failure_message}"
    );
}

fn validate_ambiguous_guard_if_failure(compile_failure: CompileError) -> Result<(), String> {
    let CompileError::Report {
        report,
        normalized_source,
    } = compile_failure
    else {
        return Err(format!(
            "ambiguous-guard-if should fail with CompileError::Report, got: {compile_failure}"
        ));
    };

    let entries = report.entries();
    ensure_single_parser_recovery_diagnostic(entries)?;

    let Some(parser_error) = find_guard_ambiguous_if_else_error(entries) else {
        return Err(format!(
            "ambiguous-guard-if should emit GuardAmbiguousIfElse parser diagnostic, got: {entries:?}"
        ));
    };

    let rendered = opalescent::errors::formatter::format_diagnostic(
        opalescent::errors::formatter::CompilerPhase::Parser,
        parser_error,
    );
    ensure_guard_help_text(&rendered)?;

    if !normalized_source.contains("entry main") {
        return Err(
            "ambiguous-guard-if normalized source should still include nearby valid declaration for recovery checks"
                .to_owned(),
        );
    }

    Ok(())
}

fn ensure_single_parser_recovery_diagnostic<T: std::fmt::Debug>(
    entries: &[(T, CompilerError)],
) -> Result<(), String> {
    if entries.len() != 1 {
        return Err(format!(
            "ambiguous-guard-if should emit exactly one diagnostic after parser recovery, got {} entries: {:?}",
            entries.len(),
            entries
        ));
    }

    let parser_error_count = entries
        .iter()
        .filter(|entry| matches!(&entry.1, &CompilerError::Parser(_)))
        .count();
    if parser_error_count != 1 {
        return Err(format!(
            "ambiguous-guard-if should emit exactly one parser diagnostic, got {parser_error_count}: {entries:?}"
        ));
    }

    Ok(())
}

fn find_guard_ambiguous_if_else_error<T>(entries: &[(T, CompilerError)]) -> Option<&CompilerError> {
    entries.iter().find_map(|entry| {
        matches!(
            &entry.1,
            &CompilerError::Parser(
                opalescent::parser::errors::ParseError::GuardAmbiguousIfElse { .. }
            )
        )
        .then_some(&entry.1)
    })
}

fn ensure_guard_help_text(rendered: &str) -> Result<(), String> {
    if !rendered.contains("opalescent::parser::guard_ambiguous_if_else") {
        return Err(format!(
            "ambiguous-guard-if rendered diagnostic should include dedicated parser code, got: {rendered}"
        ));
    }

    if !rendered.contains("parentheses") {
        return Err(format!(
            "ambiguous-guard-if rendered diagnostic should mention parentheses help, got: {rendered}"
        ));
    }

    Ok(())
}
