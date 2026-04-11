extern crate alloc;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;

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

    program_opt.map_or_else(
        || Program {
            declarations: Vec::new(),
            span: Span::single(Position::start()),
            id: crate::ast::NodeId(0),
        },
        |program| program,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_length_returns_int64() {
        const SOURCE: &str = "
entry main = f(values: int64[]): int64 =>
    return values.length()
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "array length intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_array_push_type_checks() {
        const SOURCE: &str = "
entry main = f(values: int64[]): unit => {
    let mutable arr: int64[] = values
    arr.push(4)
    return void
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(result.is_ok(), "array push should type check: {result:?}");
    }

    #[test]
    fn test_array_pop_returns_element_type() {
        const SOURCE: &str = "
entry main = f(values: int64[]): int64 => {
    let mutable arr: int64[] = values
    return arr.pop()
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "array pop should return the array element type: {result:?}",
        );
    }

    #[test]
    fn test_array_map_returns_mapped_array_type() {
        const SOURCE: &str = "
entry main = f(values: int64[]): string[] => {
    let arr: int64[] = values
    return arr.map(f(n: int64): string => 'x')
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "array map should infer mapped element type: {result:?}",
        );
    }

    #[test]
    fn test_array_filter_returns_same_array_type() {
        const SOURCE: &str = "
entry main = f(values: int64[]): int64[] => {
    let arr: int64[] = values
    return arr.filter(f(n: int64): boolean => true)
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "array filter should preserve element type: {result:?}",
        );
    }

    #[test]
    fn test_array_reduce_returns_accumulator_type() {
        const SOURCE: &str = "
entry main = f(values: int64[]): int64 => {
    return values.reduce(0, f(acc: int64, n: int64): int64 => acc + n)
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "array reduce should infer accumulator return type: {result:?}",
        );
    }

    #[test]
    fn test_array_zip_type_checks() {
        const SOURCE: &str = "
entry main = f(values: int64[], labels: string[]): unit => {
    values.zip(labels)
    return void
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(result.is_ok(), "array zip should type check: {result:?}");
    }

    #[test]
    fn test_string_length_returns_int64() {
        const SOURCE: &str = "
entry main = f(): int64 =>
    return 'hello'.length()
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string length intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_to_upper_returns_string() {
        const SOURCE: &str = "
entry main = f(): string =>
    return 'hello'.to_upper()
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string to_upper intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_contains_returns_boolean() {
        const SOURCE: &str = "
entry main = f(): boolean =>
    return 'hello'.contains('ell')
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string contains intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_split_returns_string_array() {
        const SOURCE: &str = "
entry main = f(text: string): string[] =>
    return text.split(',')
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string split intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_starts_and_ends_with_return_boolean() {
        const SOURCE: &str = "
entry main = f(text: string): boolean => {
    let starts: boolean = text.starts_with('a')
    let ends: boolean = text.ends_with('z')
    return starts or ends
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string starts_with/ends_with intrinsics should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_slice_and_to_lower_return_string() {
        const SOURCE: &str = "
entry main = f(text: string): string => {
    let sliced: string = text.slice(0, 1)
    return sliced.to_lower()
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string slice/to_lower intrinsics should type check: {result:?}",
        );
    }

    #[test]
    fn test_string_join_accepts_string_array() {
        const SOURCE: &str = "
entry main = f(parts: string[]): string =>
    return ','.join(parts)
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "string join intrinsic should type check: {result:?}",
        );
    }

    #[test]
    fn test_for_loop_over_string_iterable_type_checks() {
        const SOURCE: &str = "
entry main = f(text: string): unit => {
    for ch in text {
        print(ch)
    }
    return void
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "for loop should accept iterable string values: {result:?}",
        );
    }

    #[test]
    fn test_array_method_chaining_infers_types() {
        const SOURCE: &str = "
entry main = f(values: int64[]): int64[] => {
    let arr: int64[] = values
    return arr.map(f(n: int64): int64 => n + 1).filter(f(n: int64): boolean => n > 1)
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "collection method chaining should infer intermediate array types: {result:?}",
        );
    }
}
