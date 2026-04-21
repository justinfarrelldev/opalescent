extern crate alloc;

use crate::codegen::error::CodegenError;
use crate::error::LexError;
use crate::errors::formatter::CompilerPhase;
use crate::errors::formatter::format_error_bundle;
use crate::parser::errors::ParseError;
use crate::type_system::errors::TypeError;
use alloc::string::String;
use alloc::vec::Vec;

/// Unified compiler error payload spanning all compilation phases.
#[derive(Debug)]
pub enum CompilerError {
    /// Lexer diagnostic.
    Lexer(LexError),
    /// Parser diagnostic.
    Parser(ParseError),
    /// Type checker diagnostic.
    TypeChecker(TypeError),
    /// Code generation diagnostic represented as text.
    Codegen(CodegenError),
}

/// Ordered multi-phase compilation error report.
#[derive(Debug, Default)]
pub struct CompilationErrorReport {
    /// Collected errors in emission order.
    errors: Vec<(CompilerPhase, CompilerError)>,
}

impl CompilationErrorReport {
    /// Create an empty compilation error report.
    #[must_use]
    pub const fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Push one lexer diagnostic.
    pub fn push_lex_error(&mut self, error: LexError) {
        self.errors
            .push((CompilerPhase::Lexer, CompilerError::Lexer(error)));
    }

    /// Push one parser diagnostic.
    pub fn push_parse_error(&mut self, error: ParseError) {
        self.errors
            .push((CompilerPhase::Parser, CompilerError::Parser(error)));
    }

    /// Push one type checker diagnostic.
    pub fn push_type_error(&mut self, error: TypeError) {
        self.errors.push((
            CompilerPhase::TypeChecker,
            CompilerError::TypeChecker(error),
        ));
    }

    /// Push one code generation diagnostic string.
    pub fn push_codegen_error(&mut self, message: String) {
        self.errors.push((
            CompilerPhase::Codegen,
            CompilerError::Codegen(CodegenError::new(message)),
        ));
    }

    pub fn push_codegen_error_full(&mut self, error: CodegenError) {
        self.errors
            .push((CompilerPhase::Codegen, CompilerError::Codegen(error)));
    }

    /// Push multiple lexer diagnostics.
    pub fn extend_lex_errors(&mut self, errors: Vec<LexError>) {
        for error in errors {
            self.push_lex_error(error);
        }
    }

    /// Push multiple parser diagnostics.
    pub fn extend_parse_errors(&mut self, errors: Vec<ParseError>) {
        for error in errors {
            self.push_parse_error(error);
        }
    }

    /// Push multiple type checker diagnostics.
    pub fn extend_type_errors(&mut self, errors: Vec<TypeError>) {
        for error in errors {
            self.push_type_error(error);
        }
    }

    /// Return true when no diagnostics were collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Return number of collected diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Borrow collected diagnostics with phase metadata.
    #[must_use]
    pub fn entries(&self) -> &[(CompilerPhase, CompilerError)] {
        self.errors.as_slice()
    }

    /// Render this report using unified formatter output.
    #[must_use]
    pub fn render(&self) -> String {
        format_error_bundle(self.entries())
    }
}
