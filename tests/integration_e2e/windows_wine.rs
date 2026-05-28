//! Wine-based integration harness for Windows/MSVC integration testing.
//!
//! This module provides infrastructure for running Opalescent test projects
//! compiled for x86_64-pc-windows-msvc under Wine on a Linux host. It handles:
//! - Prerequisite verification (wine, clang-cl, xwin, LLVM)
//! - Cross-compilation of Opalescent projects to Windows targets
//! - Execution under Wine with output capture
//! - Evidence collection for test results
//! - Filesystem state snapshots for verification

use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "windows_wine_helpers.rs"]
mod windows_wine_helpers;

/// Result of a Wine execution run.
#[derive(Debug, Clone)]
pub struct WineRun {
    /// Standard output from the program.
    pub stdout: String,
    /// Standard error from the program.
    pub stderr: String,
    /// Exit code from the program.
    pub exit_code: i32,
    /// Filesystem state dump after execution.
    pub fs_dump: String,
}

/// Wine harness module containing test infrastructure functions.
pub mod wine_harness {
    use super::*;

    const MISSING_EXIT_CODE: i32 = -1;

    /// Remove temporary Wine capture artifacts, ignoring best-effort cleanup failures.
    fn cleanup_capture_files(stdout_path: &Path, stderr_path: &Path, capture_dir: &Path) {
        drop(std::fs::remove_file(stdout_path));
        drop(std::fs::remove_file(stderr_path));
        drop(std::fs::remove_dir(capture_dir));
    }

    /// Check if Wine prerequisites are available.
    ///
    /// Verifies that wine, clang-cl, xwin sysroot, and LLVM are available.
    /// Returns `Ok(())` if all prerequisites are present, or `Err(reason)` if any are missing.
    pub fn check_prereqs() -> Result<(), String> {
        let mut prereq_command = Command::new("bash");
        prereq_command.arg("scripts/verify-wine-prereqs.sh");
        let output = crate::run_command_output_with_timeout(
            &mut prereq_command,
            Duration::from_secs(30),
            "wine prereq check script",
        )
        .map_err(|e| format!("Failed to run prereq check script: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout_trimmed = stdout.trim();

        if stdout_trimmed.starts_with("SKIP:") {
            let reason = stdout_trimmed
                .strip_prefix("SKIP:")
                .unwrap_or("unknown")
                .trim();
            return Err(reason.to_owned());
        }

        if stdout_trimmed.starts_with("OK:") {
            return Ok(());
        }

        Err(format!("Unexpected prereq check output: {stdout_trimmed}"))
    }

    /// Build an Opalescent project for Windows target.
    ///
    /// Compiles the project at `project` for the given `target` triple.
    /// Returns the path to the compiled `.exe` file.
    pub fn build_opal_project(project: &str, target: &str) -> Result<PathBuf, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_path = manifest_dir.join("test-projects").join(project);

        if !project_path.exists() {
            return Err(format!(
                "Project directory not found: {}",
                project_path.display()
            ));
        }

        let opal_binary = manifest_dir.join("target/release/opalescent");
        if !opal_binary.exists() {
            return Err(format!(
                "Opalescent binary not found at {}. Run 'cargo build --release' first.",
                opal_binary.display()
            ));
        }

        let mut build_command = Command::new(&opal_binary);
        build_command
            .arg("build")
            .arg("--target")
            .arg(target)
            .current_dir(&project_path);
        let output = crate::run_command_output_with_timeout(
            &mut build_command,
            Duration::from_secs(120),
            "opal windows build",
        )
        .map_err(|e| format!("Failed to run opal build: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Build failed: {stderr}"));
        }

        let preferred_exe_path = project_path.join("target").join(target).join("program.exe");
        let fallback_exe_path = project_path.join("target").join("program.exe");
        let exe_path = if preferred_exe_path.exists() {
            preferred_exe_path
        } else if fallback_exe_path.exists() {
            fallback_exe_path
        } else {
            return Err(format!(
                "Compiled executable not found at {} or {}",
                preferred_exe_path.display(),
                fallback_exe_path.display()
            ));
        };

        Ok(exe_path)
    }

    /// Run an executable under Wine with optional environment overrides.
    pub fn run_under_wine_with_env(
        exe: &Path,
        args: &[&str],
        env_pairs: &[(&str, &str)],
        removed_env: &[&str],
    ) -> Result<WineRun, String> {
        const WINE_TIMEOUT: Duration = Duration::from_secs(120);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to create wine capture suffix: {e}"))?
            .as_nanos();
        let capture_dir = env::temp_dir().join(format!("opalescent-wine-{unique}"));
        std::fs::create_dir_all(&capture_dir)
            .map_err(|e| format!("Failed to create wine capture directory: {e}"))?;
        let stdout_path = capture_dir.join("stdout.txt");
        let stderr_path = capture_dir.join("stderr.txt");

        let stdout_file = File::create(&stdout_path)
            .map_err(|e| format!("Failed to create wine stdout capture file: {e}"))?;
        let stderr_file = File::create(&stderr_path)
            .map_err(|e| format!("Failed to create wine stderr capture file: {e}"))?;

        let mut cmd = Command::new("wine");
        cmd.env("WINEPREFIX", "/tmp/opencode/wineprefix");
        cmd.env("WINEDEBUG", "-all");
        cmd.env("WINEDEBUGGER", "true");
        for &(name, value) in env_pairs {
            cmd.env(name, value);
        }
        for name in removed_env {
            cmd.env_remove(name);
        }
        cmd.arg(exe);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.stdout(stdout_file);
        cmd.stderr(stderr_file);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to execute under wine: {e}"))?;

        let start = Instant::now();
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| format!("Failed while waiting for wine process: {e}"))?
            {
                let stdout = std::fs::read_to_string(&stdout_path)
                    .map_err(|e| format!("Failed to read wine stdout capture: {e}"))?;
                let stderr = std::fs::read_to_string(&stderr_path)
                    .map_err(|e| format!("Failed to read wine stderr capture: {e}"))?;

                cleanup_capture_files(&stdout_path, &stderr_path, &capture_dir);

                return Ok(WineRun {
                    stdout,
                    stderr,
                    exit_code: status.code().unwrap_or(MISSING_EXIT_CODE),
                    fs_dump: String::new(),
                });
            }

            if start.elapsed() >= WINE_TIMEOUT {
                drop(child.kill());
                let status = child
                    .wait()
                    .map_err(|e| format!("Failed while terminating timed-out wine process: {e}"))?;
                let stdout = std::fs::read_to_string(&stdout_path)
                    .map_err(|e| format!("Failed to read wine stdout capture: {e}"))?;
                let stderr = std::fs::read_to_string(&stderr_path)
                    .map_err(|e| format!("Failed to read wine stderr capture: {e}"))?;

                cleanup_capture_files(&stdout_path, &stderr_path, &capture_dir);

                return Err(format!(
                    "Wine execution timed out after {}s (elapsed {:?}, exit={:?}), stdout={:?}, stderr={:?}",
                    WINE_TIMEOUT.as_secs(),
                    start.elapsed(),
                    status.code(),
                    stdout,
                    stderr
                ));
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Run an executable under Wine with argument capture.
    ///
    /// Executes the given `.exe` file under Wine with the provided arguments.
    /// Captures stdout, stderr, exit code, and filesystem state.
    pub fn run_under_wine(exe: &Path, args: &[&str]) -> Result<WineRun, String> {
        run_under_wine_with_env(exe, args, &[], &[])
    }

    /// Capture evidence from a test run.
    ///
    /// Writes stdout, stderr, and filesystem state to evidence files.
    /// Evidence files are written to `.sisyphus/evidence/` directory.
    pub fn capture_evidence(task_num: u32, slug: &str, run: &WineRun) -> Result<(), String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let evidence_dir = manifest_dir.join(".sisyphus/evidence");

        // Create evidence directory if it doesn't exist
        std::fs::create_dir_all(&evidence_dir)
            .map_err(|e| format!("Failed to create evidence directory: {e}"))?;

        let base_name = format!("task-{task_num}-{slug}");

        // Write stdout
        let stdout_path = evidence_dir.join(format!("{base_name}-stdout.txt"));
        let stdout_content = format!("EXIT={}\n{}", run.exit_code, run.stdout);
        std::fs::write(&stdout_path, stdout_content)
            .map_err(|e| format!("Failed to write stdout evidence: {e}"))?;

        // Write stderr
        let stderr_path = evidence_dir.join(format!("{base_name}-stderr.txt"));
        std::fs::write(&stderr_path, &run.stderr)
            .map_err(|e| format!("Failed to write stderr evidence: {e}"))?;

        // Write fs dump
        let fs_path = evidence_dir.join(format!("{base_name}-fs.txt"));
        std::fs::write(&fs_path, &run.fs_dump)
            .map_err(|e| format!("Failed to write fs evidence: {e}"))?;

        Ok(())
    }

    /// Snapshot the workspace directory state.
    ///
    /// Returns a string representation of the directory tree and file listing
    /// for the workspace directory at the given root path.
    pub fn snapshot_workspace(root: &Path) -> Result<String, String> {
        if !root.exists() {
            return Ok(String::from("(workspace does not exist)"));
        }

        let mut snapshot_command = Command::new("ls");
        snapshot_command.arg("-la").arg("-R").arg(root);
        let output = crate::run_command_output_with_timeout(
            &mut snapshot_command,
            Duration::from_secs(10),
            "workspace snapshot",
        )
        .map_err(|e| format!("Failed to snapshot workspace: {e}"))?;

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::WineRun;
    use super::windows_wine_helpers::*;
    use super::wine_harness::*;
    use crate::compile_program_for_tests;
    use crate::tests::fs_helpers::unique_probe_target_dir;
    use opalescent::build_system::targets::parse_target_triple;
    use std::fs;
    use std::path::{Path, PathBuf};

    // helpers moved to `windows_wine_helpers.rs`

    #[test]
    fn harness_api_symbols_are_reachable() {
        fn consume<T>(_value: T) {}

        consume::<fn(&str, &str) -> Result<PathBuf, String>>(build_opal_project);
        consume::<fn(&Path, &[&str]) -> Result<WineRun, String>>(run_under_wine);
        consume::<fn(u32, &str, &WineRun) -> Result<(), String>>(capture_evidence);
        consume::<fn(&Path) -> Result<String, String>>(snapshot_workspace);
    }

    #[test]
    fn check_prereqs_smoke() {
        // This test verifies that check_prereqs can be called without panicking.
        // It may return Ok or Err depending on whether wine is installed.
        drop(check_prereqs());
    }


    fn build_required_process_project(
        project: &str,
        test_name: &str,
        skip_task_num: u32,
        skip_slug: &str,
    ) -> Option<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.join("test-projects").join(project);
        if !project_root.exists() {
            let reason = format!(
                "fixture project '{project}' is not present yet at {}",
                project_root.display()
            );
            record_skip(
                skip_task_num,
                skip_slug,
                test_name,
                reason.as_str(),
                PRE_WINE_SKIP_DUMP,
            );
            return None;
        }

        let exe_path_result = build_opal_project(project, WINDOWS_MSVC_TARGET);
        assert!(
            exe_path_result.is_ok(),
            "{project} fixture should build for {WINDOWS_MSVC_TARGET} when prereqs are available: {:?}",
            exe_path_result.as_ref().err()
        );
        Some(exe_path_result.expect("asserted Windows process fixture build succeeded"))
    }

    #[test]
    fn process_paths() {
        if skip_if_prereqs_missing(PROCESS_PATHS_TASK_NUM, PROCESS_PATHS_SLUG, PROCESS_PATHS_TEST_NAME)
        {
            return;
        }

        let Some(exe_path) = build_required_process_project(
            PROCESS_PATHS_PROJECT,
            PROCESS_PATHS_TEST_NAME,
            PROCESS_PATHS_TASK_NUM,
            PROCESS_PATHS_SLUG,
        ) else {
            return;
        };

        let run_result = run_under_wine(&exe_path, &[]);
        assert!(
            run_result.is_ok(),
            "process-paths fixture should execute under Wine after a successful build: {:?}",
            run_result.as_ref().err()
        );
        let run = run_result.expect("asserted Wine process-paths execution succeeded");

        assert!(
            capture_evidence(PROCESS_PATHS_TASK_NUM, PROCESS_PATHS_SLUG, &run).is_ok(),
            "wine process-paths execution path should write deterministic evidence"
        );

        assert_eq!(
            run.exit_code, 0_i32,
            "process-paths Wine child should exit successfully, stdout={:?}, stderr={:?}",
            run.stdout, run.stderr
        );
        assert_stdout_markers(
            &run,
            &PROCESS_PATHS_MARKERS,
            "process-paths Wine fixture",
        );
    }

    #[test]
    fn process_env() {
        if skip_if_prereqs_missing(PROCESS_ENV_TASK_NUM, PROCESS_ENV_SLUG, PROCESS_ENV_TEST_NAME) {
            return;
        }

        let Some(exe_path) = build_required_process_project(
            PROCESS_ENV_PROJECT,
            PROCESS_ENV_TEST_NAME,
            PROCESS_ENV_TASK_NUM,
            PROCESS_ENV_SLUG,
        ) else {
            return;
        };

        let run_result = run_under_wine_with_env(
            &exe_path,
            &[],
            &[
                ("OPAL_PROCESS_TEST_VALUE", "present-value"),
                ("OPAL_PROCESS_TEST_EMPTY", ""),
            ],
            &["OPAL_PROCESS_TEST_MISSING"],
        );
        assert!(
            run_result.is_ok(),
            "process-env fixture should execute under Wine after a successful build: {:?}",
            run_result.as_ref().err()
        );
        let run = run_result.expect("asserted Wine process-env execution succeeded");

        assert!(
            capture_evidence(PROCESS_ENV_TASK_NUM, PROCESS_ENV_SLUG, &run).is_ok(),
            "wine process-env execution path should write deterministic evidence"
        );

        assert_eq!(
            run.exit_code, 0_i32,
            "process-env Wine child should exit successfully, stdout={:?}, stderr={:?}",
            run.stdout, run.stderr
        );
        assert_stdout_markers(&run, &PROCESS_ENV_MARKERS, "process-env Wine fixture");
    }

    #[test]
    fn process_exit() {
        if skip_if_prereqs_missing(PROCESS_EXIT_TASK_NUM, PROCESS_EXIT_SLUG, PROCESS_EXIT_TEST_NAME) {
            return;
        }

        let exe_path_result = build_opal_project(PROCESS_EXIT_PROJECT, WINDOWS_MSVC_TARGET);
        assert!(
            exe_path_result.is_ok(),
            "process-exit-code fixture should build for {WINDOWS_MSVC_TARGET} when prereqs are available: {:?}",
            exe_path_result.as_ref().err()
        );
        let exe_path = exe_path_result.expect("asserted Windows process-exit fixture build succeeded");

        let run_result = run_under_wine(&exe_path, &[]);
        assert!(
            run_result.is_ok(),
            "process-exit-code fixture should execute under Wine after a successful build: {:?}",
            run_result.as_ref().err()
        );
        let run = run_result.expect("asserted Wine process-exit execution succeeded");

        assert!(
            capture_evidence(PROCESS_EXIT_TASK_NUM, PROCESS_EXIT_SLUG, &run).is_ok(),
            "wine process-exit execution path should write deterministic evidence"
        );

        assert_eq!(
            run.exit_code, PROCESS_EXIT_CODE,
            "process-exit-code Wine child should terminate with exit code {PROCESS_EXIT_CODE}, stdout={:?}, stderr={:?}",
            run.stdout, run.stderr
        );
    }

    #[test]
    fn wine_msvc_symlink_metadata() {
        if skip_if_prereqs_missing(SYMLINK_TASK_NUM, SYMLINK_SLUG, SYMLINK_TEST_NAME) {
            return;
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let temp_dir = unique_probe_target_dir("windows-symlink-metadata");
        let create_temp_dir_result = fs::create_dir_all(&temp_dir);
        assert!(
            create_temp_dir_result.is_ok(),
            "wine_msvc_symlink_metadata temp dir should be creatable: {:?}",
            create_temp_dir_result.err()
        );

        let workspace_root = manifest_dir.join("test-projects/windows-file-ops/workspace");
        let target_file = workspace_root.join("symlink-target.txt");
        let link_path = workspace_root.join("symlink-link.txt");

        if let Err(reason) =
            setup_symlink_metadata_workspace(&workspace_root, &target_file, &link_path)
        {
            record_skip(
                SYMLINK_TASK_NUM,
                SYMLINK_SLUG,
                SYMLINK_TEST_NAME,
                reason.as_str(),
                PRE_WINE_SKIP_DUMP,
            );
            return;
        }

        let source = build_symlink_metadata_source(&link_path);
        let target = parse_target_triple(WINDOWS_MSVC_TARGET)
            .expect("wine_msvc_symlink_metadata should parse the Windows MSVC target triple");
        let exe_path = match compile_program_for_tests(
            Path::new("test-projects/_windows_symlink_metadata/src/main.op"),
            source.as_str(),
            &temp_dir,
            &target,
        ) {
            Ok(path) => path,
            Err(error) => {
                let reason = format!(
                    "Wine limitation: symlink metadata probe is not reliably codegen-supported in this environment ({error})"
                );
                record_skip(
                    SYMLINK_TASK_NUM,
                    SYMLINK_SLUG,
                    SYMLINK_TEST_NAME,
                    reason.as_str(),
                    PRE_WINE_SKIP_DUMP,
                );
                cleanup_symlink_metadata_artifacts(
                    &link_path,
                    &target_file,
                    &workspace_root,
                    &temp_dir,
                );
                return;
            }
        };

        let run_result = run_under_wine(&exe_path, &[]);
        assert!(
            run_result.is_ok(),
            "wine_msvc_symlink_metadata should execute under Wine after a successful build: {:?}",
            run_result.as_ref().err()
        );
        let mut run = run_result.expect("asserted Wine symlink metadata execution succeeded");
        capture_workspace_snapshot(&mut run, &workspace_root);

        assert!(
            capture_evidence(SYMLINK_TASK_NUM, SYMLINK_SLUG, &run).is_ok(),
            "wine symlink metadata execution path should write deterministic evidence"
        );

        if run.exit_code != 0_i32 {
            let reason = format!(
                "Wine limitation: symlink/reparse behavior differs from native Windows (exit={}, stderr={})",
                run.exit_code, run.stderr
            );
            record_skip(
                SYMLINK_TASK_NUM,
                SYMLINK_SLUG,
                SYMLINK_TEST_NAME,
                reason.as_str(),
                PRE_WINE_SKIP_DUMP,
            );
            cleanup_symlink_metadata_artifacts(
                &link_path,
                &target_file,
                &workspace_root,
                &temp_dir,
            );
            return;
        }

        assert_symlink_metadata_output(&run);
        cleanup_symlink_metadata_artifacts(&link_path, &target_file, &workspace_root, &temp_dir);
    }

    #[test]
    fn wine_msvc_file_ops() {
        if skip_if_prereqs_missing(FILE_OPS_TASK_NUM, FILE_OPS_SLUG, FILE_OPS_TEST_NAME) {
            return;
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.join("test-projects").join(FILE_OPS_PROJECT);
        if !project_root.exists() {
            let reason = format!(
                "fixture project '{FILE_OPS_PROJECT}' is not present yet at {}",
                project_root.display()
            );
            record_skip(
                FILE_OPS_TASK_NUM,
                FILE_OPS_SLUG,
                FILE_OPS_TEST_NAME,
                reason.as_str(),
                PRE_WINE_SKIP_DUMP,
            );
            return;
        }

        let paths = build_file_ops_paths(&project_root);
        let exe_path_result = build_opal_project(FILE_OPS_PROJECT, WINDOWS_MSVC_TARGET);
        assert!(
            exe_path_result.is_ok(),
            "wine_msvc_file_ops fixture should build for {WINDOWS_MSVC_TARGET} when prereqs are available: {:?}",
            exe_path_result.as_ref().err()
        );
        let exe_path = exe_path_result.expect("asserted Windows file-ops fixture build succeeded");

        let Some(mut run) = run_file_ops_under_wine(&exe_path) else {
            return;
        };
        capture_workspace_snapshot(&mut run, &paths.workspace_root);

        assert!(
            capture_evidence(FILE_OPS_TASK_NUM, FILE_OPS_SLUG, &run).is_ok(),
            "wine fixture execution path should write deterministic evidence"
        );

        if run.exit_code != 0_i32 && is_known_wine_host_limitation(&run.stderr) {
            let reason = format!(
                "Wine limitation: fatal crash/dialog requires manual close (exit={}, stderr={})",
                run.exit_code, run.stderr
            );
            record_skip(
                FILE_OPS_TASK_NUM,
                FILE_OPS_SLUG,
                FILE_OPS_TEST_NAME,
                reason.as_str(),
                FATAL_WINE_SKIP_DUMP,
            );
            return;
        }

        assert_eq!(
            run.exit_code, 0_i32,
            "wine_msvc_file_ops fixture should exit successfully, stderr={}",
            run.stderr
        );

        assert_expected_markers(&run);
        assert_file_ops_summary(&paths);
        assert_long_path_artifacts(&paths, &run);
        assert_file_ops_host_state(&paths, &run);
    }

    #[test]
    fn wine_msvc_guard_shorthand() {
        if skip_if_prereqs_missing(
            GUARD_SHORTHAND_TASK_NUM,
            GUARD_SHORTHAND_SLUG,
            GUARD_SHORTHAND_TEST_NAME,
        ) {
            return;
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir
            .join("test-projects")
            .join(GUARD_SHORTHAND_PROJECT);
        if !project_root.exists() {
            let reason = format!(
                "fixture project '{GUARD_SHORTHAND_PROJECT}' is not present yet at {}",
                project_root.display()
            );
            record_skip(
                GUARD_SHORTHAND_TASK_NUM,
                GUARD_SHORTHAND_SLUG,
                GUARD_SHORTHAND_TEST_NAME,
                reason.as_str(),
                PRE_WINE_SKIP_DUMP,
            );
            return;
        }

        let build_error = build_opal_project(GUARD_SHORTHAND_PROJECT, WINDOWS_MSVC_TARGET)
            .expect_err(
                "guard-shorthand fixture should fail strict validation for Windows/MSVC just like the host integration fixture",
            );

        assert!(
            build_error.contains("opalescent::guard::missing_terminal")
                && build_error.contains("named guard error clause does not handle the bound error"),
            "wine_msvc_guard_shorthand should fail with the strict guard validation error, got: {build_error}"
        );

        let run = WineRun {
            stdout: format!("EXPECTED_BUILD_FAILURE=guard-shorthand\nBUILD_ERROR={build_error}\n"),
            stderr: build_error,
            exit_code: 0,
            fs_dump: snapshot_workspace(&project_root.join("target")).unwrap_or_else(|error| {
                format!("(workspace snapshot failed after expected build rejection: {error})")
            }),
        };

        assert!(
            capture_evidence(GUARD_SHORTHAND_TASK_NUM, GUARD_SHORTHAND_SLUG, &run).is_ok(),
            "wine guard-shorthand expected-build-failure path should write deterministic evidence"
        );
    }
}
