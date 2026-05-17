# Bytes Initialization: `bytes_new` Coverage and `new Bytes` Syntax

## TL;DR
> **Summary**: Add compatibility coverage for the existing `bytes_new()` constructor, then implement `let buffer: Bytes = new Bytes` as Bytes-only syntax sugar that lowers to the existing runtime `bytes_new()` ABI. Update live docs/examples to prefer `new Bytes` while preserving source-level `bytes_new()` compatibility.
> **Deliverables**:
> - Permanent legacy test project: `test-projects/bytes-empty-construct-legacy`
> - Permanent new-syntax test project: `test-projects/bytes-empty-construct-new-syntax`
> - Parser/typechecker/codegen/formatter support for propertyless `new Bytes`
> - Documentation update for live Bytes initialization docs
> - Three green commits with pre-commit issues fixed
> **Effort**: Medium
> **Parallel**: YES - 4 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Tasks 4-6 → Task 7 → Task 8

## Context
### Original Request
Add a test project to test current `bytes_new` functionality; verify it works, commit it, and fix pre-commit issues. Then add a test project for `let buffer: Bytes = new Bytes`, ensure it fails before implementation, implement the change, verify it passes, refactor, and update documentation references from `bytes_new` appropriately. Use subagents and Serena because the repo is large.

### Interview Summary
No additional user interview was required after repository research. Defaults applied from research and project conventions:
- `bytes_new()` remains source-compatible and runtime-compatible.
- `new Bytes` is canonical public syntax after this change.
- RED evidence is captured before implementation but not committed as a failing test-only commit, because every commit must pass pre-commit.
- Only live/current docs are updated; historical planning/proposal docs are reviewed and changed only if they present themselves as current user-facing documentation.

### Metis Review (gaps addressed)
- Clarified grammar: only `new Bytes` without colon, parens, or field block is newly accepted.
- Chose enforcement layer: parser accepts propertyless `new <Ident>` as an AST shape, typechecker restricts this plan to `Bytes` only and emits diagnostics for other propertyless constructors.
- Preserved runtime ABI: `runtime/opal_bytes.c` and `runtime/opal_runtime.h` symbol names must not be changed.
- Added formatter round-trip coverage.
- Added negative tests for `new Person`, `new Message.Text`, `new Bytes:`, and `new Bytes()`.
- Avoided committing a RED-only failing state; RED failure is captured as evidence before GREEN changes.

## Work Objectives
### Core Objective
Prove the current empty Bytes constructor works through `bytes_new()`, then add and verify `new Bytes` as the canonical source syntax for empty Bytes construction while preserving the runtime ABI and compatibility alias.

### Deliverables
- `test-projects/bytes-empty-construct-legacy/opal.toml`
- `test-projects/bytes-empty-construct-legacy/README.md`
- `test-projects/bytes-empty-construct-legacy/src/main.op`
- `test-projects/bytes-empty-construct-legacy/expected/stdout.txt`
- `test-projects/bytes-empty-construct-new-syntax/opal.toml`
- `test-projects/bytes-empty-construct-new-syntax/README.md`
- `test-projects/bytes-empty-construct-new-syntax/src/main.op`
- `test-projects/bytes-empty-construct-new-syntax/expected/stdout.txt`
- Integration coverage in `tests/integration_e2e/bytes_stdlib.rs`
- Parser/typechecker/codegen/formatter/unit tests for `new Bytes`
- Live docs updated in `stdlib/prelude.op` and any live README/tutorial discovered by the docs audit

### Definition of Done (verifiable conditions with commands)
- `cargo test --features integration empty_bytes_via_bytes_new` exits 0.
- Before implementation, `set -o pipefail; cargo test --features integration empty_bytes_via_new_bytes 2>&1 | tee .sisyphus/evidence/red-new-bytes.txt` fails and captures parser/typechecker failure text mentioning `new Bytes` or constructor syntax.
- After implementation, `cargo test --features integration empty_bytes_via_new_bytes` exits 0.
- `cargo test bare_new_bytes_parses` exits 0.
- `cargo test propertyless_new_non_bytes_rejected` exits 0.
- `cargo test new_bytes_codegen_calls_bytes_new` exits 0 and asserts generated IR declares/calls `@bytes_new()`.
- `cargo test fmt_new_bytes_roundtrip` exits 0.
- `cargo test` exits 0.
- `cargo test --features integration` exits 0.
- `rg -n 'bytes_new\(\)|bytes_new:|import bytes_new' stdlib README.md test-projects --glob '*.{md,op}'` returns no live user-facing stale references except the allowlisted legacy compatibility test project `test-projects/bytes-empty-construct-legacy`; if a `docs/` directory exists at execution time, run the same grep against `docs` separately.

### Must Have
- `let buffer: Bytes = new Bytes` works.
- `let buffer = new Bytes` also works, because `new Bytes` must be a real `Bytes` expression and not rely on the left-hand type annotation only.
- Existing `bytes_new()` source continues to compile and run.
- Generated LLVM/runtime linkage still uses `@bytes_new()`.
- `new Type:` and `new Type.Variant:` behavior remains unchanged.
- Commits are made only at green checkpoints.

### Must NOT Have
- Must NOT rename or remove the C runtime symbol `bytes_new` in `runtime/opal_bytes.c` or `runtime/opal_runtime.h`.
- Must NOT remove `bytes_new` from typechecker/module-resolver/codegen registration in this plan.
- Must NOT generalize propertyless constructors beyond `Bytes`.
- Must NOT accept `new Person`, `new Message.Text`, `new Bytes:`, `new Bytes()`, or lowercase `new bytes`.
- Must NOT rewrite existing source calls to `bytes_new()` via formatter.
- Must NOT commit a failing RED-only state.
- Must NOT require human/manual QA for task completion.

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: TDD / RED-GREEN-REFACTOR using Rust unit tests plus integration e2e projects.
- QA policy: Every task has agent-executed scenarios.
- Evidence: `.sisyphus/evidence/task-N-slug.txt` and `.sisyphus/evidence/red-new-bytes.txt`.

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. <3 per wave (except final) = under-splitting.
> Extract shared dependencies as Wave-1 tasks for max parallelism.

Wave 1: Task 1 (hook/baseline discovery), Task 2 (legacy compatibility test project)
Wave 2: Task 3 (RED new syntax test project), Task 4 (parser/formatter), Task 5 (typechecker diagnostics), Task 6 (codegen lowering)
Wave 3: Task 7 (integration GREEN/refactor/implementation commit), Task 8 (docs update/docs commit)
Wave 4: Final Verification Wave F1-F4

### Dependency Matrix (full, all tasks)
- Task 1 blocks Tasks 2, 3, 7, 8 because commit/pre-commit commands must be known before commit checkpoints.
- Task 2 blocks Task 3 because legacy compatibility must be committed before the syntax migration begins.
- Task 3 blocks Tasks 4-6 because RED evidence must exist before implementation.
- Tasks 4, 5, and 6 can proceed in parallel after Task 3 but must be reconciled together.
- Task 7 is blocked by Tasks 3-6.
- Task 8 is blocked by Task 7 to avoid documenting unmerged syntax.
- Final Verification Wave is blocked by Tasks 1-8.

### Agent Dispatch Summary (wave → task count → categories)
- Wave 1 → 2 tasks → quick, quick
- Wave 2 → 4 tasks → quick, deep, deep, deep
- Wave 3 → 2 tasks → unspecified-high, writing
- Wave 4 → 4 review tasks → oracle, unspecified-high, unspecified-high, deep

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Discover commit hooks and baseline verification commands

  **What to do**: Inspect `.git/hooks/pre-commit`, `Makefile.toml`, and existing cargo/test commands without modifying source. Record the exact hook command behavior in `.sisyphus/evidence/task-1-hooks.txt`. Confirm whether hooks run tests, fmt, clippy, or custom checks. If `.git/hooks/pre-commit` is absent or not executable, record that and use `git diff --check`, `cargo test`, and task-specific tests as the green gates.
  **Must NOT do**: Do not edit hooks, git config, Makefile tasks, or source files.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: focused repo inspection and command verification.
  - Skills: [`git-master`] - Required because the task inspects commit/pre-commit workflow and later commit checkpoints depend on it.
  - Omitted: [`playwright`] - No browser/UI verification involved.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [2, 3, 7, 8] | Blocked By: []

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `.git/hooks/pre-commit` - local pre-commit hook to run before commits if executable.
  - Pattern: `Makefile.toml:259` - existing make task mentions formatting push string; inspect nearby tasks for relevant gates.
  - Test: `tests/integration_e2e/bytes_stdlib.rs` - later tests will use this harness.
  - Test: `tests/fmt_integration.rs` - formatter integration tests exist and must be considered for `new Bytes` formatting.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `.sisyphus/evidence/task-1-hooks.txt` exists and states whether `.git/hooks/pre-commit` exists, is executable, and which commands it runs.
  - [ ] `git status --short` output is captured before source/test changes.
  - [ ] `cargo test --features integration bytes_hex_roundtrip_compiles_and_runs` exits 0 or any pre-existing failure is captured with exact stderr in `.sisyphus/evidence/task-1-baseline-bytes.txt`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Hook discovery
    Tool: Bash
    Steps: Run `test -x .git/hooks/pre-commit && .git/hooks/pre-commit --help || true`, inspect `.git/hooks/pre-commit` if present, and write findings to `.sisyphus/evidence/task-1-hooks.txt`.
    Expected: Evidence file names exact hook status and commands; no tracked source files changed.
    Evidence: .sisyphus/evidence/task-1-hooks.txt

  Scenario: Baseline bytes e2e still works
    Tool: Bash
    Steps: Run `cargo test --features integration bytes_hex_roundtrip_compiles_and_runs`.
    Expected: Exit code 0, or captured pre-existing failure with no source changes.
    Evidence: .sisyphus/evidence/task-1-baseline-bytes.txt
  ```

  **Commit**: NO | Message: N/A | Files: [.sisyphus/evidence/task-1-hooks.txt, .sisyphus/evidence/task-1-baseline-bytes.txt]

- [x] 2. Add and commit legacy `bytes_new()` empty-construction test project

  **What to do**: Add `test-projects/bytes-empty-construct-legacy` following `test-projects/bytes-hex-roundtrip` layout. Program must import/use current `bytes_new()` and prove it creates an empty `Bytes` value. Add an integration test function `empty_bytes_via_bytes_new` to `tests/integration_e2e/bytes_stdlib.rs` that compiles/runs the project and asserts exact or substring output. Use expected stdout file with exactly `legacy length: 0\n`. Verify green, fix pre-commit issues, then commit.
  **Must NOT do**: Do not touch parser/typechecker/codegen for `new Bytes`. Do not update docs in this commit.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: add a small test fixture plus a harness assertion.
  - Skills: [`git-master`] - Required for the explicit commit checkpoint.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [3] | Blocked By: [1]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `test-projects/bytes-hex-roundtrip/opal.toml` - copy manifest style.
  - Pattern: `test-projects/bytes-hex-roundtrip/src/main.op` - follow import/entry/print style for Bytes stdlib.
  - Test: `tests/integration_e2e/bytes_stdlib.rs` - add `empty_bytes_via_bytes_new` near existing bytes e2e test.
  - API/Type: `stdlib/prelude.op` - current documented `bytes_new(): Bytes` signature and `.length` member behavior.
  - API/Type: `src/type_system/checker/bytes_builtins.rs` - current `bytes_new` builtin registration.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `test-projects/bytes-empty-construct-legacy/opal.toml` exists with name `bytes-empty-construct-legacy`.
  - [ ] `test-projects/bytes-empty-construct-legacy/src/main.op` uses `bytes_new()` and prints `legacy length: 0`.
  - [ ] `test-projects/bytes-empty-construct-legacy/expected/stdout.txt` is exactly `legacy length: 0\n`.
  - [ ] `cargo test --features integration empty_bytes_via_bytes_new` exits 0.
  - [ ] `cargo test` exits 0.
  - [ ] Pre-commit hook from Task 1 exits 0 or nonexistence is documented.
  - [ ] Commit created with message `test(bytes): cover legacy bytes_new empty construction`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Legacy empty Bytes happy path
    Tool: Bash
    Steps: Run `cargo test --features integration empty_bytes_via_bytes_new`.
    Expected: Exit code 0; stdout assertion verifies `legacy length: 0`.
    Evidence: .sisyphus/evidence/task-2-legacy-green.txt

  Scenario: Commit gate catches formatting/test issues
    Tool: Bash
    Steps: Run `git diff --check`, `cargo test`, the executable `.git/hooks/pre-commit` if present, then `git status --short`.
    Expected: Diff check, cargo tests, and hook pass; only intended test-project and integration harness files are staged/committed.
    Evidence: .sisyphus/evidence/task-2-commit-gate.txt
  ```

  **Commit**: YES | Message: `test(bytes): cover legacy bytes_new empty construction` | Files: [test-projects/bytes-empty-construct-legacy/**, tests/integration_e2e/bytes_stdlib.rs]

- [x] 3. Add new-syntax test project and capture RED failure evidence

  **What to do**: Add `test-projects/bytes-empty-construct-new-syntax` with `let buffer: Bytes = new Bytes`, expected stdout exactly `new syntax length: 0\n`, and an integration test function `empty_bytes_via_new_bytes` in `tests/integration_e2e/bytes_stdlib.rs`. Before implementing syntax support, run the targeted integration test and capture the expected failure to `.sisyphus/evidence/red-new-bytes.txt`. Do not commit while failing; continue to implementation tasks in the same working tree.
  **Must NOT do**: Do not change parser/typechecker/codegen before the RED run. Do not commit a failing RED-only state.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: small fixture and one failing test harness entry.
  - Skills: [] - No special skill required until commit checkpoint later.
  - Omitted: [`git-master`] - No commit in this RED-only task.

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [4, 5, 6] | Blocked By: [2]

  **References** (executor has NO interview context - be exhaustive):
  - Pattern: `test-projects/bytes-empty-construct-legacy/src/main.op` - same project shape, replace `bytes_new()` with `new Bytes`.
  - Test: `tests/integration_e2e/bytes_stdlib.rs` - add `empty_bytes_via_new_bytes` next to `empty_bytes_via_bytes_new`.
  - Parser: `src/parser/new_expression.rs` - expected current failure because parser requires colon/fields.

  **Acceptance Criteria** (agent-executable only):
  - [ ] New test project files exist under `test-projects/bytes-empty-construct-new-syntax/`.
  - [ ] `src/main.op` contains exactly the intended initialization pattern `let buffer: Bytes = new Bytes`.
  - [ ] `cargo test --features integration empty_bytes_via_new_bytes` exits nonzero before syntax implementation.
  - [ ] `.sisyphus/evidence/red-new-bytes.txt` captures the nonzero failure and includes parser/typechecker failure text.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: New syntax RED failure
    Tool: Bash
    Steps: Run `set -o pipefail; cargo test --features integration empty_bytes_via_new_bytes 2>&1 | tee .sisyphus/evidence/red-new-bytes.txt` before parser/typechecker/codegen edits.
    Expected: Command exits nonzero; evidence includes failure related to `new Bytes` parsing/typechecking/compilation.
    Evidence: .sisyphus/evidence/red-new-bytes.txt

  Scenario: Fixture shape check
    Tool: Bash
    Steps: Run `rg -n 'let buffer: Bytes = new Bytes|new syntax length: 0' test-projects/bytes-empty-construct-new-syntax tests/integration_e2e/bytes_stdlib.rs`.
    Expected: Matches show the fixture and harness use the requested syntax and expected output.
    Evidence: .sisyphus/evidence/task-3-fixture-check.txt
  ```

  **Commit**: NO | Message: N/A | Files: [test-projects/bytes-empty-construct-new-syntax/**, tests/integration_e2e/bytes_stdlib.rs, .sisyphus/evidence/red-new-bytes.txt]

- [x] 4. Implement parser and formatter support for Bytes-only propertyless constructor syntax

  **What to do**: Update parser handling so `new Bytes` can be parsed as a constructor expression with zero fields while preserving existing `new Type:` and `new Type.Variant:` parsing. Add parser tests for `let buffer: Bytes = new Bytes`, `let buffer = new Bytes`, and malformed `new Bytes()` / `new Bytes:` cases. Update formatter/printer tests so formatting preserves `new Bytes` and does not rewrite `bytes_new()` calls. If parser architecture requires parser-general acceptance of `new <Ident>`, ensure typechecker negative tests in Task 5 reject non-Bytes.
  **Must NOT do**: Do not accept `new Message.Text` without colon. Do not add parser-level support for paren constructor calls.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: parser and formatter changes risk broad syntax regressions.
  - Skills: [] - Use Serena/LSP navigation; no browser needed.
  - Omitted: [`git-master`] - No commit until Task 7.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7] | Blocked By: [3]

  **References** (executor has NO interview context - be exhaustive):
  - Parser: `src/parser/new_expression.rs` - primary change point; current grammar expects `new <Type>:` and field block.
  - Parser dispatch: `src/parser/expressions.rs` - ensure parse caller contract remains intact.
  - Formatter test: `tests/fmt_integration.rs` - add/extend round-trip or fixture coverage for `new Bytes`.
  - Existing syntax examples: use AST/grep for `new $TYPE:` before editing to ensure colon form still covered.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test bare_new_bytes_parses` exits 0.
  - [ ] `cargo test bare_new_bytes_inferred_let_parses` exits 0.
  - [ ] `cargo test new_bytes_parens_rejected` exits 0.
  - [ ] `cargo test empty_colon_new_bytes_rejected` exits 0 unless existing parser behavior already emits equivalent invalid-field diagnostics; evidence must name the diagnostic.
  - [ ] `cargo test fmt_new_bytes_roundtrip` exits 0.
  - [ ] Existing tests for `new Type:` / `new Type.Variant:` still pass.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Parser accepts requested syntax only
    Tool: Bash
    Steps: Run `cargo test bare_new_bytes_parses`, then `cargo test bare_new_bytes_inferred_let_parses`, then `cargo test new_bytes_parens_rejected`, then `cargo test empty_colon_new_bytes_rejected`.
    Expected: Exit code 0; rejection tests assert parse/type errors for invalid forms.
    Evidence: .sisyphus/evidence/task-4-parser.txt

  Scenario: Formatter preserves syntax
    Tool: Bash
    Steps: Run `cargo test fmt_new_bytes_roundtrip` and compare formatted output fixture/assertion.
    Expected: Output contains `new Bytes`; no `bytes_new()` rewrite occurs.
    Evidence: .sisyphus/evidence/task-4-formatter.txt
  ```

  **Commit**: NO | Message: N/A | Files: [src/parser/new_expression.rs, src/parser/expressions.rs if needed, tests/fmt_integration.rs or formatter fixtures]

- [x] 5. Implement typechecker support and negative diagnostics for propertyless `new Bytes`

  **What to do**: Update constructor typechecking so zero-field constructor expressions for bare `Bytes` return the existing nominal Bytes type. Use explicit Bytes-only gating for this plan. Add negative tests that `new Person`, `new Message.Text`, `new bytes`, and any non-Bytes propertyless constructor remain invalid. Add positive tests for explicit and inferred lets using `new Bytes`.
  **Must NOT do**: Do not remove `bytes_new` builtin registration. Do not make all nominal types propertylessly constructible.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: type inference and diagnostics can affect many compiler paths.
  - Skills: [] - Use Serena symbol navigation.
  - Omitted: [`git-master`] - No commit until Task 7.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7] | Blocked By: [3]

  **References** (executor has NO interview context - be exhaustive):
  - Typechecker: `src/type_system/checker/constructors.rs` - `type_check_constructor_expr` / constructor field validation.
  - Typechecker entry: `src/type_system/checker/expressions.rs` - expression typing integration.
  - Bytes registration: `src/type_system/checker/bytes_builtins.rs` - `BYTES_TYPE_NAME = "Bytes"` and `register_bytes_builtin("bytes_new", ...)`.
  - Resolver: `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs` - keep `bytes_new` standard symbol intact.
  - Tests: `src/type_system/tests.rs` or adjacent typechecker tests - add positive/negative coverage according to project organization.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test new_bytes_typechecks_as_bytes` exits 0.
  - [ ] `cargo test new_bytes_inferred_let_typechecks_as_bytes` exits 0.
  - [ ] `cargo test propertyless_new_person_rejected` exits 0.
  - [ ] `cargo test propertyless_new_variant_rejected` exits 0.
  - [ ] `cargo test propertyless_new_lowercase_bytes_rejected` exits 0.
  - [ ] `rg -n 'register_bytes_builtin\("bytes_new"|"bytes_new"' src/type_system/checker/bytes_builtins.rs src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs` still finds existing registration/symbol references.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Typechecker accepts Bytes-only constructor
    Tool: Bash
    Steps: Run `cargo test new_bytes_typechecks_as_bytes`, then `cargo test new_bytes_inferred_let_typechecks_as_bytes`.
    Expected: Exit code 0; tests assert resulting expression/binding type is Bytes.
    Evidence: .sisyphus/evidence/task-5-typechecker-positive.txt

  Scenario: Typechecker rejects non-Bytes propertyless constructors
    Tool: Bash
    Steps: Run `cargo test propertyless_new_person_rejected`, then `cargo test propertyless_new_variant_rejected`, then `cargo test propertyless_new_lowercase_bytes_rejected`.
    Expected: Exit code 0; tests assert clear diagnostics and no generic propertyless constructor support.
    Evidence: .sisyphus/evidence/task-5-typechecker-negative.txt
  ```

  **Commit**: NO | Message: N/A | Files: [src/type_system/checker/constructors.rs, src/type_system/checker/expressions.rs if needed, src/type_system/tests.rs or adjacent checker tests]

- [x] 6. Implement codegen lowering for `new Bytes` to existing `@bytes_new()` runtime symbol

  **What to do**: Update constructor expression codegen so zero-field `new Bytes` emits a no-argument call to the existing stdlib/runtime function `bytes_new`. Reuse the existing declaration path in `src/codegen/functions_stdlib.rs`; do not add a new runtime symbol. Add codegen tests proving the generated IR declares and calls `@bytes_new()` for `new Bytes`, and that existing `bytes_new()` call codegen remains unchanged.
  **Must NOT do**: Do not edit `runtime/opal_bytes.c` or `runtime/opal_runtime.h` except to inspect. Do not rename LLVM/runtime symbol to `Bytes_new`, `bytes_constructor`, or similar.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: LLVM codegen changes can silently miscompile if return types are wrong.
  - Skills: [] - No external docs needed; use local codegen patterns.
  - Omitted: [`frontend-ui-ux`] - No UI.

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [7] | Blocked By: [3]

  **References** (executor has NO interview context - be exhaustive):
  - Codegen: `src/codegen/expressions.rs` - constructor expression lowering path.
  - Codegen ADT helpers: `src/codegen/adts.rs` - preserve field-based constructor behavior.
  - Stdlib declarations: `src/codegen/functions_stdlib.rs` - existing `declare_stdlib_function` and `STDLIB_NAMES` include `bytes_new`.
  - Statement inference: `src/codegen/statements.rs` - known runtime/guard return type mappings include `bytes_new`; ensure `new Bytes` bindings are not inferred as `int64`.
  - Tests: `src/codegen/tests.rs` - existing tests assert `declare i8* @bytes_new()`.
  - Runtime ABI: `runtime/opal_bytes.c`, `runtime/opal_runtime.h` - inspect only; symbol must remain `bytes_new`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test new_bytes_codegen_calls_bytes_new` exits 0.
  - [ ] Generated IR test asserts both `declare i8* @bytes_new()` and `call i8* @bytes_new()` or repository-equivalent opaque pointer syntax.
  - [ ] `cargo test legacy_bytes_new_codegen_still_calls_bytes_new` exits 0 or existing equivalent test remains green.
  - [ ] `rg -n 'bytes_new' runtime/opal_bytes.c runtime/opal_runtime.h src/codegen/functions_stdlib.rs` confirms runtime and declaration symbol remain intact.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: New syntax emits existing runtime ABI
    Tool: Bash
    Steps: Run `cargo test new_bytes_codegen_calls_bytes_new`.
    Expected: Exit code 0; assertion confirms generated IR declares/calls `@bytes_new()`.
    Evidence: .sisyphus/evidence/task-6-codegen-new-syntax.txt

  Scenario: Legacy codegen compatibility remains
    Tool: Bash
    Steps: Run `cargo test legacy_bytes_new_codegen_still_calls_bytes_new` or the existing bytes_new codegen test filter recorded by `cargo test bytes_new`.
    Expected: Exit code 0; legacy source call still emits/calls `@bytes_new()`.
    Evidence: .sisyphus/evidence/task-6-codegen-legacy.txt
  ```

  **Commit**: NO | Message: N/A | Files: [src/codegen/expressions.rs, src/codegen/statements.rs if needed, src/codegen/functions_stdlib.rs only if declaration helper must be reused/exposed, src/codegen/tests.rs]

- [x] 7. Reconcile implementation, refactor, run GREEN verification, and commit feature

  **What to do**: Integrate Tasks 3-6 into one green feature state. Run targeted parser/typechecker/codegen/formatter tests, the new syntax integration test, the legacy integration test, `cargo test`, and `cargo test --features integration`. Refactor only within touched parser/typechecker/codegen/formatter/test code to remove duplication and clarify Bytes-only constructor handling. Commit all implementation and new-syntax test project changes.
  **Must NOT do**: Do not broaden scope into general constructor design. Do not touch docs in this commit except README files inside the new test projects. Do not remove legacy `bytes_new()` support.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: cross-cutting reconciliation, verification, and commit.
  - Skills: [`git-master`] - Required for staging, inspecting diff, committing, and handling pre-commit failures.
  - Omitted: [`playwright`] - No browser tests.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [8] | Blocked By: [4, 5, 6]

  **References** (executor has NO interview context - be exhaustive):
  - Plan guardrails: Must preserve `bytes_new` runtime ABI and source-level compatibility.
  - All changed files from Tasks 3-6.
  - Evidence: `.sisyphus/evidence/red-new-bytes.txt` - proves RED happened before GREEN.
  - Integration tests: `tests/integration_e2e/bytes_stdlib.rs` - must have both `empty_bytes_via_bytes_new` and `empty_bytes_via_new_bytes`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `cargo test --features integration empty_bytes_via_bytes_new` exits 0.
  - [ ] `cargo test --features integration empty_bytes_via_new_bytes` exits 0.
  - [ ] `cargo test bare_new_bytes_parses`, `cargo test new_bytes_typechecks_as_bytes`, `cargo test new_bytes_codegen_calls_bytes_new`, and `cargo test fmt_new_bytes_roundtrip` each exit 0.
  - [ ] `cargo test` exits 0.
  - [ ] `cargo test --features integration` exits 0.
  - [ ] `.sisyphus/evidence/red-new-bytes.txt` exists and predates GREEN evidence files.
  - [ ] `git diff --check` exits 0.
  - [ ] Pre-commit hook from Task 1 exits 0 or nonexistence is documented.
  - [ ] Commit created with message `feat(bytes): support new Bytes initialization`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: New syntax full GREEN
    Tool: Bash
    Steps: Run `cargo test --features integration empty_bytes_via_new_bytes`.
    Expected: Exit code 0; compiled project prints exactly `new syntax length: 0` according to harness assertion.
    Evidence: .sisyphus/evidence/task-7-new-syntax-green.txt

  Scenario: Regression and compatibility suite
    Tool: Bash
    Steps: Run `cargo test`, `cargo test --features integration`, `git diff --check`, and the pre-commit hook if present.
    Expected: All commands exit 0; no runtime ABI files modified except if explicitly inspected-only; commit succeeds.
    Evidence: .sisyphus/evidence/task-7-regression-gate.txt
  ```

  **Commit**: YES | Message: `feat(bytes): support new Bytes initialization` | Files: [test-projects/bytes-empty-construct-new-syntax/**, tests/integration_e2e/bytes_stdlib.rs, src/parser/**, src/type_system/**, src/codegen/**, tests/fmt_integration.rs or formatter fixtures, .sisyphus/evidence/red-new-bytes.txt if project convention allows evidence commits]

- [x] 8. Update live documentation and examples from `bytes_new` to `new Bytes`, verify, and commit docs

  **What to do**: Update live/current documentation so users see `new Bytes` as the preferred constructor. Always update `stdlib/prelude.op`. Review `PLAN.md`, `plan/bytes-stdlib-integration-plan.md`, `plan/bytes-type-plan.md`, and `stdlib-proposals/byte-buffer-type/dedicated-bytes-type/**`; edit only lines that are clearly current user-facing guidance, not historical planning records. Keep references to `bytes_new` in legacy compatibility tests and internal runtime/codegen docs/tests. Run docs grep allowlist verification and commit.
  **Must NOT do**: Do not remove `bytes_new` from runtime/typechecker/module resolver/codegen registration. Do not rewrite historical design history unless it claims to be current docs. Do not change test project semantics.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: focused documentation updates and consistency checks.
  - Skills: [`git-master`] - Required for docs commit.
  - Omitted: [`playwright`] - No browser docs preview required.

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [Final Verification] | Blocked By: [7]

  **References** (executor has NO interview context - be exhaustive):
  - Live docs: `stdlib/prelude.op` - must replace preferred `bytes_new(): Bytes` guidance with `new Bytes` examples.
  - Review docs: `PLAN.md`, `plan/bytes-stdlib-integration-plan.md`, `plan/bytes-type-plan.md` - update only current guidance/checklists, not historical notes.
  - Review proposal examples: `stdlib-proposals/byte-buffer-type/dedicated-bytes-type/proposal.md`, `file_io.op`, `manipulation.op` - already use `new Bytes:` in places; ensure no stale `bytes_new()` prose remains.
  - Compatibility fixture: `test-projects/bytes-empty-construct-legacy/src/main.op` - allowed to retain `bytes_new()`.

  **Acceptance Criteria** (agent-executable only):
  - [ ] `stdlib/prelude.op` contains `new Bytes` guidance and no preferred `bytes_new(): Bytes` user-facing constructor guidance.
  - [ ] `rg -n 'bytes_new\(\)|import bytes_new|bytes_new\s*:' stdlib README.md test-projects --glob '*.{md,op}'` returns only allowed legacy compatibility references or exits 1 with no matches; if `docs/` exists, run `rg -n 'bytes_new\(\)|import bytes_new|bytes_new\s*:' docs --glob '*.{md,op}'` and apply the same allowlist.
  - [ ] `rg -n 'new Bytes' stdlib/prelude.op test-projects/bytes-empty-construct-new-syntax stdlib-proposals/byte-buffer-type/dedicated-bytes-type --glob '*.{md,op}'` returns at least one current-doc match.
  - [ ] `cargo test --features integration empty_bytes_via_bytes_new` and `cargo test --features integration empty_bytes_via_new_bytes` each exit 0 after docs edits.
  - [ ] `git diff --check` exits 0.
  - [ ] Pre-commit hook from Task 1 exits 0 or nonexistence is documented.
  - [ ] Commit created with message `docs(bytes): prefer new Bytes initialization`.

  **QA Scenarios** (MANDATORY - task incomplete without these):
  ```
  Scenario: Docs no longer prefer bytes_new
    Tool: Bash
    Steps: Run `rg -n 'bytes_new\(\)|import bytes_new|bytes_new\s*:' stdlib README.md test-projects --glob '*.{md,op}'` and compare matches against the allowlist containing only legacy compatibility tests/internal docs. If `docs/` exists, run the same command against `docs` separately.
    Expected: No live user-facing docs prefer `bytes_new()`; allowed legacy references are explicitly recorded.
    Evidence: .sisyphus/evidence/task-8-docs-grep.txt

  Scenario: Docs change does not break Bytes tests
    Tool: Bash
    Steps: Run `cargo test --features integration empty_bytes_via_bytes_new`, then `cargo test --features integration empty_bytes_via_new_bytes`.
    Expected: Exit code 0 for both compatibility and new syntax projects.
    Evidence: .sisyphus/evidence/task-8-docs-tests.txt
  ```

  **Commit**: YES | Message: `docs(bytes): prefer new Bytes initialization` | Files: [stdlib/prelude.op, PLAN.md or plan/*.md only if current guidance is changed, stdlib-proposals/byte-buffer-type/dedicated-bytes-type/** only if stale user-facing `bytes_new` prose exists]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**
> **Never mark F1-F4 as checked before getting user's okay.** Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.
- [x] F1. Plan Compliance Audit — oracle
  - Verify every task acceptance criterion was met, all commits are green, and `bytes_new()` compatibility was preserved.
- [x] F2. Code Quality Review — unspecified-high
  - Inspect parser/typechecker/codegen/formatter changes for minimality, no over-generalized constructors, and no runtime ABI churn.
- [x] F3. Real Manual QA — unspecified-high (+ interactive_bash)
  - Run the built legacy and new-syntax projects, capture stdout, inspect artifacts with command-line tools, and verify exact expected output.
- [x] F4. Scope Fidelity Check — deep
  - Confirm no unrelated Bytes APIs, historical docs, or constructor semantics were changed beyond the plan.

## Commit Strategy
1. Commit 1 after Task 2 only: `test(bytes): cover legacy bytes_new empty construction`
2. Commit 2 after Task 7: `feat(bytes): support new Bytes initialization`
3. Commit 3 after Task 8: `docs(bytes): prefer new Bytes initialization`

Each commit checkpoint MUST run:
- `git status --short`
- `git diff --check`
- Relevant task-specific tests
- Repository pre-commit hook if present and executable: `.git/hooks/pre-commit`
- If hooks report issues, fix them and rerun before committing.

## Success Criteria
- Three commits exist with the messages above or equivalent repo-style messages.
- Legacy and new syntax integration tests both pass.
- `new Bytes` works with and without explicit left-hand type annotation.
- Negative syntax/typing cases are tested and rejected.
- Runtime ABI and `bytes_new` registrations remain intact.
- Live docs no longer present `bytes_new()` as the preferred public constructor.
