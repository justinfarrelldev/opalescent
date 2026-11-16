# Native testing framework: verified integration map

Implements the delivery plan in `TESTING_FRAMEWORK.md`; progress is tracked in
`plans/checklists/native-testing-framework.md`.

## Baseline (Phase 0)

- CLI dispatch: `src/app.rs::run_test_command` currently constructs an empty
  `src/testing/runner.rs::TestSuite`. The existing Rust function-pointer runner
  is not a compiled Opalescent runner and is insufficient for this specification.
- Native pipeline: `src/compiler.rs::compile_to_module_for_target` lexes,
  parses, validates file role, checks types, and lowers through LLVM. Project
  entry points live in the same file, with project checks in
  `src/compiler/project_check.rs`.
- Frontend: `src/lexer.rs`, `src/token.rs`, `src/parser.rs`, split declaration,
  import, expression, statement and type parsers under `src/parser/`; AST in
  `src/ast.rs` and `src/ast/`.
- Resolution/checking: `src/type_system/checker/{declarations,module_checking,
  call_resolution,lambda_signatures,error_sets,ref_rules}.rs` and
  `src/type_system/module_resolver/`. File-role checks must remain enforced.
- Lowering: `src/codegen/` includes call, propagate and guard lowering; native
  runtime support lives under `runtime/`. Abort propagation must be tested
  through these call paths, not implemented as an application error.
- Build configuration/cache/targets/linking: `src/build_system/`.
- Editor fixture APIs confirmed in
  `test-projects/terminal-simple-editor/src/{buffer,input,state}.op`:
  `buffer_cursor(int64,int64)`, `backspace_at_cursor` returning labeled
  `updated_lines, updated_cursor`, `move_down` returning `CursorPosition`,
  `apply_text_input` and `apply_named_key` returning labeled
  `updated_lines, transition`, and `initial_editor_state(string,boolean,boolean)`.
  `save_lines` calls the imported `write_text_atomic_sync` edge. Existing errors
  are explicit nominal leaf sets. The fixture must preserve those labels and
  use `=>`, normal reference rules and `.types.op` declarations.
- Production compiler currently normalizes tabs before lexing. Test source
  positions must instead stay mapped to original bytes; prose is scanned from
  the original input, never source-rewritten.

Baseline verification:

- `cargo test --lib lexer::tests --quiet`: 24 passed.
- `cargo test --lib parser::tests --quiet`: 258 passed, 1 ignored.

## First frontend increment

Explicit `Lexer::new_test_module` enables contextual prose headers without
adding globally reserved keywords. Header and static-title tokens carry
original spans; callbacks resume ordinary lexing after the colon. This is a
lexer increment only, not yet a complete test parser or executable framework.

Red–green tests cover apostrophes, hashes, Unicode spans, escaped colons,
quoted titles, bare scenario, inline callback tokenization, malformed titles,
and unchanged ordinary identifiers. Quoted malformed titles stop on the
original line rather than consuming later source. Normal multiline application
strings retain their existing behavior.
