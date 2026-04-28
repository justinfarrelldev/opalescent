#![cfg(feature = "integration")]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the absolute path to a test project by name.
/// Example: `fs_project_root("_fs_path_from")` → `<repo>/test-projects/_fs_path_from`
pub fn fs_project_root(name: &str) -> PathBuf {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(repo_root).join("test-projects").join(name)
}

/// Reads an evidence file from `.sisyphus/evidence/`.
/// Convenience helper for asserting evidence in tests.
#[expect(
    dead_code,
    reason = "Evidence helper is used selectively by targeted integration checks"
)]
pub fn read_evidence(name: &str, scenario: &str) -> String {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let evidence_path = PathBuf::from(repo_root)
        .join(".sisyphus")
        .join("evidence")
        .join(format!("{name}-{scenario}"));

    fs::read_to_string(&evidence_path)
        .unwrap_or_else(|_| format!("Evidence file not found: {evidence_path:?}"))
}

/// Asserts that both `target/` and `workspace/` directories are empty or missing
/// for the given project.
pub fn assert_workspace_empty(project: &str) {
    let project_path = fs_project_root(project);

    assert_dir_empty_or_missing(&project_path.join("target"), project, "target");
    assert_dir_empty_or_missing(&project_path.join("workspace"), project, "workspace");
}

fn assert_dir_empty_or_missing(dir: &PathBuf, project: &str, dir_name: &str) {
    match fs::read_dir(dir) {
        Ok(entries) => {
            let entries: Vec<_> = entries.collect();
            assert!(
                entries.is_empty(),
                "{dir_name}/ directory should be empty for project {}, but found {} entries",
                project,
                entries.len()
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            assert!(
                matches!(error.kind(), std::io::ErrorKind::NotFound),
                "Failed to read {dir_name} dir for {project}: {error}"
            );
        }
    }
}

/// Normalizes line endings by replacing all `\r\n` with `\n` and stripping trailing `\r`.
/// Used to normalize line endings before assertion comparisons across platforms.
pub fn strip_crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "")
}

/// Creates a unique, process-local target directory for inline probe builds.
///
/// These dirs are intentionally outside fixture project `target/` trees to avoid
/// cross-test contamination when multiple fs integration probes compile in the
/// same suite run.
pub fn unique_probe_target_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-probe-target-{label}-{}-{nanos}",
        std::process::id()
    ))
}

/// Type alias for `FsStateGuard` for convenience in test modules.
pub type FsStateGuard = super::fs_state_guard::FsStateGuard;
