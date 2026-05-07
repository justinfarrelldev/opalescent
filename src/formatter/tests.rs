//! Tests for the Opalescent code formatter.
//!
//! All tests use inline strings — no file I/O is performed.

#[cfg(test)]
mod formatter_tests {
    use crate::formatter::command::FormatCommand;
    use crate::formatter::config::FormatterConfig;
    use crate::formatter::naming::{
        NamingStyle, NamingViolation, check_program, is_pascal_case, is_snake_case,
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
            output.contains("\n    while true:\n"),
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

    #[test]
    fn test_naming_guard_omitted_success_binding_has_no_violation() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    guard compute_result() else err =>\n",
            "        return void\n",
            "    return void\n"
        );
        let lexer = crate::lexer::Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = crate::parser::Parser::new(tokens);
        let (prog, _) = parser.parse();
        assert!(prog.is_some(), "source should parse successfully: {source}");
        if let Some(program) = prog {
            let violations = check_program(&program);
            assert!(
                violations
                    .iter()
                    .all(|v| v.location != "guard success binding"),
                "shorthand guard should not report a success-binding violation, got: {violations:?}"
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

    // VERIFIED: use_tabs=true produces tabs
    #[test]
    fn test_use_tabs_produces_tab_indentation() {
        let input = "entry main = f(args: string[]): void =>\n    let x = 1\n    return void\n";
        let config = FormatterConfig::new(4, 100, true);
        let output = FormatCommand::new(input.to_owned())
            .execute_with_config(config)
            .expect("format should succeed");

        assert!(
            output.lines().any(|l| l.starts_with('\t')),
            "expected at least one indented line to start with a tab, got: {output:?}"
        );
        assert!(
            output
                .lines()
                .filter(|l| !l.trim().is_empty())
                .all(|l| !l.starts_with("    ")),
            "expected no non-empty output lines to start with four spaces, got: {output:?}"
        );
    }

    // VERIFIED: tab-indented input is converted to spaces with default config
    #[test]
    fn test_tab_input_converted_to_spaces_by_default() {
        let input = "entry main = f(args: string[]): void =>\n\tlet x = 1\n\treturn void\n";
        let config = FormatterConfig::default();
        let output = FormatCommand::new(input.to_owned())
            .execute_with_config(config)
            .expect("format should succeed");

        assert!(
            !output.contains('\t'),
            "output should not contain tabs, got: {output:?}"
        );
        assert!(
            output
                .lines()
                .any(|l| l.starts_with("    ") && !l.trim().is_empty()),
            "expected at least one non-empty line with 4-space indentation, got: {output:?}"
        );
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
        let cmd = FormatCommand::new(String::from("entry main = f(): void => return void"));
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
        let cmd = FormatCommand::new(String::from(source));
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
        let cmd = FormatCommand::new(String::from("let n = 1"));
        let result = cmd.execute().expect("in_place command should succeed");
        assert!(
            !result.is_empty(),
            "in_place command should still return formatted output"
        );
    }

    // ─── Tab/Space Conversion Matrix Tests ──────────────────────────────────────

    /// Tab-indented input formatted with default config produces 4-space output.
    #[test]
    fn test_tabs_to_spaces_default_config() {
        let source = "entry main = f(args: string[]): void =>\n\tlet x = 1\n\treturn void\n";
        let config = FormatterConfig::default();
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            !output.contains('\t'),
            "tab-indented input with default config must produce no tabs, got: {output:?}"
        );
        assert!(
            output
                .lines()
                .any(|l| l.starts_with("    ") && !l.trim().is_empty()),
            "expected at least one 4-space-indented line, got: {output:?}"
        );
    }

    /// Space-indented input formatted with `use_tabs=true` produces tab output.
    #[test]
    fn test_spaces_to_tabs() {
        let source = "entry main = f(args: string[]): void =>\n    let x = 1\n    return void\n";
        let config = FormatterConfig::new(4, 100, true);
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output.lines().any(|l| l.starts_with('\t')),
            "space-indented input with use_tabs=true must produce tab output, got: {output:?}"
        );
    }

    /// Mixed tab+space input formatted with default config produces 4-space output (no tabs).
    #[test]
    fn test_mixed_to_spaces() {
        let source = "entry main = f(args: string[]): void =>\n\tlet x = 1\n    return void\n";
        let config = FormatterConfig::default();
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            !output.contains('\t'),
            "mixed-indent input with default config must produce no tabs, got: {output:?}"
        );
    }

    /// Mixed tab+space input formatted with `use_tabs=true` produces tab output.
    #[test]
    fn test_mixed_to_tabs() {
        let source = "entry main = f(args: string[]): void =>\n\tlet x = 1\n    return void\n";
        let config = FormatterConfig::new(4, 100, true);
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output.lines().any(|l| l.starts_with('\t')),
            "mixed-indent input with use_tabs=true must produce tab output, got: {output:?}"
        );
    }

    /// Formatting already-4-space-indented code with default config is idempotent.
    #[test]
    fn test_idempotent_spaces() {
        let source = "entry main = f(args: string[]): void =>\n    let x = 1\n    return void\n";
        let config = FormatterConfig::default();
        let once = FormatCommand::new(source.to_owned())
            .execute_with_config(config.clone())
            .unwrap();
        let twice = FormatCommand::new(once.clone())
            .execute_with_config(config)
            .unwrap();
        assert_eq!(
            once, twice,
            "formatting 4-space input twice with default config must be idempotent"
        );
    }

    /// Formatting already-tab-indented code with `use_tabs=true` is idempotent.
    #[test]
    fn test_idempotent_tabs() {
        let source = "entry main = f(args: string[]): void =>\n\tlet x = 1\n\treturn void\n";
        let config = FormatterConfig::new(4, 100, true);
        let once = FormatCommand::new(source.to_owned())
            .execute_with_config(config.clone())
            .unwrap();
        let twice = FormatCommand::new(once.clone())
            .execute_with_config(config)
            .unwrap();
        assert_eq!(
            once, twice,
            "formatting tab-indented input twice with use_tabs=true must be idempotent"
        );
    }

    /// Custom `indent_size=2` produces 2-space-indented output.
    #[test]
    fn test_custom_indent_size_2() {
        let source = "entry main = f(args: string[]): void =>\n    let x = 1\n    return void\n";
        let config = FormatterConfig::new(2, 100, false);
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output
                .lines()
                .any(|l| l.starts_with("  ") && !l.trim().is_empty()),
            "indent_size=2 must produce at least one 2-space-indented line, got: {output:?}"
        );
        assert!(
            output
                .lines()
                .filter(|l| !l.trim().is_empty())
                .all(|l| !l.starts_with("    ")),
            "indent_size=2 must not produce 4-space-indented lines, got: {output:?}"
        );
    }

    /// Custom `indent_size=8` produces 8-space-indented output.
    #[test]
    fn test_custom_indent_size_8() {
        let source = "entry main = f(args: string[]): void =>\n    let x = 1\n    return void\n";
        let config = FormatterConfig::new(8, 100, false);
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output
                .lines()
                .any(|l| l.starts_with("        ") && !l.trim().is_empty()),
            "indent_size=8 must produce at least one 8-space-indented line, got: {output:?}"
        );
    }

    /// Multi-level nesting (if inside if) with `use_tabs=true` produces 2-tab depth-2 indentation.
    #[test]
    fn test_nested_indentation_tabs() {
        let source = concat!(
            "entry main = f(args: string[]): void => {\n",
            "    let x = 1\n",
            "    if x is 1 {\n",
            "        if x is 1 {\n",
            "            return void\n",
            "        }\n",
            "    }\n",
            "    return void\n",
            "}"
        );
        let config = FormatterConfig::new(4, 100, true);
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output.lines().any(|l| l.starts_with("\t\t")),
            "nested if with use_tabs=true must have lines starting with 2 tabs, got: {output:?}"
        );
    }

    /// Multi-level nesting (if inside if) with default config produces 8-space depth-2 indentation.
    #[test]
    fn test_nested_indentation_spaces() {
        let source = concat!(
            "entry main = f(args: string[]): void => {\n",
            "    let x = 1\n",
            "    if x is 1 {\n",
            "        if x is 1 {\n",
            "            return void\n",
            "        }\n",
            "    }\n",
            "    return void\n",
            "}"
        );
        let config = FormatterConfig::default();
        let output = FormatCommand::new(source.to_owned())
            .execute_with_config(config)
            .unwrap();
        assert!(
            output
                .lines()
                .any(|l| l.starts_with("        ") && !l.trim().is_empty()),
            "nested if with default config must have lines with 8-space indent, got: {output:?}"
        );
    }

    // ─── Comment Preservation Tests ─────────────────────────────────────────────

    /// Single-line `#` comment between two declarations must appear in output.
    #[test]
    fn test_formatter_preserves_single_line_comments_between_declarations() {
        let source = "# Section header\nentry main = f(): void =>\n    return void\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        assert!(
            result.contains("# Section header"),
            "single-line comment between declarations should be preserved, got: {result}"
        );
    }

    /// `#` comments between statements inside a function body must appear in output.
    #[test]
    fn test_formatter_preserves_body_comments() {
        let source = "entry main = f(): void =>\n    # Step 1: setup\n    let x = 42\n    # Step 2: finish\n    return void\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        assert!(
            result.contains("# Step 1: setup"),
            "first body comment should be preserved, got: {result}"
        );
        assert!(
            result.contains("# Step 2: finish"),
            "second body comment should be preserved, got: {result}"
        );
    }

    /// `## ... ##` doc comment before a function must appear in output.
    #[test]
    fn test_formatter_preserves_doc_comments() {
        let source = "## Description: Adds two integers ##\nlet add = f(a: int32, b: int32): int32 =>\n    return a\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        assert!(
            result.contains("##"),
            "doc comment delimiters should be preserved, got: {result}"
        );
        assert!(
            result.contains("Description: Adds two integers"),
            "doc comment content should be preserved, got: {result}"
        );
    }

    /// A `#` comment at the very start of the file must appear in output.
    #[test]
    fn test_formatter_preserves_file_header_comment() {
        let source = "# File header comment\nentry main = f(): void =>\n    return void\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        assert!(
            result.contains("# File header comment"),
            "file header comment should be preserved, got: {result}"
        );
    }

    /// Regression: first body comment was dropped when a doc comment preceded the function.
    #[test]
    fn test_formatter_preserves_first_body_comment_after_doc_comment() {
        let source = concat!(
            "## Description: Adds two integers ##\n",
            "let add = f(a: int32, b: int32): int32 =>\n",
            "    # Step 1: Add the values\n",
            "    let result = a + b\n",
            "    # Step 2: Return\n",
            "    return result\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        assert!(
            result.contains("# Step 1: Add the values"),
            "first body comment after doc comment should be preserved, got: {result}"
        );
        assert!(
            result.contains("# Step 2: Return"),
            "second body comment should be preserved, got: {result}"
        );
    }

    /// Formatting a file with comments twice should produce identical output.
    #[test]
    fn test_formatter_comment_idempotency() {
        let source = "# Header\nentry main = f(): void =>\n    # Body comment\n    return void\n";
        let fmt = Formatter::with_defaults();
        let first_pass = fmt.format_source(source).unwrap();
        let second_pass = fmt.format_source(&first_pass).unwrap();
        assert_eq!(
            first_pass, second_pass,
            "formatting with comments should be idempotent"
        );
    }

    #[test]
    fn test_formatter_string_interpolation_escaped_quote() {
        // String interpolation containing a single quote must be escaped in output
        let source = "entry main = f(): void =>\n    print('Let\\'s go {name}')\n    return void\n";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(
            result.is_ok(),
            "formatter should succeed on string with escaped quote"
        );
        let output = result.unwrap();
        assert!(
            output.contains("\\'"),
            "formatted output should contain escaped quote"
        );
        assert!(
            !output.contains("Let's go"),
            "formatted output should not contain unescaped quote in string"
        );
    }

    /// Multi-line `## ... ##` non-doc comment blocks should be preserved in output.
    #[test]
    fn test_formatter_preserves_multiline_non_doc_comment() {
        let source = concat!(
            "##\n",
            "  This is a multi-line comment block\n",
            "##\n",
            "entry main = f(): void =>\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("formatter should handle multi-line non-doc comments without panicking");
        assert!(
            result.contains("##"),
            "multi-line comment delimiters should be preserved in output, got: {result}"
        );
    }

    /// Formatted output containing comments should lex and parse without errors.
    #[test]
    fn test_formatter_comments_reparse_clean() {
        let source = concat!(
            "# File header\n",
            "entry main = f(): void =>\n",
            "    # Body comment\n",
            "    return void\n",
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt
            .format_source(source)
            .expect("formatter should succeed on source with comments");

        let lexer = crate::lexer::Lexer::new(&formatted);
        let (tokens, lex_errors) = lexer.tokenize();
        assert!(
            lex_errors.errors.is_empty(),
            "formatted output with comments should lex without errors: {lex_errors:?}"
        );

        let parser = crate::parser::Parser::new(tokens);
        let (_program, parse_errors) = parser.parse();
        assert!(
            parse_errors.errors.is_empty(),
            "formatted output with comments should parse without errors: {parse_errors:?}"
        );
    }

    #[test]
    fn test_formatter_loop_body_leading_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    loop =>\n",
            "        # first\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    loop =>\n",
            "        # first\n",
            "        break\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_formatter_for_body_leading_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    for x in items:\n",
            "        # first\n",
            "        continue\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    for x in items:\n",
            "        # first\n",
            "        continue\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_formatter_while_body_leading_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    while cond:\n",
            "        # first\n",
            "        continue\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    while cond:\n",
            "        # first\n",
            "        continue\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_formatter_if_body_leading_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    if cond:\n",
            "        # first\n",
            "        return void\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    if cond:\n",
            "        # first\n",
            "        return void\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_formatter_guard_body_leading_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    guard expr into n else e =>\n",
            "        # first\n",
            "        return void\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    guard expr into n else e =>\n",
            "        # first\n",
            "        return void\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_formatter_guard_shorthand_roundtrip() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    guard compute_result() else err =>\n",
            "        return void\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let first_pass = fmt
            .format_source(source)
            .expect("guard shorthand should format");
        assert!(
            first_pass.contains("guard compute_result() else err =>"),
            "shorthand guard should omit `into`, got: {first_pass}"
        );
        assert!(
            !first_pass.contains("into"),
            "shorthand guard should not introduce `into`, got: {first_pass}"
        );
        let second_pass = fmt
            .format_source(&first_pass)
            .expect("formatted shorthand guard should reformat");
        assert_eq!(
            first_pass, second_pass,
            "guard shorthand formatting must be idempotent"
        );
    }

    #[test]
    fn test_formatter_guard_into_underscore_preserved() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    guard compute_result() into _ else err =>\n",
            "        return void\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("explicit underscore guard should format");
        assert!(
            result.contains("guard compute_result() into _ else err =>"),
            "explicit underscore binding should remain explicit, got: {result}"
        );
        assert!(
            !result.contains("guard compute_result() else err =>"),
            "explicit underscore binding must not be rewritten to shorthand, got: {result}"
        );
    }

    #[test]
    fn test_formatter_loop_body_leading_doc_comment() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    loop =>\n",
            "        ## doc ##\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source).unwrap();
        let expected = concat!(
            "entry main = f(): void => {\n",
            "    loop =>\n",
            "        ## doc ##\n",
            "        break\n",
            "    return void\n",
            "}\n"
        );
        assert_eq!(result, expected);
    }

    // ─── Language Spec Compliance Tests (Regression Prevention) ────────────────
    //
    // These tests enforce that the formatter outputs syntactically valid
    // Opalescent code per the language specification. They use colon-block
    // syntax for control flow (if/while/for) and arrow syntax for loop.
    //
    // IMPORTANT: These tests are expected to FAIL before the formatter is fixed
    // (TDD RED phase). After fixing printer.rs, they should all pass (GREEN).

    // REGRESSION TEST: Ensures if-statements use colon-block syntax per the Opalescent
    // language spec. See language-spec/fib_iterative.op for canonical examples.
    #[test]
    fn test_spec_compliance_if_no_braces() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    if true:\n",
            "        return void\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            formatted.contains("if true:"),
            "if-statement should use colon syntax per language spec, got: {formatted}"
        );
        assert!(
            !formatted.contains("if true {"),
            "if-statement must NOT use brace syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures while-loops use colon-block syntax per the Opalescent
    // language spec. See language-spec/fib_iterative.op for canonical examples.
    #[test]
    fn test_spec_compliance_while_no_braces() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    while true:\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            formatted.contains("while true:"),
            "while-loop should use colon syntax per language spec, got: {formatted}"
        );
        assert!(
            !formatted.contains("while true {"),
            "while-loop must NOT use brace syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures for-loops use colon-block syntax per the Opalescent
    // language spec. See language-spec/array_helpers.op for canonical examples.
    #[test]
    fn test_spec_compliance_for_no_braces() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    for x in items:\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            formatted.contains("for x in items:"),
            "for-loop should use colon syntax per language spec, got: {formatted}"
        );
        assert!(
            !formatted.contains("for x in items {"),
            "for-loop must NOT use brace syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures loop expressions use arrow syntax per the Opalescent
    // language spec. See language-spec/simple_quiz.op for canonical examples.
    #[test]
    fn test_spec_compliance_loop_no_braces() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    loop =>\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            formatted.contains("loop =>"),
            "loop should use arrow syntax per language spec, got: {formatted}"
        );
        assert!(
            !formatted.contains("loop {"),
            "loop must NOT use brace syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures control flow statements do not contain semicolons
    // per the Opalescent language spec (statements use newlines for termination).
    #[test]
    fn test_spec_compliance_no_semicolons_in_control_flow() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    if true:\n",
            "        return void\n",
            "    while true:\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            !formatted.contains(';'),
            "formatted output must not contain semicolons per language spec, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures function bodies use arrow-brace syntax (=> {)
    // while control flow uses colon-block syntax. Function bodies KEEP braces.
    #[test]
    fn test_spec_compliance_function_body_arrow_syntax() {
        let source = "entry main = f(): void =>\n    return void\n";
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            formatted.contains("=> {"),
            "function bodies should use arrow-brace syntax per language spec, got: {formatted}"
        );
        assert!(
            !formatted.contains("=> :"),
            "function bodies must NOT use colon syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures nested control flow statements all use colon-block
    // syntax (no braces) per the Opalescent language spec.
    #[test]
    fn test_spec_compliance_nested_control_flow_no_braces() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    while true:\n",
            "        if true:\n",
            "            break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");
        assert!(
            !formatted.contains("while true {"),
            "nested while should use colon syntax, got: {formatted}"
        );
        assert!(
            !formatted.contains("if true {"),
            "nested if should use colon syntax, got: {formatted}"
        );
        assert!(
            formatted.contains("while true:"),
            "while should use colon syntax, got: {formatted}"
        );
        assert!(
            formatted.contains("if true:"),
            "if should use colon syntax, got: {formatted}"
        );
    }

    // REGRESSION TEST: Ensures formatted output with control flow statements
    // can be lexed and parsed cleanly (no syntax errors). This validates that
    // the formatter produces valid Opalescent code per the language spec.
    #[test]
    fn test_spec_compliance_formatted_output_parses_cleanly() {
        let source = concat!(
            "entry main = f(): void =>\n",
            "    if true:\n",
            "        return void\n",
            "    while true:\n",
            "        break\n",
            "    return void\n"
        );
        let fmt = Formatter::with_defaults();
        let formatted = fmt.format_source(source).expect("should format");

        let lexer = crate::lexer::Lexer::new(&formatted);
        let (tokens, lex_errors) = lexer.tokenize();
        assert!(
            lex_errors.errors.is_empty(),
            "formatted output should lex cleanly, got errors: {lex_errors:?}"
        );

        let parser = crate::parser::Parser::new(tokens);
        let (_program, parse_errors) = parser.parse();
        assert!(
            parse_errors.errors.is_empty(),
            "formatted output should parse cleanly, got errors: {parse_errors:?}"
        );
    }

    #[test]
    fn test_spec_files_format_and_reparse() {
        // REGRESSION TEST: Ensures formatted output of language spec files is valid
        // Opalescent syntax per language spec. Formatted code must lex and parse without errors.
        let spec_files = [
            include_str!("../../language-spec/fib_iterative.op"),
            include_str!("../../language-spec/fib_recursive.op"),
            include_str!("../../language-spec/array_helpers.op"),
        ];
        for source in spec_files {
            let formatted = Formatter::with_defaults()
                .format_source(source)
                .expect("formatter should succeed on spec file");
            let lexer = crate::lexer::Lexer::new(&formatted);
            let (tokens, lex_errors) = lexer.tokenize();
            assert!(
                lex_errors.errors.is_empty(),
                "formatted spec file output should lex without errors: {lex_errors:?}"
            );
            let parser = crate::parser::Parser::new(tokens);
            let (_program, parse_errors) = parser.parse();
            assert!(
                parse_errors.errors.is_empty(),
                "formatted spec file output should parse without errors: {parse_errors:?}"
            );
        }
    }

    /// A `new Type:` constructor must be emitted with the keyword, callee,
    /// colon, and each field on its own line indented one level past the
    /// statement that owns the expression.
    #[test]
    fn test_formatter_emits_new_constructor_indented_block() {
        let source = "\
entry main = f(): void =>
    let alice = new Person:
        name: 'Alice'
        age: 30
    return void
";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("constructor should format");
        assert!(
            result.contains("let alice = new Person:\n"),
            "formatter should emit `let alice = new Person:` header, got: {result}"
        );
        assert!(
            result.contains("        name: 'Alice'\n"),
            "first field should be indented one level past the `let`, got: {result}"
        );
        assert!(
            result.contains("        age: 30\n"),
            "second field should share the same indent as the first, got: {result}"
        );
        assert!(
            !result.contains('{') || !result.contains("Person {"),
            "formatter must not emit the legacy brace form, got: {result}"
        );
    }

    /// Formatting a `new` constructor source must be idempotent and the
    /// formatted output must round-trip through the parser without errors.
    #[test]
    fn test_formatter_new_constructor_round_trips() {
        let source = "\
entry main = f(): void =>
    let alice = new Person:
        name: 'Alice'
        age: 30
    return void
";
        let fmt = Formatter::with_defaults();
        let first_pass = fmt.format_source(source).expect("first pass should format");
        let second_pass = fmt
            .format_source(&first_pass)
            .expect("second pass should format");
        assert_eq!(
            first_pass, second_pass,
            "new-constructor formatting must be idempotent"
        );

        let lexer = crate::lexer::Lexer::new(&first_pass);
        let (tokens, lex_errors) = lexer.tokenize();
        assert!(
            lex_errors.errors.is_empty(),
            "formatted output must lex cleanly, got: {lex_errors:?}"
        );
        let parser = crate::parser::Parser::new(tokens);
        let (_program, parse_errors) = parser.parse();
        assert!(
            parse_errors.errors.is_empty(),
            "formatted output must parse cleanly, got: {parse_errors:?}"
        );
    }

    /// Sum-type variant constructors (e.g. `new Message.Text:`) must be
    /// emitted with the full member callee preserved on the header line.
    #[test]
    fn test_formatter_emits_new_variant_constructor() {
        let source = "\
entry main = f(): void =>
    let m = new Message.Text:
        sender: 'alice'
        body: 'hi'
    return void
";
        let fmt = Formatter::with_defaults();
        let result = fmt
            .format_source(source)
            .expect("variant ctor should format");
        assert!(
            result.contains("new Message.Text:\n"),
            "formatter should preserve `new Message.Text:` callee, got: {result}"
        );
    }

    /// Misaligned field lines inside a `new Type:` block must fail to parse
    /// entirely (unexpected dedent/indent), guaranteeing that `fmt --check`
    /// — which runs the formatter and diffs — bubbles the error up fast.
    #[test]
    fn test_formatter_rejects_misaligned_new_constructor_fields() {
        // The first field is indented 8 spaces, the second only 4 — parser
        // sees this as a premature Dedent and cannot produce a valid program.
        let source = "\
entry main = f(): void =>
    let alice = new Person:
        name: 'Alice'
    age: 30
    return void
";
        let fmt = Formatter::with_defaults();
        let result = fmt.format_source(source);
        assert!(
            result.is_err(),
            "formatter must reject misaligned `new Type:` field block so `fmt --check` fails fast, got: {result:?}"
        );
    }
}
