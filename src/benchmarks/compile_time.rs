extern crate alloc;

use crate::ast::Decl;
use crate::benchmarks::suite::measure_iterations;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::functions::codegen_function_declaration;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::testing::bench::BenchmarkResult;
use crate::type_system::checker::TypeChecker;
use inkwell::context::Context;

/// Measures parser latency for one source string.
#[must_use]
pub fn bench_parse(source: &str) -> BenchmarkResult {
    measure_iterations("compile_parse", 20, || {
        let lexer = Lexer::new(source);
        let (tokens, _lex_errors) = lexer.tokenize();
        let parser = Parser::new(tokens);
        consume(parser.parse());
    })
}

/// Measures type-checking latency for one source string.
#[must_use]
pub fn bench_typecheck(source: &str) -> BenchmarkResult {
    measure_iterations("compile_typecheck", 20, || {
        let lexer = Lexer::new(source);
        let (tokens, lex_errors) = lexer.tokenize();
        if !lex_errors.is_empty() {
            return;
        }

        let parser = Parser::new(tokens);
        let (program, parse_errors) = parser.parse();
        if !parse_errors.is_empty() {
            return;
        }

        if let Some(parsed_program) = program {
            let mut checker = TypeChecker::new();
            consume(checker.type_check_program(&parsed_program));
        }
    })
}

/// Measures LLVM codegen lowering latency for function declarations.
#[must_use]
pub fn bench_codegen(source: &str) -> BenchmarkResult {
    measure_iterations("compile_codegen", 10, || {
        let lexer = Lexer::new(source);
        let (tokens, lex_errors) = lexer.tokenize();
        if !lex_errors.is_empty() {
            return;
        }

        let parser = Parser::new(tokens);
        let (program, parse_errors) = parser.parse();
        if !parse_errors.is_empty() {
            return;
        }

        if let Some(parsed_program) = program {
            let context = Context::create();
            let codegen_context = CodegenContext::new(&context, "benchmark_codegen");
            let mut env = CodegenEnv::new(false);

            for declaration in &parsed_program.declarations {
                if let &Decl::Function { .. } = declaration {
                    consume(codegen_function_declaration(
                        &codegen_context,
                        &mut env,
                        declaration,
                    ));
                }
            }
        }
    })
}

/// Consumes a value to make benchmark side effects explicit.
fn consume<T>(value: T) {
    drop(value);
}
