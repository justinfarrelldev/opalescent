extern crate alloc;

use crate::error::LexError;
use crate::errors::reporter::CompilerError;
use crate::parser::errors::ParseError;
use crate::type_system::errors::TypeError;
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::min;

/// Maximum edit distance threshold for typo suggestions.
pub const SUGGESTION_DISTANCE_THRESHOLD: usize = 2;

/// Ranked suggestion candidate for unresolved identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierSuggestion {
    /// Misspelled identifier from source.
    pub input: String,
    /// Best-matching identifier in visible scope.
    pub suggestion: String,
    /// Edit distance between `input` and `suggestion`.
    pub distance: usize,
}

/// Return the best identifier suggestion within threshold, if any.
#[must_use]
pub fn closest_identifier_suggestion(
    input: &str,
    known_identifiers: &[String],
) -> Option<IdentifierSuggestion> {
    let mut best: Option<IdentifierSuggestion> = None;

    for candidate in known_identifiers {
        if candidate == input {
            continue;
        }

        let distance = levenshtein_distance(input, candidate.as_str());
        if distance > SUGGESTION_DISTANCE_THRESHOLD {
            continue;
        }

        let next = IdentifierSuggestion {
            input: input.to_owned(),
            suggestion: candidate.clone(),
            distance,
        };

        best = match best {
            Some(current)
                if current.distance < next.distance
                    || (current.distance == next.distance
                        && current.suggestion.as_str() <= next.suggestion.as_str()) =>
            {
                Some(current)
            }
            _ => Some(next),
        };
    }

    best
}

/// Return type-annotation guidance for generic inference failures.
#[must_use]
pub fn did_you_mean_type_annotation(error: &TypeError) -> Option<String> {
    match *error {
        TypeError::CannotInferGenericType { ref param_name, .. } => Some(format!(
            "Consider adding a type annotation or explicit generic argument for '{param_name}'."
        )),
        _ => None,
    }
}

/// Return a context-aware suggestion for a lex error, if available.
#[must_use]
pub fn suggest_for_lex_error(error: &LexError) -> Option<String> {
    match *error {
        LexError::UnterminatedString { .. } => Some(String::from(
            "String literal is not closed. Add a closing `'` to terminate the string.\n  Example: let msg = 'hello world'",
        )),
        LexError::InvalidEscapeSequence { ref sequence, .. } => Some(format!(
            "Invalid escape sequence `\\{sequence}`. Valid escape sequences are: `\\n` (newline), `\\t` (tab), `\\\\` (backslash), `\\'` (single quote)."
        )),
        _ => None,
    }
}

/// Return a context-aware suggestion for a parse error, if available.
#[must_use]
pub fn suggest_for_parse_error(error: &ParseError) -> Option<String> {
    match *error {
        ParseError::UnexpectedToken {
            ref expected,
            ref found,
            ..
        } => Some(format!(
            "Expected {expected} but found `{found}`. Check the syntax around this location."
        )),
        ParseError::MissingToken { ref expected, .. } => Some(format!(
            "Expected `{expected}` here — did you forget to add it?"
        )),
        _ => None,
    }
}

/// Return a context-aware suggestion for a type error, if available.
#[must_use]
pub fn suggest_for_type_error(error: &TypeError) -> Option<String> {
    match *error {
        TypeError::TypeMismatch {
            ref expected,
            ref found,
            ..
        } => Some(format!(
            "Expected type `{expected}` but found `{found}`.\n  If the conversion is intentional, use an explicit cast: `value as {expected}`.\n  Otherwise, change the expression to produce a `{expected}` value."
        )),
        TypeError::ArityMismatch {
            expected, found, ..
        } => Some(format!(
            "Function expects {expected} argument(s) but {found} were provided.\n  Check the function signature and update the call to pass the correct number of arguments."
        )),
        TypeError::ImmutableAssignment { ref name, .. } => Some(format!(
            "Cannot assign to immutable variable `{name}`.\n  To make it mutable, declare it with `let mutable`:\n    let mutable {name} = <value>"
        )),
        TypeError::TypeNotFound { ref type_name, .. } => Some(format!(
            "Type `{type_name}` was not found in scope.\n  Ensure the type is defined in a `.types.op` file and is visible from this location."
        )),
        TypeError::NotCallable { ref type_name, .. } => Some(format!(
            "Type `{type_name}` is not callable. Only functions can be called with `()`.\n  Check that this expression is a function or function-typed variable."
        )),
        TypeError::InvalidCast {
            ref from_type,
            ref to_type,
            ..
        } => Some(format!(
            "Cannot cast `{from_type}` to `{to_type}`.\n  These types are not directly convertible. Consider using an intermediate type or a conversion function."
        )),
        TypeError::MissingElseBranch {
            ref expected_type, ..
        } => Some(format!(
            "This `if` expression is missing an `else` branch that returns `{expected_type}`.\n  Add an `else` branch:\n    if <condition>:\n        return <value>\n    else:\n        return <{expected_type}-value>"
        )),
        TypeError::MissingEntryPoint { .. } => Some(String::from(
            "No `entry main` function was found. Add an entry point to your program:\n\n    entry main = f(args: string[]): void =>\n        return void",
        )),
        TypeError::CannotInferGenericType { ref param_name, .. } => Some(format!(
            "Cannot infer the generic type for `{param_name}`.\n  Consider adding a type annotation or explicit generic argument for `{param_name}`."
        )),
        TypeError::SymbolNotFound {
            suggestion: Some(ref suggestion),
            ..
        } => Some(format!("Did you mean `{suggestion}`?")),
        _ => None,
    }
}

/// Return a context-aware suggestion for any compiler error, if available.
#[must_use]
pub fn get_suggestion(error: &CompilerError) -> Option<String> {
    match *error {
        CompilerError::Lexer(ref lex_error) => suggest_for_lex_error(lex_error),
        CompilerError::Parser(ref parse_error) => suggest_for_parse_error(parse_error),
        CompilerError::TypeChecker(ref type_error) => suggest_for_type_error(type_error),
        CompilerError::Codegen(_) => None,
    }
}

/// Compute Levenshtein edit distance between two strings.
#[must_use]
pub fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }

    let right_chars: Vec<char> = right.chars().collect();
    let right_len = right_chars.len();

    if right_len == 0 {
        return left.chars().count();
    }

    let left_len = left.chars().count();
    if left_len == 0 {
        return right_len;
    }

    let mut previous: Vec<usize> = (0..=right_len).collect();
    let current_len = right_len.saturating_add(1);
    let mut current: Vec<usize> = vec![0; current_len];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index.saturating_add(1);
        for (right_index, right_char) in right_chars.iter().copied().enumerate() {
            let cost = usize::from(left_char != right_char);
            let insertion = current[right_index].saturating_add(1);
            let right_plus_one = right_index.saturating_add(1);
            let deletion = previous[right_plus_one].saturating_add(1);
            let substitution = previous[right_index].saturating_add(cost);
            current[right_plus_one] = min(min(insertion, deletion), substitution);
        }
        previous.clone_from(&current);
    }

    previous[right_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::reporter::CompilerError;
    use miette::SourceSpan;

    fn unknown_span() -> SourceSpan {
        SourceSpan::new(0.into(), 0)
    }

    #[test]
    fn test_type_mismatch_suggestion_mentions_both_types() {
        let error = TypeError::TypeMismatch {
            expected: String::from("int32"),
            found: String::from("string"),
            found_span: unknown_span(),
            expected_span: None,
        };
        let suggestion = suggest_for_type_error(&error).unwrap();
        assert!(suggestion.contains("int32"), "should mention expected type");
        assert!(suggestion.contains("string"), "should mention found type");
        assert!(suggestion.contains("as int32"), "should suggest cast");
    }

    #[test]
    fn test_immutable_assignment_suggests_let_mutable() {
        let error = TypeError::ImmutableAssignment {
            name: String::from("counter"),
            assignment_span: unknown_span(),
            declaration_span: None,
        };
        let suggestion = suggest_for_type_error(&error).unwrap();
        assert!(
            suggestion.contains("let mutable"),
            "should use Opalescent syntax 'let mutable', not 'let mut'"
        );
        assert!(
            suggestion.contains("counter"),
            "should mention variable name"
        );
    }

    #[test]
    fn test_missing_entry_point_shows_full_example() {
        let error = TypeError::MissingEntryPoint {
            span: unknown_span(),
        };
        let suggestion = suggest_for_type_error(&error).unwrap();
        assert!(
            suggestion.contains("entry main"),
            "should show entry keyword"
        );
        assert!(
            suggestion.contains("f(args: string[]): void"),
            "should show correct function signature"
        );
    }

    #[test]
    fn test_get_suggestion_dispatches_to_type_error() {
        let error = CompilerError::TypeChecker(TypeError::ImmutableAssignment {
            name: String::from("x"),
            assignment_span: unknown_span(),
            declaration_span: None,
        });
        let suggestion = get_suggestion(&error).unwrap();
        assert!(suggestion.contains("let mutable"));
    }

    #[test]
    fn test_unterminated_string_suggestion() {
        let pos = crate::token::Position {
            line: 1,
            column: 1,
            offset: 0,
        };
        let error = LexError::UnterminatedString {
            start: pos,
            span: unknown_span(),
        };
        let suggestion = suggest_for_lex_error(&error).unwrap();
        assert!(suggestion.contains("closing `'`"));
    }
}
