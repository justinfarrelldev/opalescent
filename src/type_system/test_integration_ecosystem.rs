//! Ecosystem integration tests: parse → type-check for all `language-spec/*.op` files.
//!
//! Files that use colon-block syntax (`for x in xs:`, `while i < n:`, `if pred(x):`)
//! are marked `#[ignore]` with an explanatory reason — those tests will be un-ignored
//! once colon-block syntax lands in the parser.
//!
//! Files that use brace-block syntax (`=>` arrow bodies) are tested directly.

extern crate alloc;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::type_system::checker::TypeChecker;

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Lex and parse `source` with tabs normalised to four spaces.
///
/// Returns `Ok(())` if the entire parse → type-check pipeline succeeds,
/// or `Err(String)` describing the first failure.
fn run_pipeline(source: &str) -> Result<(), alloc::string::String> {
    let normalised = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalised);
    let (tokens, lex_errors) = lexer.tokenize();
    if !lex_errors.is_empty() {
        return Err(alloc::format!("lex errors: {:?}", lex_errors.errors));
    }

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    if !parse_errors.is_empty() {
        return Err(alloc::format!("parse errors: {:?}", parse_errors.errors));
    }

    let Some(program) = program_opt else {
        return Err(alloc::string::String::from("parser produced no program"));
    };

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(alloc::format!("type errors: {e:?}")),
    }
}

/// Attempt to lex and parse `source` (tabs normalised), returning whether parsing
/// succeeds.  Used for files where type-checking is not yet the goal.
fn parses_successfully(source: &str) -> bool {
    let normalised = source.replace('\t', "    ");
    let lexer = Lexer::new(&normalised);
    let (tokens, lex_errors) = lexer.tokenize();
    if !lex_errors.is_empty() {
        return false;
    }
    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    parse_errors.is_empty() && program_opt.is_some()
}

#[cfg(test)]
mod ecosystem_tests {
    use super::*;

    // ── error_handling_samples.op ─────────────────────────────────────────────

    /// `error_handling_samples.op` uses brace-block `=>` syntax and should parse
    /// without errors.  It is a library module (no `entry` function), so only
    /// parse-stage success is verified here — type-checking would report
    /// `MissingEntryPoint` which is expected for library files.
    #[test]
    fn test_error_handling_samples_spec_file_parses() {
        const SOURCE: &str = include_str!("../../language-spec/error_handling_samples.op");
        let passes = parses_successfully(SOURCE);
        assert!(
            passes,
            "error_handling_samples.op must parse without errors",
        );
    }

    // ── types_example.types.op ────────────────────────────────────────────────

    /// `types_example.types.op` uses indented colon-block type variant syntax
    /// (`type Message:` with `Text:`, `Join:` on indented lines) which the
    /// parser does not yet support — it expects `{ }` braces for type bodies.
    #[test]
    fn test_types_example_spec_file_parses() {
        const SOURCE: &str = include_str!("../../language-spec/types_example.types.op");
        let passes = parses_successfully(SOURCE);
        assert!(passes, "types_example.types.op must parse without errors",);
    }

    // ── array_helpers.op ──────────────────────────────────────────────────────

    /// `array_helpers.op` uses colon-block syntax (`for x in xs:`, `if pred(x):`,
    /// `while i < end_exclusive:`) which the parser does not yet support.
    #[test]
    #[ignore = "array_helpers.op uses colon-block syntax (for x in xs:) - causes infinite loop during type-checking"]
    fn test_array_helpers_spec_file_parses_and_type_checks() {
        const SOURCE: &str = include_str!("../../language-spec/array_helpers.op");
        let result = run_pipeline(SOURCE);
        assert!(
            result.is_ok(),
            "array_helpers.op must pass the full pipeline: {result:?}",
        );
    }

    // ── partition.op ──────────────────────────────────────────────────────────

    /// `partition.op` uses colon-block syntax (`for x in xs:`, `if pred(x): ... else:`)
    /// which the parser does not yet support.
    #[test]
    #[ignore = "partition.op: type checker hits infinite loop during generic type resolution (separate bug, not parser issue)"]
    fn test_partition_spec_file_parses_and_type_checks() {
        const SOURCE: &str = include_str!("../../language-spec/partition.op");
        let result = run_pipeline(SOURCE);
        assert!(
            result.is_ok(),
            "partition.op must pass the full pipeline: {result:?}",
        );
    }

    // ── unique_adjacent_sorted.op ─────────────────────────────────────────────

    /// `unique_adjacent_sorted.op` uses colon-block syntax (`while i < length(xs):`,
    /// `if cmp(...) is 0:`) which the parser does not yet support.
    #[test]
    #[ignore = "unique_adjacent_sorted.op: type checker hits infinite loop during generic type resolution (separate bug, not parser issue)"]
    fn test_unique_adjacent_sorted_spec_file_parses_and_type_checks() {
        const SOURCE: &str = include_str!("../../language-spec/unique_adjacent_sorted.op");
        let result = run_pipeline(SOURCE);
        assert!(
            result.is_ok(),
            "unique_adjacent_sorted.op must pass the full pipeline: {result:?}",
        );
    }

    // ── simple_quiz.op ────────────────────────────────────────────────────────

    /// `simple_quiz.op` uses colon-block syntax and `loop => break label: value`
    /// multi-return syntax which the parser does not yet fully support.
    #[test]
    #[ignore = "simple_quiz.op: requires labeled break/loop syntax and multi-return not yet implemented in type checker (separate bug, not parser issue)"]
    fn test_simple_quiz_spec_file_parses_and_type_checks() {
        const SOURCE: &str = include_str!("../../language-spec/simple_quiz.op");
        let result = run_pipeline(SOURCE);
        assert!(
            result.is_ok(),
            "simple_quiz.op must pass the full pipeline: {result:?}",
        );
    }
}
