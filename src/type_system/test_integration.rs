//! End-to-end integration tests for the full parse → type check pipeline.
//!
//! These tests exercise the complete compiler front-end: source string → lexer →
//! parser → type checker → `Result`. They serve as regression guards for
//! language-spec programs and for error-reporting quality (span accuracy,
//! error codes, multi-error collection).
//!
//! ## Source format note
//!
//! The Opalescent `.op` language spec files (`language-spec/*.op`) use
//! colon-based block syntax (`if cond:` / `while cond:`) and tab indentation,
//! which the current parser does not yet support — blocks require `{` / `}`.
//! Brace-syntax equivalents of the spec files are tested here; the raw
//! spec-file `#[ignore]` tests track the colon-block feature once it lands.
//!
//! ## Integer literal sizing
//!
//! Integer literals (e.g. `0`, `1`, `10`) are inferred as `int64` by default.
//! All arithmetic fib sources therefore use `int64` for parameters and return
//! types so that `is` comparisons and return-type checks remain consistent.
//! This matches the spec intention (language decides default numeric type).

extern crate alloc;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Lex and parse `source`, asserting that both stages succeed.
///
/// Panics with a descriptive message on any lex or parse failure so that
/// integration-test failures are immediately actionable.
#[expect(
    clippy::panic,
    reason = "Integration-test helper: unrecoverable lex/parse failures should abort the test"
)]
fn parse_pipeline(source: &str) -> Program {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "integration source must lex without errors; lex errors: {:?}",
        lex_errors.errors,
    );

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "integration source must parse without errors; parse errors: {:?}",
        parse_errors.errors,
    );

    program_opt.unwrap_or_else(|| panic!("parser produced no program for valid source"))
}

/// Lex and parse `source` after normalising tab characters to four spaces.
///
/// The Opalescent spec files use hard tabs; normalising them ensures that
/// the lexer does not reject mixed-whitespace input.
fn parse_pipeline_with_spaces(source: &str) -> Program {
    let normalised = source.replace('\t', "    ");
    parse_pipeline(&normalised)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Language-spec: hello_world ────────────────────────────────────────────

    /// The raw `hello_world.op` spec source, included via `include_str!` so any
    /// drift between the file and the test is impossible.
    const HELLO_WORLD_SOURCE: &str = include_str!("../../language-spec/hello_world.op");

    /// Full pipeline test against the canonical `hello_world.op` spec file.
    ///
    /// The file uses tab-based indentation and `=>` function-body syntax, both
    /// of which are fully supported by the current lexer and parser.
    #[test]
    fn test_hello_world_full_pipeline_parses_and_type_checks() {
        let program = parse_pipeline_with_spaces(HELLO_WORLD_SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "hello_world.op must pass the full parse → type-check pipeline: {result:?}",
        );
    }

    // ── Language-spec: fib_recursive (brace-syntax equivalent) ───────────────

    /// Brace-syntax equivalent of `language-spec/fib_recursive.op`.
    ///
    /// Uses `public` visibility so the recursive call site can resolve the symbol
    /// during the type checker's second pass (body checking).
    /// Integer literals default to `int64`, so both parameters and return types
    /// use `int64` to keep comparisons and returns consistent.
    const FIB_RECURSIVE_BRACE_SOURCE: &str = "
public fib_recursive = f(n: int64): int64 =>
    if n is 0 { return 0 }
    if n is 1 { return 1 }
    return fib_recursive(n - 1) + fib_recursive(n - 2)

entry main = f(args: string[]): void =>
    let n: int64 = 10
    let result: int64 = fib_recursive(n)
    print(result)
    return void
";

    /// Full pipeline test for the recursive fibonacci logic.
    ///
    /// Exercises: `public` function declarations, recursive calls, `if` statements
    /// with `is`-equality, integer arithmetic, and `let` bindings with annotations.
    #[test]
    fn test_fib_recursive_equivalent_parses_and_type_checks() {
        let program = parse_pipeline(FIB_RECURSIVE_BRACE_SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "fib_recursive brace-syntax equivalent must pass the full pipeline: {result:?}",
        );
    }

    /// Tracks the raw `fib_recursive.op` spec file against the pipeline.
    ///
    /// Marked `#[ignore]` until `if cond:` / colon-indentation block support
    /// lands in the parser (Phase 2 parser work).
    #[test]
    #[ignore = "fib_recursive.op uses colon-block syntax (if n is 0:) not yet supported by the parser"]
    fn test_fib_recursive_spec_file_parses_and_type_checks() {
        const FIB_RECURSIVE_SOURCE: &str = include_str!("../../language-spec/fib_recursive.op");
        let program = parse_pipeline_with_spaces(FIB_RECURSIVE_SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "fib_recursive.op must pass the full pipeline: {result:?}",
        );
    }

    // ── Language-spec: fib_iterative (brace-syntax equivalent) ───────────────

    /// Brace-syntax equivalent of `language-spec/fib_iterative.op`.
    ///
    /// Uses `public` visibility for the function and `int64` throughout so that
    /// integer literals in comparisons and arithmetic remain type-consistent.
    /// The `while` body is enclosed in braces as the parser requires.
    const FIB_ITERATIVE_BRACE_SOURCE: &str = "
public fib_iter = f(n: int64): int64 =>
    if n is 0 { return 0 }
    if n is 1 { return 1 }
    let mutable a: int64 = 0
    let mutable b: int64 = 1
    let mutable i: int64 = 2
    let mutable result: int64 = 0
    while i <= n { result = a + b
        a = b
        b = result
        i = i + 1 }
    return result

entry main = f(args: string[]): void =>
    let n: int64 = 10
    let result: int64 = fib_iter(n)
    print(result)
    return void
";

    /// Full pipeline test for the iterative fibonacci logic.
    ///
    /// Exercises: `while` loops, `let mutable` bindings, variable assignments,
    /// `<=` comparisons, integer arithmetic, and multi-statement function bodies.
    #[test]
    fn test_fib_iterative_equivalent_parses_and_type_checks() {
        let program = parse_pipeline(FIB_ITERATIVE_BRACE_SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "fib_iterative brace-syntax equivalent must pass the full pipeline: {result:?}",
        );
    }

    /// Tracks the raw `fib_iterative.op` spec file against the pipeline.
    ///
    /// Marked `#[ignore]` until `while cond:` / colon-indentation block support
    /// lands in the parser (Phase 2 parser work).
    #[test]
    #[ignore = "fib_iterative.op uses colon-block syntax (while i <= n:) not yet supported by the parser"]
    fn test_fib_iterative_spec_file_parses_and_type_checks() {
        const FIB_ITERATIVE_SOURCE: &str = include_str!("../../language-spec/fib_iterative.op");
        let program = parse_pipeline_with_spaces(FIB_ITERATIVE_SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "fib_iterative.op must pass the full pipeline: {result:?}",
        );
    }

    // ── Multi-error reporting ─────────────────────────────────────────────────

    /// Verifies that the type checker collects and reports *multiple* type errors
    /// from a single program rather than stopping on the first failure.
    ///
    /// Both functions return the wrong type (`string` where `int32` is expected),
    /// so the resulting error vector must contain at least two entries.
    #[test]
    fn test_multi_error_reporting_returns_all_errors() {
        const SOURCE: &str = "
let bad_return_string = f(): int32 =>
    return 'not an int'

let bad_return_bool = f(): int32 =>
    return true
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        let errors =
            result.expect_err("program with two type-mismatched functions must fail type checking");

        assert!(
            errors.len() >= 2,
            "type checker must report at least two errors for two erroneous declarations; \
             got {} error(s): {errors:?}",
            errors.len(),
        );
    }

    /// Verifies that multi-error programs surface `TypeMismatch` diagnostics.
    ///
    /// Confirms that the error kind is preserved end-to-end through the pipeline
    /// and that individual errors carry the correct diagnostic code.
    #[test]
    fn test_multi_error_reporting_errors_have_type_mismatch_kind() {
        const SOURCE: &str = "
let wrong_a = f(): int32 =>
    return 'string_a'

let wrong_b = f(): int32 =>
    return 'string_b'
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        let errors = result.expect_err("two type-mismatched functions must fail type checking");

        let has_type_mismatch = errors
            .iter()
            .any(|err| matches!(*err, TypeError::TypeMismatch { .. }));

        assert!(
            has_type_mismatch,
            "at least one error must be a TypeMismatch diagnostic; got: {errors:?}",
        );
    }

    /// Verifies that erroneous declarations do not prevent subsequent valid ones
    /// from being checked, and that all errors are collected rather than aborting.
    #[test]
    fn test_multi_error_correct_and_bad_declarations_all_checked() {
        const SOURCE: &str = "
let first_bad = f(): int32 =>
    return 'wrong type'

let correct = f(): int32 =>
    return 42

let third_bad = f(): string =>
    return true
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        let errors =
            result.expect_err("program with two erroneous functions must fail type checking");

        assert!(
            errors.len() >= 2,
            "all erroneous declarations must be reported; \
             expected at least 2 errors, got {}: {errors:?}",
            errors.len(),
        );
    }

    // ── Error span accuracy ───────────────────────────────────────────────────

    /// Verifies that a `TypeMismatch` error carries a non-zero source span,
    /// confirming diagnostic location information propagates through the pipeline.
    ///
    /// The span should locate the mismatched string literal in the return
    /// statement, not point to the start of the source file.
    #[test]
    fn test_error_span_is_non_zero_for_type_mismatch() {
        const SOURCE: &str = "let typed_fn = f(): int32 => return 'oops'";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        let errors = result.expect_err("type-mismatch source must fail type checking");

        let mismatch = errors
            .iter()
            .find(|err| matches!(**err, TypeError::TypeMismatch { .. }));

        assert!(
            mismatch.is_some(),
            "expected a TypeMismatch error; got: {errors:?}",
        );

        let span_is_located = mismatch.is_some_and(|err| {
            if let TypeError::TypeMismatch { found_span, .. } = *err {
                found_span.offset() > 0 || !found_span.is_empty()
            } else {
                false
            }
        });

        assert!(
            span_is_located,
            "TypeMismatch error must carry a non-trivial source span to aid IDE diagnostics",
        );
    }

    /// Verifies that `SymbolNotFound` errors carry source spans that identify the
    /// use site of the undefined symbol, not a zero/default position.
    #[test]
    fn test_error_span_for_undefined_symbol_is_non_zero() {
        const SOURCE: &str = "let caller = f(): int32 => return undefined_function()";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        let errors = result.expect_err("calling undefined function must fail type checking");

        let symbol_error = errors
            .iter()
            .find(|err| matches!(**err, TypeError::SymbolNotFound { .. }));

        assert!(
            symbol_error.is_some(),
            "expected a SymbolNotFound error for undefined_function; got: {errors:?}",
        );

        let span_is_located = symbol_error.is_some_and(|err| {
            if let TypeError::SymbolNotFound { span, .. } = *err {
                span.offset() > 0 || !span.is_empty()
            } else {
                false
            }
        });

        assert!(
            span_is_located,
            "SymbolNotFound must carry a meaningful source span for IDE integration",
        );
    }

    // ── Pipeline isolation: parse-only ────────────────────────────────────────

    /// Verifies that a syntactically valid but semantically invalid program
    /// produces zero parse errors, confirming that parsing and type checking
    /// remain cleanly separated stages.
    #[test]
    fn test_parse_succeeds_on_semantically_invalid_program() {
        const SOURCE: &str = "let bad = f(): int32 => return 'bad'";

        let lexer = Lexer::new(SOURCE);
        let (tokens, lex_errors) = lexer.tokenize();
        assert!(
            lex_errors.is_empty(),
            "source must lex without errors: {:?}",
            lex_errors.errors,
        );

        let parser = Parser::new(tokens);
        let (program_opt, parse_errors) = parser.parse();
        assert!(
            parse_errors.is_empty(),
            "parser must succeed on syntactically valid program: {:?}",
            parse_errors.errors,
        );

        assert!(
            program_opt.is_some(),
            "parser must return a program for valid source",
        );
    }

    /// Verifies that a program which is both syntactically and semantically valid
    /// passes through the entire pipeline producing zero errors and zero warnings.
    #[test]
    fn test_clean_program_produces_zero_errors_and_zero_warnings() {
        const SOURCE: &str = "
let add = f(x: int32, y: int32): int32 =>
    return x + y
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        assert!(
            result.is_ok(),
            "clean program must pass type checking: {result:?}",
        );

        assert!(
            checker.warnings().is_empty(),
            "clean program must produce zero warnings; got: {:?}",
            checker.warnings(),
        );
    }
}
