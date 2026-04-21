# Types File Imports — Enforce `.types.op` Separation & End-to-End Flow

## TL;DR

> **Quick Summary**: Make type-imports from `.types.op` modules work end-to-end across a multi-file project, and enforce strict file-role separation: `type` declarations MUST live in `.types.op`; value/function/`entry` declarations MUST NOT appear in `.types.op`. Most infrastructure already exists — the gap is a validation pass, two new `TypeError` variants, codegen skip for type-only modules, and integration fixtures.
>
> **Deliverables**:
> - New validator in `src/module_loader.rs` (or dedicated `src/compiler/validation.rs`) plus wiring in `src/compiler.rs:compile_project`, `compile_to_module` (signature extended to accept `source_path: &Path`; 23 call sites updated across 4 files), `compile_program` (signature extended to thread the path through), AND `src/app.rs:run_check_command` (the `opal check` CLI path, which bypasses `compile_to_module` and runs its own pipeline).
> - Two new `TypeError` variants in `src/type_system/errors.rs`: `TypeDeclarationOutsideTypesFile`, `NonTypeDeclarationInTypesFile`.
> - Codegen skip for type-only modules in `compile_project`.
> - Three success test projects: `test-projects/import-types-basic/`, `test-projects/import-types-aliased/`, `test-projects/import-types-multiple/`.
> - Two compile-fail test projects: `test-projects/type-in-regular-file-fail/`, `test-projects/value-in-types-file-fail/`.
> - Unit tests + integration tests (success + failure) under `tests/integration_e2e/`.
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: T1 (helper + error variants) → T2 (validator unit tests RED→GREEN) → T5 (compile_project wiring) → T6/T7 (fixtures) → T8/T9 (integration tests) → F1-F4 (final verification)

---

## Context

### Original Request

> "Please add support for importing types files as specified in the language spec. Use TDD with red-green-refactor and create test projects to try out the types file importing. Users should be able to import types from types files and use them in their regular opalescent projects. Enforce that types should stay in .types.op files, and that they aren't allowed in regular .op files (check the language spec first though to make sure that's intended)"

### Interview Summary

**Key Discussions**:
- Spec verified: `language-spec/requirements/modules.md` lines 30-33, 41 describe `import PrimeFactorization from ./nums.types` and `import type User, Address from ./models.types`. README explicitly states: "Type definitions must live in files ending in `.types.op`".
- User chose **STRICT** separation: `.types.op` files may NOT contain `let`/`entry`/function declarations.
- User chose **TDD + internal unit tests + test-project integration fixtures** (both, not one-or-the-other).

**Research Findings** (file:line citations):
- Parser already handles `import type X from ./path.types` (`src/parser/declarations.rs:622-856`). Parser tests at `src/parser/tests.rs:4350-4465` already cover type imports.
- Module loader already resolves `./foo.types` → `foo.types.op` with passing test `resolve_import_path_types_module_suffix` (`src/module_loader.rs:48-61`).
- AST already has `ImportItem::Type { name, alias }` (`src/ast.rs:898-926`).
- `is_type_import` flag already computed (`src/module_loader.rs:286-295`).
- Type checker cross-file resolution treats `Type` imports identically to `Named` (`src/type_system/checker/module_checking.rs:63-147`).
- `compile_project` orchestrates multi-file builds (`src/compiler.rs:633-806`) and **currently emits an object file for every module** including `.types.op` (line 800). This must be guarded.
- Validation hook point exists: `validate_entry_declarations_for_module` at `src/compiler.rs:648` — the new validator chains here.
- Stdlib shortcut: `module_loader.rs:37` returns sentinel paths for `standard`/`math` — validator must not recurse into these.
- Integration test harness: `tests/integration_e2e/tests.rs` uses `compile_project(&project_dir, &temp_dir)`; `tests/integration_e2e/compile_failures.rs` pattern-matches on `CompileError::Report.entries()` for `TypeError` variants. `cargo test --features integration`.
- Reference compile-fail fixture layout: `test-projects/ref-compile-fail/`.
- Multi-file reference: `test-projects/multi-file/` (value imports only; no `.types.op` in any existing test project).
- Only `.types.op` file in-repo: `language-spec/types_example.types.op` — some parser ecosystem tests consuming it are `#[ignore]` due to colon-block type body parsing limitations. **Fixtures must use brace-form or confirmed-parseable colon-form type bodies.**

### Self Gap Analysis (Metis subagent was unavailable; performed inline)

- **Q: Can the project entry file be a `.types.op`?** — No. `compile_project` hardcodes `src/main.op` (`src/compiler.rs:637`). Validator reinforces by rejecting `entry` in `.types.op` and rejecting `type` in `src/main.op`. Guardrail: main.op is always `.op` so no special case.
- **Q: Can a `.types.op` import another `.types.op`?** — Yes, spec-compatible. Validator does not restrict import direction.
- **Q: Can a `.types.op` import from a `.op` file?** — Not forbidden by spec; no restriction in validator. (Type aliases may reference types transitively; but types files typically don't import values.) Out of scope for enforcement in this plan.
- **Q: What about `import type` from a `.op` file (not `.types.op`)?** — Spec doesn't forbid this explicitly. Keep permissive: `import type` resolves by name like any named import. Not restricted.
- **Q: What if `main.op` contains only imports + `entry`? Does it count as non-type-only?** — Yes. Never tag `main.op` as type-only. Validator only classifies by filename suffix.
- **Q: Does codegen currently skip `Decl::Type`?** — Codegen drops type declarations, but compile_project unconditionally emits one `.o` per module. For a pure `.types.op` the `.o` would be empty (or nearly so). Linker may handle empty objects; still, skipping is cleaner and faster.
- **Q: Does the `check` CLI path need enforcement?** — `opal check <file.op>` runs lex+parse+typecheck. Single-file check path `compile_program` must also apply validation so `opal check src/models.types.op` correctly rejects a stray `let`.
- **Q: Hot-reload / ABI guard interaction?** — Out of scope; hot-reload operates on compiled objects post-codegen.
- **Q: Will existing `#[ignore]`d ecosystem tests for `types_example.types.op` be affected?** — They should remain `#[ignore]`d (parser gap is separate). Do not touch.

### Decisions Locked

- **DEC-1**: Enforce `Decl::Type` forbidden in `.op` files (spec: types MUST live in `.types.op`).
- **DEC-2**: Enforce `Decl::Let` / function decls / `entry` forbidden in `.types.op` files (user: strict).
- **DEC-3**: Validator runs once per module after parsing, before type-checking. Integrated into THREE hooks: `compile_project` (per-module after parse), `compile_to_module` (single-file path — requires extending signature to accept `source_path: &Path`, plus updating all 23 call sites in `src/compiler.rs`, `src/codegen/tests.rs`, `src/errors/tests.rs`, `tests/integration_e2e/tests.rs`; `compile_program` then threads its own new `source_path` arg through), AND `src/app.rs:run_check_command` (the `opal check` CLI path, which runs its own lex/parse/typecheck and bypasses `compile_to_module`).
- **DEC-4**: Both `import X from ./x.types` and `import type X from ./x.types` remain valid (parser already supports both; no change).
- **DEC-5**: Codegen skip for type-only modules (modules whose file ends in `.types.op`). Nice-to-have as final task; plan includes it.
- **DEC-6**: New `TypeError` variants, surfaced via `CompileError::Type` (not `CompileError::Report`) to match existing patterns. Integration tests match pattern: either `CompileError::Type(TypeError::TypeDeclarationOutsideTypesFile { .. })` or search report entries.
- **DEC-7**: Fixtures use brace-form or simple colon-form type bodies known to parse cleanly (see T6 reference to `simple_quiz` and existing parser test cases).
- **DEC-8**: Stdlib paths (`__stdlib__/standard`, `__stdlib__/math`) bypass validator.

---

## Work Objectives

### Core Objective

Enable end-to-end type imports from `.types.op` files AND enforce strict file-role separation as mandated by the language spec. Ship with TDD tests and integration fixtures that demonstrate both happy and failing paths.

### Concrete Deliverables

- `src/module_loader.rs` (or new `src/validation.rs`): `is_types_file(path: &Path) -> bool` helper + `validate_module_file_role(path, program) -> Result<(), TypeError>` validator.
- `src/type_system/errors.rs`: two new `TypeError` variants with miette diagnostics.
- `src/compiler.rs`: extend `compile_to_module` signature to accept `source_path: &Path` and invoke validator internally between parse and type-check; extend `compile_program` signature to accept `source_path: &Path` and thread it through; invoke validator in `compile_project` per-module after parse; skip codegen for type-only modules in `compile_project`.
- `src/app.rs`: update both `compile_program` call sites (`:186`, `:216`) to pass source path; wire validator directly into `run_check_command` between parse and type-check, using existing `report.extend_type_errors → render_report → Err(1)` idiom.
- `src/codegen/tests.rs`, `src/errors/tests.rs`, `tests/integration_e2e/tests.rs`: update all `compile_to_module` call sites (18 across these files, plus 4 in `src/compiler.rs` own tests, plus 1 production call from `compile_program`) to pass a `Path` arg. Use `Path::new("test.op")` for test sites.
- `test-projects/import-types-basic/` — happy path, single type import.
- `test-projects/import-types-aliased/` — `import type X as Y`.
- `test-projects/import-types-multiple/` — `import type A, B, C`.
- `test-projects/type-in-regular-file-fail/` — compile-fail fixture.
- `test-projects/value-in-types-file-fail/` — compile-fail fixture.
- Unit tests for validator in `src/` (TDD red→green).
- Integration tests in `tests/integration_e2e/tests.rs` (success) and `tests/integration_e2e/compile_failures.rs` (failures).
- Inline doctest + docstring updates for `is_types_file`.

### Definition of Done

- [ ] `cargo build` succeeds.
- [ ] `cargo test` (unit) passes — including new validator unit tests.
- [ ] `cargo test --features integration` passes — including three new success tests and two new failure tests.
- [ ] Running the compiled binary from `import-types-basic` prints the expected string.
- [ ] `cargo clippy --all-targets` produces no new warnings.
- [ ] Previously-ignored tests remain ignored (no accidental regressions).

### Must Have

- Type imports from `.types.op` work at parse + type-check + codegen + link + run for at least one end-to-end fixture.
- `.types.op` enforcement: non-type decls rejected with new `TypeError::NonTypeDeclarationInTypesFile`.
- `.op` enforcement: `type` decls rejected with new `TypeError::TypeDeclarationOutsideTypesFile`.
- Single-file check (`compile_program`) ALSO enforces (so `opal check src/foo.types.op` works via this path).
- `opal check` CLI path (`src/app.rs:run_check_command`) ALSO enforces — it bypasses `compile_program` so must wire the validator directly.
- Unit tests for the validator (TDD, covering both positive and negative cases).
- Integration tests for success AND failure fixtures.

### Must NOT Have (Guardrails)

- Do NOT modify `language-spec/types_example.types.op` or un-`#[ignore]` any existing ecosystem tests — parser gaps there are separate work.
- Do NOT add parser support for `import *` glob imports or per-item `type` mixing (`import A, type B from …`) — both explicitly out of scope.
- Do NOT change the stdlib (`standard`, `math`) resolution path — stdlib must bypass the validator.
- Do NOT restrict imports by direction (e.g., do NOT forbid `.types.op` importing `.op` or vice-versa beyond the spec's explicit rules) — only validate declarations within a module, not its imports.
- Do NOT introduce package-level (`@scope/name`) support — still returns `PackageImportNotSupported`.
- Do NOT rename `PathBuf`/`Path` conventions or restructure `compile_project` beyond adding the validator call and codegen skip.
- Do NOT over-abstract: no trait-based validator plugin system; a single function is sufficient.
- Do NOT add documentation comments wider than the existing codebase's style (`## … ##` conventions).
- Do NOT introduce `as any`-style casts or `TODO:` comments.

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — all verification is agent-executed.

### Test Decision

- **Infrastructure exists**: YES (Rust `cargo test` + `--features integration` flag).
- **Automated tests**: YES (TDD for validator) + YES (integration fixtures after).
- **Framework**: `cargo test` + `cargo test --features integration`.
- **TDD cycle**: each validator task follows RED (failing unit/integration test) → GREEN (minimal code) → REFACTOR (cleanup, move helpers, docstrings).

### QA Policy

- **CLI/build tool**: Use `Bash` for `cargo build`, `cargo test`, `cargo test --features integration`, and direct binary invocation of generated test project executables.
- **Library-level**: Use `cargo test -p opalescent <test_name> -- --exact` for unit tests.
- **Integration**: Use `cargo test --features integration <test_name> -- --exact --nocapture` to capture stdout of compiled test-project binaries.
- Evidence captured to `.sisyphus/evidence/task-{N}-{scenario-slug}.txt` (stdout + exit code) or `.log` for test output.

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundations, MAX PARALLEL):
├── T1: is_types_file helper + 2 new TypeError variants (errors.rs + module_loader.rs) [quick]
├── T2: Validator unit tests (RED — write tests that fail) [quick]
├── T6a: Fixture scaffolding — import-types-basic [quick]
├── T6b: Fixture scaffolding — import-types-aliased [quick]
├── T6c: Fixture scaffolding — import-types-multiple [quick]
├── T7a: Fixture scaffolding — type-in-regular-file-fail [quick]
└── T7b: Fixture scaffolding — value-in-types-file-fail [quick]

Wave 2 (After Wave 1 — implementation + wiring):
├── T3: Implement validator function (GREEN — make unit tests pass) [depends: T1, T2]  [unspecified-high]
├── T4: Wire validator into single-file paths: extend compile_to_module + compile_program signatures, update 23 call sites, wire run_check_command CLI path [depends: T3; blocks T5 (both modify src/compiler.rs)]  [unspecified-high]
├── T5: Wire validator into compile_project (multi-file path) + codegen skip for type-only modules [depends: T3]  [unspecified-high]
└── T10: Update docstrings + ensure clippy clean [depends: T3]  [quick]

Wave 3 (After Wave 2 — integration tests):
├── T8a: Integration success test — import-types-basic [depends: T5, T6a]  [quick]
├── T8b: Integration success test — import-types-aliased [depends: T5, T6b]  [quick]
├── T8c: Integration success test — import-types-multiple [depends: T5, T6c]  [quick]
├── T9a: Integration failure test — type-in-regular-file-fail [depends: T5, T7a]  [quick]
└── T9b: Integration failure test — value-in-types-file-fail [depends: T5, T7b]  [quick]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high)
└── F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: T1 → T2 → T3 → T5 → T8a/T9a → F1-F4 → user okay
Parallel Speedup: ~55% vs sequential
Max Concurrent: 7 (Wave 1)
```

### Dependency Matrix

- **T1**: — / Blocks: T2, T3
- **T2**: T1 / Blocks: T3
- **T3**: T1, T2 / Blocks: T4, T5, T10
- **T4**: T3 / Blocks: T5 (both touch `src/compiler.rs`; must serialize), single-file regression check inside T10 verification
- **T5**: T3 / Blocks: T8a, T8b, T8c, T9a, T9b
- **T6a/b/c**: — / Blocks: T8a/b/c respectively
- **T7a/b**: — / Blocks: T9a/b respectively
- **T8a/b/c**: T5 + respective T6 / Blocks: F1-F4
- **T9a/b**: T5 + respective T7 / Blocks: F1-F4
- **T10**: T3 / Blocks: F1-F4

### Agent Dispatch Summary

- **Wave 1**: **7** — T1 → `quick`, T2 → `quick`, T6a/b/c → `quick`, T7a/b → `quick`
- **Wave 2**: **4** — T3 → `unspecified-high`, T4 → `unspecified-high`, T5 → `unspecified-high` (sequential with T4), T10 → `quick`
- **Wave 3**: **5** — T8a/b/c → `quick`, T9a/b → `quick`
- **FINAL**: **4** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. Add `is_types_file` helper + two new `TypeError` variants

  **What to do**:
  - In `src/module_loader.rs`: add `pub fn is_types_file(path: &Path) -> bool` that returns `true` iff the path's filename ends with `.types.op` (case-insensitive check of the `.op` extension combined with a `.types` stem suffix). Add a rustdoc comment with example. Export via existing module.
  - In `src/type_system/errors.rs`: add two new variants to the `TypeError` enum (near `ModuleNotFound`, ~line 750-764):
    ```rust
    /// `type` declaration found outside a `.types.op` file.
    #[error("type declaration '{type_name}' is not allowed in '{file_path}'")]
    #[diagnostic(
        code(opalescent::type_system::type_declaration_outside_types_file),
        help("Move this type to a file ending in .types.op — the language spec requires type declarations to live in .types.op files")
    )]
    TypeDeclarationOutsideTypesFile {
        type_name: String,
        file_path: String,
        #[label("type declaration not allowed here")]
        span: SourceSpan,
    },

    /// Non-type declaration found inside a `.types.op` file.
    #[error("'{decl_kind}' declaration '{decl_name}' is not allowed in types file '{file_path}'")]
    #[diagnostic(
        code(opalescent::type_system::non_type_declaration_in_types_file),
        help(".types.op files may only contain type declarations — move this declaration to a regular .op file")
    )]
    NonTypeDeclarationInTypesFile {
        decl_kind: String,    // "let" | "entry" | "function"
        decl_name: String,
        file_path: String,
        #[label("not allowed in .types.op file")]
        span: SourceSpan,
    },
    ```
  - Ensure enum ordering/comma punctuation matches existing style.

  **Must NOT do**:
  - Do NOT touch stdlib resolution or `PackageImportNotSupported`.
  - Do NOT rename existing variants.
  - Do NOT add extra optional fields.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single-file additions to stable enums; no cross-cutting logic.
  - **Skills**: none required.
  - **Skills Evaluated but Omitted**: `git-master` — overkill for one commit.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T2, T6a-c, T7a-b)
  - **Blocks**: T2 (tests reference variants), T3
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/type_system/errors.rs:723-764` — Existing `EntryNotInMainModule`, `ModuleNotFound`, `PackageImportNotSupported` variants show the required miette attribute shape (`#[error]`, `#[diagnostic(code, help)]`, `#[label]`, `SourceSpan`).
  - `src/module_loader.rs:48-61` — Existing logic that detects `.types` extension; mirror that pattern for `is_types_file` but check for the combined `.types.op` suffix (filename, not extension).

  **API/Type References**:
  - `miette::Diagnostic` derive and `miette::SourceSpan` (already imported at `src/type_system/errors.rs` top).

  **Test References**:
  - `src/module_loader.rs` existing test `resolve_import_path_types_module_suffix` shows where/how unit tests are placed — same test module can host `is_types_file` tests.

  **WHY Each Reference Matters**:
  - `EntryNotInMainModule` is the closest analog — both variants carry a `file_path` string + span, both emit a "move it elsewhere" help. Copy-paste the shape, then adjust fields.
  - The stem-trimming code at lines 52-54 shows the idiomatic way to detect `.types` in this codebase.

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds.
  - [ ] `rg "TypeDeclarationOutsideTypesFile|NonTypeDeclarationInTypesFile" src/type_system/errors.rs` returns two matches.
  - [ ] `rg "pub fn is_types_file" src/module_loader.rs` returns one match.
  - [ ] New variants appear in `TypeError`'s Debug output when constructed in a test.

  **QA Scenarios**:

  ```
  Scenario: Helper returns true for .types.op paths and false otherwise
    Tool: Bash (cargo test)
    Preconditions: Code compiles after T1.
    Steps:
      1. Add a smoke unit test within src/module_loader.rs test module (or inline #[cfg(test)] block) that asserts:
         - is_types_file(Path::new("foo.types.op")) == true
         - is_types_file(Path::new("foo.op")) == false
         - is_types_file(Path::new("foo.types")) == false (no .op suffix)
         - is_types_file(Path::new("dir/sub/models.types.op")) == true
      2. Run: cargo test module_loader::tests::is_types_file -- --nocapture
    Expected Result: Test passes; output shows "test result: ok".
    Failure Indicators: Assertion failure or compile error.
    Evidence: .sisyphus/evidence/task-1-is-types-file-helper.txt

  Scenario: New error variants compile and render via miette
    Tool: Bash (cargo test)
    Preconditions: Variants added to TypeError.
    Steps:
      1. Add unit test that constructs a TypeDeclarationOutsideTypesFile variant and formats it via {:?}.
      2. Run: cargo test type_system::errors -- --nocapture
    Expected Result: No panic; variant can be constructed and formatted.
    Evidence: .sisyphus/evidence/task-1-error-variants.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-1-is-types-file-helper.txt`
  - [ ] `task-1-error-variants.txt`

  **Commit**: Groups with T2, T3, T10 — see Commit Strategy.

- [x] 2. Write validator unit tests (RED phase)

  **What to do**:
  - Add a new test module (or extend existing) at `src/module_loader.rs` bottom, behind `#[cfg(test)] mod tests`.
  - Write unit tests that invoke `validate_module_file_role(path: &Path, program: &Program) -> Result<(), TypeError>` (this function does NOT yet exist; tests fail to compile until T3). Tests to include:
    1. `.op` file with only `entry` + `let` + function decls → returns `Ok(())`.
    2. `.op` file with a `type` decl → returns `Err(TypeDeclarationOutsideTypesFile { .. })`; assert `type_name` field matches the declared type name.
    3. `.types.op` file with only `type` decls → returns `Ok(())`.
    4. `.types.op` file with a `let` decl → returns `Err(NonTypeDeclarationInTypesFile { decl_kind: "let", .. })`.
    5. `.types.op` file with an `entry` decl → returns `Err(NonTypeDeclarationInTypesFile { decl_kind: "entry", .. })`.
    6. `.types.op` file with a function decl (`let` binding a `f(...) =>`) → returns `Err(NonTypeDeclarationInTypesFile { decl_kind: "let" or "function", .. })` — decide classification in T3 and update test to match.
    7. Empty `.op` file → returns `Ok(())`.
    8. Empty `.types.op` file → returns `Ok(())`.
  - Parse fixture sources via existing test-utility (e.g., `Parser::new(Lexer::new(src).tokens())` — mirror existing unit tests in the file).
  - Tests MUST fail to compile (RED) because the function does not exist yet. This is the intended TDD red state.

  **Must NOT do**:
  - Do NOT mark tests `#[ignore]`.
  - Do NOT implement the validator yet (save for T3).
  - Do NOT reach into `compile_project` integration here — that's T5.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Pure test-writing, mechanical.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T1, T6, T7)
  - **Parallel Group**: Wave 1
  - **Blocks**: T3
  - **Blocked By**: T1 (tests reference new error variants)

  **References**:

  **Pattern References**:
  - `src/module_loader.rs` (look at existing `#[cfg(test)]` block near EOF) — parse-fixture helpers and assertion style.
  - `src/parser/tests.rs:4350-4465` — how parser tests construct `Program` from source strings.

  **API/Type References**:
  - `crate::ast::{Decl, Program, ImportItem}` — decl variants to inspect.
  - `crate::ast::Decl::Type`, `Decl::Let`, `Decl::EntryPoint` (or equivalent — confirm exact variant names in T1/T3 inspection of `src/ast.rs:788-926`).

  **WHY Each Reference Matters**:
  - Copy the fixture-parsing helper from `module_loader.rs` tests to avoid reinventing; reuse the same `Lexer → Parser → Program` invocation to keep tests idiomatic.

  **Acceptance Criteria**:

  - [ ] File compiles only after T3 lands; tests are present and named per `validate_*_allows_*` / `validate_*_rejects_*` scheme.
  - [ ] Initially: `cargo test module_loader::tests::validate` shows compile error "function not found" (RED confirmed).
  - [ ] After T3: `cargo test module_loader::tests::validate` → all 8 scenarios pass.

  **QA Scenarios**:

  ```
  Scenario: Red state confirmed before T3
    Tool: Bash (cargo test)
    Preconditions: T1 merged, T3 not yet merged.
    Steps:
      1. Run: cargo test module_loader::tests::validate 2>&1 | head -40
    Expected Result: Compile error mentioning unresolved function `validate_module_file_role`.
    Evidence: .sisyphus/evidence/task-2-red-state.txt

  Scenario: All 8 validator unit tests pass after T3
    Tool: Bash (cargo test)
    Preconditions: T3 merged.
    Steps:
      1. Run: cargo test module_loader::tests::validate -- --nocapture
    Expected Result: 8 tests, 0 failures; output contains "test result: ok. 8 passed".
    Evidence: .sisyphus/evidence/task-2-green-state.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-2-red-state.txt` (compile error captured pre-T3)
  - [ ] `task-2-green-state.txt` (all passing post-T3)

  **Commit**: Groups with T1, T3, T10.

- [x] 3. Implement `validate_module_file_role` (GREEN phase)

  **What to do**:
  - In `src/module_loader.rs`, add:
    ```rust
    /// Validate that a module's declarations match its file-role.
    ///
    /// - `.types.op` files may contain ONLY `type` declarations (and `import` declarations).
    /// - Other `.op` files must NOT contain `type` declarations.
    ///
    /// # Errors
    /// Returns `TypeError::TypeDeclarationOutsideTypesFile` or
    /// `TypeError::NonTypeDeclarationInTypesFile` on the first offending declaration.
    pub fn validate_module_file_role(path: &Path, program: &Program) -> Result<(), TypeError> { ... }
    ```
  - Logic:
    - Skip validation for stdlib sentinel paths (`path` starts with `__stdlib__/`). Return `Ok(())` immediately.
    - If `is_types_file(path)`:
      - Iterate `program.declarations`. For each non-`Import`, non-`Type` decl: return `Err(NonTypeDeclarationInTypesFile { decl_kind, decl_name, file_path, span })`.
        - `decl_kind`: `"let"`, `"entry"`, `"function"` — classify by inspecting `Decl::Let`, `Decl::EntryPoint`, and any function-typed `let` (`Decl::Let` whose initializer is `Expr::Function`). Use `"let"` for value `let`s and `"function"` for `let` bindings whose initializer is a function literal (simpler: use `"let"` for all non-`type` `let`s — T2 test-6 adjusts to match).
        - `decl_name`: read from the `Decl` variant's `name` field.
        - `span`: read from the `Decl` variant's span field (confirm exact accessor in `src/ast.rs:788-926`).
    - Else (regular `.op`):
      - Iterate declarations. For each `Decl::Type { name, span, .. }` (or whatever the variant shape is), return `Err(TypeDeclarationOutsideTypesFile { type_name: name, file_path, span })`.
    - Return `Ok(())` if no violations.
  - Classification rule for T2 scenario 6: all function-typed `let`s classified as `"let"` (keep simple). Update T2 test to expect `decl_kind: "let"`.
  - `file_path`: use `path.display().to_string()`.
  - Add rustdoc on the function and on `is_types_file`.

  **Must NOT do**:
  - Do NOT validate imports (import statements are always OK in both file types).
  - Do NOT validate stdlib sentinel modules.
  - Do NOT iterate nested decls — only top-level.
  - Do NOT introduce trait-based abstractions.
  - Do NOT change `Program` or `Decl` shapes.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires careful pattern-matching across the `Decl` enum, proper span plumbing, and adherence to existing error-construction idioms.
  - **Skills**: none required beyond standard Rust.

  **Parallelization**:
  - **Can Run In Parallel**: NO (must land before T4, T5, and T2 green state)
  - **Parallel Group**: Wave 2
  - **Blocks**: T4, T5, T10
  - **Blocked By**: T1, T2

  **References**:

  **Pattern References**:
  - `src/module_loader.rs:277-297` (`parse_module`) — shows iteration over `program.declarations` and the `is_type_import` pattern; mirror the iterator style.
  - `src/compiler.rs:validate_entry_declarations_for_module` (search for the function definition) — reference for how a project-level validator inspects declarations and returns `TypeError`.

  **API/Type References**:
  - `src/ast.rs:780-830` — exact `Decl` enum variant names and span field names. Confirm:
    - `Decl::Type { name, span, .. }`
    - `Decl::Let { name, span, value, .. }` or similar
    - `Decl::Import { source, items, .. }`
    - Whether a dedicated `Decl::EntryPoint` exists or whether `entry` is a flag on `Decl::Let`.
  - `crate::token::Span` — conversion to `miette::SourceSpan` via existing helper `TypeError::span_from_span(span)`.

  **Test References**:
  - `src/module_loader.rs` existing unit test module — how `resolve_import_path` tests construct fixtures and assert error variants.

  **WHY Each Reference Matters**:
  - `validate_entry_declarations_for_module` is the closest existing validator; copy its shape (match declarations, return on first error).
  - Span conversion helper `span_from_span` is already the codebase's standard.

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds.
  - [ ] All 8 T2 unit tests pass.
  - [ ] `validate_module_file_role` has rustdoc with `# Errors` section.
  - [ ] `is_types_file` is pub-exported alongside.
  - [ ] Stdlib path short-circuit verified by a unit test.

  **QA Scenarios**:

  ```
  Scenario: Validator accepts valid .op and rejects type decl in .op
    Tool: Bash (cargo test)
    Preconditions: T1, T2 merged.
    Steps:
      1. Run: cargo test module_loader::tests::validate -- --nocapture
    Expected Result: 8 tests pass.
    Evidence: .sisyphus/evidence/task-3-validator-unit-tests.txt

  Scenario: Stdlib bypass
    Tool: Bash (cargo test)
    Preconditions: T3 merged.
    Steps:
      1. Add inline test: validate_module_file_role(Path::new("__stdlib__/standard"), &any_program) == Ok(()).
      2. Run: cargo test module_loader::tests::validate_stdlib_bypass
    Expected Result: Test passes.
    Evidence: .sisyphus/evidence/task-3-stdlib-bypass.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-3-validator-unit-tests.txt`
  - [ ] `task-3-stdlib-bypass.txt`

  **Commit**: Groups with T1, T2, T10.

- [x] 4. Wire validator into single-file paths (`compile_to_module` + `compile_program` + CLI `run_check_command`)

  > **IMPORTANT (Momus Round 2 finding)**: `compile_program` at `src/compiler.rs:590` delegates lex+parse+typecheck to `compile_to_module` (line 124-234), which returns only an LLVM `Module` — there is NO `Program` AST bound in `compile_program`'s scope to validate against. The ONLY clean way to wire the validator into the single-file path is to extend `compile_to_module`'s signature to accept `source_path: &Path` and invoke the validator internally between the parse step (line 150) and the type-check step (line 152).
  >
  > AND `src/app.rs:run_check_command` (line 523) runs its OWN lex/parse/typecheck pipeline and bypasses `compile_to_module` entirely, so it must be wired separately.

  **What to do**:

  **Step 1 — Extend `compile_to_module` signature** (`src/compiler.rs:124-234`):
  - Change signature from:
    ```rust
    pub fn compile_to_module<'context>(
        context: &'context Context,
        source: &str,
    ) -> Result<Module<'context>, (CompilationErrorReport, String)>
    ```
    to:
    ```rust
    pub fn compile_to_module<'context>(
        context: &'context Context,
        source_path: &Path,
        source: &str,
    ) -> Result<Module<'context>, (CompilationErrorReport, String)>
    ```
  - Between the `let Some(program) = program_option else { ... };` block (line 144-150) and `let mut checker = TypeChecker::new();` (line 152), insert validator invocation:
    ```rust
    if let Err(role_error) = crate::module_loader::validate_module_file_role(source_path, &program) {
        report.extend_type_errors(vec![role_error]);
        return Err((report, normalized_source));
    }
    ```
  - This mirrors the existing error-accumulation idiom used for type-checker errors (line 153-162).

  **Step 2 — Extend `compile_program` signature to thread the path through** (`src/compiler.rs:590`):
  - Change signature from `compile_program(source: &str, output_dir: &Path)` to `compile_program(source_path: &Path, source: &str, output_dir: &Path)`.
  - At line 594, update the call to `compile_to_module(&context, source_path, source)`.

  **Step 3 — Update ALL 23 `compile_to_module` call sites** to pass a path:
  - `src/compiler.rs:594` — passes `source_path` from `compile_program`'s new arg (see Step 2).
  - `src/compiler.rs:819, 841, 873, 895` — 4 `#[cfg(test)]` call sites. Pass `Path::new("test.op")` (or `Path::new("test.types.op")` where relevant) as the second arg. (Add `use std::path::Path;` at the top of the test module if not already imported.)
  - `src/codegen/tests.rs:700, 1222, 1260, 1296, 1328, 1360, 1392, 1425, 1459, 1502` — 10 call sites. Each pass `Path::new("test.op")`.
  - `src/errors/tests.rs:272, 297, 322, 347, 369, 377` — 6 call sites. Each pass `Path::new("test.op")` (line 377's empty source stays `""`).
  - `tests/integration_e2e/tests.rs:55, 98` — 2 call sites. Each pass `Path::new("test.op")`.
  - **Use `sed` or batch Edit to minimize errors**: the pattern `compile_to_module(&context, source)` → `compile_to_module(&context, Path::new("test.op"), source)` is uniform for all test sites.
  - Verify with `grep -rn "compile_to_module(" src/ tests/ | wc -l` (expect 23 post-change) and `cargo build --tests --all-features 2>&1 | grep "error\[E" | head -20` (expect zero).

  **Step 4 — Update `compile_program` call sites in `src/app.rs`**:
  - `src/app.rs:186`: `compile_program(&source, Path::new("target"))` → `compile_program(Path::new(source_path), &source, Path::new("target"))`.
  - `src/app.rs:216`: same transformation (inside `compile_and_run`; `source_path` is already in scope).
  - Run `grep -n "compile_program(" src/ tests/` to confirm no other call sites exist. (Previously audited: only these two in `src/app.rs`.)

  **Step 5 — Wire validator into `run_check_command`** (`src/app.rs:523-562`):
  - This function runs its OWN lex/parse/typecheck and never touches `compile_to_module` or `compile_program`. `source_path: &str` is already in scope (line 524).
  - After `let Some(program) = program_opt else { ... };` and BEFORE `checker.type_check_program(&program)` (line 555), insert:
    ```rust
    if let Err(role_error) = validate_module_file_role(Path::new(source_path), &program) {
        report.extend_type_errors(vec![role_error]);
        eprintln!("{}", render_report(source_path, &source, &report));
        return Err(1);
    }
    ```
  - Add `validate_module_file_role` to the existing `use crate::module_loader::...` block (grep for the existing import in `src/app.rs` or add a new one at the top); do NOT use inline `use` inside the function.
  - **Confirm before coding**: `grep -n "extend_type_errors\|extend_type" src/` to verify the exact method name on `CompilationErrorReport`. If the method signature differs (e.g., `push_type_error` accepting a single error), adapt accordingly — do NOT invent names.

  **Step 6 — Verify**:
  - `cargo build --tests --all-features` exits 0.
  - `cargo test` (full suite) passes with zero regressions.
  - `./target/release/opalescent check <violation-fixture>` exits non-zero with a readable diagnostic.

  **Must NOT do**:
  - Do NOT duplicate validation logic in the type-checker or parser — keep it ONLY in `module_loader::validate_module_file_role`.
  - Do NOT add a second `compile_to_module_with_path` wrapper — update the single function's signature; the 23 call sites are all mechanical transforms.
  - Do NOT skip `run_check_command`'s wiring — Momus Round 1 explicitly flagged this as a required hook; without it, `opal check` silently accepts violations.
  - Do NOT change the error-reporting flow in `run_check_command` to `panic!`/`expect` — follow the existing `report.extend_* → render_report → Err(1)` pattern.
  - Do NOT re-parse in `compile_program` (e.g., calling `parse_source_to_program` then `compile_to_module`) — that parses twice. Thread the path through `compile_to_module` instead.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Non-trivial — requires coordinated updates across `src/compiler.rs` (two signature changes), 23 test call sites in 4 files, and `src/app.rs` (3 call-site updates + new `run_check_command` wiring). Mechanical but high-volume; a single missed call site breaks the build.
  - **Skills**: (none required)
  - **Skills Evaluated but Omitted**:
    - `quick`: Too many coordinated touch points (23+ call sites).
    - `deep` / `ultrabrain`: Overkill — mechanics are clear, just voluminous.

  **Parallelization**:
  - **Can Run In Parallel**: NO (blocks T5; both modify `src/compiler.rs`)
  - **Parallel Group**: Wave 2 — sequential with T5 (T4 first, T5 after)
  - **Blocks**: T5 (T5 must not run concurrently — both modify `src/compiler.rs`; serialize to avoid merge conflicts)
  - **Blocked By**: T3 (validator must exist)

  **References**:

  **Pattern References**:
  - `src/compiler.rs:124-234` — full `compile_to_module` body; note existing error-accumulation via `report.extend_lex_errors` (line 132), `report.extend_parse_errors` (line 139), `report.extend_type_errors` (line 154). Validator invocation must match this pattern.
  - `src/compiler.rs:311-360` — `validate_entry_declarations_for_module`: same shape as the validator being wired; reference for error-wrapping conventions.
  - `src/app.rs:186, 216` — existing `compile_program` call sites; mirror how `source_path` flows through.
  - `src/app.rs:523-562` — `run_check_command` structure; new wiring inserts between parse success and type-check invocation.

  **API/Type References**:
  - `src/compiler.rs:124:pub fn compile_to_module` — primary function to modify (new `source_path: &Path` second arg).
  - `src/compiler.rs:590:pub fn compile_program` — secondary function to modify (threads `source_path` through).
  - `src/module_loader::validate_module_file_role(source_path: &Path, program: &Program) -> Result<(), TypeError>` — defined in T3.
  - `CompilationErrorReport::extend_type_errors(Vec<TypeError>)` — confirm name via `grep -n "extend_type_errors" src/` before use.
  - `CompileError::Type(TypeError)` — variant for validator errors when promoting from `run_check_command`.

  **Test References**:
  - `src/app.rs:975-1006` — existing `#[test]` functions for `check_*` commands. Add two new tests (`check_rejects_type_in_regular_file`, `check_rejects_value_in_types_file`) following their fixture-file + `run_check_command` pattern.
  - `src/compiler.rs:816-900` — existing `compile_to_module_*` unit tests demonstrate the call pattern that must be updated; use them as templates.

  **WHY Each Reference Matters**:
  - `compile_to_module`'s internals (not just its signature) hold the ONLY point in the single-file pipeline where `Program` is bound after parse and before type-check. Any other integration strategy either re-parses (wasteful) or introduces duplicate pipeline code.
  - 23 call sites sound daunting but are 100% mechanical `s/compile_to_module(&context, source)/compile_to_module(&context, Path::new("test.op"), source)/` substitutions — Rust's type system will catch any miss at build time.
  - `run_check_command` is a separate code path. Leaving it unwired means `opal check foo.types.op` silently passes illegal files — violating the "zero human intervention" verification criterion and the STRICT separation policy.
  - The existing `report.extend_* → render_report → Err(1)` idiom in `run_check_command` (lines 539, 545, 556) is the ONLY sanctioned error-output pattern; deviating produces a user-facing regression.

  **Acceptance Criteria**:
  - [ ] `cargo build --tests --all-features` succeeds after all signature changes + call-site updates.
  - [ ] `grep -n "pub fn compile_to_module" src/compiler.rs` shows signature `(&'context Context, &Path, &str)`.
  - [ ] `grep -n "pub fn compile_program" src/compiler.rs` shows signature `(&Path, &str, &Path)`.
  - [ ] `grep -rn "compile_to_module(" src/ tests/` shows 23 call sites, each with a `Path` as the second argument.
  - [ ] `grep -rn "compile_program(" src/ tests/` shows every call site passes a `Path` as the first argument.
  - [ ] `src/app.rs:run_check_command` calls `validate_module_file_role(Path::new(source_path), &program)` between parse and type-check.
  - [ ] `cargo test check_rejects_type_in_regular_file` passes.
  - [ ] `cargo test check_rejects_value_in_types_file` passes.
  - [ ] `cargo test` (full suite) passes with zero regressions.

  **QA Scenarios**:

  ```
  Scenario: `opal check foo.op` rejects a `type` decl
    Tool: Bash (opalescent binary)
    Preconditions: T4 complete; `cargo build --release` succeeds. A temp file `/tmp/role-test-bad.op` exists containing `type Foo:\n    x: int32\n\nentry main = f(args: string[]): void =>\n    return void`.
    Steps:
      1. cargo build --release 2>&1 | tail -5
      2. ./target/release/opalescent check /tmp/role-test-bad.op 2>&1 | tee .sisyphus/evidence/task-4-check-op.txt
      3. echo "exit=$?" | tee -a .sisyphus/evidence/task-4-check-op.txt
    Expected Result: stderr contains diagnostic referencing TypeDeclarationOutsideTypesFile (or its user-facing rendered form — e.g. "type declarations must live in .types.op files"); exit code != 0.
    Evidence: .sisyphus/evidence/task-4-check-op.txt

  Scenario: `opal check foo.types.op` rejects a `let` decl
    Tool: Bash (opalescent binary)
    Preconditions: Temp file `/tmp/role-test-bad.types.op` exists containing `type Foo:\n    x: int32\n\nlet x: int32 = 5\n`.
    Steps:
      1. ./target/release/opalescent check /tmp/role-test-bad.types.op 2>&1 | tee .sisyphus/evidence/task-4-check-types-op.txt
      2. echo "exit=$?" | tee -a .sisyphus/evidence/task-4-check-types-op.txt
    Expected Result: stderr contains diagnostic referencing NonTypeDeclarationInTypesFile; exit code != 0.
    Evidence: .sisyphus/evidence/task-4-check-types-op.txt

  Scenario: `opal check foo.op` STILL passes for a normal valid file (no regression)
    Tool: Bash (opalescent binary)
    Preconditions: Any existing valid `.op` file, e.g. `test-projects/hello-world/src/main.op`.
    Steps:
      1. ./target/release/opalescent check test-projects/hello-world/src/main.op 2>&1 | tee .sisyphus/evidence/task-4-check-regression.txt
      2. echo "exit=$?" | tee -a .sisyphus/evidence/task-4-check-regression.txt
    Expected Result: Output contains "check passed"; exit code 0.
    Evidence: .sisyphus/evidence/task-4-check-regression.txt

  Scenario: Unit test — direct `compile_program` call rejects type in .op
    Tool: Bash (cargo test)
    Preconditions: T3, T4 merged.
    Steps:
      1. cargo test compile_program_rejects_type_in_regular_op_file -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-unit-reject-op.txt
    Expected Result: Test passes; output contains `test ... ok`.
    Evidence: .sisyphus/evidence/task-4-unit-reject-op.txt

  Scenario: Unit test — direct `compile_program` call rejects let in .types.op
    Tool: Bash (cargo test)
    Preconditions: T3, T4 merged.
    Steps:
      1. cargo test compile_program_rejects_let_in_types_op_file -- --nocapture 2>&1 | tee .sisyphus/evidence/task-4-unit-reject-types-op.txt
    Expected Result: Test passes.
    Evidence: .sisyphus/evidence/task-4-unit-reject-types-op.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-4-check-op.txt`
  - [ ] `task-4-check-types-op.txt`
  - [ ] `task-4-check-regression.txt`
  - [ ] `task-4-unit-reject-op.txt`
  - [ ] `task-4-unit-reject-types-op.txt`

  **Commit**: Groups with T5 — see Commit Strategy.

- [x] 5. Wire validator into multi-file `compile_project` + skip codegen for type-only modules

  **What to do**:
  - In `src/compiler.rs` `compile_project` (lines 633-806):
    - After `validate_entry_declarations_for_module(project_dir, module_path, &program)?;` at line 648, add:
      ```rust
      crate::module_loader::validate_module_file_role(module_path, &program)
          .map_err(CompileError::Type)?;
      ```
      This runs the validator for every discovered module right after parse, before type-check.
    - In the codegen loop at line 780 (`for (index, module_path) in discovered_module_paths.iter().enumerate()`), skip codegen+emit when `crate::module_loader::is_types_file(module_path)` returns `true`:
      ```rust
      if crate::module_loader::is_types_file(module_path) {
          continue;
      }
      ```
      Place the check at the top of the loop body, before `let context = Context::create();`.
  - Verify that at least one `.o` is still emitted (the main module must produce an object), otherwise `link_object_files` fails. If a project consisted ONLY of `.types.op` files (impossible — `main.op` is required), would fail validation earlier anyway.

  **Must NOT do**:
  - Do NOT remove `validate_entry_declarations_for_module` or change its signature.
  - Do NOT reorder the existing discovery/type-check/codegen phases.
  - Do NOT modify linker logic.
  - Do NOT skip type-checking for `.types.op` — only codegen.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Two insertion points in a 170-line function that handles discovery, typing, and codegen — requires care to avoid breaking the existing multi-file test.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T4, T10)
  - **Parallel Group**: Wave 2
  - **Blocks**: T8a/b/c, T9a/b
  - **Blocked By**: T3

  **References**:

  **Pattern References**:
  - `src/compiler.rs:648` — where `validate_entry_declarations_for_module` is called; the new call goes immediately after.
  - `src/compiler.rs:780-802` — codegen loop; the skip goes at the top of the body.

  **API/Type References**:
  - `crate::module_loader::{is_types_file, validate_module_file_role}` — newly added.
  - `CompileError::Type(TypeError)` — wrap errors.

  **Test References**:
  - `tests/integration_e2e/tests.rs::multi_file_project_compiles_and_runs` — canonical success pattern; must remain green after changes.

  **WHY Each Reference Matters**:
  - The validator MUST run after parse (to have a `Program`) but BEFORE type-check (so errors surface early and consistently).
  - The codegen skip MUST come before LLVM `Context::create()` to avoid pointless work.
  - `multi_file_project_compiles_and_runs` is the canary — if this test breaks, wiring is wrong.

  **Acceptance Criteria**:

  - [ ] `cargo build` succeeds.
  - [ ] `cargo test --features integration multi_file_project_compiles_and_runs` still passes (no regression).
  - [ ] `cargo test --features integration` as a whole still passes (no regression).
  - [ ] `rg "validate_module_file_role" src/compiler.rs` returns one match.
  - [ ] `rg "is_types_file" src/compiler.rs` returns one match (the skip).

  **QA Scenarios**:

  ```
  Scenario: Existing multi-file test still passes
    Tool: Bash (cargo test)
    Preconditions: T5 merged.
    Steps:
      1. Run: cargo test --features integration multi_file_project_compiles_and_runs -- --nocapture
    Expected Result: 1 passed; stdout contains the expected "5\n" or equivalent from multi-file project.
    Evidence: .sisyphus/evidence/task-5-multi-file-regression.txt

  Scenario: compile_project rejects type decl in .op member file
    Tool: Bash (cargo test)
    Preconditions: T5 merged.
    Steps:
      1. Use a temp directory: create opal.toml + src/main.op (with `type Bad:` decl and entry main) + .gitignore.
      2. Call compile_project and assert Err matches CompileError::Type(TypeError::TypeDeclarationOutsideTypesFile).
      3. Run: cargo test --features integration compiler::tests::compile_project_rejects_type_in_main
    Expected Result: Test passes.
    Evidence: .sisyphus/evidence/task-5-project-reject-type.txt

  Scenario: Type-only module does not produce .o
    Tool: Bash (cargo test + std::fs::read_dir)
    Preconditions: T5 + T6a merged. Fixture uses a models.types.op.
    Steps:
      1. After compile_project succeeds on import-types-basic, read the output dir.
      2. Assert the count of .o files equals the number of .op (non-.types.op) modules discovered. For import-types-basic that's 1 (main.op).
      3. Run the assertion in-test inside the existing integration test body.
    Expected Result: Exactly 1 .o file emitted; 0 .o for the .types.op module.
    Evidence: .sisyphus/evidence/task-5-codegen-skip.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-5-multi-file-regression.txt`
  - [ ] `task-5-project-reject-type.txt`
  - [ ] `task-5-codegen-skip.txt`

  **Commit**: Groups with T4. Message: `feat(compiler): enforce .types.op separation and skip codegen for type-only modules`.

- [ ] 10. Polish: docstrings, clippy-clean, ensure public surface documented

  **What to do**:
  - Ensure rustdoc on `is_types_file` and `validate_module_file_role` includes: one-line summary, `# Arguments`, `# Errors`, `# Examples` with a 3-line doctest.
  - Run `cargo clippy --all-targets --all-features -- -D warnings`; fix any new lints introduced by T1/T3/T4/T5.
  - Run `cargo fmt --all` to ensure consistent formatting.
  - Update the module-level doc comment at top of `src/module_loader.rs` (line 1) to mention the new validator: "…and validation of file-role invariants (types-only in `.types.op`)."

  **Must NOT do**:
  - Do NOT broaden public surface beyond `is_types_file` + `validate_module_file_role`.
  - Do NOT add `#[allow(...)]` lint suppressions unless absolutely necessary.
  - Do NOT touch unrelated files.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical polish.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T4, T5)
  - **Parallel Group**: Wave 2
  - **Blocks**: F1-F4
  - **Blocked By**: T3

  **References**:

  **Pattern References**:
  - `src/module_loader.rs:17-29` — `resolve_import_path` rustdoc shape; mirror for new public functions.

  **Acceptance Criteria**:

  - [ ] `cargo doc --no-deps` builds without warnings for the new items.
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
  - [ ] `cargo fmt --all --check` exits 0.

  **QA Scenarios**:

  ```
  Scenario: Clippy clean
    Tool: Bash
    Preconditions: T1, T3, T4, T5 merged.
    Steps:
      1. Run: cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee .sisyphus/evidence/task-10-clippy.txt
    Expected Result: Exit 0; no warnings.
    Evidence: .sisyphus/evidence/task-10-clippy.txt

  Scenario: Doctest on is_types_file passes
    Tool: Bash
    Preconditions: Doctest added.
    Steps:
      1. Run: cargo test --doc is_types_file
    Expected Result: 1 doctest passed.
    Evidence: .sisyphus/evidence/task-10-doctest.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-10-clippy.txt`
  - [ ] `task-10-doctest.txt`

  **Commit**: Groups with T1/T2/T3.

- [x] 6a. Fixture: `test-projects/import-types-basic/`

  **What to do**:
  - Create directory tree:
    ```
    test-projects/import-types-basic/
    ├── opal.toml
    ├── .gitignore
    ├── README.md
    └── src/
        ├── main.op
        └── models.types.op
    ```
  - `opal.toml`:
    ```toml
    name = "import-types-basic"
    version = "0.1.0"
    ```
  - `.gitignore`:
    ```
    /target/
    *.o
    ```
  - `README.md`: 3-5 lines describing the fixture: demonstrates a single type imported from `models.types.op` and constructed in `main.op`.
  - `src/models.types.op`:
    ```opal
    ##
      Description: A person with a name and age
    ##
    type Person:
        name: string
        age: int32
    ```
  - `src/main.op`:
    ```opal
    import Person from ./models.types

    entry main = f(args: string[]): void =>
        let alice: Person = Person { name: 'Alice', age: 30 }
        print('{alice.name} is {alice.age} years old')
        return void
    ```
  - Confirm the colon-block `type` body parses. If it does NOT (known parser gap per ignored tests), fall back to the currently-used syntax in `language-spec/types_example.types.op` that IS accepted by the parser — verify by running `cargo run --release -- check test-projects/import-types-basic/src/models.types.op` as part of the T8a integration test.

  **Must NOT do**:
  - Do NOT add dependencies.
  - Do NOT use sum types (keep fixture minimal).
  - Do NOT include features outside the minimal success path (no generics, no pattern match).

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File scaffolding.
  - **Skills**: none.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T6b, T6c, T7a, T7b)
  - **Blocks**: T8a
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `test-projects/multi-file/` — reference layout (opal.toml + src/main.op + module file).
  - `test-projects/hello-world/opal.toml` — minimal opal.toml shape.
  - `language-spec/types_example.types.op` — type declaration syntax examples.
  - `README.md` lines covering "Test Project Conventions" — exact conventions to honor.

  **Acceptance Criteria**:

  - [ ] Five files exist as above.
  - [ ] `cargo run --release -- check test-projects/import-types-basic/src/main.op` — outcome depends on whether single-file check resolves the import; if not, defer full verification to T8a integration test.
  - [ ] The fixture uses only the minimal language features needed.

  **QA Scenarios**:

  ```
  Scenario: Fixture files exist with correct shape
    Tool: Bash
    Preconditions: T6a complete.
    Steps:
      1. ls -R test-projects/import-types-basic/
      2. cat test-projects/import-types-basic/opal.toml | grep '^name = "import-types-basic"'
      3. cat test-projects/import-types-basic/src/main.op | grep "import Person from ./models.types"
      4. cat test-projects/import-types-basic/src/models.types.op | grep "type Person:"
    Expected Result: All asserts succeed; tree matches spec.
    Evidence: .sisyphus/evidence/task-6a-fixture-tree.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-6a-fixture-tree.txt`

  **Commit**: Groups with T6b, T6c, T7a, T7b — see Commit Strategy.

- [x] 6b. Fixture: `test-projects/import-types-aliased/`

  **What to do**:
  - Same layout as T6a but demonstrate aliased type import.
  - `src/models.types.op`:
    ```opal
    ##
      Description: A user with an identifier and display name
    ##
    type User:
        id: int32
        display_name: string
    ```
  - `src/main.op`:
    ```opal
    import type User as Account from ./models.types

    entry main = f(args: string[]): void =>
        let bob: Account = Account { id: 42, display_name: 'Bob' }
        print('User {bob.id}: {bob.display_name}')
        return void
    ```
  - Note the `import type` keyword + `as Account` alias form; both forms are parser-supported per `src/parser/tests.rs:4350-4465`.

  **Must NOT do**:
  - Do NOT reuse `Person` (different fixture).
  - Do NOT include value imports alongside.

  **Recommended Agent Profile**:
  - **Category**: `quick`. **Skills**: none.

  **Parallelization**: YES / Wave 1 / Blocks T8b / Blocked by: none.

  **References**:
  - Same as T6a.
  - `src/parser/tests.rs::test_import_type` — parser test asserting `import type X as Y` works.

  **Acceptance Criteria**:
  - [ ] Four files exist.
  - [ ] `main.op` uses the alias `Account`, not `User`.

  **QA Scenarios**:

  ```
  Scenario: Aliased fixture tree and content
    Tool: Bash
    Steps:
      1. ls -R test-projects/import-types-aliased/
      2. grep "import type User as Account" test-projects/import-types-aliased/src/main.op
      3. grep "Account {" test-projects/import-types-aliased/src/main.op
    Expected Result: All greps succeed.
    Evidence: .sisyphus/evidence/task-6b-fixture-tree.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-6b-fixture-tree.txt`

  **Commit**: Groups with T6a, T6c, T7a, T7b.

- [x] 6c. Fixture: `test-projects/import-types-multiple/`

  **What to do**:
  - Same layout. Demonstrate multi-item type import.
  - `src/models.types.op`:
    ```opal
    ##
      Description: A point in 2D space
    ##
    type Point:
        x: int32
        y: int32

    ##
      Description: An axis-aligned rectangle
    ##
    type Rect:
        top_left: Point
        width: int32
        height: int32
    ```
  - `src/main.op`:
    ```opal
    import type Point, Rect from ./models.types

    entry main = f(args: string[]): void =>
        let origin: Point = Point { x: 0, y: 0 }
        let r: Rect = Rect { top_left: origin, width: 10, height: 20 }
        print('Rect {r.width}x{r.height} at ({r.top_left.x},{r.top_left.y})')
        return void
    ```
  - Demonstrates that two types can be imported from one `.types.op`, and one type can reference another within the same types file.

  **Must NOT do**:
  - Do NOT use generics.
  - Do NOT split across two `.types.op` files — keep both types in one file for this fixture.

  **Recommended Agent Profile**:
  - **Category**: `quick`. **Skills**: none.

  **Parallelization**: YES / Wave 1 / Blocks T8c / Blocked by: none.

  **References**:
  - Same as T6a/b.
  - `src/parser/tests.rs` mixed-type-import tests (around line 4430).

  **Acceptance Criteria**:
  - [ ] Four files exist.
  - [ ] `main.op` uses BOTH `Point` and `Rect`.
  - [ ] `Rect` references `Point` in its field type — tests cross-type reference within the same types file.

  **QA Scenarios**:

  ```
  Scenario: Multi-type fixture
    Tool: Bash
    Steps:
      1. ls -R test-projects/import-types-multiple/
      2. grep "import type Point, Rect from ./models.types" test-projects/import-types-multiple/src/main.op
      3. grep "top_left: Point" test-projects/import-types-multiple/src/models.types.op
    Expected Result: All greps succeed.
    Evidence: .sisyphus/evidence/task-6c-fixture-tree.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-6c-fixture-tree.txt`

  **Commit**: Groups with T6a, T6b, T7a, T7b.

- [x] 7a. Fixture: `test-projects/type-in-regular-file-fail/` (compile-fail)

  **What to do**:
  - Create `test-projects/type-in-regular-file-fail/opal.toml`:
    ```toml
    name = "type-in-regular-file-fail"
    version = "0.1.0"
    ```
  - Create `test-projects/type-in-regular-file-fail/.gitignore`:
    ```
    target/
    ```
  - Create `test-projects/type-in-regular-file-fail/README.md` explaining this fixture is intentionally invalid: it places a `type` declaration inside `src/main.op`, which violates the file-role separation rule.
  - Create `test-projects/type-in-regular-file-fail/src/main.op` containing both an `entry main` AND a `type` declaration so that only the new validator (not parser/type-checker alone) catches it:
    ```opal
    ##
      Description: Intentionally-invalid type declaration in a .op file
    ##
    type Person:
        name: string
        age: int32

    ##
      Description: Entry point (will never compile due to validator error)
    ##
    entry main = f(args: string[]): void =>
        print('unreachable')
        return void
    ```
  - Verify manually with `cargo run --release -- check test-projects/type-in-regular-file-fail/src/main.op` is NOT required here — this fixture is consumed by integration test T9a, which asserts `compile_project` returns `TypeError::TypeDeclarationOutsideTypesFile`.

  **Must NOT do**:
  - Do NOT add `.types.op` files to this fixture — the failure must be triggered purely by the `type` in `main.op`.
  - Do NOT add multiple error sources — keep the fixture minimal so the expected error variant is unambiguous.
  - Do NOT expect runnable output — this fixture is compile-fail only.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Creating 4 small static files — trivial scope, no logic.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**:
    - `git-master`: Commit handled by shared T7a/b commit in T6/T7 batch.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T6a, T6b, T6c, T7b)
  - **Blocks**: T9a (integration test needs this fixture)
  - **Blocked By**: None (fixture files only; no dependency on T1-T5)

  **References**:

  **Pattern References** (existing code to follow):
  - `test-projects/ref-compile-fail/opal.toml` — layout for a compile-fail fixture (`name`, `version` only).
  - `test-projects/ref-compile-fail/src/main.op` — how a fail fixture includes `entry main` plus the intentional error.
  - `test-projects/hello-world/.gitignore` — canonical `.gitignore` content.

  **API/Type References**:
  - `src/ast.rs:Decl::Type` — the shape of type decls (already parseable in `.op` files; only validator rejects).
  - T1's new variant `TypeError::TypeDeclarationOutsideTypesFile` — what T9a will assert on.

  **Test References**:
  - `tests/integration_e2e/compile_failures.rs` — T9a will consume this fixture via `compile_project(Path::new("test-projects/type-in-regular-file-fail"), &temp_dir)`.

  **WHY Each Reference Matters**:
  - `ref-compile-fail/` is the canonical shape for "this fixture is expected to fail compilation" — copying its layout guarantees harness compatibility.
  - Keeping the `type Person` decl structurally valid (parses + typechecks individually) isolates the failure to the validator, which is what T9a asserts on.

  **Acceptance Criteria**:
  - [ ] Directory exists: `test-projects/type-in-regular-file-fail/`.
  - [ ] Files exist: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`.
  - [ ] `src/main.op` contains a `type` decl AND `entry main`.
  - [ ] No `.types.op` files in this fixture.
  - [ ] `grep -R "^type " test-projects/type-in-regular-file-fail/src/main.op` returns 1 match.

  **QA Scenarios**:

  ```
  Scenario: Fixture tree is complete and contains a type decl in .op
    Tool: Bash
    Preconditions: T7a files written.
    Steps:
      1. ls -R test-projects/type-in-regular-file-fail/
      2. grep -c "^type Person:" test-projects/type-in-regular-file-fail/src/main.op
      3. test ! -e test-projects/type-in-regular-file-fail/src/models.types.op
    Expected Result: All four files present; grep returns "1"; no .types.op.
    Evidence: .sisyphus/evidence/task-7a-fixture-tree.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-7a-fixture-tree.txt`

  **Commit**: Groups with T6a/b/c, T7b.

- [x] 7b. Fixture: `test-projects/value-in-types-file-fail/` (compile-fail)

  **What to do**:
  - Create `test-projects/value-in-types-file-fail/opal.toml`:
    ```toml
    name = "value-in-types-file-fail"
    version = "0.1.0"
    ```
  - Create `test-projects/value-in-types-file-fail/.gitignore`:
    ```
    target/
    ```
  - Create `test-projects/value-in-types-file-fail/README.md` explaining that this fixture intentionally places a `let` declaration inside `src/models.types.op`, violating STRICT separation.
  - Create `test-projects/value-in-types-file-fail/src/main.op`:
    ```opal
    ##
      Description: Imports from the offending types module
    ##
    import type Person from ./models.types

    ##
      Description: Entry point (will never compile due to validator error in models.types.op)
    ##
    entry main = f(args: string[]): void =>
        print('unreachable')
        return void
    ```
  - Create `test-projects/value-in-types-file-fail/src/models.types.op` — a valid type plus an INVALID `let` decl:
    ```opal
    ##
      Description: A person record (valid)
    ##
    type Person:
        name: string
        age: int32

    ##
      Description: Illegal value declaration inside a .types.op file
    ##
    let default_age: int32 = 18
    ```

  **Must NOT do**:
  - Do NOT include an `entry` in `models.types.op` — `let` alone is sufficient to trigger the violation; adding `entry` would compound error sources.
  - Do NOT make `main.op` syntactically invalid — the failure must originate in the `.types.op` file.
  - Do NOT rely on the `let` being type-checked first — validator runs before type-checking for that module.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Four static files, no logic.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**:
    - `git-master`: Shared commit with T6/T7 batch.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T6a, T6b, T6c, T7a)
  - **Blocks**: T9b (integration test needs this fixture)
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `test-projects/ref-compile-fail/` — compile-fail fixture conventions.
  - `test-projects/multi-file/src/main.op` — how `import` statements look in a multi-file project.

  **API/Type References**:
  - T1's new variant `TypeError::NonTypeDeclarationInTypesFile` — what T9b will assert on.
  - `src/ast.rs:Decl::Let` — the declaration kind that must be rejected inside `.types.op`.

  **Test References**:
  - `tests/integration_e2e/compile_failures.rs` — consumed by T9b.

  **WHY Each Reference Matters**:
  - Splitting across two files (`main.op` + `models.types.op`) mirrors realistic usage and exercises the validator's per-module iteration in `compile_project`.
  - Including a valid `type Person` alongside the illegal `let` proves the validator's rejection is about role separation, not about the file itself being unparseable.

  **Acceptance Criteria**:
  - [ ] Directory exists: `test-projects/value-in-types-file-fail/`.
  - [ ] Files exist: `opal.toml`, `.gitignore`, `README.md`, `src/main.op`, `src/models.types.op`.
  - [ ] `src/models.types.op` contains `type Person` AND `let default_age`.
  - [ ] `src/main.op` imports from `./models.types`.
  - [ ] `grep "^let default_age" test-projects/value-in-types-file-fail/src/models.types.op` returns 1 match.

  **QA Scenarios**:

  ```
  Scenario: Fixture tree contains .types.op with illegal let decl
    Tool: Bash
    Preconditions: T7b files written.
    Steps:
      1. ls -R test-projects/value-in-types-file-fail/
      2. grep -c "^let default_age" test-projects/value-in-types-file-fail/src/models.types.op
      3. grep -c "^type Person:" test-projects/value-in-types-file-fail/src/models.types.op
      4. grep -c "import type Person from ./models.types" test-projects/value-in-types-file-fail/src/main.op
    Expected Result: All files present; each grep returns "1".
    Evidence: .sisyphus/evidence/task-7b-fixture-tree.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-7b-fixture-tree.txt`

  **Commit**: Groups with T6a/b/c, T7a.

- [x] 8a. Integration test: `import_types_basic_compiles_and_runs`

  **What to do**:
  - Append a new `#[test]` function to `tests/integration_e2e/tests.rs` named `import_types_basic_compiles_and_runs`.
  - Gate behind the existing integration feature (follow the pattern of `multi_file_project_compiles_and_runs` in the same file).
  - Test flow:
    1. Call `prepare_dir` helper (see `tests/integration_print.rs` for pattern) or construct `PathBuf::from("test-projects/import-types-basic")` and a `tempfile::tempdir()` output dir.
    2. Call `compile_project(&project_dir, temp_dir.path())` and `.expect("compile_project succeeded")`.
    3. Spawn the resulting binary via `std::process::Command::new(binary_path).output()`.
    4. Assert `output.status.success()` is true.
    5. Assert `String::from_utf8_lossy(&output.stdout).trim() == "Alice is 30 years old"` (exact stdout from T6a).
    6. Call `cleanup_dir` / let `tempdir` drop.

  **Must NOT do**:
  - Do NOT modify `compile_project`'s return signature.
  - Do NOT replicate the `multi_file_project_compiles_and_runs` logic wholesale — factor via the same helpers it already uses if present.
  - Do NOT hardcode absolute paths — use `PathBuf::from("test-projects/import-types-basic")` which is relative to repo root, matching existing conventions.
  - Do NOT leave stray artifacts in `test-projects/import-types-basic/target/` — all output goes to tempdir.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mirrors an existing test function with trivial modifications.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**:
    - `deep` / `unspecified-high`: Overkill — this is a copy-and-modify-constants task.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T8b, T8c, T9a, T9b)
  - **Blocks**: F1, F2, F3 (final verification needs the full integration suite green)
  - **Blocked By**: T5 (codegen skip must exist), T6a (fixture must exist)

  **References**:

  **Pattern References**:
  - `tests/integration_e2e/tests.rs` — find `multi_file_project_compiles_and_runs`. Copy its structure: `prepare_dir → compile_project → spawn binary → assert stdout → cleanup_dir`.
  - `tests/integration_print.rs` — `prepare_dir(&PathBuf)` and `cleanup_dir(&PathBuf)` helpers.

  **API/Type References**:
  - `src/compiler.rs:compile_project(project_dir: &Path, output_dir: &Path) -> Result<PathBuf, CompileError>` — function under test; returns binary path on success.

  **Test References**:
  - `tests/integration_e2e/mod.rs` — confirm feature gate; new test must be within the same module boundary.

  **WHY Each Reference Matters**:
  - Copying `multi_file_project_compiles_and_runs` guarantees feature-gate, imports, and cleanup semantics are consistent — avoiding CI flakes.
  - `compile_project` is the public entry point; using it (not `compile_program`) exercises the full multi-file pipeline including T5's codegen-skip.

  **Acceptance Criteria**:
  - [ ] New test function `import_types_basic_compiles_and_runs` exists in `tests/integration_e2e/tests.rs`.
  - [ ] `cargo test --features integration import_types_basic_compiles_and_runs` passes.
  - [ ] Test asserts stdout equals `"Alice is 30 years old"` (trimmed).
  - [ ] No leftover files in `test-projects/import-types-basic/target/` after test runs.

  **QA Scenarios**:

  ```
  Scenario: Integration test passes — basic type import round-trip
    Tool: Bash
    Preconditions: T5 wiring done, T6a fixture exists, codebase compiles.
    Steps:
      1. cargo test --features integration import_types_basic_compiles_and_runs -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8a-run.txt
      2. grep -c "test import_types_basic_compiles_and_runs ... ok" .sisyphus/evidence/task-8a-run.txt
      3. grep -c "Alice is 30 years old" .sisyphus/evidence/task-8a-run.txt
    Expected Result: Both greps return "1"; exit code 0.
    Evidence: .sisyphus/evidence/task-8a-run.txt

  Scenario: No artifacts leak to fixture directory
    Tool: Bash
    Preconditions: Test has run once.
    Steps:
      1. test ! -d test-projects/import-types-basic/target || ls test-projects/import-types-basic/target
    Expected Result: Directory does not exist, OR is empty (depending on harness convention).
    Evidence: .sisyphus/evidence/task-8a-cleanup.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-8a-run.txt`
  - [ ] `task-8a-cleanup.txt`

  **Commit**: Groups with T8b, T8c, T9a, T9b.

- [x] 8b. Integration test: `import_types_aliased_compiles_and_runs`

  **What to do**:
  - Append `#[test]` `import_types_aliased_compiles_and_runs` to `tests/integration_e2e/tests.rs`.
  - Identical structure to T8a, pointing at `test-projects/import-types-aliased`.
  - Assert stdout equals the string produced by T6b's fixture (e.g. `"Renamed: Bob"` — confirm actual string from T6b's `print` call; plan author's T6b section specifies this).

  **Must NOT do**:
  - Do NOT duplicate fixture-construction logic — rely on the existing `test-projects/import-types-aliased` directory created in T6b.
  - Do NOT change the assertion string without cross-updating T6b.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Clone-and-retarget of T8a.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**: (none)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: F1, F2, F3
  - **Blocked By**: T5, T6b

  **References**:

  **Pattern References**:
  - T8a (in this plan) — the template to clone.
  - T6b (in this plan) — source of truth for expected stdout.

  **API/Type References**:
  - Same as T8a.

  **Test References**:
  - `tests/integration_e2e/tests.rs:multi_file_project_compiles_and_runs` — baseline pattern.

  **WHY Each Reference Matters**:
  - Aliasing (`import type User as Account from ./models.types`) exercises the parser's `ImportItem::Type { alias: Some(..) }` branch end-to-end through codegen — a separate code path from the basic case.

  **Acceptance Criteria**:
  - [ ] New test function `import_types_aliased_compiles_and_runs` exists.
  - [ ] `cargo test --features integration import_types_aliased_compiles_and_runs` passes.
  - [ ] Stdout assertion matches T6b's documented expected output.

  **QA Scenarios**:

  ```
  Scenario: Integration test passes — aliased type import works
    Tool: Bash
    Preconditions: T5 + T6b complete.
    Steps:
      1. cargo test --features integration import_types_aliased_compiles_and_runs -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8b-run.txt
      2. grep -c "test import_types_aliased_compiles_and_runs ... ok" .sisyphus/evidence/task-8b-run.txt
    Expected Result: grep returns "1"; exit code 0.
    Evidence: .sisyphus/evidence/task-8b-run.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-8b-run.txt`

  **Commit**: Groups with T8a, T8c, T9a, T9b.

- [x] 8c. Integration test: `import_types_multiple_compiles_and_runs`

  **What to do**:
  - Append `#[test]` `import_types_multiple_compiles_and_runs` to `tests/integration_e2e/tests.rs`.
  - Structure mirrors T8a/T8b, pointing at `test-projects/import-types-multiple`.
  - Assert stdout equals the string produced by T6c's fixture (e.g. contains both `Point` and `Rect` usages — exact string per T6c's `print` call).

  **Must NOT do**:
  - Do NOT restate the full test body if a helper can be extracted — but keep refactoring out of scope (T10 may consolidate).
  - Do NOT weaken the assertion to a partial match when an exact match is feasible.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Clone-and-retarget of T8a.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**: (none)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: F1, F2, F3
  - **Blocked By**: T5, T6c

  **References**:

  **Pattern References**:
  - T8a, T8b (this plan).
  - T6c (this plan) — expected stdout.

  **API/Type References**:
  - Same as T8a.

  **Test References**:
  - Same as T8a.

  **WHY Each Reference Matters**:
  - Importing multiple items from a single `.types.op` exercises the comma-list path in `Decl::Import` and the per-item type registration loop in `module_checking.rs:63-147`.

  **Acceptance Criteria**:
  - [ ] New test function `import_types_multiple_compiles_and_runs` exists.
  - [ ] `cargo test --features integration import_types_multiple_compiles_and_runs` passes.
  - [ ] Stdout assertion matches T6c's documented expected output exactly.

  **QA Scenarios**:

  ```
  Scenario: Integration test passes — multiple type imports from one module
    Tool: Bash
    Preconditions: T5 + T6c complete.
    Steps:
      1. cargo test --features integration import_types_multiple_compiles_and_runs -- --nocapture 2>&1 | tee .sisyphus/evidence/task-8c-run.txt
      2. grep -c "test import_types_multiple_compiles_and_runs ... ok" .sisyphus/evidence/task-8c-run.txt
    Expected Result: grep returns "1"; exit code 0.
    Evidence: .sisyphus/evidence/task-8c-run.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-8c-run.txt`

  **Commit**: Groups with T8a, T8b, T9a, T9b.

- [x] 9a. Integration failure test: `type_declaration_in_regular_file_is_rejected`

  **What to do**:
  - Append a new `#[test]` function to `tests/integration_e2e/compile_failures.rs` named `type_declaration_in_regular_file_is_rejected`.
  - Test flow:
    1. Build `project_dir = PathBuf::from("test-projects/type-in-regular-file-fail")`, tempdir output.
    2. Call `compile_project(&project_dir, temp_dir.path())`.
    3. Assert `result.is_err()`.
    4. Inspect the error. Because `compile_project` wraps validator errors via `.map_err(CompileError::Type)` (confirmed in `src/compiler.rs:648` area), the expected shape is `CompileError::Type(TypeError::TypeDeclarationOutsideTypesFile { .. })`. Handle both possibilities defensively:
       ```rust
       match result.unwrap_err() {
           CompileError::Type(TypeError::TypeDeclarationOutsideTypesFile { .. }) => {}
           CompileError::Report { report, .. } => {
               assert!(
                   report.entries().iter().any(|(_, e)| matches!(
                       e,
                       CompilerError::TypeChecker(TypeError::TypeDeclarationOutsideTypesFile { .. })
                   )),
                   "expected TypeDeclarationOutsideTypesFile in report"
               );
           }
           other => panic!("unexpected error: {other:?}"),
       }
       ```

  **Must NOT do**:
  - Do NOT assert on error-message strings — use the variant match above. Message strings may change; variant identity is the contract.
  - Do NOT rely on `propagate`/display formatting.
  - Do NOT add `#[should_panic]` — use explicit `is_err()` + variant match.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Template-driven; the shape is a well-known pattern from `compile_failures.rs`.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**:
    - `unspecified-high`: Not needed; no design decisions.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T8a/b/c, T9b)
  - **Blocks**: F1, F2, F3, F4
  - **Blocked By**: T1 (error variants must exist), T3 (validator must emit them), T5 (compile_project must invoke validator), T7a (fixture must exist)

  **References**:

  **Pattern References**:
  - `tests/integration_e2e/compile_failures.rs` — existing `#[test]` functions demonstrate the `compile_project → is_err → variant match` pattern. Find a sibling test and copy its shape verbatim.

  **API/Type References**:
  - `src/type_system/errors.rs:TypeError::TypeDeclarationOutsideTypesFile` (added in T1).
  - `src/compiler.rs:CompileError` — the wrapper enum.
  - `src/compiler.rs:compile_project` — the function under test.

  **Test References**:
  - `tests/integration_e2e/compile_failures.rs` — siblings.

  **WHY Each Reference Matters**:
  - Variant-matching (not string-matching) keeps the assertion robust against future miette-message refinements.
  - The defensive two-branch match guards against wrapping behavior drift between `CompileError::Type` and `CompileError::Report` — both are acceptable as long as the variant surfaces.

  **Acceptance Criteria**:
  - [ ] New test function `type_declaration_in_regular_file_is_rejected` exists.
  - [ ] `cargo test --features integration type_declaration_in_regular_file_is_rejected` passes.
  - [ ] Test asserts on the `TypeError::TypeDeclarationOutsideTypesFile` variant (not message text).

  **QA Scenarios**:

  ```
  Scenario: Failure test passes — .op file containing `type` is rejected with correct variant
    Tool: Bash
    Preconditions: T1, T3, T5, T7a complete.
    Steps:
      1. cargo test --features integration type_declaration_in_regular_file_is_rejected -- --nocapture 2>&1 | tee .sisyphus/evidence/task-9a-run.txt
      2. grep -c "test type_declaration_in_regular_file_is_rejected ... ok" .sisyphus/evidence/task-9a-run.txt
    Expected Result: grep returns "1"; exit code 0.
    Evidence: .sisyphus/evidence/task-9a-run.txt

  Scenario: Wrong variant would fail the test (negative verification)
    Tool: Bash
    Preconditions: Reviewer can inspect test source.
    Steps:
      1. grep -c "TypeDeclarationOutsideTypesFile" tests/integration_e2e/compile_failures.rs
    Expected Result: Returns at least "1".
    Evidence: .sisyphus/evidence/task-9a-variant-reference.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-9a-run.txt`
  - [ ] `task-9a-variant-reference.txt`

  **Commit**: Groups with T8a/b/c, T9b.

- [x] 9b. Integration failure test: `value_declaration_in_types_file_is_rejected`

  **What to do**:
  - Append `#[test]` `value_declaration_in_types_file_is_rejected` to `tests/integration_e2e/compile_failures.rs`.
  - Test flow mirrors T9a:
    1. `project_dir = PathBuf::from("test-projects/value-in-types-file-fail")`, tempdir output.
    2. `compile_project(&project_dir, temp_dir.path())` → expect `Err`.
    3. Variant match on `CompileError::Type(TypeError::NonTypeDeclarationInTypesFile { .. })` (with the same defensive `CompileError::Report` fallback as T9a).

  **Must NOT do**:
  - Do NOT assert on the offending file path string; the variant itself is the contract.
  - Do NOT expect `TypeDeclarationOutsideTypesFile` — this fixture triggers the OPPOSITE variant.
  - Do NOT combine T9a and T9b into a parameterized test — separate tests produce clearer failure output.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Clone of T9a with a different fixture + variant.
  - **Skills**: (none)
  - **Skills Evaluated but Omitted**: (none)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: F1, F2, F3, F4
  - **Blocked By**: T1, T3, T5, T7b

  **References**:

  **Pattern References**:
  - T9a (this plan) — identical structure.
  - `tests/integration_e2e/compile_failures.rs` — siblings.

  **API/Type References**:
  - `src/type_system/errors.rs:TypeError::NonTypeDeclarationInTypesFile` (added in T1).

  **Test References**:
  - Same as T9a.

  **WHY Each Reference Matters**:
  - This test proves the validator catches violations in BOTH directions (type-in-.op AND value-in-.types.op), not just one. Without it, a bug that swaps the two variants' detection logic could silently pass T9a.

  **Acceptance Criteria**:
  - [ ] New test function `value_declaration_in_types_file_is_rejected` exists.
  - [ ] `cargo test --features integration value_declaration_in_types_file_is_rejected` passes.
  - [ ] Test asserts on `TypeError::NonTypeDeclarationInTypesFile` variant.

  **QA Scenarios**:

  ```
  Scenario: Failure test passes — .types.op file containing `let` is rejected with correct variant
    Tool: Bash
    Preconditions: T1, T3, T5, T7b complete.
    Steps:
      1. cargo test --features integration value_declaration_in_types_file_is_rejected -- --nocapture 2>&1 | tee .sisyphus/evidence/task-9b-run.txt
      2. grep -c "test value_declaration_in_types_file_is_rejected ... ok" .sisyphus/evidence/task-9b-run.txt
    Expected Result: grep returns "1"; exit code 0.
    Evidence: .sisyphus/evidence/task-9b-run.txt

  Scenario: Both directions of separation are tested (completeness check)
    Tool: Bash
    Preconditions: T9a and T9b both merged.
    Steps:
      1. grep -c "TypeDeclarationOutsideTypesFile\|NonTypeDeclarationInTypesFile" tests/integration_e2e/compile_failures.rs
    Expected Result: Returns at least "2".
    Evidence: .sisyphus/evidence/task-9b-completeness.txt
  ```

  **Evidence to Capture**:
  - [ ] `task-9b-run.txt`
  - [ ] `task-9b-completeness.txt`

  **Commit**: Groups with T8a/b/c, T9a.

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (grep for new variants, read files, run the binary). For each "Must NOT Have": confirm absence (no parser changes to glob; no touched `types_example.types.op`; stdlib paths untouched). Compare deliverables against plan. Check evidence files exist in `.sisyphus/evidence/`.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build`, `cargo clippy --all-targets --all-features`, `cargo test`, `cargo test --features integration`. Review diffs for `as any`/`.unwrap()` in prod paths, `TODO:` markers, `println!` in library code, dead code, generic names. Verify docstrings on new public items.
  Output: `Build [PASS/FAIL] | Clippy [clean/N warnings] | Unit tests [N pass] | Integration tests [N pass] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start clean. Run the five new test projects through `compile_project` via integration tests; verify stdout for success fixtures and error variant shapes for failure fixtures. Spot-check one end-to-end via `cargo run --release --features integration` + binary exec. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Binaries exec [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance (no glob, no ecosystem test changes). Detect cross-task contamination.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

- **T1 + T2 + T3 + T10**: `feat(type-system): add file-role validator and error variants` — `src/type_system/errors.rs`, `src/module_loader.rs`, validator unit tests; pre-commit: `cargo test` + `cargo clippy`.
- **T4 + T5**: `feat(compiler): enforce .types.op separation and skip codegen for type-only modules` — `src/compiler.rs`; pre-commit: `cargo test`.
- **T6a/b/c + T7a/b**: `test(fixtures): add types-import test projects` — all `test-projects/*` additions; pre-commit: `cargo fmt --check`.
- **T8a/b/c + T9a/b**: `test(integration): assert type-import success and .types.op separation failures` — `tests/integration_e2e/*`; pre-commit: `cargo test --features integration`.

---

## Success Criteria

### Verification Commands

```bash
cargo build --release                                           # Expected: success
cargo clippy --all-targets --all-features -- -D warnings        # Expected: clean
cargo test                                                      # Expected: all pass (incl. new validator unit tests)
cargo test --features integration                               # Expected: all pass (incl. 5 new integration tests)
cargo test --features integration import_types                  # Expected: 3 success + 2 failure pass
# End-to-end sanity (one happy path):
cargo test --features integration import_types_basic_compiles_and_runs -- --nocapture
# Expected: stdout contains "Alice is 30 years old"
```

### Final Checklist

- [ ] Validator helper `is_types_file` + function `validate_module_file_role` exist and are unit-tested.
- [ ] Two new `TypeError` variants emit miette-formatted diagnostics with help text pointing to the spec.
- [ ] `compile_to_module` accepts `source_path: &Path` and invokes validator between parse and type-check.
- [ ] `compile_program` accepts `source_path: &Path` and threads it through to `compile_to_module`.
- [ ] All 23 `compile_to_module` call sites (across `src/compiler.rs`, `src/codegen/tests.rs`, `src/errors/tests.rs`, `tests/integration_e2e/tests.rs`) pass a `Path` arg.
- [ ] `src/app.rs:run_check_command` invokes validator between parse and type-check (so `opal check` CLI enforces the rule).
- [ ] `compile_project` invokes validator per-module after parse; skips codegen for modules where `is_types_file(path)` is true.
- [ ] Five new `test-projects/*` directories exist with `opal.toml`, `.gitignore`, `README.md`, `src/`.
- [ ] Five new integration tests: 3 success (importing types) + 2 failure (separation violations).
- [ ] `cargo test --features integration` green.
- [ ] No ignored-test flag changes; `language-spec/types_example.types.op` untouched.
- [ ] No new clippy warnings.
