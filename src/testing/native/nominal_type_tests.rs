//! Native checking must distinguish declarations, not just their spelling.
use super::checking::check_project;
use super::project::NativeTestProject;
use std::path::Path;

/// Two distinct nominal declarations intentionally have identical enum shapes.
fn nominal_sources(root: &Path) {
    for name in ["left.types.op", "right.types.op"] {
        std::fs::write(root.join(name), "##\nDescription: A nominal error belonging only to its defining module.\n##\npublic type BoundaryError:\n    Broken\npublic type Data:\n    value: int64\npublic type Container:\n    inner: Data\n").expect("type source");
    }
    std::fs::write(root.join("io.op"), "import type BoundaryError from ./left.types\n##\nDescription: An application boundary declaring the original nominal error.\n##\npublic let operation = f(value: int64): int64 errors BoundaryError =>\n    return value\n").expect("boundary");
    std::fs::write(root.join("app.op"), "import operation from ./io\nimport type BoundaryError from ./left.types\n##\nDescription: Calls the imported operation at a concrete source edge.\n##\npublic let apply = f(value: int64): int64 errors BoundaryError =>\n    return propagate operation(value)\n").expect("application");
}

#[test]
fn implicit_runner_context_cannot_be_redeclared_by_later_source_types() {
    let directory = tempfile::tempdir().expect("fixture");
    std::fs::write(
        directory.path().join("context.types.op"),
        "public type TestContext:\n    value: int64\n",
    )
    .expect("ordinary type");
    std::fs::write(directory.path().join("case.test.op"), "import type TestContext from ./context.types\nscenario context identity:\n    it should preserve the runner context contract: f(): void =>\n        expect true to be true\n        return void\n").expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    assert!(
        check_project(&project).is_err(),
        "source declarations cannot replace runner-owned context identity"
    );
}

#[test]
fn canonical_source_typing_does_not_enable_test_only_imports_in_applications() {
    let directory = tempfile::tempdir().expect("fixture");
    std::fs::write(directory.path().join("app.op"), "import terminal_session_open_sync from standard.testing.terminal\n##\nDescription: An ordinary application helper must not gain testing imports.\n##\npublic let value = f(): int64 =>\n    return 1\n").expect("application");
    std::fs::write(directory.path().join("case.test.op"), "import value from ./app\nscenario application isolation:\n    it should preserve test-only availability gates: f(): void =>\n        expect value() to be 1\n        return void\n").expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("ordinary grammar sources");
    let errors = match check_project(&project) {
        Err(super::checking::NativeCheckError::Type { errors, .. }) => errors,
        _ => Vec::new(),
    };
    assert!(errors.iter().any(|error| matches!(*error, crate::type_system::errors::TypeError::ModuleUnavailable { ref module, .. } if module == "standard.testing.terminal")), "{errors:?}");
}

#[test]
fn source_error_aliases_cannot_be_reinterpreted_as_same_spelled_standard_families() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    std::fs::write(directory.path().join("io.op"), "import type BoundaryError as FilesystemWriteError from ./left.types\n##\nDescription: A genuine source alias must retain its original nominal error.\n##\npublic let operation = f(value: int64): int64 errors FilesystemWriteError =>\n    return value\n").expect("aliased boundary");
    std::fs::write(directory.path().join("case.test.op"), "import type BoundaryError as Allowed from ./left.types\ndouble Writer replaces operation from ./io\n    as used by ./app\nscenario error alias priority:\n    it should preserve the declared nominal source error: f(): void =>\n        let writer = new Writer\n        stub writer(1) fails with Allowed.Broken\n        return void\n").expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    check_project(&project).expect("genuine source error alias");
}

#[test]
fn mock_errors_cannot_invent_missing_variant_or_required_payload_values() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    for (declaration, reference) in [
        ("public type BoundaryError:\n    Broken\n", "AllowedError"),
        (
            "public type BoundaryError:\n    Broken:\n        message: string\n",
            "AllowedError.Broken",
        ),
    ] {
        std::fs::write(directory.path().join("left.types.op"), declaration).expect("error shape");
        let source = format!(
            "import type BoundaryError as AllowedError from ./left.types\ndouble Writer replaces operation from ./io\n    as used by ./app\nscenario invalid mock errors:\n    it should require an actual constructible error variant: f(): void =>\n        let writer = new Writer\n        stub writer(1) fails with {reference}\n        return void\n"
        );
        std::fs::write(directory.path().join("case.test.op"), source).expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("sources");
        assert!(
            check_project(&project).is_err(),
            "mock errors must not fabricate a variant or payload for {reference}"
        );
    }
}

#[test]
fn public_layout_fields_retain_canonical_source_type_identities() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    std::fs::write(directory.path().join("case.test.op"), "import type Container from ./left.types\nscenario layout metadata:\n    it should retain field origins: f(): void =>\n        expect true to be true\n        return void\n").expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    let checked = check_project(&project).expect("checked source");
    let path = directory.path().join("left.types.op");
    let interface = checked.modules[&path]
        .checker
        .module_interface(&path.display().to_string())
        .expect("interface");
    let manifest = interface
        .adt_layout_manifests
        .values()
        .find(|layout| layout.type_id.type_name == "Container")
        .expect("public layout");
    let fields = match manifest.kind {
        crate::type_system::AdtLayoutManifestKind::Product { ref fields } => Some(fields),
        _ => None,
    }
    .expect("product layout");
    assert_eq!(fields[0].core_type, interface.exports["Data"].core_type);
}

#[test]
fn typed_local_annotations_resolve_record_aliases_to_the_same_defining_type() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    let source = "import type Data as FirstData from ./left.types\nimport type Data as AliasData from ./left.types\nscenario typed data aliases:\n    it should preserve aliases in ordinary annotated bindings: f(): void =>\n        let first: FirstData = new FirstData:\n            value: 1\n        let other: AliasData = new AliasData:\n            value: 1\n        expect first to be other\n        return void\n";
    std::fs::write(directory.path().join("case.test.op"), source).expect("case");
    let project =
        NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
            .expect("sources");
    check_project(&project).expect("typed local record aliases");
}

#[test]
fn stub_errors_use_nominal_defining_identity_through_import_aliases() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    for (alias, expected) in [("AllowedError", true), ("WrongError", false)] {
        let source = format!(
            "import type BoundaryError as AllowedError from ./left.types\nimport type BoundaryError as WrongError from ./right.types\ndouble Writer replaces operation from ./io\n    as used by ./app\nscenario nominal errors:\n    it should reject a same-spelled different declaration: f(): void =>\n        let writer = new Writer\n        stub writer(1) fails with {alias}.Broken\n        return void\n"
        );
        std::fs::write(directory.path().join("case.test.op"), source).expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("sources");
        let checked = check_project(&project);
        assert_eq!(checked.is_ok(), expected, "{alias}: {:?}", checked.err());
    }
}

#[test]
fn variant_matchers_reject_same_spelled_different_nominal_owners() {
    let directory = tempfile::tempdir().expect("fixture");
    nominal_sources(directory.path());
    for (alias, expected) in [("AllowedError", true), ("WrongError", false)] {
        let source = format!(
            "import type BoundaryError as AllowedError from ./left.types\nimport type BoundaryError as WrongError from ./right.types\nscenario nominal variants:\n    it should compare genuine defining types: f(): void =>\n        expect new AllowedError.Broken to be the Broken variant of {alias}\n        return void\n"
        );
        std::fs::write(directory.path().join("case.test.op"), source).expect("case");
        let project =
            NativeTestProject::load(directory.path(), &[directory.path().join("case.test.op")])
                .expect("sources");
        let checked = check_project(&project);
        assert_eq!(checked.is_ok(), expected, "{alias}: {:?}", checked.err());
    }
}
