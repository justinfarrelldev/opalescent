//! Real C diagnostic storage ownership, bounded UTF-8 and scalar value bits.
#![cfg(unix)]

#[test]
fn diagnostic_snapshots_own_utf8_and_preserve_scalar_value_bits() {
    let directory = tempfile::tempdir().expect("snapshot fixture");
    let driver = directory.path().join("snapshot.c");
    let executable = directory.path().join("snapshot");
    std::fs::write(&driver, r#"
#include "opal_testing_runtime.h"
#include <string.h>
#include <stdio.h>
int main(void) {
    if (!opal_test_attempt_begin("snapshot", 1)) return 1;
    char value[] = "original café";
    void *actual = opal_test_snapshot_new();
    void *expected = opal_test_snapshot_new();
    opal_test_snapshot_text(actual, value);
    value[0] = 'X';
    opal_test_snapshot_scalar(expected, UINT64_MAX, 1);
    uint64_t first = opal_test_snapshot_failure(actual, expected, "equality", "string", "snapshot.test.op", 8, 9);
    if (!first || !strstr(opal_test_failure_at(0)->message, "original café")) return 2;
    if (!strstr(opal_test_failure_at(0)->message, "18446744073709551615")) return 3;
    actual = opal_test_snapshot_new();
    expected = opal_test_snapshot_new();
    double number = 1.25;
    uint64_t bits;
    memcpy(&bits, &number, sizeof(bits));
    opal_test_snapshot_scalar(actual, bits, 3);
    opal_test_snapshot_scalar(expected, UINT64_MAX, 0);
    opal_test_snapshot_failure(actual, expected, "equality", "number", "snapshot.test.op", 8, 9);
    if (!strstr(opal_test_failure_at(1)->message, "1.25") || !strstr(opal_test_failure_at(1)->message, "-1")) return 4;
    actual = opal_test_snapshot_new();
    expected = opal_test_snapshot_new();
    char unicode[6001];
    for (int index = 0; index < 6000; index += 3) memcpy(unicode + index, "界", 3);
    unicode[6000] = 0;
    opal_test_snapshot_text(actual, unicode);
    if (opal_test_snapshot_available(actual)) return 5;
    opal_test_snapshot_failure(actual, expected, "equality", "string", "snapshot.test.op", 8, 9);
    const OpalTestFailure *failure = opal_test_failure_at(2);
    if (!strstr(failure->message, "...") || strlen(failure->message) > 4607) return 6;
    puts(failure->message);
    opal_test_snapshot_failure(0, 0, "equality", "string", "snapshot.test.op", 8, 9);
    if (opal_test_failure_at(3)->kind != OPAL_TEST_FAILURE_INFRASTRUCTURE) return 7;
    if (opal_test_primary_failure_id() != first) return 8;
    opal_test_attempt_end();
    return 0;
}
"#).expect("snapshot C driver");
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
    let execution = std::process::Command::new(&executable)
        .output()
        .expect("snapshot executable");
    assert!(
        execution.status.success(),
        "snapshot status {:?}",
        execution.status.code()
    );
    let text = String::from_utf8(execution.stdout).expect("UTF-8 codepoint-safe truncation");
    assert!(text.contains("界"));
    assert!(text.contains("..."));
}
