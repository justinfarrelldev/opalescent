#![cfg(feature = "integration")]

use super::*;

#[test]
fn immutability_compile_error() {
    let temp_dir = Path::new("test-projects/immutability/target");
    let prepare = prepare_dir(temp_dir);
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
            &TargetTriple::host(),
        );
        if binary_result.is_ok() {
            return Err(
                "immutability source should fail to compile (assignment to immutable variable), but compilation succeeded".to_owned()
            );
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
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
    let temp_dir = Path::new("test-projects/no-doc-comments/target");
    let prepare = prepare_dir(temp_dir);
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
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
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(
                        TypeError::MissingDocComment { .. } | TypeError::DocCommentTooShort { .. }
                    )
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

    let cleanup = cleanup_dir(temp_dir);
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
    let temp_dir = Path::new("test-projects/multiple-entry/target");
    let prepare = prepare_dir(temp_dir);
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

        let binary_result = compile_program(
            source_path,
            source_str.as_str(),
            temp_dir,
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
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(TypeError::DuplicateEntryPoint { .. })
                )
            )
        });

        if !has_expected_error {
            return Err("multiple-entry should emit DuplicateEntryPoint error".to_owned());
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(temp_dir);
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
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "type-in-regular-file-fail target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
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
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(TypeError::TypeDeclarationOutsideTypesFile { .. })
                )
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
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "value-in-types-file-fail target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
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
                entry,
                &(
                    _,
                    CompilerError::TypeChecker(TypeError::NonTypeDeclarationInTypesFile { .. })
                )
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
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/ref-basic");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "ref-basic target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        if binary_result.is_ok() {
            return Err("ref-basic project should fail to compile, but compilation succeeded".to_owned());
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "ref-basic target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(failure_message.is_empty(), "ref-basic should be rejected at compile time: {failure_message}");
}

#[test]
fn mutable_ref_fails_to_compile() {
    let cwd = std::env::current_dir();
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/mutable-ref");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "mutable-ref target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        if binary_result.is_ok() {
            return Err("mutable-ref project should fail to compile, but compilation succeeded".to_owned());
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "mutable-ref target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(failure_message.is_empty(), "mutable-ref should be rejected at compile time: {failure_message}");
}
