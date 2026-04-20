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

        let binary_result = compile_program(source_str.as_str(), temp_dir);
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

        let binary_result = compile_program(source_str.as_str(), temp_dir);
        let compile_error = match binary_result {
            Ok(_path) => {
                return Err("no-doc-comments source should fail to compile, but compilation succeeded".to_owned());
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
                    CompilerError::TypeChecker(TypeError::MissingDocComment { .. }
                        | TypeError::DocCommentTooShort { .. })
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
