//! Tests for the Opalescent code formatter.
//!
//! All tests use inline strings — no file I/O is performed.

#[cfg(test)]
mod formatter_tests {
    use crate::formatter::command::FormatCommand;
    use crate::formatter::config::FormatterConfig;
    use crate::formatter::naming::{
        check_program, is_pascal_case, is_snake_case, NamingStyle, NamingViolation,
    };
    use crate::formatter::printer::Formatter;
    use crate::formatter::rules;

    // ─── FormatterConfig Tests ──────────────────────────────────────────────────

    /// Default configuration has the correct field values.
    #[test]
    fn test_config_defaults() {
        let cfg = FormatterConfig::default();
        assert_eq!(cfg.indent_size, 4, "default indent_size should be 4");
        assert_eq!(
            cfg.max_line_width, 100,
            "default max_line_width should be 100"
        );
        assert!(!cfg.use_tabs, "default use_tabs should be false");
    }

    /// `indent_unit()` returns spaces when `use_tabs` is false.
    #[test]
    fn test_config_indent_unit_spaces() {
        let cfg = FormatterConfig::new(2, 80, false);
        assert_eq!(
            cfg.indent_unit(),
            "  ",
            "2-space indent should produce two spaces"
        );
    }

    /// `indent_unit()` returns a tab when `use_tabs` is true.
    #[test]
    fn test_config_indent_unit_tab() {
        let cfg = FormatterConfig::new(4, 80, true);
        assert_eq!(
            cfg.indent_unit(),
            "\t",
            "tab indent should produce a single tab"
        );
    }

    /// `from_toml_str` parses all three recognised keys.
    #[test]
    fn test_config_from_toml_str() {
        let toml = "indent_size = 2\nmax_line_width = 80\nuse_tabs = false\n";
        let cfg =
            FormatterConfig::from_toml_str(toml).expect("valid TOML should parse without error");
        assert_eq!(cfg.indent_size, 2, "parsed indent_size should be 2");
        assert_eq!(cfg.max_line_width, 80, "parsed max_line_width should be 80");
        assert!(!cfg.use_tabs, "parsed use_tabs should be false");
    }

    /// `from_toml_str` accepts `use_tabs = true`.
    #[test]
    fn test_config_from_toml_str_tabs() {
        let toml = "use_tabs = true\n";
        let cfg =
            FormatterConfig::from_toml_str(toml).expect("valid TOML should parse without error");
        assert!(cfg.use_tabs, "parsed use_tabs should be true");
    }

    /// `from_toml_str` ignores unknown keys.
    #[test]
    fn test_config_from_toml_str_ignores_unknown_keys() {
        let toml = "indent_size = 4\nunknown_key = 99\n";
        let result = FormatterConfig::from_toml_str(toml);
        assert!(result.is_ok(), "unknown keys should be silently ignored");
    }

    /// `from_toml_str` errors on invalid `indent_size`.
    #[test]
    fn test_config_from_toml_str_invalid_indent_size() {
        let toml = "indent_size = abc\n";
        let result = FormatterConfig::from_toml_str(toml);
        assert!(
            result.is_err(),
            "non-numeric indent_size should produce an error"
        );
    }

    /// `from_toml_str` errors on zero `indent_size`.
    #[test]
    fn test_config_from_toml_str_zero_indent_size() {
        let toml = "indent_size = 0\n";
        let result = FormatterConfig::from_toml_str(toml);
        assert!(result.is_err(), "zero indent_size should produce an error");
    }

    /// `from_toml_str` errors on invalid `use_tabs` value.
    #[test]
    fn test_config_from_toml_str_invalid_use_tabs() {
        let toml = "use_tabs = yes\n";
        let result = FormatterConfig::from_toml_str(toml);
        assert!(
            result.is_err(),
            "invalid use_tabs value should produce an error"
        );
    }

    // ─── Formatting Rules Tests ──────────────────────────────────────────────────

    /// `normalize_line_endings` converts CRLF to LF.
    #[test]
    fn test_rules_normalize_crlf() {
        let input = "line1\r\nline2\r\n";
        let output = rules::normalize_line_endings(input);
        assert_eq!(output, "line1\nline2\n", "CRLF should be converted to LF");
    }

    /// `normalize_line_endings` converts bare CR to LF.
    #[test]
    fn test_rules_normalize_bare_cr() {
        let input = "line1\rline2";
        let output = rules::normalize_line_endings(input);
        assert_eq!(output, "line1\nline2", "bare CR should be converted to LF");
    }

    /// `remove_trailing_whitespace` strips spaces from line ends.
    #[test]
    fn test_rules_remove_trailing_whitespace() {
        let input = "hello   \nworld  \n";
        let output = rules::remove_trailing_whitespace(input);
        assert_eq!(
            output, "hello\nworld\n",
            "trailing spaces should be removed"
        );
    }

    /// `ensure_trailing_newline` adds exactly one newline.
    #[test]
    fn test_rules_ensure_trailing_newline_adds_one() {
        let input = "hello";
        let output = rules::ensure_trailing_newline(input);
        assert_eq!(output, "hello\n", "trailing newline should be added");
    }

    /// `ensure_trailing_newline` normalises multiple trailing newlines to one.
    #[test]
    fn test_rules_ensure_trailing_newline_normalises_multiple() {
        let input = "hello\n\n\n";
        let output = rules::ensure_trailing_newline(input);
        assert_eq!(
            output, "hello\n",
            "multiple trailing newlines should collapse to one"
        );
    }

    /// `collapse_consecutive_blank_lines` collapses runs of blank lines.
    #[test]
    fn test_rules_collapse_blank_lines() {
        let input = "a\n\n\n\nb";
        let output = rules::collapse_consecutive_blank_lines(input);
        assert_eq!(output, "a\n\nb", "multiple blanks should collapse to one");
    }

    /// `apply_all` is idempotent on plain text.
    #[test]
    fn test_rules_apply_all_idempotent_text() {
        let input = "hello world\n";
        let once = rules::apply_all(input);
        let twice = rules::apply_all(&once);
        assert_eq!(once, twice, "apply_all must be idempotent");
    }

    /// Operator spacing must preserve spaces inside single-quoted strings.
    #[test]
    fn test_rules_operator_spacing_preserves_single_quoted_string_whitespace() {
        let input = "entry main = f(): void =>\n    print('a    b')\n";
        let output = rules::apply_all(input);
        assert!(
            output.contains("'a    b'"),
            "single-quoted string whitespace should be preserved: {output}"
        );
    }

    #[test]
    fn test_rules_apply_all_preserves_four_space_leading_indent() {
        let input = "    print('hello')\n";
        let output = rules::apply_all(input);
        assert!(
            output.starts_with("    print('hello')"),
            "4-space indentation should be preserved, got: {output:?}"
        );
    }

    #[test]
    fn test_rules_apply_all_preserves_eight_space_leading_indent() {
        let input = "        let x = 1\n";
        let output = rules::apply_all(input);
        assert!(
            output.starts_with("        let x = 1"),
            "8-space indentation should be preserved, got: {output:?}"
        );
    }

    #[test]
    fn test_formatter_preserves_multi_level_indentation_for_nested_block() {
        let source = "entry main = f(): void => {\n    print('hello')\n    while true {\n        print('loop')\n    }\n}";
        let fmt = Formatter::with_defaults();
        let output = fmt
            .format_source(source)
            .expect("nested block should format successfully");

        assert!(
            output.contains("\n    print('hello')\n"),
            "level-1 statement should keep 4-space indentation, got: {output}"
        );
        assert!(
            output.contains("\n    while true {\n"),
            "while header should keep 4-space indentation, got: {output}"
        );
        assert!(
            output.contains("\n        print('loop')\n"),
            "nested statement should keep 8-space indentation, got: {output}"
        );
    }

    #[test]
    fn test_rules_operator_spacing_collapses_non_leading_spaces() {
        let input = "  let x  =  1\n";
        let output = rules::apply_all(input);
        assert!(
            output.starts_with("  let x = 1"),
            "non-leading operator spacing should normalize while preserving indent, got: {output:?}"
        );
    }

    #[test]
    fn test_rules_operator_spacing_preserves_leading_spaces_inside_string_literals() {
        let input = "entry main = f(): void =>\n    print('    keep')\n";
        let output = rules::apply_all(input);
        assert!(
            output.contains("'    keep'"),
            "leading string-literal whitespace should be preserved, got: {output}"
        );
    }

    // ─── Naming Convention Tests ─────────────────────────────────────────────────

    /// `is_snake_case` accepts valid `snake_case` identifiers.
    #[test]
    fn test_naming_snake_case_valid() {
        assert!(is_snake_case("foo"), "single word should be snake_case");
        assert!(
            is_snake_case("foo_bar"),
            "underscored identifier should be snake_case"
        );
        assert!(
            is_snake_case("foo_bar_42"),
            "alphanumeric snake_case should be valid"
        );
        assert!(
            is_snake_case("_unused"),
            "underscore-prefixed should be snake_case"
        );
    }

    /// `is_snake_case` rejects `camelCase` and `PascalCase`.
    #[test]
    fn test_naming_snake_case_invalid() {
        assert!(
            !is_snake_case("fooBar"),
            "camelCase should not be snake_case"
        );
        assert!(
            !is_snake_case("FooBar"),
            "PascalCase should not be snake_case"
        );
        assert!(
            !is_snake_case("MyType"),
            "PascalCase should not be snake_case"
        );
    }

    /// `is_pascal_case` accepts valid `PascalCase` identifiers.
    #[test]
    fn test_naming_pascal_case_valid() {
        assert!(
            is_pascal_case("Foo"),
            "single-word PascalCase should be valid"
        );
        assert!(
            is_pascal_case("FooBar"),
            "multi-word PascalCase should be valid"
        );
        assert!(
            is_pascal_case("MyType"),
            "type-style PascalCase should be valid"
        );
    }

    /// `is_pascal_case` rejects `snake_case` and `camelCase`.
    #[test]
    fn test_naming_pascal_case_invalid() {
        assert!(
            !is_pascal_case("foo"),
            "lowercase first should not be PascalCase"
        );
        assert!(
            !is_pascal_case("foo_bar"),
            "snake_case should not be PascalCase"
        );
        assert!(
            !is_pascal_case("fooBar"),
            "camelCase should not be PascalCase"
        );
        assert!(
            !is_pascal_case("Foo_Bar"),
            "underscore with uppercase should not be PascalCase"
        );
    }

    /// `NamingViolation::Display` includes the identifier name and expected style.
    #[test]
    fn test_naming_violation_display() {
        let v = NamingViolation {
            name: String::from("fooBar"),
            expected: NamingStyle::SnakeCase,
            location: String::from("let binding"),
        };
        let s = format!("{v}");
        assert!(
            s.contains("fooBar"),
            "display should include the identifier"
        );
        assert!(
            s.contains("snake_case"),
            "display should mention snake_case"
        );
    }

    /// `check_program` detects a camelCase function name.
    ///
    /// Uses valid Opalescent syntax: `entry name = f(params): ret => body`
    #[test]
    fn test_naming_check_program_camel_function() {
        // myFunc is camelCase — should trigger a naming violation
        let source = "entry myFunc = f(): void => return void";
        let lexer = crate::lexer::Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = crate::parser::Parser::new(tokens);
        let (prog, _) = parser.parse();
        assert!(prog.is_some(), "source should parse successfully: {source}");
        if let Some(program) = prog {
            let violations = check_program(&program);
            let names: Vec<&str> = violations.iter().map(|v| v.name.as_str()).collect();
            assert!(
                names.contains(&"myFunc"),
                "camelCase function name should produce a violation, got: {names:?}"
            );
        }
    }

    /// `check_program` detects a `snake_case` type name.
    ///
    /// Uses valid Opalescent syntax: `type Name:\n    field: Type`
    #[test]
    fn test_naming_check_program_snake_type() {
        // my_type is snake_case — types must be PascalCase
        let source = "type my_type:\n    x: int32";
        let lexer = crate::lexer::Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = crate::parser::Parser::new(tokens);
        let (prog, _) = parser.parse();
        assert!(prog.is_some(), "source should parse successfully: {source}");
        if let Some(program) = prog {
            let violations = check_program(&program);
            let names: Vec<&str> = violations.iter().map(|v| v.name.as_str()).collect();
            assert!(
                names.contains(&"my_type"),
                "snake_case type name should produce a PascalCase violation, got: {names:?}"
            );
        }
    }

    /// `check_program` produces no violations for well-named code.
    ///
    /// Uses valid Opalescent syntax: `entry name = f(): ret => body`
    #[test]
    fn test_naming_check_program_no_violations() {
        // my_func is snake_case — correct for functions
        let source = "entry my_func = f(): void => return void";
        let lexer = crate::lexer::Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = crate::parser::Parser::new(tokens);
        let (prog, _) = parser.parse();
        assert!(prog.is_some(), "source should parse successfully: {source}");
        if let Some(program) = prog {
            let violations = check_program(&program);
            assert!(
                violations.is_empty(),
                "well-named code should produce no violations, got: {violations:?}"
            );
        }
    }

    // ─── Formatter / Printer Tests ───────────────────────────────────────────────

    /// Formatting a simple function declaration produces syntactically valid
    /// source code with a trailing newline.
    #[test]
    fn test_formatter_simple_function() {
        let source = "entry hello = f(): void => return void";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("simple function should format");
        assert!(
            result.ends_with('\n'),
            "formatted output must end with a newline"
        );
        assert!(
            result.contains("hello"),
            "formatted output should contain the function name"
        );
    }

    /// Formatting preserves integer literals.
    #[test]
    fn test_formatter_integer_literal() {
        let source = "entry answer = f(): int64 => return 42";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("integer literal should format");
        assert!(
            result.contains("42"),
            "formatted output should contain the integer literal"
        );
    }

    /// Formatting preserves boolean literals.
    #[test]
    fn test_formatter_boolean_literal() {
        let source = "entry is_ready = f(): boolean => return true";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("boolean literal should format");
        assert!(
            result.contains("true"),
            "formatted output should contain the boolean literal"
        );
    }

    /// Formatter should emit single-quoted strings for string literals.
    #[test]
    fn test_formatter_emits_single_quoted_string_literals() {
        let source = "entry greet = f(): string => return 'hi'";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("string literal formatting should succeed");
        assert!(
            result.contains("'hi'"),
            "formatter should output single-quoted literal, got: {result}"
        );
        assert!(
            !result.contains("\"hi\""),
            "formatter output should not contain double-quoted literal, got: {result}"
        );
    }

    /// Match expressions should print with brace-arm syntax so output stays parseable.
    #[test]
    fn test_formatter_emits_match_brace_syntax() {
        let source =
            "entry classify = f(n: int32): string => return match n { 0 => 'zero', _ => 'other' }";
        let fmt = Formatter::with_defaults();
        let formatted = fmt
            .format_source(source)
            .expect("match formatting should succeed");

        assert!(
            formatted.contains("match n {"),
            "match expression should use brace syntax, got: {formatted}"
        );
        assert!(
            !formatted.contains("match n:"),
            "match expression should not emit colon-block syntax, got: {formatted}"
        );

        let lexer = crate::lexer::Lexer::new(&formatted);
        let (tokens, lex_errors) = lexer.tokenize();
        assert!(
            lex_errors.errors.is_empty(),
            "formatted output should lex without errors: {lex_errors:?}"
        );

        let parser = crate::parser::Parser::new(tokens);
        let (_program, parse_errors) = parser.parse();
        assert!(
            parse_errors.errors.is_empty(),
            "formatted output should parse without errors: {parse_errors:?}"
        );
    }

    /// An invalid source returns a `ParseError`.
    #[test]
    fn test_formatter_parse_error() {
        let source = "( invalid";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(result.is_err(), "invalid source should return an error");
        let err = result.expect_err("already checked above");
        let msg = format!("{err}");
        assert!(
            msg.contains("parse error"),
            "error message should mention parse error, got: {msg}"
        );
    }

    #[test]
    fn test_formatter_handles_tab_indented_source() {
        let source = "##\n  Description: entry point\n##\nentry main = f(args: string[]): void =>\n\treturn void\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(
            result.is_ok(),
            "formatter should succeed on tab-indented source, got: {result:?}"
        );
        let output = result.unwrap();
        assert!(
            !output.contains('\t'),
            "formatter output must not contain raw tab characters"
        );
    }

    #[test]
    fn test_formatter_handles_crlf_line_endings() {
        let source = "##\r\n  Description: entry point\r\n##\r\nentry main = f(args: string[]): void =>\r\n    return void\r\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(
            result.is_ok(),
            "formatter should succeed on CRLF source, got: {result:?}"
        );
        let output = result.unwrap();
        assert!(
            !output.contains('\r'),
            "formatter output must not contain carriage return characters"
        );
    }

    #[test]
    fn test_formatter_handles_crlf_and_tabs_combined() {
        let source = "##\r\n  Description: entry point\r\n##\r\nentry main = f(args: string[]): void =>\r\n\treturn void\r\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(
            result.is_ok(),
            "formatter should succeed on CRLF + tab-indented source, got: {result:?}"
        );
        let output = result.unwrap();
        assert!(!output.contains('\r'), "output must not contain CR");
        assert!(!output.contains('\t'), "output must not contain tab");
    }

    // ─── Idempotency Tests ───────────────────────────────────────────────────────

    /// Formatting is idempotent for a simple function.
    #[test]
    fn test_formatter_idempotent_simple_function() {
        let source = "entry greet = f(): void => return void";
        let fmt = Formatter::with_defaults();
        let once = fmt
            .format_source(source)
            .expect("first format should succeed");
        let twice = fmt
            .format_source(&once)
            .expect("second format should succeed");
        assert_eq!(
            once, twice,
            "format(format(x)) must equal format(x) for simple function"
        );
    }

    /// Formatting is idempotent for a function with a return statement.
    #[test]
    fn test_formatter_idempotent_with_return() {
        let source = "public add = f(x: int64, y: int64): int64 => return x + y";
        let fmt = Formatter::with_defaults();
        let once = fmt
            .format_source(source)
            .expect("first format should succeed");
        let twice = fmt
            .format_source(&once)
            .expect("second format should succeed");
        assert_eq!(
            once, twice,
            "format(format(x)) must equal format(x) for function with return"
        );
    }

    /// Formatting is idempotent for a let declaration.
    #[test]
    fn test_formatter_idempotent_let_decl() {
        let source = "let x = 10";
        let fmt = Formatter::with_defaults();
        let once = fmt
            .format_source(source)
            .expect("first format should succeed");
        let twice = fmt
            .format_source(&once)
            .expect("second format should succeed");
        assert_eq!(
            once, twice,
            "format(format(x)) must equal format(x) for let declaration"
        );
    }

    // ─── FormatCommand Tests ─────────────────────────────────────────────────────

    /// `FormatCommand::execute` returns formatted source.
    #[test]
    fn test_format_command_execute() {
        let cmd = FormatCommand::new(String::from("entry main = f(): void => return void"), false);
        let result = cmd
            .execute()
            .expect("FormatCommand::execute should succeed");
        assert!(
            result.contains("main"),
            "FormatCommand output should contain the function name"
        );
    }

    /// `FormatCommand::execute_with_config` respects custom indent.
    #[test]
    fn test_format_command_custom_config() {
        let source = "entry main = f(): void => return void";
        let cmd = FormatCommand::new(String::from(source), false);
        let cfg = FormatterConfig::new(2, 80, false);
        let result = cmd
            .execute_with_config(cfg)
            .expect("FormatCommand with custom config should succeed");
        assert!(
            result.contains("main"),
            "custom config output should contain the function name"
        );
    }

    /// `FormatCommand` with `in_place = true` still returns the formatted
    /// output without performing any file I/O.
    #[test]
    fn test_format_command_in_place_no_file_io() {
        let cmd = FormatCommand::new(
            String::from("let n = 1"),
            true, // in_place flag
        );
        let result = cmd.execute().expect("in_place command should succeed");
        assert!(
            !result.is_empty(),
            "in_place command should still return formatted output"
        );
    }
}
