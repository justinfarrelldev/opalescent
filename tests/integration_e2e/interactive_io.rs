#![cfg(feature = "integration")]

use super::*;
use super::fs_helpers::unique_probe_target_dir;
use std::time::Duration;

const INTERACTIVE_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(feature = "integration")]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "integration test covers full stdin/stdout quiz flow"
)]
fn simple_quiz_compiles_links_and_runs() {
    let temp_dir = unique_probe_target_dir("simple-quiz");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "simple-quiz target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/simple-quiz/src/main.op");
        let source_result = fs::read_to_string(source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "simple-quiz source file should be readable from disk: {error}"
                ));
            }
        };

        let binary_result = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "simple-quiz source should compile and link into a binary: {error}"
                ));
            }
        };

        let child_result = std::process::Command::new(&binary_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let mut child = match child_result {
            Ok(child_process) => child_process,
            Err(error) => {
                return Err(format!(
                    "simple-quiz compiled binary should spawn with piped stdio: {error}"
                ));
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            let write_result = std::io::Write::write_all(&mut stdin, b"TestUser\n3\n");
            if let Err(error) = write_result {
                return Err(format!(
                    "simple-quiz stdin should accept scripted user input: {error}"
                ));
            }
            drop(stdin);
        } else {
            return Err(
                "simple-quiz process stdin should be piped so test input can be written".to_owned(),
            );
        }

        let run_output = super::fs_helpers::wait_for_child_output_with_timeout(
            child,
            INTERACTIVE_TEST_TIMEOUT,
            "simple-quiz compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("What is your name?") {
            return Err(format!(
                "simple-quiz stdout should contain prompt 'What is your name?', got: '{stdout}'"
            ));
        }

        if !stdout.contains("Hello, TestUser!") {
            return Err(format!(
                "simple-quiz stdout should contain greeting 'Hello, TestUser!', got: '{stdout}'"
            ));
        }

        if !stdout.contains("Guess a number") {
            return Err(format!(
                "simple-quiz stdout should contain prompt 'Guess a number', got: '{stdout}'"
            ));
        }

        if !stdout.contains("you guessed correctly")
            && !stdout.contains("too low")
            && !stdout.contains("Too high")
        {
            return Err(format!(
                "simple-quiz stdout should contain one of ['you guessed correctly', 'too low', 'Too high'] due to random outcome, got: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "simple-quiz binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "simple-quiz target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "simple-quiz end-to-end flow should compile, link, run with stdin, print prompts/results, and exit cleanly: {failure_message}"
    );
}
