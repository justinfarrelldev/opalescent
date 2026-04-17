extern crate alloc;

use crate::errors::reporter::CompilerError;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use miette::Diagnostic;

/// Compiler phase used by unified error formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerPhase {
    /// Lexer phase.
    Lexer,
    /// Parser phase.
    Parser,
    /// Type checker phase.
    TypeChecker,
    /// Code generation phase.
    Codegen,
}

impl CompilerPhase {
    /// Render human-readable compiler phase label.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Lexer => "lexer",
            Self::Parser => "parser",
            Self::TypeChecker => "type checker",
            Self::Codegen => "codegen",
        }
    }
}

/// Render a single compiler error in a miette-inspired textual layout.
#[must_use]
pub fn format_diagnostic(phase: CompilerPhase, error: &CompilerError) -> String {
    match *error {
        CompilerError::Lexer(ref lex_error) => format_miette_phase_error(
            phase,
            &format!("{lex_error}"),
            lex_error
                .code()
                .map_or_else(
                    || String::from("opalescent::lexer::unknown"),
                    |value| value.to_string(),
                )
                .as_str(),
            lex_error
                .help()
                .map_or_else(
                    || String::from("No additional help available."),
                    |value| value.to_string(),
                )
                .as_str(),
        ),
        CompilerError::Parser(ref parse_error) => format_miette_phase_error(
            phase,
            &format!("{parse_error}"),
            parse_error
                .code()
                .map_or_else(
                    || String::from("opalescent::parser::unknown"),
                    |value| value.to_string(),
                )
                .as_str(),
            parse_error
                .help()
                .map_or_else(
                    || String::from("No additional help available."),
                    |value| value.to_string(),
                )
                .as_str(),
        ),
        CompilerError::TypeChecker(ref type_error) => {
            let help_suffix = match *type_error {
                crate::type_system::errors::TypeError::SymbolNotFound {
                    suggestion: Some(ref suggestion),
                    ..
                } => format!("\n  suggestion: did you mean '{suggestion}'?"),
                crate::type_system::errors::TypeError::CannotInferGenericType { .. } => {
                    String::from("\n  suggestion: Consider adding type annotation.")
                }
                _ => String::new(),
            };

            format!(
                "{}{}",
                format_miette_phase_error(
                    phase,
                    &format!("{type_error}"),
                    type_error
                        .code()
                        .map_or_else(
                            || String::from("opalescent::type_system::unknown"),
                            |value| value.to_string()
                        )
                        .as_str(),
                    type_error
                        .help()
                        .map_or_else(
                            || String::from("No additional help available."),
                            |value| value.to_string()
                        )
                        .as_str(),
                ),
                help_suffix
            )
        }
        CompilerError::Codegen(ref codegen_error) => format_miette_phase_error(
            phase,
            codegen_error.message.as_str(),
            "opalescent::codegen::backend_failure",
            "Inspect generated IR and source spans near this expression.",
        ),
    }
}

/// Render a standalone codegen error message with consistent formatting.
#[must_use]
pub fn format_codegen_error(message: &str) -> String {
    format_miette_phase_error(
        CompilerPhase::Codegen,
        message,
        "opalescent::codegen::backend_failure",
        "Inspect generated IR and source spans near this expression.",
    )
}

/// Render a multi-error bundle with blank-line separators.
#[must_use]
pub fn format_error_bundle(entries: &[(CompilerPhase, CompilerError)]) -> String {
    let mut rendered_entries = Vec::new();
    for &(phase, ref error) in entries {
        rendered_entries.push(format_diagnostic(phase, error));
    }
    rendered_entries.join("\n\n")
}

/// Build a stable docs URL for a diagnostic code.
#[must_use]
pub fn error_doc_link(code: &str) -> String {
    format!("https://docs.opalescent.dev/errors/{code}")
}

/// Shared formatter used by all compiler phases.
fn format_miette_phase_error(
    phase: CompilerPhase,
    message: &str,
    code: &str,
    help: &str,
) -> String {
    format!(
        "error[{code}]\n  phase: {}\n  x {message}\n  help: {help}\n  docs: {}",
        phase.display_name(),
        error_doc_link(code)
    )
}
