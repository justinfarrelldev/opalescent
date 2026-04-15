#![cfg(feature = "integration")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("opalescent")
}

fn fmt_test_src(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join("fmt-test")
        .join("src")
        .join(filename)
}

fn fmt_test_expected(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join("fmt-test")
        .join("expected")
        .join(filename)
}

fn fmt_test_config(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-projects")
        .join("fmt-test")
        .join(filename)
}

fn temp_output_path(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opalescent-fmt-{test_name}"));
    fs::create_dir_all(&dir).expect("test temp directory should be created");
    dir.join("output.op")
}

fn cleanup_temp(path: &std::path::Path) -> Result<(), std::io::Error> {
    path.parent().map_or(Ok(()), fs::remove_dir_all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_output_spaces_default_config() {
        let binary = binary_path();
        let input = fmt_test_src("input-spaces.op");
        let expected = fmt_test_expected("input-spaces.expected.op");
        let output = temp_output_path("spaces-default");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with default config on input-spaces.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden =
            fs::read_to_string(&expected).expect("input-spaces golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-spaces.op formatted with default config should match input-spaces.expected.op"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for spaces-default test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_tabs_to_spaces() {
        let binary = binary_path();
        let input = fmt_test_src("input-tabs.op");
        let expected = fmt_test_expected("input-tabs.expected.op");
        let output = temp_output_path("tabs-to-spaces");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with default config on input-tabs.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden =
            fs::read_to_string(&expected).expect("input-tabs golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-tabs.op formatted with default config should match input-tabs.expected.op (spaces)"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for tabs-to-spaces test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_tabs_to_tabs() {
        let binary = binary_path();
        let input = fmt_test_src("input-tabs.op");
        let expected = fmt_test_expected("input-tabs-to-tabs.expected.op");
        let config = fmt_test_config("opal-fmt-tabs.toml");
        let output = temp_output_path("tabs-to-tabs");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--config")
            .arg(&config)
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with tabs config on input-tabs.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden = fs::read_to_string(&expected)
            .expect("input-tabs-to-tabs golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-tabs.op formatted with tabs config should match input-tabs-to-tabs.expected.op"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for tabs-to-tabs test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_mixed_to_spaces() {
        let binary = binary_path();
        let input = fmt_test_src("input-mixed.op");
        let expected = fmt_test_expected("input-mixed.expected.op");
        let output = temp_output_path("mixed-to-spaces");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with default config on input-mixed.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden =
            fs::read_to_string(&expected).expect("input-mixed golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-mixed.op formatted with default config should match input-mixed.expected.op"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for mixed-to-spaces test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_mixed_to_tabs() {
        let binary = binary_path();
        let input = fmt_test_src("input-mixed.op");
        let expected = fmt_test_expected("input-mixed-to-tabs.expected.op");
        let config = fmt_test_config("opal-fmt-tabs.toml");
        let output = temp_output_path("mixed-to-tabs");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--config")
            .arg(&config)
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with tabs config on input-mixed.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden = fs::read_to_string(&expected)
            .expect("input-mixed-to-tabs golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-mixed.op formatted with tabs config should match input-mixed-to-tabs.expected.op"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for mixed-to-tabs test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_idempotent_spaces() {
        let binary = binary_path();
        let input = fmt_test_src("input-clean-spaces.op");
        let expected = fmt_test_expected("input-clean-spaces.expected.op");
        let output = temp_output_path("idempotent-spaces");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with default config on input-clean-spaces.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden = fs::read_to_string(&expected)
            .expect("input-clean-spaces golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-clean-spaces.op formatted with default config should be idempotent"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for idempotent-spaces test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_idempotent_tabs() {
        let binary = binary_path();
        let input = fmt_test_src("input-clean-tabs.op");
        let expected = fmt_test_expected("input-clean-tabs.expected.op");
        let config = fmt_test_config("opal-fmt-tabs.toml");
        let output = temp_output_path("idempotent-tabs");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--config")
            .arg(&config)
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with tabs config on input-clean-tabs.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden =
            fs::read_to_string(&expected).expect("input-clean-tabs golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-clean-tabs.op formatted with tabs config should be idempotent"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for idempotent-tabs test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_2space_indent() {
        let binary = binary_path();
        let input = fmt_test_src("input-spaces.op");
        let expected = fmt_test_expected("input-spaces-2indent.expected.op");
        let config = fmt_test_config("opal-fmt-2spaces.toml");
        let output = temp_output_path("2space-indent");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--config")
            .arg(&config)
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output with 2-space config on input-spaces.op should exit with code 0"
        );

        let actual = fs::read_to_string(&output).expect("output file should be readable");
        let golden = fs::read_to_string(&expected)
            .expect("input-spaces-2indent golden file should be readable");
        assert_eq!(
            actual, golden,
            "input-spaces.op formatted with 2-space config should match input-spaces-2indent.expected.op"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for 2space-indent test should be removed after test"
        );
    }

    #[test]
    fn fmt_check_and_output_mutually_exclusive() {
        let binary = binary_path();
        let input = fmt_test_src("input-spaces.op");
        let output = temp_output_path("mutual-exclusion");

        let result = Command::new(&binary)
            .arg("fmt")
            .arg("--check")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .output()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            !result.status.success(),
            "--check and --output together should exit with non-zero code"
        );
        assert_eq!(
            result.status.code(),
            Some(1_i32),
            "--check and --output together should exit with code 1"
        );

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            !stderr.is_empty(),
            "--check and --output together should print an error message to stderr"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for mutual-exclusion test should be removed after test"
        );
    }

    #[test]
    fn fmt_output_preserves_source_file() {
        let binary = binary_path();
        let input = fmt_test_src("input-spaces.op");
        let output = temp_output_path("source-preservation");

        let before = fs::read_to_string(&input).expect("source file should be readable before fmt");

        let status = Command::new(&binary)
            .arg("fmt")
            .arg("--output")
            .arg(&output)
            .arg(&input)
            .status()
            .expect("opalescent fmt command should spawn and complete");

        assert!(
            status.success(),
            "fmt --output should exit with code 0 for source preservation test"
        );

        let after = fs::read_to_string(&input).expect("source file should be readable after fmt");

        assert_eq!(
            before, after,
            "source file content should be unchanged when --output redirects output"
        );

        let cleanup = cleanup_temp(&output);
        assert!(
            cleanup.is_ok(),
            "temp directory for source-preservation test should be removed after test"
        );
    }
}
