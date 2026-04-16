# Learnings

## 2026-04-16 Task: Setup

### Test Patterns (CRITICAL)
- `Formatter::with_defaults().format_source(source).unwrap()` - format source
- `Formatter::with_defaults().format_source(source).expect("msg")` - with message
- Reparse pattern:
  ```rust
  let lexer = crate::lexer::Lexer::new(&formatted);
  let (tokens, lex_errors) = lexer.tokenize();
  assert!(lex_errors.errors.is_empty(), "...: {lex_errors:?}");
  let parser = crate::parser::Parser::new(tokens);
  let (_program, parse_errors) = parser.parse();
  assert!(parse_errors.errors.is_empty(), "...: {parse_errors:?}");
  ```

### Current Formatter Output (WRONG - uses braces)
- `loop {` instead of `loop =>`
- `for x in items {` instead of `for x in items:`
- `while cond {` instead of `while cond:`
- `if cond {` instead of `if cond:`
- Function bodies: `entry main = f(): void => {` (KEEP THIS - don't change)

### Existing Tests That Need Updating (T7)
- `test_formatter_loop_body_leading_comment` (line 1059) - expects `loop {`
- `test_formatter_for_body_leading_comment` (line 1082) - expects `for x in items {`
- `test_formatter_while_body_leading_comment` (line 1105) - expects `while cond {`
- `test_formatter_if_body_leading_comment` (line 1128) - expects `if cond {`
- `test_formatter_loop_body_leading_doc_comment` (line 1173) - expects `loop {`

### Architecture Decision (CRITICAL)
- Do NOT change `Stmt::Block` globally - it's used for function bodies too
- Instead: control flow (If/While/For/Loop) emit their own `:` or `=>` header
  then call a helper `print_block_body_indented` for the body contents
- Function bodies keep `=> { ... }` pattern

### Language Spec Files Available
- language-spec/fib_iterative.op - has `if n is 0:`, `while i <= n:`
- language-spec/fib_recursive.op - has `if n is 0:`
- language-spec/array_helpers.op - has `for x in xs:`
- language-spec/simple_quiz.op - has `loop =>`
- language-spec/partition.op - has `for x in xs:` inline

### Test File Structure
- All tests in `src/formatter/tests.rs` inside `mod formatter_tests { ... }`
- Uses `use crate::formatter::printer::Formatter;`
- Tests end at line 1194 (closing `}` of mod)
