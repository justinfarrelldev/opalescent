#![cfg(feature = "integration")]

use super::*;

#[test]
fn rc_basic_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/rc-basic");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "rc-basic target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!("rc-basic project should compile into a binary: {error}"));
            }
        };

        let run_output = std::process::Command::new(&binary_path)
            .output()
            .map_err(|error| format!("rc-basic compiled binary should execute: {error}"))?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "rc-basic: hello world" {
            return Err(format!(
                "rc-basic stdout should equal 'rc-basic: hello world', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "rc-basic binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "rc-basic target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "rc-basic should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn rc_reuse_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/rc-reuse");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "rc-reuse target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!("rc-reuse project should compile into a binary: {error}"));
            }
        };

        let run_output = std::process::Command::new(&binary_path)
            .output()
            .map_err(|error| format!("rc-reuse compiled binary should execute: {error}"))?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "rc-reuse: first second" {
            return Err(format!(
                "rc-reuse stdout should equal 'rc-reuse: first second', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "rc-reuse binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "rc-reuse target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "rc-reuse should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn iterative_drop_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/iterative-drop");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "iterative-drop target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "iterative-drop project should compile into a binary: {error}"
                ));
            }
        };

        let run_output = std::process::Command::new(&binary_path)
            .output()
            .map_err(|error| format!("iterative-drop compiled binary should execute: {error}"))?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "iterative-drop: done" {
            return Err(format!(
                "iterative-drop stdout should equal 'iterative-drop: done', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "iterative-drop binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "iterative-drop target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "iterative-drop should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn weak_ref_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(cwd.is_ok(), "current working directory should be readable for integration tests");
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/weak-ref");
    let temp_dir = project_dir.join("target");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "weak-ref target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!("weak-ref project should compile into a binary: {error}"));
            }
        };

        let run_output = std::process::Command::new(&binary_path)
            .output()
            .map_err(|error| format!("weak-ref compiled binary should execute: {error}"))?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "weak-ref: ok" {
            return Err(format!("weak-ref stdout should equal 'weak-ref: ok', got: '{stdout}'"));
        }
        if !run_output.status.success() {
            return Err(format!(
                "weak-ref binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "weak-ref target directory should be removed");
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "weak-ref should compile, run, and print expected output: {failure_message}"
    );
}
