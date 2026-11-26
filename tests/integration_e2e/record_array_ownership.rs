use super::fs_helpers::unique_probe_target_dir;
use super::*;

fn run_record_ownership_fixture(project_name: &str) -> String {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join(project_name);
    let output_dir = unique_probe_target_dir(project_name);
    prepare_dir(&output_dir).expect("create record ownership output directory");
    let result = compile_project_for_tests(&project, &output_dir, &TargetTriple::host())
        .map_err(|error| error.to_string())
        .and_then(|binary| {
            run_binary_output_with_timeout(binary, Duration::from_secs(30), project_name)
        });
    cleanup_dir(&output_dir).expect("remove record ownership output directory");
    let output = result.expect("compile and run the record ownership fixture");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).expect("fixture output should be UTF-8")
}

#[test]
fn returned_record_owns_its_array_field() {
    assert_eq!(
        run_record_ownership_fixture("record-array-ownership").trim(),
        "stored=0, other=1"
    );
}

#[test]
fn record_keeps_its_array_after_field_binding_leaves_scope() {
    assert_eq!(
        run_record_ownership_fixture("array-field-let-ownership")
            .lines()
            .collect::<Vec<_>>(),
        ["alias=0", "stored=0, other=1", "payload=0, independent=1"]
    );
}
