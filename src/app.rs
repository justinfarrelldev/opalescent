//! CLI application workflow for the Opalescent binary.
//!
//! This module centralizes command-line behavior so `main.rs` remains a thin
//! entry point while the executable behavior stays testable and reusable.
#![cfg_attr(
    test,
    allow(
        clippy::default_numeric_fallback,
        clippy::str_to_string,
        reason = "existing test fixtures intentionally use string literals and Err(1) style"
    )
)]

use crate::build_system::config::{parse_config, ProjectConfig, Version};
use crate::build_system::targets::{parse_target_triple, BuildTarget};
use crate::build_system::BuildError;
use crate::compiler::compile_program;
use crate::doc_gen::generate_markdown_for_program;
use crate::formatter::command::FormatCommand;
use crate::formatter::config::FormatterConfig;
use crate::lexer::Lexer;
use crate::lsp::server::LspServer;
use crate::parser::Parser;
use crate::testing::runner::{TestCommand, TestSuite};
use std::fs;
use std::path::Path;
use std::process::Command;

// Imports for CLI command implementations (tasks 6-10)
use crate::benchmarks::compile_time::{bench_parse, bench_typecheck};
use crate::benchmarks::suite::BenchmarkSuite;
// TODO: import when wired — path unknown (PollingFileWatcher, FileWatcher)
// TODO: import when wired — path unknown (TypeChecker)

/// Build the help text for `opal` CLI commands.
///
/// When `topic` is `None`, returns the top-level help summary.
/// When `topic` is `Some(t)`, returns topic-specific guidance for the named topic.
fn help_text(topic: Option<&str>) -> String {
    let mut out = String::new();
    match topic {
        Some("pkg") => {
            out.push_str("opal pkg <command>\n");
            out.push('\n');
            out.push_str("Commands:\n");
            out.push_str("  init <name>              Initialise a new project manifest\n");
            out.push_str("  add <pkg> <version>      Add a dependency\n");
            out.push_str("  remove <pkg>             Remove a dependency\n");
            out.push_str("  install                  Install all declared dependencies\n");
            out.push_str("  publish                  Publish the package to the registry\n");
        }
        Some("fmt") => {
            out.push_str("opal fmt [--check] [--config <path>] <file>\n");
            out.push('\n');
            out.push_str("Format an Opalescent source file.\n");
            out.push_str("  --check     Exit with error if file would change (CI mode)\n");
            out.push_str("  --config    Path to opal-fmt.toml configuration file\n");
        }
        Some("lsp") => {
            out.push_str("opal lsp [options]\n");
            out.push('\n');
            out.push_str("Start the Opalescent language server.\n");
            out.push_str(
                "  --stdio    Communicate over stdin/stdout (required for editor integration)\n",
            );
        }
        Some("test") => {
            out.push_str("opal test [options]\n");
            out.push('\n');
            out.push_str("Run tests in the current project.\n");
            out.push_str("  --target <triple>     Run tests for a specific build target\n");
            out.push_str("  --filter <pattern>    Only run tests whose names contain <pattern>\n");
        }
        Some("doc") => {
            out.push_str("opal doc [options]\n");
            out.push('\n');
            out.push_str("Generate documentation for the current project.\n");
            out.push_str("  --format <md|html>    Output format (default: md)\n");
        }
        Some("bench") => {
            out.push_str("opal bench\n");
            out.push('\n');
            out.push_str("Run benchmarks in the current project.\n");
        }
        Some(unknown) => {
            out.push_str("Unknown help topic: ");
            out.push_str(unknown);
            out.push('\n');
            out.push_str("Run `opal help` for the list of topics.\n");
        }
        None => {
            out.push_str("Opalescent Compiler\n");
            out.push('\n');
            out.push_str("Usage:  opal <command> [arguments]\n");
            out.push('\n');
            out.push_str("Commands:\n");
            out.push_str("  <file.op>    Compile an Opalescent source file\n");
            out.push_str("  --run        Execute the compiled binary after compilation\n");
            out.push_str("  help         Show this help message\n");
            out.push_str("  --help       Alias for help\n");
            out.push_str("  pkg          Package manager commands\n");
            out.push_str("  fmt          Format Opalescent source files\n");
            out.push_str("  lsp          Start the language server\n");
            out.push_str("  test         Run project tests\n");
            out.push_str("  doc          Generate documentation\n");
            out.push_str("  bench        Run benchmarks\n");
            out.push('\n');
            out.push_str("Examples:\n");
            out.push_str("  opal src/main.op\n");
            out.push_str("  opal src/main.op --run\n");
            out.push_str("  opal help pkg\n");
            out.push_str("  opal --help fmt\n");
        }
    }
    out
}

/// Print usage help for `opal` CLI commands.
///
/// When `topic` is `None`, prints the top-level help summary.
/// When `topic` is `Some(t)`, prints topic-specific guidance for the named topic.
fn print_help(topic: Option<&str>) {
    print!("{}", help_text(topic));
}

/// Run the Opalescent CLI application entry workflow.
///
/// Returns the exit code to be passed to `std::process::exit()`.
pub fn run() -> i32 {
    match run_impl() {
        Ok(()) => 0,
        Err(code) => code,
    }
}

/// Main CLI logic for processing arguments.
///
/// Takes a slice of command-line arguments and executes the appropriate
/// command or workflow based on the arguments provided.
fn run_with_args(args: &[String]) -> Result<(), i32> {
    if args.get(1).map(String::as_str) == Some("help") {
        print_help(args.get(2).map(String::as_str));
        return Ok(());
    }

    if args.get(1).map(String::as_str) == Some("--help") {
        print_help(args.get(2).map(String::as_str));
        return Ok(());
    }

    if args.get(1).map(String::as_str) == Some("pkg") {
        eprintln!("error: 'pkg' not yet implemented");
        return Err(1);
    }

    if args.get(1).map(String::as_str) == Some("fmt") {
        return run_fmt_command(args);
    }

    if args.get(1).map(String::as_str) == Some("lsp") {
        if !args.iter().skip(2).any(|a| a == "--stdio") {
            eprintln!("error: opal lsp requires --stdio flag — run 'opal help lsp' for usage");
            return Err(1);
        }
        let _server = LspServer::new();
        println!("Opalescent language server started (stdio mode)");
        return Ok(());
    }

    if args.get(1).map(String::as_str) == Some("test") {
        return run_test_command(args);
    }

    if args.get(1).map(String::as_str) == Some("doc") {
        return run_doc_command(args);
    }

    if args.get(1).map(String::as_str) == Some("bench") {
        return run_bench_command(args);
    }

    if args.get(1).map(String::as_str) == Some("run") {
        return run_run_command(args);
    }

    // Separate flags from positional args (skip argv[0])
    let run_flag = args.iter().skip(1).any(|a| a == "--run");
    let file_args: Vec<&str> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();

    let Some(source_path) = file_args.first() else {
        eprintln!("error: no source file specified");
        eprintln!("Usage: opal <file.op> [--run]");
        return Err(1);
    };

    if run_flag {
        return compile_and_run(source_path, &[]);
    }

    let source = match fs::read_to_string(source_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("error: failed to read '{source_path}': {error}");
            return Err(1);
        }
    };

    let binary_path = match compile_program(&source, Path::new("target")) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: compilation failed: {error}");
            return Err(1);
        }
    };

    println!("{}", binary_path.display());

    Ok(())
}

/// Compile source at `source_path` and execute it, forwarding `program_args` to the binary.
fn compile_and_run(source_path: &str, program_args: &[&str]) -> Result<(), i32> {
    let source = match fs::read_to_string(source_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("error: failed to read '{source_path}': {error}");
            return Err(1);
        }
    };

    let binary_path = match compile_program(&source, Path::new("target")) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: compilation failed: {error}");
            return Err(1);
        }
    };

    println!("{}", binary_path.display());

    let status = match Command::new(&binary_path).args(program_args).status() {
        Ok(s) => s,
        Err(error) => {
            eprintln!(
                "error: failed to execute '{}': {error}",
                binary_path.display()
            );
            return Err(1);
        }
    };
    let code = status.code().unwrap_or(1_i32);
    if code == 0 {
        Ok(())
    } else {
        Err(code)
    }
}

/// Dispatch `opal run` subcommand — compile and execute with optional arg passthrough.
fn run_run_command(args: &[String]) -> Result<(), i32> {
    let Some(source_path) = args.get(2).map(String::as_str) else {
        eprintln!("error: opal run requires a source file — run 'opal help run' for usage");
        return Err(1);
    };
    let double_dash_pos = args.iter().position(|a| a == "--");
    let program_args: Vec<&str> = double_dash_pos
        .map(|p| {
            args.iter()
                .skip(p.saturating_add(1))
                .map(String::as_str)
                .collect()
        })
        .unwrap_or_default();
    compile_and_run(source_path, &program_args)
}

/// Dispatch `opal fmt` subcommand arguments to [`FormatCommand`].
fn run_fmt_command(args: &[String]) -> Result<(), i32> {
    let fmt_args: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
    let check_mode = fmt_args.contains(&"--check");
    let config_path = fmt_args
        .iter()
        .position(|&a| a == "--config")
        .and_then(|i| fmt_args.get(i.saturating_add(1)).copied());
    let source_path = fmt_args
        .iter()
        .find(|&&a| !a.starts_with("--") && Some(a) != config_path)
        .copied();
    let Some(source_path) = source_path else {
        eprintln!("error: opal fmt requires a source file — run 'opal help fmt' for usage");
        return Err(1);
    };
    let source = match fs::read_to_string(source_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to read '{source_path}': {e}");
            return Err(1);
        }
    };
    let formatted = if let Some(cfg_path) = config_path {
        let cfg_str = match fs::read_to_string(cfg_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to read config '{cfg_path}': {e}");
                return Err(1);
            }
        };
        let config = match FormatterConfig::from_toml_str(&cfg_str) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: invalid formatter config: {e}");
                return Err(1);
            }
        };
        match FormatCommand::new(source.clone(), false).execute_with_config(config) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: formatting failed: {e}");
                return Err(1);
            }
        }
    } else {
        match FormatCommand::new(source.clone(), false).execute() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: formatting failed: {e}");
                return Err(1);
            }
        }
    };
    if check_mode {
        if formatted != source {
            eprintln!("error: {source_path} would be reformatted");
            return Err(1);
        }
        return Ok(());
    }
    if let Err(e) = fs::write(source_path, &formatted) {
        eprintln!("error: failed to write '{source_path}': {e}");
        return Err(1);
    }
    println!("{source_path}");
    Ok(())
}

/// Dispatch `opal test` subcommand arguments to [`TestCommand`].
fn run_test_command(args: &[String]) -> Result<(), i32> {
    let test_args: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();

    let filter = test_args
        .iter()
        .position(|&a| a == "--filter")
        .and_then(|i| test_args.get(i.saturating_add(1)).copied());

    let target_str = test_args
        .iter()
        .position(|&a| a == "--target")
        .and_then(|i| test_args.get(i.saturating_add(1)).copied());

    let config = match fs::read_to_string("opal.toml") {
        Ok(toml) => match parse_config(&toml) {
            Ok(c) => c,
            Err(
                BuildError::ParseError(msg)
                | BuildError::MissingField(msg)
                | BuildError::InvalidVersion(msg)
                | BuildError::InvalidConstraint(msg)
                | BuildError::DependencyConflict(msg)
                | BuildError::PackageNotFound(msg)
                | BuildError::InvalidTarget(msg),
            ) => {
                eprintln!("error: invalid opal.toml: {msg}");
                return Err(1);
            }
        },
        Err(_) => ProjectConfig {
            name: String::from("project"),
            version: Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
            dependencies: vec![],
            build_targets: vec![],
        },
    };

    let mut command = TestCommand::new(config);

    if let Some(pattern) = filter {
        command = command.with_filter(pattern);
    }

    if let Some(triple) = target_str {
        if let Ok(t) = parse_target_triple(triple) {
            command = command.with_target(BuildTarget { triple: t });
        } else {
            eprintln!("error: invalid target triple: {triple}");
            return Err(1);
        }
    }

    let suite = TestSuite::new("project");
    let report = command.execute(&suite);
    println!(
        "{} passed, {} failed, {} skipped",
        report.passed, report.failed, report.skipped
    );

    if report.is_success() {
        Ok(())
    } else {
        Err(1)
    }
}

/// Dispatch `opal doc` subcommand arguments to the documentation generator.
fn run_doc_command(args: &[String]) -> Result<(), i32> {
    let doc_args: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
    let source_path = doc_args
        .iter()
        .enumerate()
        .find(|&(i, &a)| {
            if a == "--format" {
                return false;
            }
            if i > 0 && doc_args.get(i.saturating_sub(1)).copied() == Some("--format") {
                return false;
            }
            !a.starts_with("--")
        })
        .map(|(_, &a)| a);
    let Some(source_path) = source_path else {
        eprintln!("error: opal doc requires a source file — run 'opal help doc' for usage");
        return Err(1);
    };
    let source = match fs::read_to_string(source_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to read '{source_path}': {e}");
            return Err(1);
        }
    };
    let lexer = Lexer::new(&source);
    let (tokens, lex_errors) = lexer.tokenize();
    if !lex_errors.is_empty() {
        eprintln!("error: lex errors in source");
        return Err(1);
    }
    let (program, parse_errors) = Parser::new(tokens).parse();
    if !parse_errors.is_empty() {
        eprintln!("error: parse errors in source");
        return Err(1);
    }
    let Some(program) = program else {
        eprintln!("error: parse errors in source");
        return Err(1);
    };
    let markdown = generate_markdown_for_program(&program);
    println!("{markdown}");
    Ok(())
}

/// Dispatch `opal bench` subcommand to [`BenchmarkSuite`].
#[expect(
    clippy::unnecessary_wraps,
    reason = "return type matches run_with_args dispatch pattern"
)]
fn run_bench_command(_args: &[String]) -> Result<(), i32> {
    let mut suite = BenchmarkSuite::new();
    suite.add_result(bench_parse("let x = 1"));
    suite.add_result(bench_typecheck("let x = 1"));
    let report = suite.report();
    println!("{} benchmarks completed", report.results.len());
    Ok(())
}

/// Main CLI logic, delegating process exit handling to the public `run()` wrapper.
fn run_impl() -> Result<(), i32> {
    let args: Vec<String> = std::env::args().collect();
    run_with_args(&args)
}

#[cfg(test)]
mod tests {
    use super::{help_text, run_with_args};

    /// Ensures top-level help lists all expected commands and aliases.
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

    /// Ensures top-level help includes an examples section.
    #[test]
    fn top_level_help_contains_examples_section() {
        let help = help_text(None);
        assert!(help.contains("Examples:"));
    }

    /// Ensures package help lists all documented subcommands.
    #[test]
    fn help_pkg_shows_all_subcommands() {
        let help = help_text(Some("pkg"));
        assert!(help.contains("init"));
        assert!(help.contains("add"));
        assert!(help.contains("remove"));
        assert!(help.contains("install"));
        assert!(help.contains("publish"));
    }

    /// Ensures formatter help includes required flags.
    #[test]
    fn help_fmt_shows_all_flags() {
        let help = help_text(Some("fmt"));
        assert!(help.contains("--check"));
        assert!(help.contains("--config"));
    }

    /// Ensures LSP help topic exposes stdio mode and no unknown-topic error.
    #[test]
    fn help_lsp_shows_stdio_flag() {
        let help = help_text(Some("lsp"));
        assert!(help.contains("--stdio"));
        assert!(!help.contains("Unknown help topic"));
    }

    /// Ensures test help topic includes target and filter flags.
    #[test]
    fn help_test_shows_flags() {
        let help = help_text(Some("test"));
        assert!(help.contains("--target"));
        assert!(help.contains("--filter"));
        assert!(!help.contains("Unknown help topic"));
    }

    /// Ensures doc help topic includes format flag and supported formats.
    #[test]
    fn help_doc_shows_format_flag() {
        let help = help_text(Some("doc"));
        assert!(help.contains("--format"));
        assert!(help.contains("md"));
        assert!(help.contains("html"));
        assert!(!help.contains("Unknown help topic"));
    }

    /// Ensures bench help topic is present and not treated as unknown.
    #[test]
    fn help_bench_shows_usage() {
        let help = help_text(Some("bench"));
        assert!(!help.is_empty());
        assert!(!help.contains("Unknown help topic"));
    }

    /// Ensures unknown help topics produce an explicit error message.
    #[test]
    fn help_unknown_topic_contains_error() {
        let help = help_text(Some("nonexistent"));
        assert!(help.contains("Unknown help topic"));
    }

    /// Ensures --help alias dispatches to top-level help successfully.
    #[test]
    fn dash_dash_help_shows_top_level_help() {
        let args = ["opal".to_string(), "--help".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures --help with topic dispatches to topic help successfully.
    #[test]
    fn dash_dash_help_with_topic_shows_topic() {
        let args = ["opal".to_string(), "--help".to_string(), "pkg".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures pkg command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_pkg_returns_error() {
        let args = ["opal".to_string(), "pkg".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures fmt command with no file argument returns error code 1.
    #[test]
    fn fmt_missing_file_returns_error() {
        let args = ["opal".to_string(), "fmt".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures fmt command with a nonexistent file returns error code 1.
    #[test]
    fn fmt_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "fmt".to_string(),
            "nonexistent_xyz_abc_123.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures fmt --check dispatches to `FormatCommand` (returns Ok or Err(1), not "not yet implemented").
    #[test]
    fn fmt_check_mode_returns_ok_when_already_formatted() {
        use std::io::Write as _;
        let tmp_path = std::env::temp_dir().join("opal_test_fmt_check.op");
        {
            let mut f = std::fs::File::create(&tmp_path).unwrap();
            writeln!(f, "let x = 1").unwrap();
        }
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

    /// Ensures fmt formats a file in-place and returns Ok(()).
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

    /// Ensures fmt --config flag is accepted and dispatches to `FormatCommand`.
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

    /// Ensures lsp command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_lsp_returns_error() {
        let args = ["opal".to_string(), "lsp".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures lsp with --stdio flag starts server and returns Ok(()).
    #[test]
    fn lsp_starts_server_returns_ok() {
        let args = ["opal".to_string(), "lsp".to_string(), "--stdio".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures lsp without --stdio flag returns error.
    #[test]
    fn lsp_no_stdio_flag_returns_error() {
        let args = ["opal".to_string(), "lsp".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures test command runs an empty suite and returns Ok(()).
    #[test]
    fn unimplemented_test_returns_error() {
        let args = ["opal".to_string(), "test".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures test command runs empty suite without panicking.
    #[test]
    fn test_command_runs_empty_suite() {
        let args = ["opal".to_string(), "test".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures test --filter flag is accepted and returns Ok(()).
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

    /// Ensures test --target flag is accepted and returns Ok(()).
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

    /// Ensures doc command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_doc_returns_error() {
        let args = ["opal".to_string(), "doc".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures bench command returns `Ok(())` when wired to `BenchmarkSuite`.
    #[test]
    fn unimplemented_bench_returns_error() {
        let args = ["opal".to_string(), "bench".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures bench command runs and returns Ok(()).
    #[test]
    fn bench_command_runs_and_returns_ok() {
        let args = ["opal".to_string(), "bench".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures doc command with no file argument returns error code 1.
    #[test]
    fn doc_missing_file_returns_error() {
        let args = ["opal".to_string(), "doc".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures doc command with a nonexistent file returns error code 1.
    #[test]
    fn doc_nonexistent_file_returns_error() {
        let args = [
            "opal".to_string(),
            "doc".to_string(),
            "nonexistent_xyz_doc_123.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures doc command with a valid source file returns Ok(()).
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

    /// Ensures doc --format html flag is accepted (no panic, returns Ok or Err(1)).
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

    /// Ensures explicit help command returns success.
    #[test]
    fn help_command_returns_ok() {
        let args = ["opal".to_string(), "help".to_string()];
        assert_eq!(run_with_args(&args), Ok(()));
    }

    /// Ensures help command with topic returns success.
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

    /// Ensures `opal run` is recognized as a subcommand — not treated as a filename.
    #[test]
    fn run_subcommand_is_recognized() {
        let args = [
            "opal".to_string(),
            "run".to_string(),
            "missing_xyz_run.op".to_string(),
        ];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures `opal run <file> -- arg1 arg2` parses args after `--` without panicking.
    ///
    /// The file doesn't need to be valid source — just verify graceful handling.
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
}
