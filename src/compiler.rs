//! Compiler orchestration helpers for front-end to LLVM module flow.
//!
//! This module provides a single pipeline entry that lexes, parses,
//! type-checks, and lowers Opalescent source into an LLVM module.

extern crate alloc;

use crate::ast::Decl;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenError;
use crate::codegen::functions::codegen_function_declaration;
use crate::codegen::expressions::CodegenEnv;
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::errors::ParseError;
use crate::token::Position;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use alloc::string::String;
use inkwell::context::Context;
use inkwell::module::Module;

/// Error type spanning every stage of compiler orchestration.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// Lexing stage returned a lexical analysis error.
    #[error("lexing failed")]
    Lex(LexError),
    /// Parsing stage returned a syntax analysis error.
    #[error("parsing failed")]
    Parse(ParseError),
    /// Type checking stage returned a semantic type error.
    #[error("type checking failed")]
    Type(TypeError),
    /// Code generation stage returned an LLVM lowering error.
    #[error("code generation failed: {0}")]
    Codegen(CodegenError),
}

/// Compile source text into an LLVM module using shared context lifetime.
///
/// # Errors
/// Returns `CompileError` when lexing, parsing, type-checking, or codegen fails.
pub fn compile_to_module<'context>(
    context: &'context Context,
    source: &str,
) -> Result<Module<'context>, CompileError> {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    if let Some(error) = lex_errors.errors.into_iter().next() {
        return Err(CompileError::Lex(error));
    }

    let parser = Parser::new(tokens);
    let (program_option, parse_errors) = parser.parse();
    if let Some(error) = parse_errors.errors.into_iter().next() {
        return Err(CompileError::Parse(error));
    }

    let Some(program) = program_option else {
        return Err(CompileError::Parse(ParseError::InvalidSyntax {
            message: String::from("parser returned no program after successful parse"),
            span: LexError::span_from_position(Position::start(), 1),
        }));
    };

    let mut checker = TypeChecker::new();
    if let Err(type_errors) = checker.type_check_program(&program) {
        if let Some(first_error) = type_errors.into_iter().next() {
            return Err(CompileError::Type(first_error));
        }
        return Err(CompileError::Type(TypeError::ConstraintSolvingFailed {
            reason: String::from("type checker returned empty error set"),
            span: TypeError::unknown_span(),
        }));
    }

    let codegen_context = CodegenContext::new(context, "opalescent_module");
    let mut env = CodegenEnv::new(true);

    for declaration in &program.declarations {
        if matches!(declaration, &Decl::Function { .. }) {
            codegen_function_declaration(&codegen_context, &mut env, declaration)
                .map_err(CompileError::Codegen)?;
        }
    }

    Ok(codegen_context.module)
}

#[cfg(test)]
mod tests {
    use super::{CompileError, compile_to_module};
    use inkwell::context::Context;

    /// Valid source should compile and produce a verifiable module.
    #[test]
    fn compile_to_module_valid_void_program() {
        let context = Context::create();
        let source = "entry main = f(): void => { return void }";
        let result = compile_to_module(&context, source);

        assert!(
            result.is_ok(),
            "valid source should compile into a module"
        );

        if let Ok(module) = result {
            let verification = module.verify();
            assert!(
                verification.is_ok(),
                "generated module should pass LLVM verification"
            );
            assert!(
                module.get_function("main").is_some(),
                "entry function codegen should emit a C ABI main wrapper"
            );
        }
    }

    /// Invalid characters should fail during lexical analysis.
    #[test]
    fn compile_to_module_lex_error() {
        let context = Context::create();
        let source = "entry main = f(): void => { let x = @@@invalid }";
        let result = compile_to_module(&context, source);

        assert!(
            matches!(result, Err(CompileError::Lex(_))),
            "invalid tokens should surface as CompileError::Lex"
        );
    }

    /// Type mismatch should fail after parse but before codegen.
    #[test]
    fn compile_to_module_type_error() {
        let context = Context::create();
        let source = "entry main = f(): void => { return 1 }";
        let result = compile_to_module(&context, source);

        assert!(
            matches!(result, Err(CompileError::Type(_))),
            "semantic mismatches should surface as CompileError::Type"
        );
    }
}
