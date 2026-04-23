#![cfg(feature = "windows-wine")]

//! Wine-based integration harness for Windows/MSVC filesystem testing.
//!
//! This module provides infrastructure for running Opalescent test projects
//! compiled for x86_64-pc-windows-msvc under Wine on a Linux host. It handles:
//! - Prerequisite verification (wine, clang-cl, xwin, LLVM)
//! - Cross-compilation of Opalescent projects to Windows targets
//! - Execution under Wine with output capture
//! - Evidence collection for test results
//! - Filesystem state snapshots for verification

use super::*;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

    /// Check if Wine prerequisites are available.
    ///
    /// Verifies that wine, clang-cl, xwin sysroot, and LLVM are available.
    /// Returns `Ok(())` if all prerequisites are present, or `Err(reason)` if any are missing.
    pub fn check_prereqs() -> Result<(), String> {
        let output = Command::new("bash")
            .arg("scripts/verify-wine-prereqs.sh")
            .output()
            .map_err(|e| format!("Failed to run prereq check script: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout_trimmed = stdout.trim();

        if stdout_trimmed.starts_with("SKIP:") {
            let reason = stdout_trimmed.strip_prefix("SKIP:").unwrap_or("unknown").trim();
            return Err(reason.to_string());
        }

        if stdout_trimmed.starts_with("OK:") {
            return Ok(());
        }

        Err(format!("Unexpected prereq check output: {}", stdout_trimmed))
    }

    /// Build an Opalescent project for Windows target.
    ///
    /// Compiles the project at `project` for the given `target` triple.
    /// Returns the path to the compiled `.exe` file.
    pub fn build_opal_project(project: &str, target: &str) -> Result<PathBuf, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_path = manifest_dir.join("test-projects").join(project);

        if !project_path.exists() {
            return Err(format!("Project directory not found: {}", project_path.display()));
        }

        let opal_binary = manifest_dir.join("target/release/opalescent");
        if !opal_binary.exists() {
            return Err(format!(
                "Opalescent binary not found at {}. Run 'cargo build --release' first.",
                opal_binary.display()
            ));
        }

        let output = Command::new(&opal_binary)
            .arg("build")
            .arg("--target")
            .arg(target)
            .current_dir(&project_path)
            .output()
            .map_err(|e| format!("Failed to run opal build: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Build failed: {}", stderr));
        }

        let exe_path = project_path
            .join("target")
            .join(target)
            .join("program.exe");

        if !exe_path.exists() {
            return Err(format!(
                "Compiled executable not found at {}",
                exe_path.display()
            ));
        }

        Ok(exe_path)
    }

    /// Run an executable under Wine with argument capture.
    ///
    /// Executes the given `.exe` file under Wine with the provided arguments.
    /// Captures stdout, stderr, exit code, and filesystem state.
    pub fn run_under_wine(exe: &Path, args: &[&str]) -> Result<WineRun, String> {
        let mut cmd = Command::new("wine");
        cmd.arg(exe);
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to execute under wine: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().unwrap_or(-1);

        // Capture filesystem state (placeholder for now)
        let fs_dump = String::new();

        Ok(WineRun {
            stdout,
            stderr,
            exit_code,
            fs_dump,
        })
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
            .map_err(|e| format!("Failed to create evidence directory: {}", e))?;

        let base_name = format!("task-{}-{}", task_num, slug);

        // Write stdout
        let stdout_path = evidence_dir.join(format!("{}-stdout.txt", base_name));
        let stdout_content = format!("EXIT={}\n{}", run.exit_code, run.stdout);
        std::fs::write(&stdout_path, stdout_content)
            .map_err(|e| format!("Failed to write stdout evidence: {}", e))?;

        // Write stderr
        let stderr_path = evidence_dir.join(format!("{}-stderr.txt", base_name));
        std::fs::write(&stderr_path, &run.stderr)
            .map_err(|e| format!("Failed to write stderr evidence: {}", e))?;

        // Write fs dump
        let fs_path = evidence_dir.join(format!("{}-fs.txt", base_name));
        std::fs::write(&fs_path, &run.fs_dump)
            .map_err(|e| format!("Failed to write fs evidence: {}", e))?;

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

        let output = Command::new("ls")
            .arg("-la")
            .arg("-R")
            .arg(root)
            .output()
            .map_err(|e| format!("Failed to snapshot workspace: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::wine_harness::*;

    #[test]
    fn check_prereqs_smoke() {
        // This test verifies that check_prereqs can be called without panicking.
        // It may return Ok or Err depending on whether wine is installed.
        let _ = check_prereqs();
    }
}
