//! Test support may import testing types; ordinary application sources may not.
use super::{checking::check_project, project::NativeTestProject};

#[test]
fn test_support_type_modules_have_test_only_import_permission_without_granting_it_to_apps() {
    for (folder, allowed) in [("tests", true), ("src", false)] {
        let directory = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(directory.path().join(folder)).expect("module directory");
        std::fs::create_dir_all(directory.path().join("tests")).expect("test directory");
        let support = directory.path().join(folder).join("support.types.op");
        std::fs::write(&support,"import type TerminalProcess from standard.testing\n\npublic type SupportState:\n    process: TerminalProcess\n").expect("support type");
        let source = directory.path().join("tests/case.test.op");
        let import = if allowed {
            "./support.types"
        } else {
            "../src/support.types"
        };
        std::fs::write(&source,format!("import type SupportState from {import}\nscenario module availability:\n    it should check availability: f(): void =>\n        return void\n")).expect("test source");
        let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
        let result = check_project(&project);
        assert_eq!(result.is_ok(), allowed, "{:?}", result.err());
    }
}
