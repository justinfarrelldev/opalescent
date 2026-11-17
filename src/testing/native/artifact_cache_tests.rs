//! Content-addressed test artifacts never reuse production or incomplete products.
#![cfg(target_os = "linux")]
use super::{
    artifact_cache::cached_native_artifact, checking::check_project, project::NativeTestProject,
};
use std::path::Path;

#[test]
fn cache_reuses_verified_artifacts_and_rejects_tampering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("project");
    let checked = check_project(&project).expect("checked");
    let directory = tempfile::tempdir().expect("cache");
    let first = cached_native_artifact(&project, &checked, directory.path()).expect("first build");
    let modified = std::fs::metadata(&first.executable)
        .expect("metadata")
        .modified()
        .expect("timestamp");
    let second = cached_native_artifact(&project, &checked, directory.path()).expect("cache hit");
    assert_eq!(first.executable, second.executable);
    assert_eq!(first.cases, second.cases);
    assert_eq!(
        modified,
        std::fs::metadata(&second.executable)
            .expect("metadata")
            .modified()
            .expect("timestamp")
    );
    let result = directory.path().join("cached-attempt.jsonl");
    let status = std::process::Command::new(&second.executable)
        .arg(serde_json::to_string(&second.cases[0]).expect("identity"))
        .arg("1")
        .arg(&result)
        .status()
        .expect("cached worker");
    assert!(status.success());
    let manifest = first
        .executable
        .parent()
        .expect("entry")
        .join("native-test-manifest.json");
    let original = std::fs::read(&manifest).expect("manifest");
    let mut foreign: serde_json::Value = serde_json::from_slice(&original).expect("manifest");
    foreign["domain"] = serde_json::json!("opalescent-production-artifact");
    std::fs::write(
        &manifest,
        serde_json::to_vec(&foreign).expect("foreign manifest"),
    )
    .expect("replace manifest");
    assert!(cached_native_artifact(&project, &checked, directory.path()).is_err());
    std::fs::write(&manifest, &original).expect("restore manifest");
    std::fs::write(&first.executable, b"corrupted cache artifact").expect("corrupt executable");
    assert!(cached_native_artifact(&project, &checked, directory.path()).is_err());
}

#[test]
fn concurrent_builders_publish_one_complete_immutable_artifact() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let directory = tempfile::tempdir().expect("cache");
    std::thread::scope(|scope| {
        let build = || {
            let project = NativeTestProject::load(&root, &[root.join("tests/addition.test.op")])
                .expect("project");
            let checked = check_project(&project).expect("checked");
            cached_native_artifact(&project, &checked, directory.path())
                .expect("concurrent artifact")
        };
        let first = scope.spawn(build);
        let second = scope.spawn(build);
        assert_eq!(
            first.join().expect("first builder").executable,
            second.join().expect("second builder").executable
        );
    });
    let entries = std::fs::read_dir(directory.path())
        .expect("cache entries")
        .count();
    assert_eq!(entries, 1);
}

#[test]
fn failed_builds_do_not_publish_entries_or_leave_staging_directories() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("project");
    let mut checked = check_project(&project).expect("checked");
    checked.suite.cases.clear();
    let directory = tempfile::tempdir().expect("cache");
    assert!(cached_native_artifact(&project, &checked, directory.path()).is_err());
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("entries")
            .count(),
        0
    );
}

#[test]
fn original_source_bytes_and_canonical_module_paths_invalidate_cache() {
    let directory = tempfile::tempdir().expect("project");
    let cache = tempfile::tempdir().expect("cache");
    let source = directory.path().join("case.test.op");
    let text = "scenario cache identity:\n    it should retain source identity: f(): void =>\n        expect true to be true\n        return void\n";
    std::fs::write(&source, text).expect("source");
    let project = NativeTestProject::load(directory.path(), &[source.clone()]).expect("project");
    let checked = check_project(&project).expect("checked");
    let first = cached_native_artifact(&project, &checked, cache.path()).expect("first");
    std::fs::write(&source, format!("{text}\n")).expect("source change");
    let changed = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let changed_checked = check_project(&changed).expect("checked");
    let second =
        cached_native_artifact(&changed, &changed_checked, cache.path()).expect("source miss");
    assert_ne!(first.executable, second.executable);
    let other = tempfile::tempdir().expect("other root");
    let other_source = other.path().join("case.test.op");
    std::fs::write(&other_source, format!("{text}\n")).expect("source");
    let other_project = NativeTestProject::load(other.path(), &[other_source]).expect("project");
    let other_checked = check_project(&other_project).expect("checked");
    let third =
        cached_native_artifact(&other_project, &other_checked, cache.path()).expect("path miss");
    assert_ne!(second.executable, third.executable);
}
