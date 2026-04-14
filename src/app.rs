//! CLI application workflow for the Opalescent binary.
//!
//! This module centralizes command-line behavior so `main.rs` remains a thin
//! entry point while the executable behavior stays testable and reusable.

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
            out.push_str("  help pkg     Package manager commands\n");
            out.push_str("  help fmt     Formatter commands\n");
            out.push('\n');
            out.push_str("Examples:\n");
            out.push_str("  opal src/main.op\n");
            out.push_str("  opal src/main.op --run\n");
            out.push_str("  opal help pkg\n");
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
