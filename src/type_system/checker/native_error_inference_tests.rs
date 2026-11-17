//! Framework callback inference uses only existing escaping application errors.
use super::TypeChecker;
use super::native_testing_tests::parse_callback;
use crate::type_system::types::CoreType;

#[test]
fn infers_nominal_errors_but_not_assertion_failures() {
    let callback = parse_callback(
        "f(): void errors inferred =>\n        let number = propagate string_to_int32('42')\n        expect number to be 42\n        return void\n",
    );
    let mut checker = TypeChecker::new();
    let result = checker
        .type_check_native_callback(&callback, &[])
        .expect("inferred callback");
    assert!(
        matches!(result, CoreType::Function { ref error_types, .. } if error_types.len() == 1 && error_types[0].to_string() == "ParseError")
    );
    let assertion_only = parse_callback(
        "f(): void errors inferred =>\n        expect 1 to be 2\n        return void\n",
    );
    let assertion_result = checker
        .type_check_native_callback(&assertion_only, &[])
        .expect("assertions do not widen errors");
    assert!(
        matches!(assertion_result, CoreType::Function { ref error_types, .. } if error_types.is_empty())
    );
}

#[test]
fn handled_errors_do_not_escape_but_guard_repropagation_is_inferred() {
    let handled = parse_callback(
        "f(): void errors inferred =>\n        let number: int32 = guard string_to_int32('bad') into value: int32 else 0\n        print(number)\n        return void\n",
    );
    let mut checker = TypeChecker::new();
    let handled_type = checker
        .type_check_native_callback(&handled, &[])
        .expect("handled error");
    assert!(
        matches!(handled_type, CoreType::Function { ref error_types, .. } if error_types.is_empty())
    );
    let propagated = parse_callback(
        "f(): void errors inferred =>\n        guard string_to_int32('bad') into number else err =>\n            print(err)\n            propagate err\n        print(number)\n        return void\n",
    );
    let propagated_type = checker
        .type_check_native_callback(&propagated, &[])
        .expect("guard propagation");
    assert!(
        matches!(propagated_type, CoreType::Function { ref error_types, .. } if error_types.len() == 1 && error_types[0].to_string() == "ParseError")
    );
}

#[test]
fn ordinary_nested_helpers_keep_their_explicit_error_rules() {
    let callback = parse_callback(
        "f(): void errors inferred =>\n        let helper = f(): void =>\n            let value = propagate string_to_int32('42')\n            print(value)\n            return void\n        helper()\n        return void\n",
    );
    let mut checker = TypeChecker::new();
    assert!(
        checker.type_check_native_callback(&callback, &[]).is_err(),
        "framework inference must not leak into ordinary nested helpers"
    );
}
