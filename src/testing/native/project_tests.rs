//! Explicit test/support module loading and ordinary dependency grammar tests.
use super::project::{LoadedModuleAst, NativeTestProject};
use std::path::Path;

#[test]
fn loads_the_minimal_fixture_without_executing_world_setup() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project = NativeTestProject::load(&root, &[root.join("tests/addition.test.op")])
        .expect("valid sources");
    assert_eq!(project.modules.len(), 3);
    assert!(
        project
            .modules
            .values()
            .any(|module| matches!(module.ast, LoadedModuleAst::Test(_)))
    );
    assert!(
        project
            .modules
            .values()
            .any(|module| matches!(module.ast, LoadedModuleAst::Application(_)))
    );
}

#[test]
fn support_imports_select_test_mode_but_their_application_dependencies_do_not() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(directory.path().join("case.test.op"), "import world Workspace from ./worlds\nscenario behavior:\n    it should run: f(): void =>\n        return void\n").expect("case");
    std::fs::write(directory.path().join("worlds.op"), "import create_state from ./application\nworld Workspace:\n    setup: f(): int64 =>\n        return create_state()\n").expect("World source");
    std::fs::write(
        directory.path().join("application.op"),
        "let scenario = 2\npublic let create_state = f(): int64 =>\n    return scenario\n",
    )
    .expect("application source");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("valid module modes");
    assert_eq!(project.modules.len(), 3);
    std::fs::write(
        directory.path().join("application.op"),
        "world Forbidden:\n    setup: f(): int64 =>\n        return 2\n",
    )
    .expect("invalid ordinary dependency");
    assert!(
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .is_err()
    );
}

#[test]
fn world_declarations_cannot_bypass_types_file_restrictions() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("case.test.op"),
        "import world Workspace from ./worlds.types\n",
    )
    .expect("case source");
    std::fs::write(
        directory.path().join("worlds.types.op"),
        "world Workspace:\n    setup: f(): int64 =>\n        return 2\n",
    )
    .expect("invalid types source");
    assert!(
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .is_err()
    );
}
