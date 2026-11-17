//! Test-only native runtime failure state, linked and exercised as real C code.
#![cfg(unix)]

#[test]
fn native_runtime_preserves_primary_secondary_identity_and_owned_diagnostics() {
    let directory = tempfile::tempdir().expect("runtime fixture");
    let driver = directory.path().join("driver.c");
    let executable = directory.path().join("driver");
    std::fs::write(&driver, r#"
#include "opal_testing_runtime.h"
#include <string.h>
int main(void) {
    if (!opal_test_attempt_begin("tests/example.op::case", 1)) return 1;
    opal_test_set_phase(OPAL_TEST_PHASE_BODY);
    char message[] = "primary assertion";
    uint64_t primary = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, message, "case.test.op", 12, 4);
    if (!primary || !opal_test_attempt_failed()) return 2;
    message[0] = 'x';
    if (strcmp(opal_test_failure_at(0)->message, "primary assertion")) return 3;
    opal_test_record_secondary_application_error(primary, "CleanupError");
    if (opal_test_primary_failure_id() != primary || opal_test_failure_count() != 2) return 4;
    const OpalTestFailure *cleanup = opal_test_failure_at(1);
    if (cleanup->parent_failure_id != primary || cleanup->primary_failure_id != primary) return 5;
    if (cleanup->kind != OPAL_TEST_FAILURE_APPLICATION_ERROR || cleanup->phase != OPAL_TEST_PHASE_BODY) return 6;
    opal_test_set_phase(OPAL_TEST_PHASE_AFTER_HOOK);
    uint64_t after = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, "after hook failed", "case.test.op", 90, 3);
    if (!after || after == primary || opal_test_primary_failure_id() != primary) return 7;
    if (opal_test_failure_at(2)->primary_failure_id != primary) return 8;
    if (opal_test_failure_at(3) != 0) return 9;
    const OpalTestFailure *first = opal_test_failure_at(0);
    for (int index = 0; index < 40; ++index)
        opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, "more diagnostics", "case.test.op", 100, 1);
    if (opal_test_failure_at(0) != first) return 11;
    if (first->id != primary || strcmp(first->message, "primary assertion")) return 12;
    opal_test_attempt_end();
    if (opal_test_attempt_begin("reused worker", 2)) return 10;
    return 0;
}
"#).expect("C driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let output = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&driver)
        .arg(runtime.join("opal_testing_runtime.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("C compiler");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status = std::process::Command::new(&executable)
        .status()
        .expect("native runtime fixture");
    assert!(
        status.success(),
        "runtime failure-state invariant failed: {status}"
    );
}
