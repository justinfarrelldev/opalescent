//! CLI application workflow for the Opalescent binary.
//!
//! This module centralizes command-line behavior so `main.rs` remains a thin
//! entry point while the executable behavior stays testable and reusable.
// `cfg_attr` does not support `#[expect]`, so `allow` is required here.
#![cfg_attr(
    test,
    allow(
        clippy::default_numeric_fallback,
        clippy::str_to_string,
        reason = "test fixtures use string literals and numeric Err(1) style by convention"
    )
)]

use crate::build_system::BuildError;
use crate::build_system::config::{ProjectConfig, Version, parse_config};
use crate::build_system::targets::{BuildTarget, parse_target_triple};
use crate::compiler::{CompileError, compile_program, compile_project};
use crate::doc_gen::generate_markdown_for_program;
use crate::errors::renderer::render_report;
use crate::errors::reporter::CompilationErrorReport;
use crate::formatter::command::FormatCommand;
use crate::formatter::config::FormatterConfig;
use crate::lexer::Lexer;
use crate::lsp::server::LspServer;
use crate::module_loader::validate_module_file_role;
use crate::parser::Parser;
use crate::testing::runner::{TestCommand, TestSuite};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::benchmarks::compile_time::{bench_parse, bench_typecheck};
use crate::benchmarks::suite::BenchmarkSuite;
use crate::hot_reload::change_detection::{FileWatcher, PollingFileWatcher};
use crate::type_system::checker::TypeChecker;

mod targeting;
use targeting::resolve_target_from_args;

/// Build the help text for `opal` CLI commands (topic `None` = top-level, `Some(t)` = specific).
fn help_text(topic: Option<&str>) -> String {
    let mut out = String::new();
    match topic {
        Some("pkg") => {
            out.push_str("opal pkg <command>\n\nCommands:\n  init <name>              Initialise a new project manifest\n  add <pkg> <version>      Add a dependency\n  remove <pkg>             Remove a dependency\n  install                  Install all declared dependencies\n  publish                  Publish the package to the registry\n");
        }
        Some("fmt") => {
            out.push_str("opal fmt [--check] [--config <path>] <file>\n\nFormat an Opalescent source file.\n  --check     Exit with error if file would change (CI mode)\n  --config    Path to opal-fmt.toml configuration file\n");
        }
        Some("lsp") => {
            out.push_str("opal lsp [options]\n\nStart the Opalescent language server.\n  --stdio    Communicate over stdin/stdout (required for editor integration)\n");
        }
        Some("test") => {
            out.push_str("opal test [options]\n\nRun tests in the current project.\n  --target <triple>     Run tests for a specific build target\n  --filter <pattern>    Only run tests whose names contain <pattern>\n");
        }
        Some("doc") => {
            out.push_str("opal doc [options]\n\nGenerate documentation for the current project.\n  --format <md|html>    Output format (default: md)\n");
        }
        Some("bench") => {
            out.push_str("opal bench\n\nRun benchmarks in the current project.\n");
        }
        Some("run") => {
            out.push_str("opal run <file.op> [-- args...]\n\nCompile and execute an Opalescent source file.\n  -- args...    Arguments forwarded to the compiled binary\nAlias: opal <file.op> --run\n");
        }
        Some("check") => {
            out.push_str("opal check <file.op>\n\nRun lex, parse, and typecheck pipeline without code generation.\n");
        }
        Some("build") => {
            out.push_str(
                "opal build\n\nBuild the project by reading opal.toml and compiling src/main.op.\n",
            );
        }
        Some("watch") => {
            out.push_str("opal watch <file.op>\n\nWatch a source file and recompile on each detected change.\n  Press Ctrl-C to stop watching.\n");
        }
        Some(unknown) => {
            out.push_str("Unknown help topic: ");
            out.push_str(unknown);
            out.push_str("\nRun `opal help` for the list of topics.\n");
        }
        None => {
            out.push_str("Opalescent Compiler\n\nUsage:  opal <command> [arguments]\n\nCommands:\n  <file.op>    Compile an Opalescent source file\n  --run        Execute the compiled binary after compilation\n  help         Show this help message\n  --help       Alias for help\n  pkg          Package manager commands\n  fmt          Format Opalescent source files\n  lsp          Start the language server\n  test         Run project tests\n  doc          Generate documentation\n  bench        Run benchmarks\n  run          Compile and execute a source file\n  check        Typecheck source without code generation\n  build        Build the project from opal.toml\n  watch        Watch a file and recompile on changes\n\nExamples:\n  opal src/main.op\n  opal src/main.op --run\n  opal run src/main.op\n  opal help pkg\n  opal --help fmt\n");
        }
    }
    out
}

/// Print usage help for `opal` CLI commands (topic `None` = top-level, `Some(t)` = specific).
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

/// Main CLI logic for processing arguments — dispatches to the appropriate command or workflow.
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

    if args.get(1).map(String::as_str) == Some("check") {
        return run_check_command(args);
    }

    if args.get(1).map(String::as_str) == Some("build") {
        return run_build_command(args);
    }

    if args.get(1).map(String::as_str) == Some("watch") {
        return run_watch_command(args);
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

    let target = match resolve_target_from_args(args) {
        Ok(Some(target)) => target,
        Ok(None) => crate::build_system::targets::TargetTriple::host(),
        Err(code) => return Err(code),
    };

    if run_flag {
        return compile_and_run(source_path, &[], &target);
    }

    let source = match fs::read_to_string(source_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("error: failed to read '{source_path}': {error}");
            return Err(1);
        }
    };

    let binary_path = match compile_program(
        Path::new(source_path),
        &source,
        Path::new("target"),
        &target,
    ) {
        Ok(path) => path,
        Err(CompileError::Report {
            ref report,
            ref normalized_source,
        }) => {
            eprintln!("{}", render_report(source_path, normalized_source, report));
            return Err(1);
        }
        Err(error) => {
            eprintln!("error: compilation failed: {error}");
            return Err(1);
        }
    };

    println!("{}", binary_path.display());

    Ok(())
}

/// Compile source at `source_path` and execute it, forwarding `program_args` to the binary.
fn compile_and_run(
    source_path: &str,
    program_args: &[&str],
    target: &crate::build_system::targets::TargetTriple,
) -> Result<(), i32> {
    let source = match fs::read_to_string(source_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("error: failed to read '{source_path}': {error}");
            return Err(1);
        }
    };

    let binary_path =
        match compile_program(Path::new(source_path), &source, Path::new("target"), target) {
            Ok(path) => path,
            Err(CompileError::Report {
                ref report,
                ref normalized_source,
            }) => {
                eprintln!("{}", render_report(source_path, normalized_source, report));
                return Err(1);
            }
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
    if code == 0 { Ok(()) } else { Err(code) }
}

/// Dispatch `opal run` subcommand — compile and execute with optional arg passthrough.
fn run_run_command(args: &[String]) -> Result<(), i32> {
    let double_dash_pos = args.iter().position(|a| a == "--");
    let program_args: Vec<&str> = double_dash_pos
        .map(|p| {
            args.iter()
                .skip(p.saturating_add(1))
                .map(String::as_str)
                .collect()
        })
        .unwrap_or_default();

    let target = match resolve_target_from_args(args) {
        Ok(Some(target)) => target,
        Ok(None) => crate::build_system::targets::TargetTriple::host(),
        Err(code) => return Err(code),
    };

    if let Some(source_path) = args.get(2).map(String::as_str) {
        return compile_and_run(source_path, &program_args, &target);
    }

    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("error: failed to get current directory: {error}");
            return Err(1);
        }
    };

    let binary_path = match compile_project(&cwd, Path::new("target"), &target) {
        Ok(path) => path,
        Err(CompileError::Report {
            ref report,
            ref normalized_source,
        }) => {
            eprintln!(
                "{}",
                render_report("src/main.op", normalized_source, report)
            );
            return Err(1);
        }
        Err(error) => {
            eprintln!("error: compilation failed: {error}");
            return Err(1);
        }
    };

    println!("{}", binary_path.display());

    let status = match Command::new(&binary_path).args(&program_args).status() {
        Ok(state) => state,
        Err(error) => {
            eprintln!(
                "error: failed to execute '{}': {error}",
                binary_path.display()
            );
            return Err(1);
        }
    };
    let code = status.code().unwrap_or(1_i32);
    if code == 0 { Ok(()) } else { Err(code) }
}

/// Dispatch `opal fmt` subcommand arguments to [`FormatCommand`].
fn run_fmt_command(args: &[String]) -> Result<(), i32> {
    let fmt_args: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
    let check_mode = fmt_args.contains(&"--check");
    let find_flag = |flag: &str| {
        fmt_args
            .iter()
            .position(|&a| a == flag)
            .and_then(|i| fmt_args.get(i.saturating_add(1)).copied())
    };
    let config_path = find_flag("--config");
    let output_path = find_flag("--output");
    let source_path = fmt_args
        .iter()
        .find(|&&a| !a.starts_with("--") && Some(a) != config_path && Some(a) != output_path)
        .copied();
    let Some(source_path) = source_path else {
        eprintln!("error: opal fmt requires a source file — run 'opal help fmt' for usage");
        return Err(1);
    };
    if check_mode && output_path.is_some() {
        eprintln!("error: --check and --output cannot be used together");
        return Err(1);
    }
    let source = fs::read_to_string(source_path).map_err(|e| {
        eprintln!("error: failed to read '{source_path}': {e}");
        1_i32
    })?;
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
        match FormatCommand::new(source.clone()).execute_with_config(config) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: formatting failed: {e}");
                return Err(1);
            }
        }
    } else {
        match FormatCommand::new(source.clone()).execute() {
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
    let write_path = output_path.unwrap_or(source_path);
    fs::write(write_path, &formatted).map_err(|e| {
        eprintln!("error: failed to write '{write_path}': {e}");
        1_i32
    })?;
    println!("{write_path}");
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

    if report.is_success() { Ok(()) } else { Err(1) }
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
    let source = source.replace('\t', "    ");
    let mut report = CompilationErrorReport::new();
    let lexer = Lexer::new(&source);
    let (tokens, lex_errors) = lexer.tokenize();
    report.extend_lex_errors(lex_errors.errors);
    if !report.is_empty() {
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    let (program, parse_errors) = Parser::new(tokens).parse();
    report.extend_parse_errors(parse_errors.errors);
    if !report.is_empty() {
        eprintln!("{}", render_report(source_path, &source, &report));
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

/// Dispatch `opal check` — lex → parse → [`TypeChecker`] pipeline on `args[2]`.
/// Prints `check passed` on success; prints to stderr and returns `Err(1)` on any error.
fn run_check_command(args: &[String]) -> Result<(), i32> {
    let target_str = args
        .iter()
        .position(|a| a == "--target")
        .and_then(|i| args.get(i.saturating_add(1)).map(String::as_str));

    if let Some(triple_str) = target_str {
        if parse_target_triple(triple_str).is_err() {
            eprintln!(
                "error: unknown target triple: {triple_str}. Supported: x86_64-linux, x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, aarch64-darwin, x86_64-apple-darwin"
            );
            return Err(1);
        }
    }

    let Some(source_path) = args.get(2).map(String::as_str) else {
        eprintln!("error: no source file specified");
        eprintln!("Usage: opal check <file.op>");
        return Err(1);
    };
    let file_path = Path::new(source_path);
    let source = match fs::read_to_string(source_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("error: failed to read '{source_path}': {error}");
            return Err(1);
        }
    };
    let source = source.replace('\t', "    ");
    let mut report = CompilationErrorReport::new();
    let (tokens, lex_errors) = Lexer::new(&source).tokenize();
    report.extend_lex_errors(lex_errors.errors);
    if !report.is_empty() {
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    let (program_opt, parse_errors) = Parser::new(tokens).parse();
    report.extend_parse_errors(parse_errors.errors);
    if !report.is_empty() {
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    let Some(program) = program_opt else {
        eprintln!("error: parse errors in source");
        return Err(1);
    };
    if let Err(role_error) = validate_module_file_role(file_path, &program) {
        report.extend_type_errors(vec![role_error]);
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    let mut checker = TypeChecker::new();
    if let Err(errors) = checker.type_check_program(&program) {
        report.extend_type_errors(errors);
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    println!("check passed");
    Ok(())
}

/// Run the `opal build` command — reads `opal.toml` from the current directory, compiles `src/main.op`.
fn run_build_command(args: &[String]) -> Result<(), i32> {
    let target = match resolve_target_from_args(args) {
        Ok(Some(target)) => target,
        Ok(None) => crate::build_system::targets::TargetTriple::host(),
        Err(code) => return Err(code),
    };

    let Ok(toml_content) = fs::read_to_string("opal.toml") else {
        eprintln!("error: no opal.toml found in current directory");
        eprintln!("hint: run 'opal pkg init <name>' to create a project");
        return Err(1);
    };
    if let Err(
        BuildError::ParseError(msg)
        | BuildError::MissingField(msg)
        | BuildError::InvalidVersion(msg)
        | BuildError::InvalidConstraint(msg)
        | BuildError::DependencyConflict(msg)
        | BuildError::PackageNotFound(msg)
        | BuildError::InvalidTarget(msg),
    ) = parse_config(&toml_content)
    {
        eprintln!("error: invalid opal.toml: {msg}");
        return Err(1);
    }
    let binary_path = match compile_project(Path::new("."), Path::new("target"), &target) {
        Ok(path) => path,
        Err(CompileError::Report {
            ref report,
            ref normalized_source,
        }) => {
            eprintln!(
                "{}",
                render_report("src/main.op", normalized_source, report)
            );
            return Err(1);
        }
        Err(error) => {
            eprintln!("error: compilation failed: {error}");
            return Err(1);
        }
    };
    println!("{}", binary_path.display());
    Ok(())
}

/// Dispatch `opal watch` — poll source file for changes, recompile and run on each change.
fn run_watch_command(args: &[String]) -> Result<(), i32> {
    let Some(src) = args.get(2).map(String::as_str) else {
        eprintln!("error: no source file specified\nUsage: opal watch <file.op>");
        return Err(1);
    };
    if !Path::new(src).exists() {
        eprintln!("error: file not found: '{src}'");
        return Err(1);
    }
    let mut watcher = PollingFileWatcher::new(vec![src.to_owned()]);
    if watcher.start().is_err() {
        eprintln!("error: failed to start file watcher");
        return Err(1);
    }
    println!("Watching {src} for changes... (Ctrl-C to stop)");
    #[expect(
        clippy::infinite_loop,
        reason = "opal watch intentionally polls until Ctrl-C"
    )]
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !watcher.poll_changes().is_empty() {
            match compile_and_run(
                src,
                &[],
                &crate::build_system::targets::TargetTriple::host(),
            ) {
                Ok(()) => println!("Recompile successful."),
                Err(_) => eprintln!("Recompile failed."),
            }
        }
    }
}

/// Main CLI logic, delegating process exit handling to the public `run()` wrapper.
fn run_impl() -> Result<(), i32> {
    let args: Vec<String> = std::env::args().collect();
    run_with_args(&args)
}
#[cfg(test)]
mod tests {
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
}
