//! Native callback-boundary lifecycle acceptance, without timing assumptions.
#![cfg(unix)]

#[test]
fn native_lifecycle_obeys_failure_boundaries_and_retains_secondary_failures() {
    let directory = tempfile::tempdir().expect("lifecycle fixture");
    let driver = directory.path().join("driver.c");
    let executable = directory.path().join("driver");
    std::fs::write(&driver, DRIVER).expect("C fixture");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let output = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&driver)
        .arg(runtime.join("opal_testing_runtime.c"))
        .arg(runtime.join("opal_testing_lifecycle.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("C compiler");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cases: [(i32, &str); 10] = [
        (0, "ABCDEFHGJIKL"),
        (1, "X"),
        (2, "ABL"),
        (3, "ABCJIKL"),
        (4, "ABCDEHGJIKL"),
        (5, "ABCDEFHGJIKL"),
        (6, "ABCDEFHGJIKL"),
        (7, "ABCDEFHGJIKL"),
        (8, "ABCDEFHGJIKL"),
        (9, "ABCDEFHGJIL"),
    ];
    for (scenario, expected) in cases {
        let result = std::process::Command::new(&executable)
            .arg(scenario.to_string())
            .arg(expected)
            .output()
            .expect("fresh native attempt");
        assert!(
            result.status.success(),
            "scenario {scenario}: {} {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

/// Uniform callback ABI matches compiler-generated typed wrapper boundaries.
const DRIVER: &str = r#"
#include "opal_testing_lifecycle.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static char log_text[64];
static size_t log_count;
static int scenario;
static int token;
static int dropped_values;
static void drop_value(void *value) { if (value == &token) ++dropped_values; }
static int event(char name, void **created, uint64_t *failure, const char **error) {
    log_text[log_count++] = name;
    if (created) *created = &token;
    int fail = (scenario == 1 && name == 'X') || (scenario == 2 && name == 'B') ||
        (scenario == 3 && name == 'C') || (scenario == 4 && name == 'E') ||
        (scenario == 5 && name == 'F') || (scenario == 6 && name == 'H') ||
        (scenario == 7 && name == 'K') || (scenario == 8 && (name == 'F' || name == 'H' || name == 'K'));
    if (!fail) return OPAL_TEST_CALLBACK_COMPLETED;
    if (name == 'F') {
        *failure = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, "body failed", "fixture.test.op", 10, 1);
        return OPAL_TEST_CALLBACK_ABORTED;
    }
    *error = "ApplicationError";
    return OPAL_TEST_CALLBACK_APPLICATION_ERROR;
}
#define CALLBACK(NAME) static int callback_##NAME(void **worlds, const void *example, void **created, uint64_t *failure, const char **error) { \
    (void)worlds; (void)example; return event(#NAME[0], created, failure, error); }
CALLBACK(X) CALLBACK(A) CALLBACK(B) CALLBACK(C) CALLBACK(D) CALLBACK(E)
CALLBACK(F) CALLBACK(G) CALLBACK(H) CALLBACK(I) CALLBACK(J) CALLBACK(K) CALLBACK(L)
#define CB(NAME) {callback_##NAME, "fixture.test.op", 0, 1}
int main(int argc, char **argv) {
    if (argc != 3) return 1;
    scenario = atoi(argv[1]);
    if (!opal_test_attempt_begin("native lifecycle", 1)) return 2;
    OpalTestWorldDescriptor worlds[] = {{"A", CB(A), CB(L), drop_value}, {"B", CB(B), CB(K), drop_value}};
    OpalTestCallback outer_before[] = {CB(C), CB(D)};
    OpalTestCallback inner_before[] = {CB(E)};
    OpalTestCallback outer_after[] = {CB(I), CB(J)};
    OpalTestCallback inner_after[] = {CB(G), CB(H)};
    OpalTestHookScope scopes[] = {{outer_before, 2, outer_after, 2}, {inner_before, 1, inner_after, 2}};
    OpalTestCaseDescriptor descriptor = {worlds, 2, scopes, 2, CB(F), {NULL, "", 0, 0}, NULL};
    if (scenario == 1) descriptor.construct_example = (OpalTestCallback)CB(X);
    if (scenario == 9) worlds[1].cleanup.invoke = NULL;
    OpalTestLifecycleResult result;
    int failed = opal_test_run_case(&descriptor, &result);
    if (strcmp(log_text, argv[2])) { fprintf(stderr, "expected %s, got %s\n", argv[2], log_text); return 3; }
    int expected_failure = scenario != 0 && scenario != 9;
    if (!!failed != expected_failure || !!opal_test_attempt_failed() != expected_failure) return 4;
    if ((scenario >= 1 && scenario <= 4) && result.body_started) return 5;
    if (scenario == 2 && result.initialized_worlds != 1) return 6;
    if (dropped_values != (scenario == 1 ? 0 : scenario == 2 ? 1 : 2)) return 13;
    if (scenario == 3 && result.entered_scopes != 1) return 7;
    if (scenario == 8) {
        if (opal_test_failure_count() != 3) return 8;
        const OpalTestFailure *first = opal_test_failure_at(0);
        if (first->phase != OPAL_TEST_PHASE_BODY || first->kind != OPAL_TEST_FAILURE_ASSERTION) return 9;
        if (opal_test_failure_at(1)->phase != OPAL_TEST_PHASE_AFTER_HOOK) return 10;
        if (opal_test_failure_at(2)->phase != OPAL_TEST_PHASE_WORLD_CLEANUP) return 11;
        if (opal_test_failure_at(2)->primary_failure_id != first->id) return 12;
    }
    size_t old_count = log_count;
    if (!opal_test_run_case(&descriptor, &result) || log_count != old_count) return 14;
    opal_test_attempt_end();
    return 0;
}
"#;
