//! Checked suites become canonical reusable callback and per-case slot plans.
use super::checking::check_project;
use super::lowering::{NativeInjection, lower_checked_project};
use super::project::NativeTestProject;
use std::path::Path;

#[test]
fn named_examples_share_one_lowered_source_body_but_keep_independent_case_plans() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("sources");
    let checked = check_project(&project).expect("checked project");
    let lowered = lower_checked_project(&checked).expect("native case plans");
    assert_eq!(lowered.cases.len(), 3);
    assert_eq!(lowered.callbacks.len(), 3);
    let examples: Vec<_> = lowered
        .cases
        .iter()
        .filter(|case| case.example.is_some())
        .collect();
    assert_eq!(examples.len(), 2);
    assert_ne!(examples[0].identity, examples[1].identity);
    assert_eq!(examples[0].body.callback, examples[1].body.callback);
    for case in examples {
        assert!(case.worlds.is_empty());
        assert_eq!(case.body.arguments, vec![NativeInjection::ExampleReference]);
    }
    let ordinary = lowered
        .cases
        .iter()
        .find(|case| case.example.is_none())
        .expect("ordinary case");
    assert_eq!(ordinary.worlds.len(), 1);
    assert_eq!(
        ordinary.body.arguments,
        vec![NativeInjection::WorldReference {
            identity: ordinary.worlds[0].identity.clone(),
            index: 0
        }]
    );
    assert!(ordinary.worlds[0].cleanup.is_none());
}

#[test]
fn diamond_dependencies_hooks_and_cleanup_subsets_use_canonical_actual_slots() {
    let directory = tempfile::tempdir().expect("fixture");
    let path = directory.path().join("diamond.test.op");
    std::fs::write(&path, DIAMOND).expect("source");
    let project = NativeTestProject::load(directory.path(), &[path]).expect("sources");
    let checked = check_project(&project).expect("checked diamond and hooks");
    let lowered = lower_checked_project(&checked).expect("lowered canonical slots");
    let case = &lowered.cases[0];
    let names: Vec<_> = case
        .worlds
        .iter()
        .map(|world| world.identity.0.name.as_str())
        .collect();
    assert_eq!(names, ["Root", "Left", "Right", "Top"]);
    assert_eq!(
        case.body.arguments,
        vec![NativeInjection::WorldReference {
            identity: case.worlds[3].identity.clone(),
            index: 3
        }]
    );
    assert_eq!(
        case.worlds[3].setup.arguments,
        vec![
            NativeInjection::WorldReference {
                identity: case.worlds[1].identity.clone(),
                index: 1
            },
            NativeInjection::WorldReference {
                identity: case.worlds[2].identity.clone(),
                index: 2
            }
        ]
    );
    assert_eq!(
        case.worlds[3]
            .cleanup
            .as_ref()
            .expect("declared cleanup")
            .arguments,
        vec![
            NativeInjection::StateReference {
                identity: case.worlds[3].identity.clone(),
                index: 3
            },
            NativeInjection::WorldReference {
                identity: case.worlds[2].identity.clone(),
                index: 2
            }
        ]
    );
    assert_eq!(
        case.scopes[0].before[0].arguments,
        vec![
            NativeInjection::WorldReference {
                identity: case.worlds[1].identity.clone(),
                index: 1
            },
            NativeInjection::ContextReference
        ]
    );
}

/// Source dependencies are explicit; the diamond root is initialized only once.
const DIAMOND: &str = "world Root:\n    setup: f(): int64 =>\n        return 1\nworld Left:\n    worlds: Root\n    setup: f(ref root: Root): int64 =>\n        return root.state\nworld Right:\n    worlds: Root\n    setup: f(ref root: Root): int64 =>\n        return root.state + 1\nworld Top:\n    worlds: Left, Right\n    setup: f(ref left: Left, ref right: Right): int64 =>\n        return left.state + right.state\n    cleanup: f(ref state: int64, ref right: Right): void =>\n        expect state to be at least right.state\n        return void\nbefore each test: f(ref left: Left, mutable ref test: TestContext): void =>\n    expect left.state to be 1\n    return void\nscenario diamond:\n    worlds: Top\n    it should use the shared canonical graph: f(ref top: Top): void =>\n        expect top.state to be 3\n        return void\n";
