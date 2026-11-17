//! Real C tests of attempt-local, compiler-typed double dispatch state.
#![cfg(unix)]

/// Driver callbacks stand in for compiler-generated typed value operations.
const DRIVER: &str = r#"
#include "opal_testing_doubles.h"
#include <stdlib.h>
#include <string.h>
typedef struct { int argument; int result; } Rule;
static int drops = 0;
static void release(void *value) { ++drops; free(value); }
static void *snapshot(const void *value) {
    int *copy = malloc(sizeof(int));
    if (copy) *copy = *(const int *)value;
    return copy;
}
static int matches(const void *configured, const void *actual) {
    return ((const Rule *)configured)->argument == *(const int *)actual;
}
static int overlaps(const void *left, const void *right) {
    return ((const Rule *)left)->argument == ((const Rule *)right)->argument;
}
static int history_matches(const void *expected, const void *actual) {
    return *(const int *)expected == *(const int *)actual;
}
static void describe(void *diagnostic, const void *arguments) {
    int64_t value = *(const int *)arguments;
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    opal_test_snapshot_text(diagnostic, "(");
    opal_test_snapshot_scalar(diagnostic, bits, 0);
    opal_test_snapshot_text(diagnostic, ")");
}
static Rule *rule(int argument, int result) {
    Rule *value = malloc(sizeof(Rule));
    if (value) { value->argument = argument; value->result = result; }
    return value;
}
int main(void) {
    if (!opal_test_attempt_begin("typed doubles", 1)) return 1;
    const OpalTestDoubleDescriptor descriptor = {"app->io.operation", "Write", snapshot, release};
    const OpalTestDoubleDescriptor conflict = {"app->io.operation", "OtherWrite", snapshot, release};
    const OpalTestDoubleDescriptor opaque = {"app->io.opaque", "Opaque", 0, 0};
    OpalTestDoubleSelection selected;
    int argument = 1;
    if (opal_test_double_select(descriptor.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_ORIGINAL) return 2;
    void *handle = 0;
    if (opal_test_double_activate(&descriptor, &handle) || !handle) return 3;
    void *other = 0;
    uint64_t conflict_id = opal_test_double_activate(&conflict, &other);
    if (!conflict_id || other || opal_test_failure_at(0)->kind != OPAL_TEST_FAILURE_INFRASTRUCTURE) return 4;
    if (opal_test_double_fallback(handle, OPAL_TEST_DOUBLE_ORIGINAL, 0, 0)) return 5;
    if (opal_test_double_add_exact(handle, OPAL_TEST_DOUBLE_RETURN, rule(1, 9), matches, overlaps, release, "double.test.op", 4, 8)) return 6;
    if (opal_test_double_select(descriptor.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_RETURN || ((Rule *)selected.payload)->result != 9) return 7;
    argument = 8;
    if (opal_test_double_select(descriptor.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_ORIGINAL) return 8;
    argument = 1;
    if (opal_test_double_select(descriptor.edge, &argument, &selected)) return 9;
    uint64_t count = 0;
    if (opal_test_double_count(handle, &count) || count != 3) return 10;
    int found = 0;
    argument = 8;
    if (opal_test_double_history_matches(handle, &argument, history_matches, &found) || !found) return 11;
    argument = 99;
    if (opal_test_double_history_matches(handle, &argument, history_matches, &found) || found) return 12;
    if (opal_test_double_verify(handle)) return 13;
    int previous_drops = drops;
    if (!opal_test_double_add_exact(handle, OPAL_TEST_DOUBLE_RETURN, rule(1, 99), matches, overlaps, release, "double.test.op", 40, 8) || drops != previous_drops + 1) return 14;
    if (opal_test_double_add_exact(handle, OPAL_TEST_DOUBLE_APPLICATION_ERROR, rule(2, 7), matches, overlaps, release, "double.test.op", 50, 8)) return 15;
    argument = 2;
    if (opal_test_double_select(descriptor.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_APPLICATION_ERROR || ((Rule *)selected.payload)->result != 7) return 16;
    if (opal_test_double_add_exact(handle, OPAL_TEST_DOUBLE_RETURN, rule(3, 3), matches, overlaps, release, "double.test.op", 60, 8)) return 17;
    uint64_t unused = opal_test_double_verify(handle);
    if (!unused || opal_test_failure_at(opal_test_failure_count()-1)->kind != OPAL_TEST_FAILURE_UNUSED_RULE) return 18;
    if (opal_test_double_restore(handle)) return 19;
    argument = 1;
    if (opal_test_double_select(descriptor.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_ORIGINAL) return 20;
    if (opal_test_double_count(handle, &count) || count != 4) return 21;
    if (opal_test_double_activate(&conflict, &other)) return 22;
    selected.kind = 99; selected.payload = &argument;
    uint64_t unexpected = opal_test_double_select(conflict.edge, &argument, &selected);
    if (!unexpected || selected.kind != 99 || selected.payload != &argument) return 23;
    if (opal_test_failure_at(opal_test_failure_count()-1)->kind != OPAL_TEST_FAILURE_UNEXPECTED_CALL) return 24;
    if (opal_test_double_restore(handle)) return 25;
    if (!opal_test_double_select(conflict.edge, &argument, &selected)) return 26;
    void *opaque_handle = 0;
    if (opal_test_double_activate(&opaque, &opaque_handle)) return 27;
    if (opal_test_double_fallback(opaque_handle, OPAL_TEST_DOUBLE_REPLACEMENT, rule(0, 17), release)) return 28;
    if (opal_test_double_select(opaque.edge, &argument, &selected) || selected.kind != OPAL_TEST_DOUBLE_REPLACEMENT) return 29;
    if (!opal_test_double_history_matches(opaque_handle, &argument, history_matches, &found)) return 30;
    if (opal_test_double_count(opaque_handle, &count) || count != 1) return 31;
    if (opal_test_primary_failure_id() != conflict_id) return 32;
    argument = 99;
    uint64_t mismatch = opal_test_double_history_failure(handle, &argument, describe, "double.test.op", 80, 3);
    const OpalTestFailure *diagnostic = opal_test_failure_at(opal_test_failure_count()-1);
    if (!mismatch || diagnostic->kind != OPAL_TEST_FAILURE_ASSERTION || diagnostic->offset != 80) return 34;
    if (!strstr(diagnostic->message, "actual (arguments) [(2)") || !strstr(diagnostic->message, "expected (arguments) (99)")) return 35;
    if (opal_test_primary_failure_id() != conflict_id) return 36;
    opal_test_doubles_end();
    /* Four original histories, two unexpected histories, three rules,
       one duplicate rejection and one replacement payload. */
    if (drops != 11) return 33;
    opal_test_attempt_end();
    return 0;
}
"#;

#[test]
fn double_runtime_preserves_edges_precedence_history_and_failure_channels() {
    let directory = tempfile::tempdir().expect("double runtime fixture");
    let driver = directory.path().join("driver.c");
    let executable = directory.path().join("driver");
    std::fs::write(&driver, DRIVER).expect("double C driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let output = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&driver)
        .arg(runtime.join("opal_testing_runtime.c"))
        .arg(runtime.join("opal_testing_doubles.c"))
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
        .expect("double runtime executable");
    assert!(
        execution.status.success(),
        "double runtime status {:?}: {}",
        execution.status.code(),
        String::from_utf8_lossy(&execution.stderr)
    );
}
