//! Stable independent case expansion and scoped metadata resolution tests.
extern crate alloc;
use super::cases::resolve_cases;
use super::project::NativeTestProject;
use super::world_resolution::resolve_worlds;
use std::path::Path;

#[test]
fn named_examples_share_one_body_but_have_independent_stable_identities() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("fixture");
    let worlds = resolve_worlds(&project).expect("providers");
    let suite = resolve_cases(&project, &worlds).expect("case metadata");
    assert_eq!(suite.bodies.len(), 2);
    assert_eq!(suite.cases.len(), 3);
    assert_eq!(suite.cases[1].body_identity, suite.cases[2].body_identity);
    assert_ne!(suite.cases[1].identity, suite.cases[2].identity);
    let parameterized = &suite.bodies[&suite.cases[1].body_identity];
    assert!(
        parameterized.injected_worlds.is_empty(),
        "worlds: none replaces inheritance"
    );
    assert_eq!(parameterized.subjects[0].0.module, "src/arithmetic.op");
}

#[test]
fn metadata_replacement_and_activation_conflicts_are_checked_on_effective_lists() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let prefix = "world A:\n    setup: f(): int64 =>\n        return 1\nworld B:\n    setup: f(): int64 =>\n        return 2\nscenario outer:\n    worlds: A\n    activate worlds: B\n";
    for (body, valid) in [
        (
            "    it should narrow:\n        worlds: none\n        activate worlds: none\n        f(): void =>\n            return void\n",
            true,
        ),
        (
            "    it should conflict:\n        worlds: B\n        f(ref b: B): void =>\n            print(b.state)\n            return void\n",
            false,
        ),
        (
            "    it should duplicate:\n        worlds: A, A\n        f(ref a: A, ref other: A): void =>\n            print(a.state + other.state)\n            return void\n",
            false,
        ),
    ] {
        std::fs::write(
            directory.path().join("case.test.op"),
            format!("{prefix}{body}"),
        )
        .expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("parse");
        let worlds = resolve_worlds(&project).expect("providers");
        assert_eq!(resolve_cases(&project, &worlds).is_ok(), valid);
    }
}

#[test]
fn hook_only_and_transitive_worlds_do_not_become_body_injections() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let source = "world A:\n    worlds: B\n    setup: f(ref b: B): int64 =>\n        return b.state\nworld B:\n    setup: f(): int64 =>\n        return 2\nbefore each test: f(ref b: B): void =>\n    print(b.state)\n    return void\nscenario hook scopes:\n    after each test: f(ref a: A): void =>\n        print(a.state)\n        return void\n    it should need no injected parameters: f(): void =>\n        return void\n";
    std::fs::write(directory.path().join("case.test.op"), source).expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    let worlds = resolve_worlds(&project).expect("graph");
    let suite = resolve_cases(&project, &worlds).expect("case needs");
    let body = &suite.bodies[&suite.cases[0].body_identity];
    assert!(body.injected_worlds.is_empty());
    assert_eq!(body.initialization_order.len(), 2);
    assert_eq!(body.initialization_order[0].0.name, "B");
    assert_eq!(body.initialization_order[1].0.name, "A");
    let dependency_origins = &body.world_origins[&body.initialization_order[0]];
    assert!(
        dependency_origins
            .iter()
            .any(|origin| matches!(*origin, super::cases::WorldUseOrigin::Hook { .. }))
    );
    assert!(
        dependency_origins
            .iter()
            .any(|origin| matches!(*origin, super::cases::WorldUseOrigin::Dependency { .. }))
    );
    assert!(!dependency_origins.contains(&super::cases::WorldUseOrigin::Injected));
}

#[test]
fn reordering_examples_preserves_the_complete_case_identity_set() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("input.types.op"),
        "public type Input:\n    value: int64\n",
    )
    .expect("example schema");
    let prefix = "import type Input from ./input.types\nscenario identities:\n    it should remain stable:\n        examples of Input:\n";
    let first = "            example first:\n                value: 1\n";
    let second = "            example second:\n                value: 2\n";
    let callback = "        f(ref example: Input): void =>\n            print(example.value)\n            return void\n";
    let mut identity_sets = Vec::new();
    for examples in [format!("{first}{second}"), format!("{second}{first}")] {
        std::fs::write(
            directory.path().join("case.test.op"),
            format!("{prefix}{examples}{callback}"),
        )
        .expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("sources");
        let worlds = resolve_worlds(&project).expect("graph");
        let suite = resolve_cases(&project, &worlds).expect("cases");
        identity_sets.push(
            suite
                .cases
                .into_iter()
                .map(|case| case.identity)
                .collect::<alloc::collections::BTreeSet<_>>(),
        );
    }
    assert_eq!(identity_sets[0], identity_sets[1]);
}

#[test]
fn sibling_names_and_mixed_subject_clauses_do_not_get_silent_suffixes() {
    let directory = tempfile::tempdir().expect("fixture directory");
    for source in [
        "scenario same:\n    it should duplicate: f(): void =>\n        return void\n    it should duplicate: f(): void =>\n        return void\n",
        "let add = f(): int64 =>\n    return 1\nscenario subjects:\n    unit under test: add\n    units under test: add\n    it should reject mixed clauses: f(): void =>\n        return void\n",
        "scenario:\n    it should require an unambiguous scenario name: f(): void =>\n        return void\n",
    ] {
        std::fs::write(directory.path().join("case.test.op"), source).expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("parse");
        let worlds = resolve_worlds(&project).expect("providers");
        assert!(
            resolve_cases(&project, &worlds).is_err(),
            "accepted invalid case metadata: {source}"
        );
    }
}
