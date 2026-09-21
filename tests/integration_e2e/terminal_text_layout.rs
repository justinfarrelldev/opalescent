#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn terminal_text_layout_project_handles_unicode_cell_widths() {
    const SOURCE_PATH: &str = "test-projects/terminal-text-layout/src/main.op";
    let source = std::fs::read_to_string(SOURCE_PATH)
        .expect("terminal-text-layout fixture should be readable");

    let temp_dir = unique_probe_target_dir("terminal-text-layout");
    prepare_dir(&temp_dir).expect("terminal-text-layout target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            &source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal-text-layout source should compile: {error}"))?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-text-layout compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-text-layout binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = concat!(
            "WIDTH_ASCII=3\n",
            "WIDTH_COMBINING=1\n",
            "WIDTH_CJK=2\n",
            "WIDTH_EMOJI=2\n",
            "WIDTH_ZWJ=2\n",
            "WIDTH_FLAG=2\n",
            "WIDTH_KEYCAP=2\n",
            "CLIP_ASCII=[abcd]/4\n",
            "CLIP_COMBINING=[e\u{301}]/1\n",
            "CLIP_CJK=[a界]/3\n",
            "CLIP_EMOJI=[🙂]/2\n",
            "CLIP_FLAG=[🇺🇸]/2\n",
            "CLIP_KEYCAP=[1\u{FE0F}\u{20E3}]/2\n",
        );
        if stdout != expected {
            return Err(format!(
                "terminal-text-layout stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-text-layout target directory should be removed");

    assert!(
        execution_result.is_ok(),
        "terminal-text-layout should compile, run, and match expected output: {}",
        execution_result.err().unwrap_or_default()
    );
}

#[test]
fn terminal_text_layout_negative_limit_reports_specific_error_leaf() {
    let temp_dir = unique_probe_target_dir("terminal-text-layout-negative-limit");
    prepare_dir(&temp_dir).expect("terminal-text-layout negative target directory should be created");

    let source = "import terminal_text_clip_to_cells from standard\n\n##\n  Description: Exercises negative terminal text layout limits.\n##\nentry main = f(): void errors TerminalTextLayoutError, AllocationFailureError => {\n    let clipped, used_cells = propagate terminal_text_clip_to_cells('abc', -1 as int64)\n    print('unexpected {clipped} {used_cells}')\n    return void\n}";

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-text-layout-negative-limit/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal-text-layout negative source should compile: {error}"))?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-text-layout negative compiled binary",
        )?;

        if run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            return Err(format!(
                "terminal-text-layout negative binary should fail, stdout:\n{stdout}"
            ));
        }

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if !stderr.contains("NegativeCellLimit") {
            return Err(format!(
                "negative limit stderr should mention NegativeCellLimit, got:\n{stderr}"
            ));
        }
        if stderr.contains("Uncaught error: TerminalTextLayoutError") {
            return Err(format!(
                "negative limit stderr should not collapse to generic TerminalTextLayoutError, got:\n{stderr}"
            ));
        }

        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-text-layout negative target directory should be removed");

    assert!(
        execution_result.is_ok(),
        "terminal-text-layout negative limit should expose specific leaf: {}",
        execution_result.err().unwrap_or_default()
    );
}
