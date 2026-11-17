//! Standalone C path-byte copying and deterministic allocator-failure coverage.
#![cfg(unix)]

#[test]
fn path_value_snapshots_copy_utf8_and_fail_without_fabricating_empty_paths() {
    let directory = tempfile::tempdir().expect("path value fixture");
    let driver = directory.path().join("path.c");
    let executable = directory.path().join("path");
    std::fs::write(
        &driver,
        r#"
#include "opal_portability.h"
#include "opal_runtime.h"
#include "opal_testing_values.h"
#include <stdlib.h>
#include <string.h>
static int allocation_fails;
char *opal_string_alloc(uint64_t bytes) {
    return allocation_fails ? NULL : malloc((size_t)bytes);
}
int main(void) {
    char source[] = "café/path.txt";
    char *copy = opal_test_path_snapshot(source);
    if (!copy || copy == source) return 1;
    source[0] = 'X';
    if (strcmp(copy, "café/path.txt")) return 2;
    free(copy);
    allocation_fails = 1;
    if (opal_test_path_snapshot(source) != NULL) return 3;
    if (opal_test_path_snapshot(NULL) != NULL) return 4;
    return 0;
}
"#,
    )
    .expect("path copy driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    let compiler = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Wpedantic", "-Werror"])
        .arg("-I")
        .arg(&runtime)
        .arg(&driver)
        .arg(runtime.join("opal_testing_values.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("C compiler");
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let execution = std::process::Command::new(&executable)
        .output()
        .expect("path copy executable");
    assert!(
        execution.status.success(),
        "path copy status {:?}",
        execution.status.code()
    );
}
