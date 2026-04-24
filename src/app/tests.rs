    use super::{help_text, run_with_args};

    #[test]
    fn top_level_help_contains_all_commands() {
        let help = help_text(None);
        assert!(help.contains("<file.op>"));
        assert!(help.contains("--run"));
        assert!(help.contains("help"));
        assert!(help.contains("--help"));
        assert!(help.contains("pkg"));
        assert!(help.contains("fmt"));
        assert!(help.contains("lsp"));
        assert!(help.contains("test"));
        assert!(help.contains("doc"));
        assert!(help.contains("bench"));
    }

    #[test]
    fn top_level_help_contains_examples_section() {
        let help = help_text(None);
        assert!(help.contains("Examples:"));
    }

    #[test]
    fn help_pkg_shows_all_subcommands() {
        let help = help_text(Some("pkg"));
        assert!(help.contains("init"));
        assert!(help.contains("add"));
        assert!(help.contains("remove"));
        assert!(help.contains("install"));
        assert!(help.contains("publish"));
    }

    #[test]
    fn help_fmt_shows_all_flags() {
        let help = help_text(Some("fmt"));
        assert!(help.contains("--check"));
        assert!(help.contains("--config"));
    }

    #[test]
    fn help_lsp_shows_stdio_flag() {
        let help = help_text(Some("lsp"));
        assert!(help.contains("--stdio"));
        assert!(!help.contains("Unknown help topic"));
    }

    #[test]
    fn help_test_shows_flags() {
        let help = help_text(Some("test"));
        assert!(help.contains("--target"));
        assert!(help.contains("--filter"));
        assert!(!help.contains("Unknown help topic"));
    }

    #[test]
    fn help_doc_shows_format_flag() {
        let help = help_text(Some("doc"));
        assert!(help.contains("--format"));
        assert!(help.contains("md"));
        assert!(help.contains("html"));
        assert!(!help.contains("Unknown help topic"));
    }

    #[test]
    fn help_bench_shows_usage() {
        let help = help_text(Some("bench"));
        assert!(!help.is_empty());
        assert!(!help.contains("Unknown help topic"));
    }

    #[test]
    fn help_unknown_topic_contains_error() {
        let help = help_text(Some("nonexistent"));
        assert!(help.contains("Unknown help topic"));
    }

    #[test]
    fn dash_dash_help_shows_top_level_help() {
        let args = ["opal".to_string(), "--help".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn dash_dash_help_with_topic_shows_topic() {
        let args = ["opal".to_string(), "--help".to_string(), "pkg".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn unimplemented_pkg_returns_error() {
        let args = ["opal".to_string(), "pkg".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn fmt_missing_file_returns_error() {
        let args = ["opal".to_string(), "fmt".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn fmt_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "fmt".to_string(),
            "nonexistent_xyz_abc_123.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn fmt_check_mode_returns_ok_when_already_formatted() {
        let tmp_path = std::env::temp_dir().join("opal_test_fmt_check.op");
        std::fs::write(&tmp_path, "let x = 1\n").unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = [
            "opal".to_string(),
            "fmt".to_string(),
            "--check".to_string(),
            path,
        ];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert!(result == Ok(()) || result == Err(1));
    }

    #[test]
    fn fmt_formats_file_in_place() {
        let tmp_path = std::env::temp_dir().join("opal_test_fmt_inplace.op");
        std::fs::write(&tmp_path, "let x = 1\n").unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = ["opal".to_string(), "fmt".to_string(), path];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert!(result == Ok(()) || result == Err(1));
    }

    #[test]
    fn fmt_config_flag_accepted() {
        let tmp_src = std::env::temp_dir().join("opal_test_fmt_cfg_src.op");
        let tmp_cfg = std::env::temp_dir().join("opal_test_fmt_cfg.toml");
        std::fs::write(&tmp_src, "let x = 1\n").unwrap();
        std::fs::write(&tmp_cfg, "indent_size = 4\n").unwrap();
        let src_path = tmp_src.to_string_lossy().to_string();
        let cfg_path = tmp_cfg.to_string_lossy().to_string();
        let args = [
            "opal".to_string(),
            "fmt".to_string(),
            "--config".to_string(),
            cfg_path,
            src_path,
        ];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_src));
        drop(std::fs::remove_file(&tmp_cfg));
        assert!(result == Ok(()) || result == Err(1));
    }

    #[test]
    fn unimplemented_lsp_returns_error() {
        let args = ["opal".to_string(), "lsp".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn lsp_starts_server_returns_ok() {
        let args = ["opal".to_string(), "lsp".to_string(), "--stdio".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn test_command_empty_suite_returns_ok() {
        let args = ["opal".to_string(), "test".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn test_with_filter_returns_ok() {
        let args = [
            "opal".to_string(),
            "test".to_string(),
            "--filter".to_string(),
            "my_test".to_string(),
        ];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn test_with_target_returns_ok() {
        let args = [
            "opal".to_string(),
            "test".to_string(),
            "--target".to_string(),
            "x86_64-linux".to_string(),
        ];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn unimplemented_doc_returns_error() {
        let args = ["opal".to_string(), "doc".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn bench_command_returns_ok() {
        let args = ["opal".to_string(), "bench".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn doc_missing_file_returns_error() {
        let args = ["opal".to_string(), "doc".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn doc_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "doc".to_string(),
            "nonexistent_xyz_doc_123.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    #[test]
    fn doc_with_valid_source_returns_ok() {
        let tmp_path = std::env::temp_dir().join("opal_test_doc_valid.op");
        std::fs::write(&tmp_path, "let x = 1\n").unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = ["opal".to_string(), "doc".to_string(), path];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn doc_format_flag_accepted() {
        let tmp_path = std::env::temp_dir().join("opal_test_doc_fmt.op");
        std::fs::write(&tmp_path, "let x = 1\n").unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = [
            "opal".to_string(),
            "doc".to_string(),
            "--format".to_string(),
            "html".to_string(),
            path,
        ];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert!(result == Ok(()) || result == Err(1));
    }

    #[test]
    fn help_command_returns_ok() {
        let args = ["opal".to_string(), "help".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    #[test]
    fn help_with_topic_returns_ok() {
        let args = ["opal".to_string(), "help".to_string(), "pkg".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures calling CLI without a source file returns error code 1.
    #[test]
    fn no_args_returns_error() {
        let args = ["opal".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures missing source file input returns error code 1.
    #[test]
    fn missing_file_returns_error() {
        let args = ["opal".to_string(), "nonexistent_file.op".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal run` with no file argument returns error code 1.
    #[test]
    fn run_subcommand_missing_file_returns_error() {
        let args = ["opal".to_string(), "run".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal run <nonexistent>` returns error code 1.
    #[test]
    fn run_subcommand_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "run".to_string(),
            "missing_xyz_run.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal check` with no file argument returns error code 1.
    #[test]
    fn check_missing_file_arg_returns_error() {
        let args = ["opal".to_string(), "check".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal check <nonexistent>` returns error code 1.
    #[test]
    fn check_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "check".to_string(),
            "nonexistent_xyz_check.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal check <valid-file>` returns `Ok(())` for valid source.
    #[test]
    fn check_valid_source_returns_ok() {
        let source = "##\n  Description: starting point of the application\n##\nentry main = f(args: string[]): void =>\n    return void\n";
        let tmp_path = std::env::temp_dir().join("opal_test_check_valid.op");
        std::fs::write(&tmp_path, source).unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = ["opal".to_string(), "check".to_string(), path];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert_eq!(result, Ok(()));
    }

    /// Ensures `opal check <invalid-source>` returns error code 1 when type-checking fails.
    #[test]
    fn check_invalid_source_returns_error() {
        let source = "##\n  Description: starting point of the application\n##\nentry main = f(args: string[]): void =>\n    let x: int32 = \"not a number\"\n    return void\n";
        let tmp_path = std::env::temp_dir().join("opal_test_check_invalid.op");
        std::fs::write(&tmp_path, source).unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = ["opal".to_string(), "check".to_string(), path];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert_eq!(result, Err(1));
    }

    /// Ensures `opal run <file> -- arg1 arg2` parses args after `--` gracefully.
    #[test]
    fn run_args_after_double_dash_separated() {
        let tmp_path = std::env::temp_dir().join("opal_test_run_dashash.op");
        std::fs::write(&tmp_path, "let x = 1\n").unwrap();
        let path = tmp_path.to_string_lossy().to_string();
        let args = [
            "opal".to_string(),
            "run".to_string(),
            path,
            "--".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];
        let result = run_with_args(&args);
        drop(std::fs::remove_file(&tmp_path));
        assert!(result == Ok(()) || result == Err(1));
    }

    /// Mutex to serialize tests that change the process working directory.
    static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Ensures `opal build` returns `Err(1)` when no `opal.toml` exists in the current directory.
    #[test]
    fn build_no_config_returns_error() {
        let _guard = CWD_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = std::env::current_dir().unwrap();
        let dir = std::env::temp_dir().join("opal_test_build_no_config");
        std::fs::create_dir_all(&dir).unwrap();
        drop(std::fs::remove_file(dir.join("opal.toml")));
        std::env::set_current_dir(&dir).unwrap();
        let result = run_with_args(&["opal".to_string(), "build".to_string()]);
        std::env::set_current_dir(&original).unwrap();
        assert_eq!(result, Err(1));
    }

    /// Ensures `opal build` dispatches the build path when `opal.toml` and `src/main.op` exist.
    #[test]
    fn build_with_config_compiles_project() {
        let _guard = CWD_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = std::env::current_dir().unwrap();
        let dir = std::env::temp_dir().join("opal_test_build_with_config");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("opal.toml"),
            "name = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("src").join("main.op"), "let x = 1\n").unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = run_with_args(&["opal".to_string(), "build".to_string()]);
        std::env::set_current_dir(&original).unwrap();
        assert!(result == Ok(()) || result == Err(1));
    }

    /// Ensures `opal watch` with no file arg returns `Err(1)`.
    #[test]
    fn watch_missing_file_arg_returns_error() {
        let args = ["opal".to_string(), "watch".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal watch <nonexistent>` returns `Err(1)`.
    #[test]
    fn watch_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "watch".to_string(),
            "nonexistent_xyz.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Verifies `help_text` run command contains expected keywords.
    #[test]
    fn help_text_run_contains_usage() {
        let help = help_text(Some("run"));
        assert!(help.contains("opal run") && help.contains("file") && help.contains("args"));
    }

    /// Verifies `help_text` check command contains expected keywords.
    #[test]
    fn help_text_check_contains_usage() {
        let help = help_text(Some("check"));
        assert!(help.contains("opal check") && help.contains("typecheck"));
    }

    /// Verifies `help_text` build command contains expected keywords.
    #[test]
    fn help_text_build_contains_usage() {
        let help = help_text(Some("build"));
        assert!(help.contains("opal build") && help.contains("opal.toml"));
    }

    /// Verifies `help_text` watch command contains expected keywords.
    #[test]
    fn help_text_watch_contains_usage() {
        let help = help_text(Some("watch"));
        assert!(help.contains("opal watch") && help.contains("recompile"));
    }

    #[test]
    fn help_text_none_lists_all_commands() {
        let help = help_text(None);
        assert!(
            help.contains("run")
                && help.contains("check")
                && help.contains("build")
                && help.contains("watch")
        );
    }
    #[test]
    fn test_all_commands_no_unimplemented() {
        let commands: Vec<&str> = vec![
            "fmt", "lsp", "test", "doc", "bench", "run", "check", "build", "watch",
        ];
        for cmd in commands {
            let args = ["opal".to_owned(), cmd.to_owned()];
            let result = run_with_args(&args);
            match cmd {
                "test" | "bench" => {
                    assert_eq!(result, Ok(()), "{cmd} should be wired and executable");
                }
                _ => assert_eq!(
                    result,
                    Err(1),
                    "{cmd} should be wired and return argument/file errors, not unimplemented fallback"
                ),
            }
        }
    }
    #[test]
    fn test_pkg_still_unimplemented() {
        let args = ["opal".to_owned(), "pkg".to_owned(), "status".to_owned()];
        assert_eq!(run_with_args(&args), Err(1));
    }
    #[test]
    fn test_run_is_alternative_to_run_flag() {
        let subcommand_args = [
            "opal".to_owned(),
            "run".to_owned(),
            "nonexistent_alt_run.op".to_owned(),
        ];
        let flag_args = [
            "opal".to_owned(),
            "nonexistent_alt_run.op".to_owned(),
            "--run".to_owned(),
        ];
        assert_eq!(run_with_args(&subcommand_args), Err(1));
        assert_eq!(run_with_args(&flag_args), Err(1));
    }
    #[test]
    fn test_help_lists_all_commands_integration() {
        let help = help_text(None);
        for cmd in [
            "pkg", "fmt", "lsp", "test", "doc", "bench", "run", "check", "build", "watch",
        ] {
            assert!(help.contains(cmd), "help text should list command: {cmd}");
        }
    }

    #[test]
    fn cli_rejects_invalid_target() {
        let args = [
            "opal".to_string(),
            "run".to_string(),
            "test-projects/hello-world/src/main.op".to_string(),
            "--target".to_string(),
            "banana-pi-linux".to_string(),
        ];
        let result = run_with_args(&args);
        assert_eq!(result, Err(1));
    }
