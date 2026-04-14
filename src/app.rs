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

use crate::compiler::compile_program;
use std::fs;
use std::path::Path;
use std::process::Command;

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

    if let Some(cmd @ ("pkg" | "fmt" | "lsp" | "test" | "doc" | "bench")) =
        args.get(1).map(String::as_str)
    {
        eprintln!("error: '{cmd}' not yet implemented");
        return Err(1);
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

    if run_flag {
        let status = match Command::new(&binary_path).status() {
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
        return Err(code);
    }

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

    /// Ensures fmt command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_fmt_returns_error() {
        let args = ["opal".to_string(), "fmt".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures lsp command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_lsp_returns_error() {
        let args = ["opal".to_string(), "lsp".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures test command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_test_returns_error() {
        let args = ["opal".to_string(), "test".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures doc command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_doc_returns_error() {
        let args = ["opal".to_string(), "doc".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
    }

    /// Ensures bench command currently returns the expected unimplemented error code.
    #[test]
    fn unimplemented_bench_returns_error() {
        let args = ["opal".to_string(), "bench".to_string()];
        assert_eq!(run_with_args(&args), Err(1));
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
}
