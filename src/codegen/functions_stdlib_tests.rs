use super::*;

#[test]
fn stdlib_names_registry_exists_and_has_correct_count() {
    assert_eq!(
        STDLIB_NAMES.len()
            + STRING_STDLIB_NAMES.len()
            + terminal_session::TERMINAL_SESSION_RUNTIME_NAMES.len(),
        195
    );
    assert!(is_stdlib_runtime_name("opal_runtime_error"));
    assert!(is_stdlib_runtime_name("print"));
    assert!(is_stdlib_runtime_name("random_int32"));
    assert!(is_stdlib_runtime_name("int64_to_int32"));
    assert!(!is_stdlib_runtime_name("int8_to_int16"));
    assert!(is_stdlib_runtime_name("current_working_directory_sync"));
    assert!(is_stdlib_runtime_name("exit_process"));
    assert!(is_stdlib_runtime_name("string_builder_push"));
}
