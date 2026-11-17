//! Source-to-typed-suite compiler integration acceptance tests.
use super::checking::check_project;
use super::project::NativeTestProject;
use std::path::Path;

#[test]
fn minimal_fixture_checks_through_the_normal_type_checker() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project = NativeTestProject::load(&root, &[root.join("tests/addition.test.op")])
        .expect("source graph");
    let checked = check_project(&project).expect("fully checked source callbacks and examples");
    assert_eq!(checked.suite.cases.len(), 3);
    assert_eq!(checked.suite.bodies.len(), 2);
    assert_eq!(checked.world_states.len(), 1);
}

#[test]
fn invalid_world_injections_and_lifecycle_contracts_fail_compilation() {
    let directory = tempfile::tempdir().expect("fixture directory");
    for source in [
        "world A:\n    setup: f(): int64 =>\n        return 1\nscenario injections:\n    worlds: A\n    it should miss the injection: f(): void =>\n        return void\n",
        "world A:\n    setup: f(): int64 =>\n        return 1\nscenario injections:\n    worlds: A\n    it should leave its own parameter unused: f(ref a: A): void =>\n        return void\n",
        "world A:\n    setup: f(): int64 =>\n        return 1\n    cleanup: f(state: int64): void =>\n        return void\n",
        "world A:\n    setup: f(): int64 =>\n        return 1\n    cleanup: f(ref state: int64): int64 =>\n        return state\n",
    ] {
        std::fs::write(directory.path().join("case.test.op"), source).expect("source");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("parsed module");
        assert!(
            check_project(&project).is_err(),
            "accepted invalid test artifact: {source}"
        );
    }
}

#[test]
fn dependencies_activation_and_cleanup_subsets_share_the_typed_pipeline() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let source = "world Root:\n    setup: f(): int64 =>\n        return 1\nworld B:\n    worlds: Root\n    setup: f(ref root: Root): int64 =>\n        return root.state\nworld A:\n    worlds: Root, B\n    setup: f(ref root: Root, ref b: B): int64 =>\n        return root.state + b.state\n    cleanup: f(ref state: int64, ref b: B): void =>\n        print(state + b.state)\n        return void\nscenario activation:\n    activate worlds: A\n    it should need no artificial body read: f(): void =>\n        expect true to be true\n        return void\n";
    std::fs::write(directory.path().join("case.test.op"), source).expect("source");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    let checked = check_project(&project).expect("typed lifecycle contracts");
    assert_eq!(checked.world_states.len(), 3);
    assert_eq!(checked.modules[&project.test_files[0]].callbacks.len(), 5);
    let body = &checked.suite.bodies[&checked.suite.cases[0].body_identity];
    assert!(body.injected_worlds.is_empty());
    assert_eq!(body.initialization_order.len(), 3);
}

#[test]
fn example_schemas_and_constant_initializers_are_checked_before_execution() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("input.types.op"),
        "public type Input:\n    value: string\n",
    )
    .expect("schema");
    for fields in [
        "                other: 'missing'",
        "                value: 2",
        "                value: 'ok'\n                extra: true",
        "                value: int64_to_string(2)",
    ] {
        let source = format!(
            "import type Input from ./input.types\nscenario examples:\n    it should reject invalid data:\n        examples of Input:\n            example invalid:\n{fields}\n        f(ref example: Input): void =>\n            print(example.value)\n            return void\n"
        );
        std::fs::write(directory.path().join("case.test.op"), source).expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("parsed module");
        assert!(
            check_project(&project).is_err(),
            "accepted invalid example: {fields}"
        );
    }
}
