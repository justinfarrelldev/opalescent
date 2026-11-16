//! Regression tests for contextual, source-preserving test prose.
use super::Lexer;
use crate::token::TokenType;

#[test]
fn test_prose_preserves_apostrophes_hashes_and_escaped_colons() {
    let source = "scenario user's # work\\: preserved:\n    it should keep café's # text: f(): void =>\n        return void\n";
    let (tokens, errors) = Lexer::new_test_module(source).tokenize();
    assert!(errors.is_empty(), "{errors:?}");
    let titles: Vec<_> = tokens
        .iter()
        .filter_map(|token| {
            if let TokenType::StringLiteral(ref value) = token.token_type {
                Some((value.as_str(), token))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(titles.len(), 2);
    assert_eq!(titles[0].0, "user's # work: preserved");
    assert_eq!(titles[1].0, "keep café's # text");
    assert_eq!(
        source.get(titles[0].1.span.start.offset..titles[0].1.span.end.offset),
        Some("user's # work\\: preserved")
    );
    assert_eq!(titles[1].1.span.start.line, 2);
    assert!(
        tokens
            .iter()
            .any(|token| token.token_type == TokenType::Function)
    );
}

#[test]
fn quoted_titles_and_bare_scenario_are_contextual() {
    let (tokens, errors) = Lexer::new_test_module(
        "scenario:\n    it should 'accept the :wq command': f(): void =>\n        return void\n",
    )
    .tokenize();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        tokens
            .iter()
            .any(|token| token.token_type
                == TokenType::StringLiteral("accept the :wq command".into()))
    );
    let (_, application_errors) =
        Lexer::new("let scenario = 'normal'\nlet example = 1\nlet world = 2\n").tokenize();
    assert!(application_errors.is_empty());
}

#[test]
fn prose_requires_a_terminating_colon_on_the_same_line() {
    let (_, errors) = Lexer::new_test_module("scenario unfinished\nlet x = 1\n").tokenize();
    assert!(!errors.is_empty());
}

#[test]
fn quoted_titles_cannot_consume_the_next_source_line() {
    let (tokens, errors) =
        Lexer::new_test_module("scenario 'unfinished\nlet next = 1\n").tokenize();
    assert!(!errors.is_empty());
    assert!(
        tokens
            .iter()
            .any(|token| token.token_type == TokenType::Let)
    );
}

#[test]
fn header_words_inside_callbacks_remain_ordinary_identifiers() {
    let (tokens, errors) = Lexer::new_test_module(
        "let scenario = 1\nlet example = scenario\nprint('it should to be')\n",
    )
    .tokenize();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        tokens
            .iter()
            .filter(|token| matches!(token.token_type, TokenType::StringLiteral(_)))
            .count(),
        1
    );
}
