#![cfg(feature = "integration")]

use serial_test::serial;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const FS_RERUNNABILITY_TIMEOUT: Duration = Duration::from_secs(180);

const FS_PROJECTS: [&str; 20] = [
    "_fs_path_from",
    "_fs_normalize_path",
    "_fs_join_path_components",
    "_fs_path_file_extension",
    "_fs_path_file_name",
    "_fs_path_parent_directory",
    "_absolute_path_sync",
    "_fs_read_text_happy",
    "_fs_read_text_invalid_utf8",
    "_fs_read_contents_happy",
    "_fs_read_contents_is_dir",
    "_fs_read_contents_not_found",
    "_fs_read_lines_lf",
    "_fs_read_lines_crlf",
    "_fs_read_lines_mixed",
    "_fs_read_offset_happy",
    "_fs_read_offset_oob",
    "fs-directory-operations",
    "fs-path-manipulation",
    "fs-markdown-roundtrip",
];

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_rerunnability() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_projects_root = repo_root.join("test-projects");
    let project_dirs: Vec<PathBuf> = FS_PROJECTS
        .iter()
        .map(|name| test_projects_root.join(name))
        .collect();

    assert_all_gitignores_present(&project_dirs)
        .expect("all fs projects should include .gitignore with workspace/ and target/");

    let pre_manifest = snapshot_manifest(&repo_root, &project_dirs)
        .expect("fs rerunnability should snapshot pre-suite manifest");

    let run = Command::new("cargo")
        .arg("test")
        .arg("-j")
        .arg("1")
        .arg("--features")
        .arg("integration")
        .arg("--test")
        .arg("integration_e2e")
        .arg("fs_")
        .arg("--")
        .arg("--skip")
        .arg("fs_rerunnability")
        .current_dir(&repo_root)
        .env("RUST_TEST_THREADS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    assert!(
        run.is_ok(),
        "fs rerunnability should spawn fs_ suite subprocess"
    );
    let Ok(child) = run else {
        return;
    };

    let run_output_result = super::fs_helpers::wait_for_child_output_with_timeout(
        child,
        FS_RERUNNABILITY_TIMEOUT,
        "fs rerunnability fs_ suite subprocess",
    );
    assert!(
        run_output_result.is_ok(),
        "{}",
        run_output_result.err().unwrap_or_default()
    );
    let Ok(run_output) = run_output_result else {
        return;
    };

    assert!(
        run_output.status.success(),
        "fs_ suite subprocess should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let post_manifest = snapshot_manifest(&repo_root, &project_dirs)
        .expect("fs rerunnability should snapshot post-suite manifest");

    assert_eq!(
        pre_manifest, post_manifest,
        "fs project manifest should be unchanged after fs_ suite run"
    );
}

fn assert_all_gitignores_present(project_dirs: &[PathBuf]) -> io::Result<()> {
    let mut failures = Vec::new();

    for project_dir in project_dirs {
        if !project_dir.is_dir() {
            failures.push(format!("missing directory {}", project_dir.display()));
            continue;
        }

        let gitignore_path = project_dir.join(".gitignore");
        if !gitignore_path.is_file() {
            failures.push(format!("missing {}", gitignore_path.display()));
            continue;
        }

        let gitignore = fs::read_to_string(&gitignore_path)?;
        if !gitignore.lines().any(|line| line == "workspace/") {
            failures.push(format!(
                "missing workspace/ in {}",
                gitignore_path.display()
            ));
        }
        if !gitignore.lines().any(|line| line == "target/") {
            failures.push(format!("missing target/ in {}", gitignore_path.display()));
        }
    }

    if !failures.is_empty() {
        return Err(io::Error::other(format!(
            "fs project .gitignore policy mismatch:\n{}",
            failures.join("\n")
        )));
    }

    Ok(())
}

fn snapshot_manifest(
    repo_root: &Path,
    project_dirs: &[PathBuf],
) -> io::Result<Vec<(PathBuf, [u8; 32])>> {
    let mut files = Vec::new();

    for project_dir in project_dirs {
        collect_files(repo_root, project_dir, &mut files)?;
    }

    files.sort();

    let mut manifest = Vec::with_capacity(files.len());
    for path in files {
        let abs_path = repo_root.join(&path);
        let bytes = fs::read(&abs_path)?;

        let file_digest = Sha256::digest(&bytes);
        let mut entry_hasher = Sha256::new();
        entry_hasher.update(path.as_os_str().as_encoded_bytes());
        entry_hasher.update(b":");
        entry_hasher.update(file_digest);

        let mut digest = [0_u8; 32];
        digest.copy_from_slice(&entry_hasher.finalize());
        manifest.push((path, digest));
    }

    Ok(manifest)
}

fn collect_files(repo_root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry_result in fs::read_dir(dir)? {
        let entry = entry_result?;
        let file_type = entry.file_type()?;
        let path = entry.path();

        let name = entry.file_name();
        if name.as_os_str() == OsStr::new(".git") {
            continue;
        }

        if file_type.is_dir() {
            if name.as_os_str() == OsStr::new("target")
                || name.as_os_str() == OsStr::new("workspace")
            {
                continue;
            }
            collect_files(repo_root, &path, out)?;
            continue;
        }

        if !file_type.is_dir() {
            let relative = path
                .strip_prefix(repo_root)
                .expect("manifest paths should be relative to repo root")
                .to_path_buf();
            out.push(relative);
        }
    }

    Ok(())
}
