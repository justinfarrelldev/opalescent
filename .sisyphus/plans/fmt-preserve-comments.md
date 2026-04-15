# Preserve Comments in Formatter

## TL;DR

> **Quick Summary**: Fix the `opal fmt` command so it preserves all comments (single-line `#` and doc-block `## ... ##`) instead of stripping them during formatting. The formatter currently discards comments because the parser skips them and the printer never emits them.
>
> **Deliverables**:
> - Doc comments (`## ... ##`) rendered before declarations in formatted output
> - Regular comments (`# ...`) preserved between declarations and between statements
> - Comments at file start (before first declaration) preserved
> - Unit tests in `src/formatter/tests.rs` matching existing test patterns
> - Integration tests with golden files in `test-projects/fmt-test/`
> - All existing tests continue to pass
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 0 → Task 1 → Task 3 → Task 4 → Task 5 → F1-F4

---

## Context

### Original Request

"The format command seems to remove comments. This is not desired. Please ensure it does not remove any comments. This should be tested with similar tests to the other format commands. Please make sure the previous work is committed before starting this work."

### Interview Summary

**Key Discussions**:
- The formatter pipeline is: `FormatCommand` → `Formatter::format_source` → `Lexer::tokenize` → `Parser::parse` → `print_program` (AST printer) → `rules::apply_all`
- The lexer correctly produces `TokenType::Comment` and `TokenType::DocComment` tokens
- The parser's `skip_newlines_and_comments()` (26 call sites) discards `Comment` tokens entirely
- Doc comments before declarations ARE collected into `Decl::doc_comment: Option<Documentation>` but the printer ignores them (uses `..` to skip `doc_comment` field)
- Regular comments are completely lost during parsing — they never reach the AST
- Two distinct sub-problems: (1) doc comments already in AST but not printed (easy), (2) regular comments not in AST at all (requires AST changes)

**Research Findings**:
- `Documentation` struct has a `raw` field containing the comment text sans delimiters — sufficient for reconstruction
- `skip_newlines_and_comments()` has 26 call sites across `statements.rs` (12), `declarations.rs` (8), `expressions.rs` (1), `helpers.rs` (1 definition), `tests.rs` (3)
- `Decl` enum has 4 variants (Function, Type, Import, Let) — all match sites must be updated when adding a `Comment` variant
- `Stmt` enum has 13 variants — all match sites must be updated when adding a `Comment` variant
- Existing test patterns: unit tests use `Formatter::with_defaults()` + `format_source()` + `assert_eq!`; integration tests use binary invocation + golden file comparison

### Metis Review

**Identified Gaps** (addressed):
- Inline trailing comments (`let x = 5 # note`) are fundamentally harder and explicitly scoped out
- Comments inside type definitions add complexity; scoped out for this task
- AST variant blast radius: adding `Stmt::Comment`/`Decl::Comment` breaks 24+ exhaustive match sites — addressed by doing all match updates in a single dedicated task before parser changes
- Idempotency regression risk: all tests must assert `format(format(x)) == format(x)`
- `collapse_consecutive_blank_lines` rule may interact with comment spacing — must test explicitly

---

## Work Objectives

### Core Objective

Ensure the `opal fmt` command preserves all standalone comment lines (both `# single-line` and `## ... ## doc-block`) in their correct positions relative to surrounding declarations and statements.

### Concrete Deliverables

- Modified `src/formatter/printer.rs` — emits doc comments from existing AST data
- New AST variants `Decl::Comment` and `Stmt::Comment` in `src/ast.rs`
- Modified parser (`src/parser/helpers.rs`, `src/parser/statements.rs`, `src/parser/declarations.rs`) to preserve comment tokens
- Updated printer to emit `Decl::Comment` and `Stmt::Comment`
- Updated exhaustive match sites across the codebase (24+ locations)
- New unit tests in `src/formatter/tests.rs`
- New golden fixture files in `test-projects/fmt-test/`
- New integration tests in `tests/fmt_integration.rs`

### Definition of Done

- [ ] `cargo test` — all existing tests pass (0 failures)
- [ ] `cargo test` — new comment-preservation tests pass
- [ ] `cargo test --features integration --test fmt_integration` — golden file tests pass
- [ ] Formatting a file with comments produces output containing those comments
- [ ] `format(format(source_with_comments)) == format(source_with_comments)` (idempotency)
- [ ] Formatted output with comments re-parses without lex or parse errors

### Must Have

- Doc comments (`## ... ##`) before declarations rendered in formatted output
- Single-line comments (`# ...`) between top-level declarations preserved
- Single-line comments (`# ...`) between statements inside function bodies preserved
- Comments at file start (before first declaration) preserved
- Idempotency: formatting twice produces same output as formatting once
- All 4 `Decl` variants + new `Decl::Comment` handled in all exhaustive matches
- All 13 `Stmt` variants + new `Stmt::Comment` handled in all exhaustive matches

### Must NOT Have (Guardrails)

- **No inline trailing comment support** — `let x = 5 # trailing` is explicitly out of scope (requires fundamentally different trivia mechanism)
- **No comments inside type definition variant bodies** — deferred to follow-up
- **No comments inside expressions** — deferred to follow-up
- **No comments inside match arm bodies** — deferred to follow-up
- **No comment formatting/normalization** — preserve comment text exactly as-is, no "ensure space after `#`" or similar
- **No modification to the lexer** — comment tokenization already works correctly
- **No trivia/comment fields on existing AST variants** — use NEW enum variants (`Stmt::Comment`, `Decl::Comment`) to avoid breaking all construction sites
- **No doc comment auto-generation** — only preserve what exists

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision

- **Infrastructure exists**: YES (Rust cargo test + feature-gated integration tests)
- **Automated tests**: YES (Tests-after — add tests in Task 5 after implementation, but verify existing tests pass after every task)
- **Framework**: `cargo test` (unit) + `cargo test --features integration` (integration)

### QA Policy

Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Compilation**: `cargo build 2>&1` — must succeed with no errors
- **Unit tests**: `cargo test 2>&1` — all tests pass, 0 failures
- **Integration tests**: `cargo test --features integration --test fmt_integration 2>&1`
- **Clippy**: `cargo clippy -- -D warnings 2>&1` — no warnings
- **Functional**: pipe source with comments through formatter, verify comments in output

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Pre-work — commit):
└── Task 0: Commit previous .sisyphus metadata [quick]

Wave 1 (Foundation — parallel):
├── Task 1: Render doc comments in printer (printer-only change) [quick]
└── Task 2: Create golden test fixture files with comments [quick]

Wave 2 (AST refactor — depends on Wave 1):
└── Task 3: Add Stmt::Comment + Decl::Comment AST variants + update ALL match sites [unspecified-high]

Wave 3 (Core implementation — depends on Wave 2):
└── Task 4: Modify parser to preserve comments + update printer to emit them [deep]

Wave 4 (Tests — depends on Wave 3):
└── Task 5: Add unit tests + integration tests for comment preservation [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 0 → Task 1 → Task 3 → Task 4 → Task 5 → F1-F4 → user okay
Parallel Speedup: ~30% faster than sequential (Wave 1 parallelism + Wave FINAL parallelism)
Max Concurrent: 2 (Wave 1) / 4 (Wave FINAL)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 0 | — | 1, 2 | 0 |
| 1 | 0 | 3 | 1 |
| 2 | 0 | 5 | 1 |
| 3 | 1 | 4 | 2 |
| 4 | 3 | 5 | 3 |
| 5 | 2, 4 | F1-F4 | 4 |
| F1-F4 | 5 | user okay | FINAL |

### Agent Dispatch Summary

- **Wave 0**: **1** — T0 → `quick` (git-master skill)
- **Wave 1**: **2** — T1 → `quick`, T2 → `quick`
- **Wave 2**: **1** — T3 → `unspecified-high`
- **Wave 3**: **1** — T4 → `deep`
- **Wave 4**: **1** — T5 → `unspecified-high`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [ ] 0. Commit Previous Work

  **What to do**:
  - Stage all modified and untracked `.sisyphus/` files (evidence, notepads, plans, boulder.json, learnings.md)
  - Do NOT stage the stray file literally named `-` in the project root — inspect it first and delete if it's an accidental redirect artifact
  - Create a commit following the project's Conventional Commits style
  - Do NOT push to remote (user hasn't requested it, and there are 59 unpushed commits)

  **Must NOT do**:
  - Do not push to remote
  - Do not stage source code files (there are no source changes pending)
  - Do not modify git config

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - `git-master`: Direct git operation — commit workflow

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0 (solo)
  - **Blocks**: Tasks 1, 2
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - Recent commit messages follow Conventional Commits: `test(formatter):`, `refactor(formatter):`, `fix(fmt):`, `feat(cli):`, `chore(sisyphus):`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Commit succeeds with clean working tree for source files
    Tool: Bash (git)
    Preconditions: Modified .sisyphus/boulder.json and .sisyphus/notepads/correctness-fixes/learnings.md; many untracked .sisyphus/ files
    Steps:
      1. Run `ls -la ./-` to inspect the stray `-` file; if empty or garbage, `rm ./-`
      2. Run `git add .sisyphus/`
      3. Run `git status` — verify only .sisyphus/ files are staged; no source files
      4. Run `git commit -m "chore(sisyphus): commit QA evidence, plans, and session metadata"`
      5. Run `git status` — verify working tree is clean for .sisyphus/ (untracked non-.sisyphus files are OK)
    Expected Result: Commit created successfully. `git log -1 --oneline` shows the new commit with chore(sisyphus) prefix.
    Failure Indicators: Commit fails, source files accidentally staged, git error
    Evidence: .sisyphus/evidence/task-0-commit.txt
  ```

  **Commit**: YES
  - Message: `chore(sisyphus): commit QA evidence, plans, and session metadata`
  - Files: `.sisyphus/*`
  - Pre-commit: N/A

- [ ] 1. Render Doc Comments in Printer

  **What to do**:
  - In `src/formatter/printer.rs`, modify `print_decl()` (line 296) to emit the `doc_comment` field for `Decl::Function`, `Decl::Type`, and `Decl::Let` variants
  - For each variant, check if `doc_comment` is `Some(ref doc)` — if so, reconstruct the doc block using `doc.raw` and prepend it before the declaration
  - Reconstruction format: `##\n{raw}\n##\n` where `{raw}` is `doc.raw` (the raw text sans delimiters)
  - Apply proper indentation to the doc block (use `self.indent(depth)` for each line)
  - In `print_program()` (line 283), no changes needed — `print_decl` already handles each declaration
  - Also modify `print_decl` to destructure `doc_comment` explicitly instead of using `..` catch-all for all three variants that have it

  **Must NOT do**:
  - Do not modify the lexer or parser — doc comments are already in the AST
  - Do not normalize/reformat doc comment content — use `doc.raw` exactly as-is
  - Do not add new AST variants (that's Task 3)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Task 3
  - **Blocked By**: Task 0

  **References**:

  **Pattern References**:
  - `src/formatter/printer.rs:296-407` — `print_decl()` method with all 4 Decl variant arms. Currently uses `..` to skip `doc_comment` field in Function (line 306), Type (line 341), and Let (line 387)
  - `src/formatter/printer.rs:283-289` — `print_program()` method that iterates `program.declarations` and joins with `"\n\n"`

  **API/Type References**:
  - `src/ast/documentation.rs:13-22` — `Documentation` struct with `raw: String` field (line 15) containing comment text sans delimiters, and `span: Span` (line 21)
  - `src/ast.rs:758` — `Decl::Function { doc_comment: Option<Documentation>, .. }` (line 758)
  - `src/ast.rs:780` — `Decl::Type { doc_comment: Option<Documentation>, .. }` (line 780)
  - `src/ast.rs:836` — `Decl::Let { doc_comment: Option<Documentation>, .. }` (line 836)

  **External References**:
  - README "Doc comments" section — shows format: `## Description: ... ##` with multi-line content indented under `##`

  **WHY Each Reference Matters**:
  - `printer.rs:296-407` — This is the EXACT code to modify. Each variant's match arm currently ignores `doc_comment` via `..` and must be updated to destructure it and emit it
  - `documentation.rs` — The `raw` field contains the content between `##` delimiters. Use this to reconstruct `##\n{raw}\n##` blocks
  - `ast.rs` Decl variants — Confirms which variants carry `doc_comment` (all except `Import`)

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Doc comments before function declarations are preserved
    Tool: Bash (cargo run)
    Preconditions: Build succeeds with `cargo build`
    Steps:
      1. Create temp file `/tmp/test_doc_comments.op` with content:
         ##
           Description: Adds two numbers
         ##
         entry main = f(a: int32, b: int32): int32 =>
             return a + b
      2. Run `cargo run -- fmt --output /tmp/test_doc_out.op /tmp/test_doc_comments.op`
      3. Read `/tmp/test_doc_out.op`
      4. Assert output contains "##" (doc comment delimiters)
      5. Assert output contains "Description: Adds two numbers"
      6. Assert output contains "entry main" (declaration still present)
    Expected Result: Output file contains both the doc comment block and the function declaration
    Failure Indicators: Output missing "##" or "Description:" lines
    Evidence: .sisyphus/evidence/task-1-doc-comments-preserved.txt

  Scenario: Formatted output with doc comments is idempotent
    Tool: Bash (cargo run)
    Preconditions: task-1 doc comment output file exists
    Steps:
      1. Run `cargo run -- fmt --output /tmp/test_doc_idem.op /tmp/test_doc_out.op` (format the already-formatted output)
      2. Run `diff /tmp/test_doc_out.op /tmp/test_doc_idem.op`
    Expected Result: diff produces no output (files are identical)
    Failure Indicators: diff shows differences between first and second format pass
    Evidence: .sisyphus/evidence/task-1-doc-idempotency.txt

  Scenario: All existing tests still pass
    Tool: Bash (cargo test)
    Preconditions: Code compiles
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert output contains "test result: ok" and "0 failed"
    Expected Result: All existing tests pass with 0 failures
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-1-existing-tests.txt
  ```

  **Commit**: YES
  - Message: `fix(fmt): render doc comments in formatted output`
  - Files: `src/formatter/printer.rs`
  - Pre-commit: `cargo test`

- [ ] 2. Create Golden Test Fixture Files with Comments

  **What to do**:
  - Create a new input fixture file `test-projects/fmt-test/src/input-comments.op` containing Opalescent source with various comment patterns:
    - A file-header comment at the very top (`# File header comment`)
    - A doc comment block (`## Description: ... ##`) before a function declaration
    - A single-line comment (`# Section separator`) between two top-level declarations
    - Comments inside a function body between statements (`# Step 1`, `# Step 2`)
    - An inline comment block (multi-line `## ... ##` that is NOT a doc comment — i.e., doesn't start with "Description:")
  - Create the matching expected output file `test-projects/fmt-test/expected/input-comments.expected.op` containing the correctly formatted version with all comments preserved
  - The expected output should be what a correct formatter would produce: proper indentation, normalized whitespace, but all comments intact
  - Ensure the fixture file is valid Opalescent that parses without errors (use existing fixtures as reference for syntax)

  **Must NOT do**:
  - Do not modify any Rust source files
  - Do not add comments in positions that are out of scope (trailing inline, inside type variants, inside expressions)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Task 5
  - **Blocked By**: Task 0

  **References**:

  **Pattern References**:
  - `test-projects/fmt-test/src/input-spaces.op` — Existing input fixture showing valid Opalescent syntax with functions, types, and formatting
  - `test-projects/fmt-test/expected/input-spaces.expected.op` — Corresponding expected output showing formatter's canonical style
  - `test-projects/fmt-test/src/input-tabs.op` — Another fixture showing tab-indented input
  - `test-projects/fmt-test/opal.toml` — Test project manifest (fixture files must be valid within this project context)

  **External References**:
  - README "Language Basics" — Comment syntax: `# single-line`, `## ... ## doc-block`
  - README "Functions" — Function declaration syntax: `let name = f(params): type => body` and `entry main = f(args: string[]): void => body`

  **WHY Each Reference Matters**:
  - Existing fixtures show the EXACT Opalescent syntax that parses successfully — new fixture must use the same patterns
  - Expected output files show the formatter's canonical style (indentation, spacing, newlines) — new expected file must match this style plus comments

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Input fixture parses without errors
    Tool: Bash (cargo run)
    Preconditions: Fixture file created
    Steps:
      1. Run `cargo run -- check test-projects/fmt-test/src/input-comments.op 2>&1` OR lex/parse the file via the formatter (which will report parse errors)
      2. If check subcommand doesn't work on standalone files, run `cargo run -- fmt --output /dev/null test-projects/fmt-test/src/input-comments.op 2>&1`
      3. Assert exit code is 0 (no parse errors)
    Expected Result: File parses successfully with no lex or parse errors
    Failure Indicators: Parse error in output, non-zero exit code
    Evidence: .sisyphus/evidence/task-2-fixture-parses.txt

  Scenario: Expected output matches canonical formatter style
    Tool: Bash (grep + diff)
    Preconditions: Expected file created, existing expected files available for comparison
    Steps:
      1. Run `grep -P '^\t' test-projects/fmt-test/expected/input-comments.expected.op | wc -l` — assert output is "0" (no tab indentation)
      2. Run `grep -P '^    ' test-projects/fmt-test/expected/input-comments.expected.op | head -3` — assert output is non-empty (4-space indentation is used)
      3. Run `tail -c 1 test-projects/fmt-test/expected/input-comments.expected.op | xxd` — assert last byte is `0a` (trailing newline)
      4. Run `grep -c '#' test-projects/fmt-test/expected/input-comments.expected.op` — assert count >= 3 (comments are present)
      5. Run `grep '##' test-projects/fmt-test/expected/input-comments.expected.op | head -2` — assert output is non-empty (doc comment delimiters present)
      6. Run `grep '# ' test-projects/fmt-test/expected/input-comments.expected.op | head -2` — assert output is non-empty (single-line comments present)
    Expected Result: File uses 4-space indentation, has trailing newline, and contains both comment types
    Failure Indicators: Tab indentation found, missing trailing newline (last byte not 0a), fewer than 3 comment lines
    Evidence: .sisyphus/evidence/task-2-expected-style.txt
  ```

  **Commit**: YES
  - Message: `test(fmt-test): add comment-preservation fixture files`
  - Files: `test-projects/fmt-test/src/input-comments.op`, `test-projects/fmt-test/expected/input-comments.expected.op`
  - Pre-commit: N/A

- [ ] 3. Add Comment AST Variants and Update All Match Sites

  **What to do**:
  - In `src/ast.rs`, add a new `Comment` variant to `Decl` enum (after `Let`, around line 843):
    ```rust
    Comment {
        text: String,
        span: Span,
        id: NodeId,
    },
    ```
  - In `src/ast.rs`, add a new `Comment` variant to `Stmt` enum (after `Continue`, around line 711):
    ```rust
    Comment {
        text: String,
        span: Span,
        id: NodeId,
    },
    ```
  - The `text` field stores the raw comment text INCLUDING the `#` prefix (e.g., `"# This is a comment"`) — this avoids any reconstruction logic
  - For multi-line comment blocks (non-doc `## ... ##`), store the entire block as-is including delimiters
  - Update ALL exhaustive match sites to handle the new variants with pass-through/skip arms (the goal is to make the code COMPILE, not change behavior yet)
  - The exhaustive match sites that need updating (exhaustive list):

  **`Decl::Comment` match sites to update:**
  1. `src/formatter/printer.rs` → `print_decl()` — add arm that returns the comment text with indentation
  2. `src/formatter/naming.rs` → `check_decl()` — add arm that does nothing (comments have no names to check)
  3. `src/doc_gen/extractor.rs` → `extract_public_api_docs()` — add arm that skips (comments are not API docs)
  4. `src/type_system/checker/declarations.rs` → `register_declaration_signature()` — add arm that does nothing
  5. `src/type_system/checker/declarations.rs` → `type_check_declaration()` — add arm that returns Ok
  6. `src/codegen/functions.rs` → codegen match on Decl — add arm that does nothing (no codegen for comments)
  7. `src/lsp/definition.rs` → `declaration_location_for_symbol()` — add arm that returns None
  8. `src/ast/node_impls.rs` → `span()` impl for Decl — add arm returning span
  9. `src/ast/node_impls.rs` → `node_id()` impl for Decl — add arm returning id
  10. `src/ast/node_impls.rs` → `abi_symbols()` impl for Decl — add arm returning empty vec
  11. `src/ast/node_impls.rs` → `dependencies()` impl for Decl — add arm returning empty vec
  12. `src/ast/node_impls.rs` → `is_hot_reloadable()` impl for Decl — add arm returning false
  13. `src/ast.rs` → `Decl::span_const()` — add arm returning span
  14. `src/ast.rs` → `Decl::node_id_const()` — add arm returning id
  15. `src/compiler.rs` → `compile_to_module()` or wherever Decl is matched — add skip arm

  **`Stmt::Comment` match sites to update:**
  1. `src/formatter/printer.rs` → `print_stmt()` — add arm that returns the comment text with indentation
  2. `src/formatter/naming.rs` → `check_stmt()` — add arm that does nothing
  3. `src/type_system/checker/statements.rs` → `type_check_stmt_with_return()` — add arm that returns Ok with unit type
  4. `src/type_system/checker/statements.rs` → `collect_break_types()` — add arm that does nothing
  5. `src/type_system/checker/control_flow.rs` → `infer_stmt_value_type()` — add arm returning None or unit
  6. `src/type_system/checker/expressions_guard.rs` → `type_check_guard_else_branch()` — add arm returning Ok
  7. `src/codegen/statements.rs` → `codegen_statement()` — add arm that does nothing
  8. `src/codegen/control_flow.rs` → `emit_loop_body_with_targets()` — add arm that does nothing
  9. `src/parser/captures.rs` → `collect_identifiers_in_stmt()` — add arm that does nothing (no identifiers in comments)
  10. `src/ast/node_impls.rs` → `span()` impl for Stmt — add arm returning span
  11. `src/ast/node_impls.rs` → `node_id()` impl for Stmt — add arm returning id
  12. `src/ast.rs` → `Stmt::span_const()` — add arm returning span
  13. `src/ast.rs` → `Stmt::node_id_const()` — add arm returning id

  **Must NOT do**:
  - Do not modify the parser yet (that's Task 4) — no comment nodes will be emitted yet
  - Do not add trivia/comment fields to existing variants
  - Do not modify the lexer

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
    - This task touches 15+ files and requires precision across all exhaustive match sites

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (solo)
  - **Blocks**: Task 4
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/ast.rs:789-802` — `Decl::Import` variant — follow this as the pattern for `Decl::Comment` (simplest existing variant)
  - `src/ast.rs:693-711` — `Stmt::Break` and `Stmt::Continue` variants — follow these as pattern for `Stmt::Comment` (simple variants with span and id)
  - `src/ast/node_impls.rs` — All AstNode trait implementations for Decl and Stmt showing how span/node_id/abi_symbols/dependencies/is_hot_reloadable are matched

  **API/Type References**:
  - `src/ast.rs:569-712` — Full `Stmt` enum definition (13 variants, lines 569-712)
  - `src/ast.rs:729-843` — Full `Decl` enum definition (4 variants, lines 729-843)
  - `src/token.rs` — `Span` type used in all AST nodes

  **WHY Each Reference Matters**:
  - `Decl::Import` is the simplest Decl variant — use its field pattern (minimal fields) as template for `Decl::Comment`
  - `Stmt::Break`/`Continue` are simple Stmt variants — use their pattern for `Stmt::Comment`
  - `node_impls.rs` shows the EXACT match patterns needed — each impl method matches exhaustively on all variants

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Code compiles after adding new AST variants
    Tool: Bash (cargo build)
    Preconditions: New Decl::Comment and Stmt::Comment variants added, all match sites updated
    Steps:
      1. Run `cargo build 2>&1`
      2. Assert exit code is 0
      3. Assert no compilation errors in output
    Expected Result: Clean compilation with no errors
    Failure Indicators: "error[E0004]: non-exhaustive patterns" or any compile error
    Evidence: .sisyphus/evidence/task-3-compiles.txt

  Scenario: All existing tests pass (no regressions)
    Tool: Bash (cargo test)
    Preconditions: Code compiles
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert output contains "test result: ok" and "0 failed"
    Expected Result: All existing tests pass with 0 failures
    Failure Indicators: Any test failure, new warnings
    Evidence: .sisyphus/evidence/task-3-existing-tests.txt

  Scenario: Clippy passes with no new warnings
    Tool: Bash (cargo clippy)
    Preconditions: Code compiles
    Steps:
      1. Run `cargo clippy -- -D warnings 2>&1`
      2. Assert exit code is 0
    Expected Result: No clippy warnings
    Failure Indicators: Any clippy warning about unreachable patterns, unused variables, etc.
    Evidence: .sisyphus/evidence/task-3-clippy.txt
  ```

  **Commit**: YES
  - Message: `refactor(ast): add Comment variants to Decl and Stmt enums`
  - Files: `src/ast.rs`, `src/ast/node_impls.rs`, `src/formatter/printer.rs`, `src/formatter/naming.rs`, `src/doc_gen/extractor.rs`, `src/type_system/checker/declarations.rs`, `src/type_system/checker/statements.rs`, `src/type_system/checker/control_flow.rs`, `src/type_system/checker/expressions_guard.rs`, `src/codegen/functions.rs`, `src/codegen/statements.rs`, `src/codegen/control_flow.rs`, `src/lsp/definition.rs`, `src/parser/captures.rs`, `src/compiler.rs`
  - Pre-commit: `cargo test`

- [ ] 4. Modify Parser to Preserve Comments and Update Printer

  **What to do**:

  **Part A — Parser changes to emit comment nodes**:
  - In `src/parser/helpers.rs`, create a new method `collect_comments(&mut self) -> Vec<Stmt>` (or similar) that collects `TokenType::Comment` and `TokenType::DocComment` tokens into `Stmt::Comment` / `Decl::Comment` nodes instead of skipping them. Use the parser's `next_id()` for node IDs and the token's span.
  - For `TokenType::Comment(text)`: store as `Stmt::Comment { text: format!("# {text}"), span, id }` (or however the lexer stores the comment text — check if the `#` prefix is included in the token's lexeme or content string; use `lexeme` if available, otherwise reconstruct)
  - For `TokenType::DocComment(text)` that appears in NON-declaration contexts (i.e., comments between statements): store similarly as a comment node
  - **Key design decision**: Rather than modifying ALL 26 `skip_newlines_and_comments()` call sites, take this approach:
    1. Modify `Parser::parse()` in `src/parser.rs` (the top-level parse loop) — when the parser encounters a `Comment` token at the top level (between declarations), emit a `Decl::Comment` node into `program.declarations`
    2. Modify statement-level parsing (e.g., in `parse_indent_block` in `src/parser/statements.rs`) — when a `Comment` token is encountered at statement level, emit a `Stmt::Comment` into the statement list
    3. Leave `skip_newlines_and_comments()` itself unchanged for contexts where comments should still be skipped (e.g., between tokens within an expression, inside type definitions)
  - Focus on these specific parser entry points:
    - `src/parser.rs` — the `parse()` method's main loop that builds `program.declarations` — handle `TokenType::Comment` here to produce `Decl::Comment`
    - `src/parser/statements.rs` — `parse_indent_block()` (or equivalent block-parsing method) that builds `Vec<Stmt>` — handle `TokenType::Comment` here to produce `Stmt::Comment`
    - `src/parser/declarations.rs` — `parse_declaration()` entry point — if a comment appears where a declaration is expected, produce `Decl::Comment`

  **Part B — Printer updates for comment nodes**:
  - In `src/formatter/printer.rs`, the `Decl::Comment` arm in `print_decl()` (added in Task 3 as a placeholder) should now emit `format!("{}{}", self.indent(depth), text)` — the comment text with proper indentation
  - The `Stmt::Comment` arm in `print_stmt()` (added in Task 3) should emit the same: `format!("{}{}", self.indent(depth), text)`
  - Ensure `print_program()` handles `Decl::Comment` naturally — it already joins declarations with `"\n\n"`, so comment declarations will get proper spacing

  **Must NOT do**:
  - Do not handle trailing inline comments (`let x = 5 # trailing`) — out of scope
  - Do not handle comments inside type definition variant bodies — out of scope
  - Do not handle comments inside expressions — out of scope
  - Do not modify the lexer
  - Do not normalize comment text — preserve exactly as tokenized

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - This is the core behavioral change requiring careful parser modification and understanding of token flow

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (solo)
  - **Blocks**: Task 5
  - **Blocked By**: Task 3

  **References**:

  **Pattern References**:
  - `src/parser/helpers.rs:223` — `skip_newlines_and_comments()` definition — understand what it does to know what to supplement (it advances past Newline, Comment, DocComment tokens)
  - `src/parser/declarations.rs:1-50` (approx) — `parse_declaration()` entry point — shows how `skip_trivia_preserving_doc_comments()` is called before parsing; add comment-node handling here
  - `src/parser/statements.rs:20-70` (approx) — `parse_indent_block()` or similar block-parsing — shows how statements are accumulated; add `Stmt::Comment` emission here

  **API/Type References**:
  - `src/token.rs` — `TokenType::Comment(String)` and `TokenType::DocComment(String)` — check what the `String` contains (the text content) and whether the `#`/`##` delimiters are included
  - `src/lexer.rs` — `scan_single_line_comment()` and `scan_multiline_comment()` — understand what content string is stored (research says content sans delimiter for Comment, full content with "Description:" for DocComment)
  - `src/ast.rs:Decl::Comment` and `src/ast.rs:Stmt::Comment` — the new variants added in Task 3

  **WHY Each Reference Matters**:
  - `skip_newlines_and_comments()` — Understand its behavior to know which call sites should be supplemented vs left alone. Call sites in expression parsing should be left alone; call sites in declaration/statement parsing are where comments need capturing
  - `parse_declaration()` — This is where top-level `Decl::Comment` nodes should be emitted when a Comment token appears where a declaration is expected
  - `token.rs` and `lexer.rs` — Must know the exact format of comment text strings to construct `text` field correctly. If lexer stores `"This is a comment"` without `#`, the parser must prepend `# `. If lexer stores `"# This is a comment"` with `#`, use directly
  - Existing `.op` test files — Show what comment syntax actually looks like in practice

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Single-line comments between declarations are preserved
    Tool: Bash (cargo run)
    Preconditions: Code compiles with parser changes
    Steps:
      1. Create temp file `/tmp/test_comments.op` with content:
         # File header comment
         entry main = f(): void =>
             return void

         # Section separator
         let helper = f(x: int32): int32 =>
             return x
      2. Run `cargo run -- fmt --output /tmp/test_comments_out.op /tmp/test_comments.op`
      3. Read `/tmp/test_comments_out.op`
      4. Assert output contains "# File header comment"
      5. Assert output contains "# Section separator"
      6. Assert output contains "entry main" and "let helper"
    Expected Result: Both comments and both declarations present in output
    Failure Indicators: Any comment missing from output
    Evidence: .sisyphus/evidence/task-4-comments-between-decls.txt

  Scenario: Comments inside function bodies are preserved
    Tool: Bash (cargo run)
    Preconditions: Code compiles
    Steps:
      1. Create temp file `/tmp/test_body_comments.op` with content:
         entry main = f(): void =>
             # Step 1: setup
             let x = 42
             # Step 2: finish
             return void
      2. Run `cargo run -- fmt --output /tmp/test_body_out.op /tmp/test_body_comments.op`
      3. Read `/tmp/test_body_out.op`
      4. Assert output contains "# Step 1: setup"
      5. Assert output contains "# Step 2: finish"
    Expected Result: Body comments preserved between statements
    Failure Indicators: Comments missing from function body
    Evidence: .sisyphus/evidence/task-4-body-comments.txt

  Scenario: Idempotency with comments
    Tool: Bash (cargo run)
    Preconditions: Formatted output from previous scenarios
    Steps:
      1. Run `cargo run -- fmt --output /tmp/test_idem2.op /tmp/test_comments_out.op`
      2. Run `diff /tmp/test_comments_out.op /tmp/test_idem2.op`
    Expected Result: No differences — formatting is idempotent
    Failure Indicators: diff shows any differences
    Evidence: .sisyphus/evidence/task-4-idempotency.txt

  Scenario: Formatted output re-parses cleanly
    Tool: Bash (cargo run)
    Preconditions: Formatted output from previous scenarios
    Steps:
      1. Run `cargo run -- fmt --output /dev/null /tmp/test_comments_out.op 2>&1`
      2. Assert exit code is 0 (no parse errors on formatted output)
    Expected Result: Formatted output with comments is valid Opalescent that re-parses
    Failure Indicators: Parse error when formatting the formatted output
    Evidence: .sisyphus/evidence/task-4-reparse.txt

  Scenario: All existing tests still pass
    Tool: Bash (cargo test)
    Preconditions: All changes applied
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert "test result: ok" and "0 failed"
    Expected Result: Zero regressions
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-4-existing-tests.txt
  ```

  **Commit**: YES
  - Message: `fix(fmt): preserve comments through parse-format pipeline`
  - Files: `src/parser/helpers.rs`, `src/parser.rs`, `src/parser/statements.rs`, `src/parser/declarations.rs`, `src/formatter/printer.rs`
  - Pre-commit: `cargo test`

- [ ] 5. Add Unit Tests and Integration Tests for Comment Preservation

  **What to do**:

  **Part A — Unit tests in `src/formatter/tests.rs`**:
  - Add test `test_formatter_preserves_doc_comments` — Format source with `## Description: ... ##` before a function → assert output contains `##` and `Description:`
  - Add test `test_formatter_preserves_single_line_comments` — Format source with `# comment` between two declarations → assert output contains `# comment`
  - Add test `test_formatter_preserves_body_comments` — Format source with `# comment` between statements inside a function body → assert output contains `# comment`
  - Add test `test_formatter_preserves_file_header_comment` — Format source starting with `# File header` → assert output starts with (or contains) `# File header`
  - Add test `test_formatter_comment_idempotency` — Format source with comments, format again, assert `first == second`
  - Add test `test_formatter_comments_reparse_clean` — Format source with comments → lex and parse the output → assert no lex/parse errors
  - Add test `test_formatter_preserves_multiline_non_doc_comment` — Format source with `## not a doc block ##` (no "Description:" prefix) → assert output preserves it
  - Follow the existing test patterns: use `Formatter::with_defaults()`, call `format_source()`, use `assert!()` / `assert_eq!()` on the result

  **Part B — Integration tests in `tests/fmt_integration.rs`**:
  - Add test `test_fmt_comments_golden` — Run the binary with `--output` on `test-projects/fmt-test/src/input-comments.op` (created in Task 2), compare output to `test-projects/fmt-test/expected/input-comments.expected.op`
  - Add test `test_fmt_comments_idempotent` — Format the expected file again, assert no diff
  - Follow existing integration test patterns: use `binary_path()`, `fmt_test_src()`, `fmt_test_expected()`, `temp_output_path()`, `cleanup_temp()`, `Command::new()`
  - NOTE: The golden expected file (`input-comments.expected.op`) may need updating if the fixture from Task 2 doesn't exactly match the formatter's output after Task 4 — update it to match actual correct output

  **Must NOT do**:
  - Do not test trailing inline comments (out of scope)
  - Do not test comments inside type definition bodies (out of scope)
  - Do not modify implementation code — only test files and fixture files

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (solo)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 2, 4

  **References**:

  **Pattern References**:
  - `src/formatter/tests.rs` — ALL existing tests. Follow the pattern: `let fmt = Formatter::with_defaults(); let result = fmt.format_source(source).unwrap(); assert!(result.contains(...));`
  - `tests/fmt_integration.rs` — ALL existing integration tests. Follow the pattern: `Command::new(&binary).arg("fmt").arg("--output").arg(&output).arg(&input).status()` then `assert_eq!(fs::read_to_string(&output), fs::read_to_string(&golden))`
  - `tests/fmt_integration.rs:binary_path()` — Helper to locate the compiled binary
  - `tests/fmt_integration.rs:fmt_test_src()` / `fmt_test_expected()` — Helpers to locate fixture/golden files
  - `tests/fmt_integration.rs:temp_output_path()` / `cleanup_temp()` — Helpers for temp file management

  **Test References**:
  - `src/formatter/tests.rs:test_format_idempotent_simple` (or similar) — Idempotency test pattern to follow
  - `src/formatter/tests.rs:test_format_source_parse_error` — Error-handling test pattern

  **WHY Each Reference Matters**:
  - `tests.rs` existing tests — MUST follow exactly the same patterns for consistency. Don't invent new assertion styles
  - `fmt_integration.rs` existing tests — Same: follow the binary invocation + golden comparison pattern exactly
  - Helper functions — Reuse them; don't create new helpers for the same purpose

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: All unit tests pass including new comment tests
    Tool: Bash (cargo test)
    Preconditions: New tests added to src/formatter/tests.rs
    Steps:
      1. Run `cargo test --lib 2>&1`
      2. Assert output shows new test names (test_formatter_preserves_doc_comments, etc.) with "ok"
      3. Assert "0 failed"
    Expected Result: All unit tests pass including 7+ new comment tests
    Failure Indicators: Any test failure, new tests not appearing in output
    Evidence: .sisyphus/evidence/task-5-unit-tests.txt

  Scenario: Integration tests pass including golden file comparison
    Tool: Bash (cargo test with integration feature)
    Preconditions: Golden files in place, binary compiles
    Steps:
      1. Run `cargo build 2>&1` to ensure binary exists
      2. Run `cargo test --features integration --test fmt_integration 2>&1`
      3. Assert new comment tests appear and pass
      4. Assert "0 failed"
    Expected Result: Golden file comparison passes — formatter output matches expected
    Failure Indicators: Golden file mismatch, test failure
    Evidence: .sisyphus/evidence/task-5-integration-tests.txt

  Scenario: Full test suite passes (regression check)
    Tool: Bash (cargo test)
    Preconditions: All tests added
    Steps:
      1. Run `cargo test 2>&1`
      2. Assert "0 failed"
      3. Compare total test count to baseline (should be baseline + new tests)
    Expected Result: All tests pass with zero regressions
    Failure Indicators: Any pre-existing test failing
    Evidence: .sisyphus/evidence/task-5-full-suite.txt
  ```

  **Commit**: YES
  - Message: `test(formatter): add comment-preservation tests and golden files`
  - Files: `src/formatter/tests.rs`, `tests/fmt_integration.rs`, `test-projects/fmt-test/src/input-comments.op` (potentially updated), `test-projects/fmt-test/expected/input-comments.expected.op` (potentially updated)
  - Pre-commit: `cargo test --features integration`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read `.sisyphus/plans/fmt-preserve-comments.md` end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build`, `cargo clippy -- -D warnings`, `cargo test`. Review all changed files for: `as any`/`#[allow]` abuse, empty match arms, `todo!()` or `unimplemented!()` in production paths, dead code. Check for AI slop: excessive comments, over-abstraction, generic variable names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Create temp `.op` files with various comment patterns. Run `cargo run -- fmt --output <out> <input>` for each. Verify comments appear in output. Test idempotency by formatting output again. Test `--check` mode with commented files. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (`git log/diff`). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance (no trailing comments, no type def comments, no comment normalization). Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Task | Commit Message | Files | Pre-commit Check |
|------|---------------|-------|-----------------|
| 0 | `chore(sisyphus): commit metadata from previous work session` | `.sisyphus/*` | N/A |
| 1 | `fix(fmt): render doc comments in formatted output` | `src/formatter/printer.rs` | `cargo test` |
| 2 | `test(fmt-test): add comment-preservation fixture files` | `test-projects/fmt-test/src/input-comments.op`, `test-projects/fmt-test/expected/input-comments.expected.op` | N/A |
| 3 | `refactor(ast): add Comment variants to Decl and Stmt enums` | `src/ast.rs`, `src/ast/node_impls.rs`, `src/formatter/printer.rs`, `src/formatter/naming.rs`, `src/doc_gen/extractor.rs`, `src/type_system/checker/declarations.rs`, `src/type_system/checker/statements.rs`, `src/type_system/checker/control_flow.rs`, `src/type_system/checker/expressions_guard.rs`, `src/codegen/functions.rs`, `src/codegen/statements.rs`, `src/codegen/control_flow.rs`, `src/lsp/definition.rs`, `src/parser/captures.rs`, `src/compiler.rs` | `cargo test` |
| 4 | `fix(fmt): preserve comments through parse-format pipeline` | `src/parser/helpers.rs`, `src/parser.rs`, `src/parser/statements.rs`, `src/parser/declarations.rs`, `src/formatter/printer.rs` | `cargo test` |
| 5 | `test(formatter): add comment-preservation tests and golden files` | `src/formatter/tests.rs`, `tests/fmt_integration.rs`, `test-projects/fmt-test/src/input-comments.op`, `test-projects/fmt-test/expected/input-comments.expected.op` | `cargo test --features integration` |

---

## Success Criteria

### Verification Commands

```bash
cargo build 2>&1                                    # Expected: compiles with no errors
cargo clippy -- -D warnings 2>&1                    # Expected: no warnings
cargo test 2>&1                                      # Expected: all tests pass, 0 failures
cargo test --features integration --test fmt_integration 2>&1  # Expected: all integration tests pass
```

### Final Checklist

- [ ] All "Must Have" items present and verified
- [ ] All "Must NOT Have" items absent (no trailing comments, no type def comments, no comment normalization)
- [ ] All existing tests pass (0 regressions)
- [ ] New tests cover doc comments, single-line comments, body comments, file-start comments
- [ ] Idempotency verified for all comment patterns
- [ ] Golden file comparison passes for comment fixtures
