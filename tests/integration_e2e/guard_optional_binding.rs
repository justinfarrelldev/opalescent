#![cfg(feature = "integration")]

use super::*;

#[test]
fn guard_shorthand_compiles_links_and_runs() {
    let temp_dir = Path::new("test-projects/_guard_shorthand/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "guard shorthand target directory should be created"
    );

    let source = "
import string_to_int32, int32_to_string from standard

##
    Description: Entry validates shorthand statement guard codegen path
##
entry main = f(args: string[]): void =>
    guard string_to_int32('42') else err =>
        print('ERR={err}')
        return void
    print('OK')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program(
            Path::new("test-projects/_guard_shorthand/src/main.op"),
            source,
            temp_dir,
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

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "guard shorthand compiled binary should execute: {error}"
                ));
            }
        };

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("OK") {
            return Err(format!(
                "guard shorthand binary stdout should contain 'OK', got: '{stdout}'"
            ));
        }
        if stdout.contains("ERR=") {
            return Err(format!(
                "guard shorthand success path should not run else body, got: '{stdout}'"
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

    let cleanup = cleanup_dir(temp_dir);
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
        "guard shorthand flow should compile, link, run, and skip else branch: {failure_message}"
    );
}

#[test]
fn guard_named_binding_still_compiles_links_and_runs() {
    let temp_dir = Path::new("test-projects/_guard_named_binding/target");
    let prepare = prepare_dir(temp_dir);
    assert!(
        prepare.is_ok(),
        "guard named-binding target directory should be created"
    );

    let source = "
import string_to_int32, int32_to_string from standard

##
    Description: Entry validates named statement guard binding remains unchanged
##
entry main = f(args: string[]): void =>
    guard string_to_int32('7') into value else err =>
        print('ERR={err}')
        return void
    print('VALUE={int32_to_string(value)}')
    return void
";

    let execution_result: Result<(), String> = (|| {
        let binary_result = compile_program(
            Path::new("test-projects/_guard_named_binding/src/main.op"),
            source,
            temp_dir,
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

        let output_result = std::process::Command::new(&binary_path).output();
        let run_output = match output_result {
            Ok(output) => output,
            Err(error) => {
                return Err(format!(
                    "guard named-binding compiled binary should execute: {error}"
                ));
            }
        };

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

    let cleanup = cleanup_dir(temp_dir);
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
