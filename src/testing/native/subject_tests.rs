//! Canonical function-association resolution acceptance tests.
use super::project::NativeTestProject;
use super::subjects::resolve_subject;
use crate::ast::testing::SourceReference;
use crate::token::{Position, Span};
use std::path::Path;

/// Unresolved function-reference syntax for compiler-only tests.
fn reference(name: &str) -> SourceReference {
    SourceReference {
        path: name.split('.').map(str::to_owned).collect(),
        span: Span::single(Position::start()),
    }
}

#[test]
fn import_aliases_report_the_defining_function_not_the_alias() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("arithmetic.op"),
        "public let add = f(left: int64, right: int64): int64 =>\n    return left + right\n",
    )
    .expect("application");
    std::fs::write(
        directory.path().join("case.test.op"),
        "import add as combine, add as plus from ./arithmetic\n",
    )
    .expect("test imports");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("source graph");
    let first = resolve_subject(&project, &project.test_files[0], &reference("combine"))
        .expect("first alias");
    let second = resolve_subject(&project, &project.test_files[0], &reference("plus"))
        .expect("second alias");
    assert_eq!(first, second);
    assert_eq!(first.0.module, "arithmetic.op");
    assert_eq!(first.0.name, "add");
}

#[test]
fn missing_private_and_non_callable_subjects_are_compiler_errors() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("arithmetic.op"),
        "let private_add = f(): int64 =>\n    return 1\npublic let number = 42\n",
    )
    .expect("application");
    std::fs::write(
        directory.path().join("case.test.op"),
        "import private_add, number from ./arithmetic\nlet local_number = 1\n",
    )
    .expect("test imports");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    for name in ["missing", "private_add", "number", "local_number"] {
        assert!(
            resolve_subject(&project, &project.test_files[0], &reference(name)).is_err(),
            "accepted {name}"
        );
    }
}

#[test]
fn qualified_standard_aliases_resolve_through_existing_module_interfaces() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("case.test.op"),
        "import math as numbers from math\n",
    )
    .expect("module alias");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    let subject = resolve_subject(
        &project,
        &project.test_files[0],
        &reference("numbers.random_int32"),
    )
    .expect("qualified genuine function");
    assert_eq!(subject.0.module, "math");
    assert_eq!(subject.0.name, "random_int32");
    assert!(
        resolve_subject(
            &project,
            &project.test_files[0],
            &reference("numbers.missing")
        )
        .is_err()
    );
}

#[test]
fn fixture_subject_resolves_to_its_actual_application_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("sources");
    let subject = resolve_subject(&project, &project.test_files[0], &reference("add"))
        .expect("function declaration");
    assert_eq!(subject.0.module, "src/arithmetic.op");
}
