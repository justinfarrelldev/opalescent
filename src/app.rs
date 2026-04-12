//! CLI application workflow for the Opalescent binary.
//!
//! This module centralizes command-line behavior so `main.rs` remains a thin
//! entry point while the executable behavior stays testable and reusable.

use crate::compiler::compile_program;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Print usage help for `opal` CLI commands.
///
/// When `topic` is `None`, prints the top-level help summary.
/// When `topic` is `Some(t)`, prints topic-specific guidance for the named topic.
fn print_help(topic: Option<&str>) {
    match topic {
        Some("pkg") => {
            println!("opal pkg <command>");
            println!();
            println!("Commands:");
            println!("  init <name>              Initialise a new project manifest");
            println!("  add <pkg> <version>      Add a dependency");
            println!("  remove <pkg>             Remove a dependency");
            println!("  install                  Install all declared dependencies");
            println!("  publish                  Publish the package to the registry");
        }
        Some("fmt") => {
            println!("opal fmt [--check] [--config <path>] <file>");
            println!();
            println!("Format an Opalescent source file.");
            println!("  --check     Exit with error if file would change (CI mode)");
            println!("  --config    Path to opal-fmt.toml configuration file");
        }
        Some(unknown) => {
            eprintln!("Unknown help topic: {unknown}");
            eprintln!("Run `opal help` for the list of topics.");
        }
        None => {
            println!("Opalescent Compiler");
            println!();
            println!("Usage:  opal <command> [arguments]");
            println!();
            println!("Commands:");
            println!("  <file.op>    Compile an Opalescent source file");
            println!("  --run        Execute the compiled binary after compilation");
            println!("  help         Show this help message");
            println!("  help pkg     Package manager commands");
            println!("  help fmt     Formatter commands");
            println!();
            println!("Examples:");
            println!("  opal src/main.op");
            println!("  opal src/main.op --run");
            println!("  opal help pkg");
        }
    }
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

/// Main CLI logic, delegating process exit handling to the public `run()` wrapper.
fn run_impl() -> Result<(), i32> {
    let args: Vec<String> = std::env::args().collect();

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
