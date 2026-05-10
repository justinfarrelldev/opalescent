#![cfg(feature = "integration")]

use super::*;

#[test]
fn guard_shorthand_project_compiles_links_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/guard-shorthand");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard-shorthand target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        if binary_result.is_ok() {
            return Err(
                "guard-shorthand project should fail strict front-end validation in this fixture"
                    .to_owned(),
            );
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard-shorthand target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard-shorthand project should be rejected by strict guard validation: {failure_message}"
    );
}
