#![cfg(feature = "integration")]

use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct FsStateGuard {
    pub(crate) project_path: PathBuf,
    pub(crate) manifest: Vec<(PathBuf, [u8; 32])>,
}

impl FsStateGuard {
    pub fn new(project_path: impl AsRef<Path>) -> io::Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let manifest = build_manifest(&project_path)?;
        reset_work_dirs(&project_path)?;

        Ok(Self {
            project_path,
            manifest,
        })
    }
}

impl Drop for FsStateGuard {
    fn drop(&mut self) {
        if let Err(error) = reset_work_dirs(&self.project_path) {
            if std::thread::panicking() {
                eprintln!(
                    "FsStateGuard cleanup failed for {}: {error}",
                    self.project_path.display()
                );
                return;
            }
            assert!(
                reset_work_dirs(&self.project_path).is_ok(),
                "FsStateGuard cleanup failed for {}: {error}",
                self.project_path.display()
            );
            return;
        }

        let mut diffs = Vec::new();
        for entry in &self.manifest {
            let relative_path = &entry.0;
            let expected_digest = &entry.1;
            let absolute_path = self.project_path.join(relative_path);
            match hash_single_file(relative_path, &absolute_path) {
                Ok(actual_digest) => {
                    if actual_digest != *expected_digest {
                        diffs.push(format!(
                            "changed {} (expected {}, got {})",
                            relative_path.display(),
                            hex_digest(expected_digest),
                            hex_digest(&actual_digest)
                        ));
                    }
                }
                Err(error) => {
                    diffs.push(format!(
                        "missing/unreadable {} ({error})",
                        relative_path.display()
                    ));
                }
            }
        }

        if !diffs.is_empty() {
            let message = format!(
                "FsStateGuard manifest mismatch for {}\n{}",
                self.project_path.display(),
                diffs.join("\n")
            );
            if std::thread::panicking() {
                eprintln!("{message}");
                return;
            }
            assert!(diffs.is_empty(), "{message}");
        }
    }
}

fn reset_work_dirs(project_path: &Path) -> io::Result<()> {
    for dir_name in ["target", "workspace"] {
        let dir_path = project_path.join(dir_name);
        if dir_path.exists() {
            fs::remove_dir_all(&dir_path)?;
        }
        fs::create_dir_all(&dir_path)?;
    }
    Ok(())
}

fn build_manifest(project_path: &Path) -> io::Result<Vec<(PathBuf, [u8; 32])>> {
    let mut tracked_files = Vec::new();

    collect_files_under(project_path, Path::new("src"), &mut tracked_files)?;

    let fixtures_path = Path::new("tests").join("fixtures");
    if project_path.join(&fixtures_path).exists() {
        collect_files_under(project_path, &fixtures_path, &mut tracked_files)?;
    }

    for top_level in [
        "opal.toml",
        "opal.pkg.toml",
        ".gitignore",
        ".gitattributes",
        "README.md",
    ] {
        let relative = PathBuf::from(top_level);
        let absolute = project_path.join(&relative);
        if absolute.is_file() {
            tracked_files.push(relative);
        }
    }

    tracked_files.sort();

    let mut manifest = Vec::with_capacity(tracked_files.len());
    for relative_path in tracked_files {
        let absolute_path = project_path.join(&relative_path);
        let digest = hash_single_file(&relative_path, &absolute_path)?;
        manifest.push((relative_path, digest));
    }

    let _manifest_digest = hash_manifest_concat(&manifest);

    Ok(manifest)
}

fn collect_files_under(
    project_path: &Path,
    relative_root: &Path,
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let absolute_root = project_path.join(relative_root);
    if !absolute_root.is_dir() {
        return Ok(());
    }

    walk_dir(project_path, relative_root, files)
}

fn walk_dir(project_path: &Path, relative_dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let absolute_dir = project_path.join(relative_dir);
    for entry_result in fs::read_dir(&absolute_dir)? {
        let entry = entry_result?;
        let file_type = entry.file_type()?;
        let relative_path = relative_dir.join(entry.file_name());

        if relative_path
            .components()
            .any(|component| component.as_os_str() == OsStr::new(".git"))
        {
            continue;
        }

        if file_type.is_dir() {
            walk_dir(project_path, &relative_path, files)?;
            continue;
        }

        if !file_type.is_dir() {
            files.push(relative_path);
        }
    }

    Ok(())
}

fn hash_single_file(relative_path: &Path, absolute_path: &Path) -> io::Result<[u8; 32]> {
    let file_bytes = fs::read(absolute_path)?;
    let file_digest = Sha256::digest(&file_bytes);

    let mut entry_hasher = Sha256::new();
    entry_hasher.update(relative_path.as_os_str().as_encoded_bytes());
    entry_hasher.update(b":");
    entry_hasher.update(file_digest);

    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&entry_hasher.finalize());
    Ok(digest)
}

fn hash_manifest_concat(manifest: &[(PathBuf, [u8; 32])]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for entry in manifest {
        hasher.update(entry.0.as_os_str().as_encoded_bytes());
        hasher.update(b":");
        hasher.update(entry.1);
    }

    let mut out = [0_u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn hex_digest(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        assert!(
            write!(&mut out, "{byte:02x}").is_ok(),
            "writing digest into String should not fail"
        );
    }
    out
}

fn create_project_fixture() -> io::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("tests/fixtures"))?;

    fs::write(root.join("src/main.op"), "entry main = f(): void => { return void }\n")?;
    fs::write(root.join("tests/fixtures/sample.txt"), "fixture\n")?;
    fs::write(root.join("opal.toml"), "name = \"guard\"\nversion = \"1.0.0\"\n")?;
    fs::write(root.join("README.md"), "FsStateGuard smoke fixture\n")?;
    fs::write(root.join(".gitignore"), "target/\nworkspace/\n")?;

    fs::create_dir_all(root.join("target"))?;
    fs::create_dir_all(root.join("workspace"))?;
    fs::write(root.join("target/stale.bin"), "stale")?;
    fs::write(root.join("workspace/stale.bin"), "stale")?;

    Ok(temp_dir)
}

fn dir_is_empty(path: &Path) -> io::Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

#[test]
fn smoke() {
    let project = create_project_fixture().expect("fixture project should be created");
    let project_root = project.path().to_path_buf();

    {
        let _guard =
            FsStateGuard::new(&project_root).expect("FsStateGuard should initialize and reset dirs");

        let target = project_root.join("target");
        let workspace = project_root.join("workspace");
        assert!(target.is_dir(), "target/ should exist after guard init");
        assert!(workspace.is_dir(), "workspace/ should exist after guard init");
        assert!(
            dir_is_empty(&target).expect("target emptiness should be readable"),
            "target/ should be empty after guard init"
        );
        assert!(
            dir_is_empty(&workspace).expect("workspace emptiness should be readable"),
            "workspace/ should be empty after guard init"
        );

        fs::write(project_root.join("target/new.bin"), "new")
            .expect("test should be able to write target artifact");
        fs::write(project_root.join("workspace/new.bin"), "new")
            .expect("test should be able to write workspace artifact");
    }

    let target = project_root.join("target");
    let workspace = project_root.join("workspace");
    assert!(target.is_dir(), "target/ should exist after guard drop");
    assert!(workspace.is_dir(), "workspace/ should exist after guard drop");
    assert!(
        dir_is_empty(&target).expect("target emptiness should be readable after drop"),
        "target/ should be empty after guard drop"
    );
    assert!(
        dir_is_empty(&workspace).expect("workspace emptiness should be readable after drop"),
        "workspace/ should be empty after guard drop"
    );
}

#[test]
fn manifest_diff() {
    let project = create_project_fixture().expect("fixture project should be created");
    let project_root = project.path().to_path_buf();

    let panic_payload = std::panic::catch_unwind(|| {
        let _guard =
            FsStateGuard::new(&project_root).expect("FsStateGuard should initialize for diff test");
        fs::write(
            project_root.join("src/main.op"),
            "entry main = f(): void => { return void }\n# changed\n",
        )
        .expect("tracked source file should be mutable for mismatch test");
    });

    assert!(
        panic_payload.is_err(),
        "dropping guard after mutating tracked files should panic"
    );

    let panic_message = panic_payload
        .err()
        .map(|payload| {
            if let Some(message) = payload.downcast_ref::<String>() {
                return message.clone();
            }
            if let Some(message) = payload.downcast_ref::<&'static str>() {
                return (*message).to_owned();
            }
            "<non-string panic payload>".to_owned()
        })
        .unwrap_or_default();

    assert!(
        panic_message.contains("FsStateGuard manifest mismatch"),
        "panic should mention manifest mismatch, got: {panic_message}"
    );
    assert!(
        panic_message.contains("src/main.op"),
        "panic should list changed file path, got: {panic_message}"
    );

    let target = project_root.join("target");
    let workspace = project_root.join("workspace");
    assert!(target.is_dir(), "target/ should exist after panic path drop");
    assert!(
        dir_is_empty(&target).expect("target emptiness should be readable after panic path"),
        "target/ should be empty after panic path drop"
    );
    assert!(workspace.is_dir(), "workspace/ should exist after panic path drop");
    assert!(
        dir_is_empty(&workspace).expect("workspace emptiness should be readable after panic path"),
        "workspace/ should be empty after panic path drop"
    );
}
