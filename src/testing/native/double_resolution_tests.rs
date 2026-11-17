//! Canonical import-edge resolution, independent of runtime activation.
use super::doubles::resolve_doubles;
use super::project::NativeTestProject;

#[test]
fn double_metadata_loads_and_resolves_the_exact_aliased_application_import_edge() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("case.test.op"),
        "double Write replaces write from ./io\n    as used by ./buffer\n",
    )
    .expect("double declaration");
    std::fs::write(
        directory.path().join("io.op"),
        "public let write = f(): int64 =>\n    return 1\n",
    )
    .expect("operation");
    std::fs::write(directory.path().join("buffer.op"), "import write as persist from ./io\npublic let save = f(): int64 =>\n    return persist()\n").expect("importing module");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("source graph");
    assert_eq!(
        project.modules.len(),
        3,
        "declaration resolution loads source but executes no function"
    );
    let resolved = resolve_doubles(&project).expect("supported direct edge");
    assert_eq!(resolved.declarations.len(), 1);
    let declaration = resolved
        .declarations
        .values()
        .next()
        .expect("resolved double");
    assert_eq!(declaration.edge.importing_module, "buffer.op");
    assert_eq!(declaration.edge.imported_function.0.module, "io.op");
    assert_eq!(declaration.edge.imported_function.0.name, "write");
}

#[test]
fn explicit_double_imports_preserve_provider_identity_through_aliases() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("case.test.op"),
        "import double Write as First, Write as Second from ./doubles\n",
    )
    .expect("imports");
    std::fs::write(
        directory.path().join("doubles.op"),
        "double Write replaces write from ./io\n    as used by ./buffer\n",
    )
    .expect("double support");
    std::fs::write(
        directory.path().join("io.op"),
        "public let write = f(): int64 =>\n    return 1\n",
    )
    .expect("operation");
    std::fs::write(
        directory.path().join("buffer.op"),
        "import write from ./io\n",
    )
    .expect("edge");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    let resolved = resolve_doubles(&project).expect("aliases");
    let bindings = &resolved.bindings[&project.test_files[0]];
    assert_eq!(bindings["First"], bindings["Second"]);
    assert_eq!(bindings["First"].0.module, "doubles.op");
    assert_eq!(bindings["First"].0.name, "Write");
}

#[test]
fn a_same_module_function_or_missing_import_edge_is_not_global_replacement() {
    let directory = tempfile::tempdir().expect("fixture directory");
    std::fs::write(
        directory.path().join("case.test.op"),
        "double Write replaces write from ./io\n    as used by ./buffer\n",
    )
    .expect("declaration");
    std::fs::write(
        directory.path().join("io.op"),
        "public let write = f(): int64 =>\n    return 1\n",
    )
    .expect("operation");
    std::fs::write(
        directory.path().join("buffer.op"),
        "let write = f(): int64 =>\n    return 2\n",
    )
    .expect("same-spelled local declaration");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    assert!(
        resolve_doubles(&project).is_err(),
        "same spelling is not a supported source import edge"
    );
}
