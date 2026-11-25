use super::fs_helpers::unique_probe_target_dir;
use super::*;

#[test]
fn returned_record_owns_its_array_field() {
    let project =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/record-array-ownership");
    let output_dir = unique_probe_target_dir("record-array-ownership");
    prepare_dir(&output_dir).expect("create record ownership output directory");
    let result = compile_project_for_tests(&project, &output_dir, &TargetTriple::host())
        .map_err(|error| error.to_string())
        .and_then(|binary| {
            run_binary_output_with_timeout(
                binary,
                Duration::from_secs(30),
                "record array ownership",
            )
        });
    cleanup_dir(&output_dir).expect("remove record ownership output directory");
    let output = result.expect("compile and run the record ownership fixture");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "stored=0, other=1"
    );
}
