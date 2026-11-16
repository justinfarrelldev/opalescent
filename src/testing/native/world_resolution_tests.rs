//! Source-to-canonical World resolution regression tests.
use super::project::NativeTestProject;
use super::world_resolution::resolve_worlds;

#[test]
fn aliases_resolve_to_one_provider_before_dependency_validation() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(directory.path().join("case.test.op"), "import world Root as First, Root as Second from ./worlds\nworld Top:\n    worlds: First\n    setup: f(ref root: First): int64 =>\n        return root.state\nscenario cases:\n    it should resolve: f(): void =>\n        return void\n").expect("root");
    std::fs::write(
        directory.path().join("worlds.op"),
        "world Root:\n    setup: f(): int64 =>\n        return 1\n",
    )
    .expect("support");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("parse project");
    let resolved = resolve_worlds(&project).expect("resolved graph");
    let bindings = resolved
        .bindings
        .get(&project.test_files[0])
        .expect("root bindings");
    assert_eq!(bindings["First"], bindings["Second"]);
    assert_ne!(bindings["First"], bindings["Top"]);
    assert_eq!(
        resolved
            .graph
            .initialization_order(&[bindings["Top"].clone()])
            .expect("topology"),
        vec![bindings["First"].clone(), bindings["Top"].clone()]
    );
}

#[test]
fn duplicate_dependency_aliases_and_cycles_fail_before_execution() {
    let directory = tempfile::tempdir().expect("fixture directory");
    for source in [
        "world A:\n    worlds: B\n    setup: f(ref b: B): int64 =>\n        return b.state\nworld B:\n    worlds: A\n    setup: f(ref a: A): int64 =>\n        return a.state\n",
        "import world Root as First, Root as Second from ./worlds\nworld A:\n    worlds: First, Second\n    setup: f(ref first: First, ref second: Second): int64 =>\n        return first.state + second.state\n",
        "world A:\n    worlds: Missing\n    setup: f(ref missing: Missing): int64 =>\n        return missing.state\n",
    ] {
        std::fs::write(directory.path().join("case.test.op"), source).expect("root");
        std::fs::write(
            directory.path().join("worlds.op"),
            "world Root:\n    setup: f(): int64 =>\n        return 1\n",
        )
        .expect("support");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("parse sources");
        assert!(
            resolve_worlds(&project).is_err(),
            "accepted invalid graph: {source}"
        );
    }
}
