//! C-level attachment storage owns its bytes independently of source mutation.
#![cfg(unix)]

#[test]
fn attachment_storage_copies_data_and_preserves_existing_primary_failures() {
    let directory = tempfile::tempdir().expect("attachment runtime");
    let source = directory.path().join("attachments.c");
    std::fs::write(&source,r#"
#include "opal_testing_attachments.h"
#include <string.h>
static unsigned observed;
static void attachment(const OpalTestAttachment *value, void *context) {
    if (context && value && value->name && value->text) ++observed;
}
int main(void) {
    int context = 1;
    if (!opal_test_attempt_begin("attachment-case", 1)) return 1;
    opal_test_set_attachment_observer(attachment, &context);
    char name[] = "owned name", text[] = "café original";
    if (opal_test_attach_text(&context, name, text)) return 2;
    name[0] = 'X'; text[0] = 'X';
    const OpalTestAttachment *value = opal_test_attachment_at(0);
    if (!value || strcmp(value->name, "owned name") || strcmp(value->text, "café original")) return 3;
    uint64_t primary = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, "body failed", "case.test.op", 10, 1);
    if (opal_test_attach_text(&context, "after", "after failure")) return 4;
    if (opal_test_primary_failure_id() != primary || opal_test_attachment_count() != 2 || observed != 2) return 5;
    if (!opal_test_attach_text(&context, "", "invalid")) return 6;
    if (opal_test_primary_failure_id() != primary || opal_test_attachment_count() != 2) return 7;
    opal_test_set_attachment_observer(NULL, NULL);
    opal_test_attachments_end();
    if (opal_test_attachment_count() || opal_test_attachment_at(0)) return 8;
    opal_test_attempt_end();
    return 0;
}
"#).expect("driver source");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let executable = directory.path().join("attachments");
    let compiler = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Wpedantic", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&source)
        .arg(runtime.join("opal_testing_runtime.c"))
        .arg(runtime.join("opal_testing_attachments.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compiler");
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let output = std::process::Command::new(executable)
        .output()
        .expect("driver");
    assert!(
        output.status.success(),
        "attachment driver {:?}",
        output.status.code()
    );
}
