#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn string_stdlib_success_project_compiles_and_runs() {
    const SOURCE_PATH: &str = "test-projects/string-stdlib-success/src/main.op";
    const SOURCE: &str = "import print, string_find_index_or, string_find_last_index_of_text, string_split_lines, string_is_blank, string_trim_whitespace, string_take_prefix, string_take_suffix, string_extract_range, string_join from standard\n\n##\n    Description: Entry function verifies string stdlib success semantics end-to-end\n##\nentry main = f(): void errors StringEmptySearchTextError, StringPatternNotFoundError, StringNegativeCountError, StringRangeOutOfBoundsError, StringRangeOrderError, AllocationFailureError => {\n    let idx = string_find_index_or('hé🙂z', '🙂', -1 as int64)\n    let last = propagate string_find_last_index_of_text('bananana', 'ana')\n    let lines = propagate string_split_lines('a\r\nb')\n    let first: string = guard lines.at(0) into value: string else '<missing>'\n    let second: string = guard lines.at(1) into value: string else '<missing>'\n    let blank_spaces = string_is_blank('   ')\n    let blank_text = string_is_blank('cat')\n    let trimmed = propagate string_trim_whitespace('  hi  ')\n    let prefix = propagate string_take_prefix('hé🙂', 2 as int64)\n    let suffix = propagate string_take_suffix('hé🙂', 2 as int64)\n    let range = propagate string_extract_range('hé🙂z', 1 as int64, 3 as int64)\n    let joined = propagate string_join(['x', 'y', 'z'], '-')\n    print('IDX={idx}')\n    print('LAST={last}')\n    print('LINES={lines.length}:{first}:{second}')\n    if blank_spaces:\n        print('BLANK_SPACES=true')\n    else:\n        print('BLANK_SPACES=false')\n    if blank_text:\n        print('BLANK_TEXT=true')\n    else:\n        print('BLANK_TEXT=false')\n    print('TRIMMED=[{trimmed}]')\n    print('PREFIX=[{prefix}]')\n    print('SUFFIX=[{suffix}]')\n    print('RANGE=[{range}]')\n    print('JOINED=[{joined}]')\n    return void\n}\n";

    let temp_dir = unique_probe_target_dir("string-stdlib-success");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-stdlib-success target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            SOURCE,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("string-stdlib-success source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "string-stdlib-success compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "string-stdlib-success binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "IDX=2\nLAST=5\nLINES=2:a:b\nBLANK_SPACES=true\nBLANK_TEXT=false\nTRIMMED=[hi]\nPREFIX=[hé]\nSUFFIX=[é🙂]\nRANGE=[é🙂]\nJOINED=[x-y-z]\n";
        if stdout != expected {
            return Err(format!(
                "string-stdlib-success stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-stdlib-success target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "string-stdlib-success should compile, run, and match expected output: {failure_message}"
    );
}
