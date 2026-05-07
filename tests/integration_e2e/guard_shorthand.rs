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
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "guard-shorthand project should compile into a binary: {error}"
                ));
            }
        };

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "guard-shorthand compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("GUARD_SHORTHAND_SUCCESS=ok") {
            return Err(format!(
                "guard-shorthand stdout should contain 'GUARD_SHORTHAND_SUCCESS=ok', got: '{stdout}'"
            ));
        }
        if !stdout.contains("GUARD_SHORTHAND_ERROR=handled") {
            return Err(format!(
                "guard-shorthand stdout should contain 'GUARD_SHORTHAND_ERROR=handled', got: '{stdout}'"
            ));
        }
        if !stdout.contains("GUARD_NAMED_BINDING=41") {
            return Err(format!(
                "guard-shorthand stdout should contain 'GUARD_NAMED_BINDING=41', got: '{stdout}'"
            ));
        }
        if stdout.contains("UNEXPECTED_SHORTHAND_SUCCESS_ERROR=") {
            return Err(format!(
                "guard-shorthand success path should not print unexpected shorthand error marker, got: '{stdout}'"
            ));
        }
        if stdout.contains("UNEXPECTED_NAMED_ERROR=") {
            return Err(format!(
                "guard-shorthand named-binding success path should not print unexpected named error marker, got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "guard-shorthand binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
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
        "guard-shorthand project should compile, run, print deterministic markers for shorthand and named guards, and exit cleanly: {failure_message}"
    );
}
