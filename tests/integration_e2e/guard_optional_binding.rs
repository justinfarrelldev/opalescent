#![cfg(feature = "integration")]
#![allow(
    clippy::pattern_type_mismatch,
    reason = "integration tests intentionally inspect borrowed error values"
)]

use super::*;
use super::fs_helpers::unique_probe_target_dir;
use opalescent::errors::reporter::CompilerError;
use opalescent::type_system::errors::TypeError;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn guard_shorthand_compiles_links_and_runs() {
    let temp_dir = unique_probe_target_dir("guard-shorthand-inline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard shorthand target directory should be created"
    );

    let source = "
import string_to_int32, int32_to_string from standard

##
    Description: Entry validates shorthand guards and named guard bindings
##
entry main = f(args: string[]): void =>
    guard string_to_int32('27') else parse_err =>
        print('UNEXPECTED_SHORTHAND_SUCCESS_ERROR={parse_err}')
        return void

    print('GUARD_SHORTHAND_SUCCESS=ok')

    guard string_to_int32('not-a-number') else invalid_err =>
        print('GUARD_SHORTHAND_ERROR=handled')
        return void

    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_guard_shorthand/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "guard shorthand source should compile and link into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "guard shorthand compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("GUARD_SHORTHAND_SUCCESS=ok") {
            return Err(format!(
                "guard shorthand binary stdout should contain GUARD_SHORTHAND_SUCCESS=ok, got: '{stdout}'"
            ));
        }
        if !stdout.contains("GUARD_SHORTHAND_ERROR=handled") {
            return Err(format!(
                "guard shorthand binary stdout should contain GUARD_SHORTHAND_ERROR=handled, got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "guard shorthand binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard shorthand target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard shorthand flow should compile, link, run, and validate shorthand handling behavior: {failure_message}"
    );
}

#[test]
fn guard_named_binding_still_compiles_links_and_runs() {
    let temp_dir = unique_probe_target_dir("guard-named-binding-inline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard named-binding target directory should be created"
    );

    let source = "
import string_to_int32, int32_to_string from standard

##
    Description: Entry validates named statement guard binding remains unchanged
##
entry main = f(args: string[]): void errors ParseError =>
    guard string_to_int32('7') into value else err =>
        print('ERR={err}')
        propagate err
    print('VALUE={int32_to_string(value)}')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_guard_named_binding/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "guard named-binding source should compile and link into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "guard named-binding compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("VALUE=7") {
            return Err(format!(
                "guard named-binding binary stdout should contain 'VALUE=7', got: '{stdout}'"
            ));
        }
        if stdout.contains("ERR=") {
            return Err(format!(
                "guard named-binding success path should not run else body, got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "guard named-binding binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard named-binding target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard named-binding flow should compile, link, run, and preserve success binding semantics: {failure_message}"
    );
}

#[test]
fn guard_error_clause_side_effect_then_propagate_err_compiles_links_and_runs() {
    let temp_dir = unique_probe_target_dir("guard-propagate-err-inline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard propagate-err target directory should be created"
    );

    let source = "
import string_to_int32 from standard

##
    Description: Inner helper logs the guard error before forwarding it unchanged
##
let parse_with_guard = f(text: string): int32 errors ParseError =>
    guard string_to_int32(text) into value else err =>
        print('INNER_GUARD_SEEN={err}')
        let handled: ParseError = err
        propagate err
    return value

##
    Description: Entry validates guard-clause propagate err lowers through normal error ABI
##
entry main = f(args: string[]): void errors ParseError =>
    guard parse_with_guard('oops') else err =>
        print('OUTER_PROPAGATED={err}')
        return void
    print('UNEXPECTED_SUCCESS')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_guard_propagate_err/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "guard propagate-err source should compile and link into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "guard propagate-err compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected_error_message = "invalid digit 'o' in input";
        if !stdout.contains(&format!("INNER_GUARD_SEEN={expected_error_message}")) {
            return Err(format!(
                "guard propagate-err stdout should show inner side effect before propagation, got: '{stdout}'"
            ));
        }
        if !stdout.contains(&format!("OUTER_PROPAGATED={expected_error_message}")) {
            return Err(format!(
                "guard propagate-err stdout should show the original guarded error reaches the outer handler, got: '{stdout}'"
            ));
        }
        if stdout.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "guard propagate-err failure path should not reach success marker, got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "guard propagate-err binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard propagate-err target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard propagate-err flow should compile, run, preserve inner side effects, and forward the original guard error: {failure_message}"
    );
}

#[test]
fn guard_error_clause_shadowed_err_still_propagates_original_guard_error() {
    let temp_dir = unique_probe_target_dir("guard-shadowed-err-inline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard shadowed-err target directory should be created"
    );

    let source = "
import string_to_int32 from standard

##
    Description: Inner helper proves shadowing the name err does not replace the active guard error payload
##
let parse_with_shadow = f(text: string): int32 errors ParseError =>
    guard string_to_int32(text) into value else err =>
        let err = 'shadowed-local-value'
        print('SHADOW_LOCAL={err}')
        let handled: ParseError = err
        propagate err
    return value

##
    Description: Entry validates guard propagation ignores the shadowed local binding
##
entry main = f(args: string[]): void errors ParseError =>
    guard parse_with_shadow('oops') else err =>
        print('OUTER_PROPAGATED={err}')
        return void
    print('UNEXPECTED_SUCCESS')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let compile_result = compile_program_for_tests(
            Path::new("test-projects/_guard_shadowed_err/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        if compile_result.is_ok() {
            return Err(
                "guard shadowed-err source should fail strict front-end validation in this fixture"
                    .to_owned(),
            );
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard shadowed-err target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard shadowed-err fixture should be rejected by strict guard validation: {failure_message}"
    );
}

#[test]
fn guard_error_clause_return_err_stays_rejected() {
    let temp_dir = unique_probe_target_dir("guard-return-err-rejected-inline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "guard return-err rejection target directory should be created"
    );

    let source = "
import string_to_int32 from standard

##
    Description: Entry proves return err remains rejected inside guard error clauses
##
entry main = f(args: string[]): void =>
    guard string_to_int32('bad') else err =>
        return err
    print('UNEXPECTED_SUCCESS')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let compile_result = compile_program_for_tests(
            Path::new("test-projects/_guard_return_err_rejected/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        let compile_error = match compile_result {
            Ok(_path) => {
                return Err(
                    "guard return-err source should fail to compile, but compilation succeeded"
                        .to_owned(),
                );
            }
            Err(error) => error,
        };

        let CompileError::Report { report, .. } = compile_error else {
            return Err(format!(
                "guard return-err source should fail with CompileError::Report, got: {compile_error}"
            ));
        };

        let has_expected_error = report.entries().iter().any(|entry| {
            matches!(
                &entry.1,
                CompilerError::TypeChecker(TypeError::GuardReturnErrInvalid { .. })
            )
        });
        if !has_expected_error {
            return Err(format!(
                "guard return-err source should keep the dedicated type-check rejection after codegen changes, got: {:?}",
                report.entries()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "guard return-err rejection target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "guard return-err should remain rejected before codegen lowering can run: {failure_message}"
    );
}
