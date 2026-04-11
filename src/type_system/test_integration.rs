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

    const HELLO_WORLD_SOURCE: &str = include_str!("../../language-spec/hello_world.op");

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

    #[test]
    fn test_clean_program_produces_zero_errors_and_zero_warnings() {
        const SOURCE: &str = "
entry add = f(x: int32, y: int32): int32 =>
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

    #[test]
    fn test_program_without_entry_reports_missing_entry_point() {
        const SOURCE: &str = "
public helper = f(): int32 =>
    return 1
";

        let program = parse_pipeline(SOURCE);
        let result = TypeChecker::validate_entry_points(&program);

        let has_missing_entry = matches!(result, Err(TypeError::MissingEntryPoint { .. }));
        assert!(
            has_missing_entry,
            "expected MissingEntryPoint error, got: {result:?}",
        );
    }

    #[test]
    fn test_program_with_duplicate_entry_reports_duplicate_entry_point() {
        const SOURCE: &str = "
entry first = f(): int32 =>
    return 1

entry second = f(): int32 =>
    return 2
";

        let program = parse_pipeline(SOURCE);
        let result = TypeChecker::validate_entry_points(&program);

        let has_duplicate_entry = matches!(result, Err(TypeError::DuplicateEntryPoint { .. }));
        assert!(
            has_duplicate_entry,
            "expected DuplicateEntryPoint error, got: {result:?}",
        );
    }

    #[test]
    fn test_call_with_uninferable_generic_reports_cannot_infer_generic_type() {
        const SOURCE: &str = "
public passthrough = f<T>(value: int32): int32 =>
    return value

entry main = f(): int32 =>
    return passthrough(1)
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("uninferable generic call must fail type checking");

        let has_cannot_infer = errors
            .iter()
            .any(|error| matches!(*error, TypeError::CannotInferGenericType { .. }));
        assert!(
            has_cannot_infer,
            "expected CannotInferGenericType error, got: {errors:?}",
        );
    }

    #[test]
    fn test_lambda_closure_captures_outer_scope_variable() {
        const SOURCE: &str = "
entry main = f(): int64 => {
    let base: int64 = 41
    let add_one: f(): int64 = f(): int64 => { return base + 1 }
    return add_one()
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        assert!(
            result.is_ok(),
            "lambda closure should capture outer variable: {result:?}",
        );
    }

    #[test]
    fn test_guard_propagate_and_multiple_returns_integrate() {
        const SOURCE: &str = "
entry main = f(): int32, int32 errors ParseError => {
    guard string_to_int32('7') into parsed else { return first: 0, second: 0 }
    return first: parsed, second: parsed
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);

        assert!(
            result.is_ok(),
            "guard/propagate + multiple return integration should pass: {result:?}",
        );
    }
}
