use super::*;

#[test]
fn test_empty_input() {
    let lexer = Lexer::new("");
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].token_type, TokenType::EndOfFile));
}

#[test]
fn test_single_tokens() {
    let input = "()[]{}:,";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 9);

    let expected = [
        TokenType::LeftParen,
        TokenType::RightParen,
        TokenType::LeftBracket,
        TokenType::RightBracket,
        TokenType::LeftBrace,
        TokenType::RightBrace,
        TokenType::Colon,
        TokenType::Comma,
    ];

    for (i, expected_type) in expected.iter().enumerate() {
        assert_eq!(tokens[i].token_type, *expected_type);
    }
}

#[test]
fn test_operators() {
    let input = "+ - * / ^ % = < <= > >= =>";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());

    let expected = [
        TokenType::Plus,
        TokenType::Minus,
        TokenType::Multiply,
        TokenType::Divide,
        TokenType::Power,
        TokenType::Modulo,
        TokenType::Assign,
        TokenType::Less,
        TokenType::LessEqual,
        TokenType::Greater,
        TokenType::GreaterEqual,
        TokenType::Arrow,
    ];

    for (i, expected_type) in expected.iter().enumerate() {
        assert_eq!(tokens[i].token_type, *expected_type);
    }
}

#[test]
fn test_keywords() {
    let input = "let mutable f return void if else";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());

    let expected = [
        TokenType::Let,
        TokenType::Mutable,
        TokenType::Function,
        TokenType::Return,
        TokenType::Void,
        TokenType::If,
        TokenType::Else,
    ];

    for (i, expected_type) in expected.iter().enumerate() {
        assert_eq!(tokens[i].token_type, *expected_type);
    }
}

#[test]
fn test_identifiers() {
    let input = "hello_world _private snake_case";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 4);

    if let TokenType::Identifier(name) = tokens[0].token_type.clone() {
        assert_eq!(name, "hello_world");
    } else {
        assert!(
            matches!(tokens[0].token_type, TokenType::Identifier(_)),
            "Expected identifier"
        );
    }
}

#[test]
#[expect(
    clippy::approx_constant,
    reason = "3.14 is not pi, just a test float value"
)]
fn test_numbers() {
    let input = "42 3.14 0 999";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 5);

    assert!(matches!(
        tokens[0].token_type,
        TokenType::IntegerLiteral(42)
    ));
    assert!(matches!(
        tokens[1].token_type,
        TokenType::FloatLiteral(f) if (f - 3.14).abs() < f64::EPSILON
    ));
    assert!(matches!(tokens[2].token_type, TokenType::IntegerLiteral(0)));
    assert!(matches!(
        tokens[3].token_type,
        TokenType::IntegerLiteral(999)
    ));
}

#[test]
fn test_string_literals() {
    let input = r"'hello' 'world with spaces' 'with\nescapes'";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 4);

    if let TokenType::StringLiteral(s) = tokens[0].token_type.clone() {
        assert_eq!(s, "hello");
    } else {
        assert!(
            matches!(tokens[0].token_type, TokenType::StringLiteral(_)),
            "Expected string literal"
        );
    }

    if let TokenType::StringLiteral(s) = tokens[2].token_type.clone() {
        assert_eq!(s, "with\nescapes");
    } else {
        assert!(
            matches!(tokens[2].token_type, TokenType::StringLiteral(_)),
            "Expected string literal with escape"
        );
    }
}

#[test]
fn test_multiline_comment_simple() {
    let input = "## hello world ##";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    if !errors.is_empty() {
        for error in &errors.errors {
            println!("Error: {error}");
        }
    }

    if !tokens.is_empty() {
        println!("Found {} tokens", tokens.len());
    }

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 2);

    if let TokenType::Comment(content) = tokens[0].token_type.clone() {
        assert_eq!(content, "hello world");
    } else {
        assert!(
            matches!(tokens[0].token_type, TokenType::Comment(_)),
            "Expected comment token"
        );
    }
}

#[test]
fn test_comments() {
    let input = "# single line comment\n##\nmulti-line\ncomment\n##";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    if !errors.is_empty() {
        for error in &errors.errors {
            println!("Error: {error}");
        }
    }

    if !tokens.is_empty() {
        println!("Found {} tokens in test", tokens.len());
    }

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 4);

    assert!(matches!(tokens[0].token_type, TokenType::Comment(_)));
    assert!(matches!(tokens[1].token_type, TokenType::Newline));
    assert!(matches!(tokens[2].token_type, TokenType::Comment(_)));
}

#[test]
fn test_unterminated_string() {
    let input = "'unterminated string";
    let lexer = Lexer::new(input);
    let (_tokens, errors) = lexer.tokenize();

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors.errors[0],
        LexError::UnterminatedString { .. }
    ));
}

#[test]
fn test_unexpected_character() {
    let input = "hello @ world";
    let lexer = Lexer::new(input);
    let (_tokens, errors) = lexer.tokenize();

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors.errors[0],
        LexError::UnexpectedCharacter { character: '@', .. }
    ));
}

#[test]
fn test_indent_dedent_after_control_flow_colon() {
    let input = "if cond:\n    body\n";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());

    let expected = [
        TokenType::If,
        TokenType::Identifier("cond".to_owned()),
        TokenType::Colon,
        TokenType::Newline,
        TokenType::Indent,
        TokenType::Identifier("body".to_owned()),
        TokenType::Newline,
        TokenType::Dedent,
        TokenType::EndOfFile,
    ];

    let actual: Vec<TokenType> = tokens
        .iter()
        .map(|token| token.token_type.clone())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_type_annotation_colon_does_not_emit_indent_or_dedent() {
    let input = "let x: int32 = 5";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());
    assert!(!tokens
        .iter()
        .any(|token| matches!(token.token_type, TokenType::Indent | TokenType::Dedent)));
}

#[test]
fn test_indent_after_arrow_block_start() {
    let input = "loop =>\n    body\n";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());

    let expected = [
        TokenType::Loop,
        TokenType::Arrow,
        TokenType::Newline,
        TokenType::Indent,
        TokenType::Identifier("body".to_owned()),
        TokenType::Newline,
        TokenType::Dedent,
        TokenType::EndOfFile,
    ];

    let actual: Vec<TokenType> = tokens
        .iter()
        .map(|token| token.token_type.clone())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_eof_emits_remaining_dedent_tokens() {
    let input = "if cond:\n    body";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty());

    let expected = [
        TokenType::If,
        TokenType::Identifier("cond".to_owned()),
        TokenType::Colon,
        TokenType::Newline,
        TokenType::Indent,
        TokenType::Identifier("body".to_owned()),
        TokenType::Dedent,
        TokenType::EndOfFile,
    ];

    let actual: Vec<TokenType> = tokens
        .iter()
        .map(|token| token.token_type.clone())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_utf8_byte_offsets_for_multibyte_identifier() {
    let input = "let π = 42";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");

    let assign = tokens
        .iter()
        .find(|token| matches!(token.token_type, TokenType::Assign))
        .expect("expected '=' token");

    assert_eq!(assign.span.start.offset, 7);
    assert_eq!(assign.span.end.offset, 8);
    assert_eq!(assign.lexeme, "=");
}

#[test]
fn test_div_euclid_keyword() {
    let input = "div_euclid";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens.len(), 2); // div_euclid + EOF

    assert!(matches!(tokens[0].token_type, TokenType::DivEuclid));
    assert_eq!(tokens[0].lexeme, "div_euclid");
}

#[test]
fn test_mod_euclid_keyword() {
    let input = "mod_euclid";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens.len(), 2); // mod_euclid + EOF

    assert!(matches!(tokens[0].token_type, TokenType::ModEuclid));
    assert_eq!(tokens[0].lexeme, "mod_euclid");
}

#[test]
fn test_euclidean_operators_in_expression() {
    let input = "a div_euclid b mod_euclid c";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens.len(), 6); // a, div_euclid, b, mod_euclid, c, EOF

    assert!(matches!(tokens[1].token_type, TokenType::DivEuclid));
    assert!(matches!(tokens[3].token_type, TokenType::ModEuclid));
}

#[test]
fn test_cast_token_as_keyword() {
    let input = "x as int32";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    assert_eq!(tokens.len(), 4);

    assert!(
        matches!(tokens[1].token_type, TokenType::Cast),
        "Expected TokenType::Cast for 'as' keyword, got {:?}",
        tokens[1].token_type
    );
}

#[test]
fn test_is_not_operator_consistency() {
    let input = "x is not None";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();

    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");

    assert_eq!(tokens.len(), 4, "Expected 4 tokens: x, is not, None, EOF");

    assert!(matches!(tokens[0].token_type, TokenType::Identifier(ref name) if name == "x"));
    assert!(matches!(tokens[1].token_type, TokenType::IsNot));
    assert!(matches!(tokens[2].token_type, TokenType::Identifier(ref name) if name == "None"));
    assert!(matches!(tokens[3].token_type, TokenType::EndOfFile));
}
