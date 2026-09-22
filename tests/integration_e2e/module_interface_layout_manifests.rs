#![cfg(feature = "integration")]

use super::*;

fn compile_and_run_project_fixture(
    project_name: &str,
    expected_stdout: &str,
) -> Result<(), String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("current directory should be readable: {error}"))?;
    let project_dir = cwd.join("test-projects").join(project_name);
    let temp_dir = tests::fs_helpers::unique_probe_target_dir(project_name);
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be created: {error}"))?;

    let result = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| format!("{project_name} should compile: {error}"))?;
        let run_output = run_binary_with_timeout(&binary_path, project_name)?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim_end() != expected_stdout {
            return Err(format!(
                "{project_name} stdout should equal '{expected_stdout}', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "{project_name} binary should exit successfully, got {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be removed: {error}"))?;
    result
}

#[test]
fn proposal_editor_layout_manifests_compile_and_run_transitively() {
    let result =
        compile_and_run_project_fixture("module-interface-layout-manifests", "command error nope");
    assert!(
        result.is_ok(),
        "proposal-style module interface layout manifest fixture should compile and run: {:?}",
        result.err()
    );
}

#[test]
fn colliding_type_display_names_work_through_explicit_aliases() {
    let result =
        compile_and_run_project_fixture("module-interface-layout-collisions", "left=7 right=blue");
    assert!(
        result.is_ok(),
        "colliding ADT display names should compile and run through explicit aliases: {:?}",
        result.err()
    );
}
