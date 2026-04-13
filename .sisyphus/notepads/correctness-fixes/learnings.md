# Learnings — correctness-fixes

## [2026-04-13] Initial Context

### Key Architecture Facts
- Rust 2024 edition, LLVM 14, inkwell for codegen
- All 10 explicitly-sized numeric types supported lexer→codegen
- NO shorthand types (int, float, uint) — design constraint
- C runtime embedded in binary at compile time
- Integration tests gated behind `--features integration` flag
- `cargo test` (unit) vs `cargo test --features integration` (e2e)

### Known Patterns
- Token enum: `src/token.rs`
- Lexer: `src/lexer.rs` + `src/lexer/` modules
- Parser: `src/parser/` — expressions.rs, statements.rs, precedence.rs
- Type system: `src/type_system/` — checker.rs is main entry, checker/ submodule
- Codegen: `src/codegen/` — expressions.rs, expressions_string.rs, functions.rs, values.rs
- C Runtime: `runtime/opal_runtime.c` (embedded at compile time)
- Test integration: `tests/integration_e2e.rs`, `src/type_system/test_integration*.rs`

### Colon-Block Syntax (VERIFIED WORKS)
- Parser supports colon-block via `parse_indent_block()` 
- Stale `#[ignore]` markers in type system tests are wrong
- Real e2e tests (fib-recursive, fib-iterative) compile and run successfully

### User Decisions
- Default integer literal: int64 (keep current behavior, update spec)
- C runtime prefix: DROP "opal_" from all functions
- C runtime types: Add ALL size-specific variants (int8-int64, uint8-uint64)
- All phases (4+5) should be complete — fix all stubs
- TDD: RED-GREEN-REFACTOR, never skip RED phase

## [2026-04-13] Task 2 — UTF-8 lexer byte offsets

### Findings
- `position.offset` is byte-based and must always advance by `ch.len_utf8()` for correctness with multibyte Unicode.
- Incrementing offset by character count (`+1`) caused byte/char drift and broke downstream lexing behavior (e.g., number scanning after Unicode identifiers).
- `lexeme()` slicing via `input[start_offset..end_offset]` remains correct once offsets are truly byte-accurate.

### Successful test pattern
- Use a Unicode identifier test (`"let π = 42"`) and assert token span offsets in bytes (`=` starts at byte 7, ends at 8).
- TDD RED signal can surface as lexer errors (not only wrong span assertions) when offsets drift.

## [2026-04-13] Task 3 — Euclidean division/modulo operators

### Findings
- Adding operators to the token and AST layer is systematic and replicates for each new operator:
  1. Add to TokenType enum with doc comment
  2. Add Display impl matching existing pattern
  3. Add keyword mapping in lexer (`keywords.insert("name", TokenType::Variant)`)
  4. Add to BinaryOp enum with doc comment
  5. Add Display impl for BinaryOp
  6. Add TryFrom conversion
  7. Add stubs in all match statements across the codebase (codegen, formatter, type system)

- Operators added follow existing patterns exactly — no special handling needed at this layer
- Type checking constraints properly inherited from similar operators (Modulo for div_euclid/mod_euclid)
  * Both require integer operands only
  * Both require non-zero divisor validation
  * Both return operand type unchanged
- Codegen layer stubs are marked as Task 22 work (not yet implemented)
- Display impls use friendly names: "div_euclid" → "operator 'div_euclid'"

### Test Pattern
- NEW: Start with RED tests before adding any implementation code
- Tests verify token recognition: `test_div_euclid_keyword` checks `TokenType::DivEuclid` match
- Expression context tests verify keyword parsing in full expressions
- All 3 new tests pass; baseline 789 → 792 tests (no regressions)

### Arch Insight
- Lexer layer: keywords.insert is the KEY bottleneck for adding new keyword operators
- AST layer: BinaryOp must have complete TryFrom coverage (compiler enforces exhaustive matching)
- Type system: match statements across codegen/formatter/checker all need stubs
- Pattern: Adding an operator touches at least 7 files (minimal cross-cutting concern)

## [2026-04-13] Task 4 — As/Cast Token Duplication Resolution

### Problem Identified
- Both `TokenType::As` and `TokenType::Cast` existed in token.rs
- Lexer was mapping "as" keyword → TokenType::As
- AST already has Expr::Cast for type casting
- Duplication: same keyword mapped to two different tokens

### Root Cause
- Import aliasing (`import foo as bar`) was using TokenType::As
- Type casting (`x as int32`) should use TokenType::Cast per AST design
- Never reconciled which token should be canonical

### Solution (TDD Protocol)
1. **RED**: Added `test_cast_token_as_keyword()` verifying "x as int32" → TokenType::Cast (failed, confirmed TokenType::As was used)
2. **GREEN**: Updated lexer line 155: `keywords.insert("as", TokenType::Cast)`
3. **REFACTOR**: Updated parser declaration line 697 to use `TokenType::Cast` for import aliases
4. **REMOVE**: Deleted TokenType::As enum variant entirely from token.rs
5. **REMOVE**: Deleted Display impl case for TokenType::As from token.rs

### Key Insight
- Single token `TokenType::Cast` now handles BOTH contexts: imports and type casts
- Parser's context-sensitive matching logic properly distinguishes the uses
- No conflict: import alias checking `self.check(&TokenType::Cast)` in a specific position is unambiguous

### Verification
- Test count: 792 → 793 (+1 new test, no regressions)
- grep -rn "TokenType::As\b" returns zero matches (fully removed)
- All existing tests pass without modification
- New test covers cast token semantics

### Files Changed
- src/lexer.rs (line 155): keyword mapping "as" → TokenType::Cast
- src/parser/declarations.rs (line 697): Check TokenType::Cast for import alias
- src/token.rs (removed line 172): Delete TokenType::As variant
- src/token.rs (removed line 369): Delete Display case for TokenType::As
- src/lexer/tests.rs (new test): test_cast_token_as_keyword

## [2026-04-13] Task 5 — IsNot Token Inconsistency Resolution

### Problem Identified
- `TokenType::IsNot` existed in token.rs enum (designed for identity comparison operator)
- Lexer never emitted this token; instead emitted two separate tokens: `Is` + `Not`
- Parser had theoretical support for handling `TokenType::IsNot` but received `Is Not` as two tokens instead
- Inconsistency: token design intended single compound token, but lexer bypassed it

### Root Cause
- Initial token design included `IsNot` for spec operators ("is, is not")
- Lexer implementation never implemented lookahead logic to detect `is not` compound
- Parser tests expected `BinaryOp::IsNot` but received two tokens instead
- Unused token variant left dead code in codebase

### Design Decision: Lexer Lookahead (Approach 2)
**Rationale:**
1. Token design originally intended `TokenType::IsNot` as single token
2. Single token = cleaner parser logic (no ambiguity between `is` alone vs `is not`)
3. Spec treats "is, is not" as operators (parallel to "and, or")
4. Lexer is natural place for compound keyword detection (other examples: "elif", "elif")

**Alternative considered (Approach 1):** Remove `TokenType::IsNot`, have parser handle two tokens. Rejected because:
- Requires parser to track state (previous token was `Is`)
- Ambiguous: `x is True` vs `x is not True` have different structure
- Less maintainable long-term

### Solution (TDD Protocol)
1. **RED**: Added `test_is_not_operator_consistency()` in src/lexer/tests.rs (lines 407-423)
   - Test input: `"x is not None"`
   - Expected: 5 tokens [x, is_not, None, EOF] (was getting 4)
   - Initial failure confirmed inconsistency

2. **GREEN PHASE A — Lexer Implementation:**
   - Added `peek_keyword_after_whitespace()` helper in src/lexer.rs
     * Uses byte offset manipulation to peek ahead
     * Skips whitespace (not newlines) to find next keyword
     * Returns Option<&str> for keyword name or None
   - Modified `scan_identifier()` (lines ~350) to detect "is" keyword and lookahead for "not":
     * After emitting `Is` token, immediately check for "not" keyword
     * If found: consume "not", update span to cover both keywords, emit `TokenType::IsNot` instead
     * If not found: "is" token stands alone
   
3. **GREEN PHASE B — Infrastructure Completion:**
   - **Task 3 carryover** (incomplete in repo):
     * Added `TokenType::DivEuclid` and `TokenType::ModEuclid` to token.rs enum
     * Added Display impls: DivEuclid → "operator 'div_euclid'", ModEuclid → "operator 'mod_euclid'"
     * Added keyword mappings in lexer: `keywords.insert("div_euclid", TokenType::DivEuclid)` etc.

4. **REFACTOR — Parser Alignment:**
   - Updated 3 parser tests to expect new `TokenType::IsNot` behavior:
     * `test_binary_op_is_not()`: parser now receives single `IsNot` token instead of `Is`, `Not` sequence
     * `test_edge_case_is_not_is_single_operator()`: compound behavior verified
     * `test_edge_case_not_a_is_not_b()`: unary `not` on left properly distinguished from `is not`

### Key Insight: Lookahead Pattern
- **Byte offset manipulation** enables safe lookahead without consuming input
- **Whitespace skipping** (not newline-breaking) allows keywords separated by spaces: `is not` vs `is\nnot`
- **Span updates** reflect compound nature: `Span { start: byte_offset(i), end: byte_offset(i+7) }` covers both words
- Pattern reusable for other multi-keyword compounds (e.g., `elif` could use similar strategy)

### Verification
- RED test now passes ✓
- Parser tests updated: 3 tests now pass with `IsNot` ✓
- Full test suite: 792 tests passed (no regressions) ✓
- Evidence saved to `.sisyphus/evidence/task-5-is-not.txt` ✓

### Files Changed
- src/token.rs: Added TokenType::DivEuclid, TokenType::ModEuclid variants + Display impls
- src/lexer.rs: Added peek_keyword_after_whitespace() helper; modified scan_identifier() lookahead
- src/lexer/tests.rs (new test lines 407-423): test_is_not_operator_consistency()
- src/parser/tests.rs (3 updated tests): Expect TokenType::IsNot, BinaryOp::IsNot instead of two-token Is+Not
