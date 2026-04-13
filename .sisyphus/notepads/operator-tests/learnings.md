# Learnings — operator-tests plan

## [2026-04-13] Operator Audit Findings

### Key Facts
- All operators are correctly implemented across lexer→parser→AST pipeline
- `<=` and `>=` correctly tokenized as 2-char greedy tokens in `src/lexer.rs:260-276`
- Precedence table has 14 levels: None, Assignment, Or, Xor, And, BitOr, BitXor, BitAnd, Equality, Comparison, Shift, Term, Factor, Power, Unary, Call, Primary
- Power (`^`) is the ONLY right-associative binary operator (`src/parser/expressions.rs:762-768`)
- Bitwise ops are keyword-based: `band`, `bor`, `bxor`, `bnot`, `bshl`, `bshr`, `bushr`
- `is not` is a two-keyword token (`TokenType::IsNot`)
- `BinaryOp` enum has dead variants `Equal` and `NotEqual` — never constructed by parser — FLAG ONLY, do NOT fix
- `<` has special-case handling to try parsing as generic call (`foo<T>(x)`) before falling back to comparison
- `Assignment` precedence is NOT a parseable expression operator — `TokenType::Assign` is not in `Precedence::from_token`
- 12 adjacent precedence pairs can be tested (not 13)
- Existing tests are minimal: `+`, `<`, `and`, `-` (unary), `not`, and `1 + 2 * 3`
- Baseline parser test count: 133 tests passing

### Test Helpers
- `parse_expression_from_string(input: &str)` at `src/parser/tests.rs:26-30`
- Pattern for binary tests: `src/parser/tests.rs:746-774` (`test_binary_expressions`)
- Pattern for precedence tests: `src/parser/tests.rs:809-837` (`test_operator_precedence`)
- Pattern for parenthesized: `src/parser/tests.rs:797-801`

### Test Naming Convention
- `test_binary_op_*`
- `test_unary_op_*`
- `test_precedence_*`
- `test_associativity_*`
- `test_edge_case_*`
