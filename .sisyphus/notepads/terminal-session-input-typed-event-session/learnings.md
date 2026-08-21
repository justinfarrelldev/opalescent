# Learnings

- 2026-08-15: Task 1 preflight found a clean `main` at `1e430e27e90ab87dbc0d531be8dda2797de67cb7`; the requested feature branch was created directly from that commit, and the acceptance ancestry check exits 0.
- 2026-08-15: The plan contains 37 implementation tasks plus final gates F1-F4; the exhaustive snapshot is recorded in `.sisyphus/evidence/task-1-branch-todo.md` before implementation begins.
- 2026-08-15: Task 2 harness docs: official Cargo docs say default `cargo test` builds/runs lib, bins as needed, integration tests, and doctests; target filtering can use `cargo test --test integration_e2e -- module::test`; ignored tests compile but do not run by default and can be run with `cargo test -- --ignored` or included with `--include-ignored`.
- 2026-08-15: External terminal docs for later tasks: Microsoft `ReadConsoleInputW` removes records and console input handles are waitable when unread input exists; Linux termios docs warn `O_NONBLOCK` may override MIN/TIME and raw `cfmakeraw` disables ISIG, which conflicts with proposal guidance not to clear ISIG unless control-key capture is requested.


- 2026-08-15: Runtime I/O is layered: `src/runtime/io.rs` (`IoHandler`, `DefaultIoHandler`, `print`, `take_input`) wraps host stdin/stdout; `src/stdlib/io.rs` provides mockable language-level `StdlibIoHandler` with `MockStdlibIoHandler` for tests.
- 2026-08-15: Terminal/stdout support already exists end-to-end in `src/type_system/checker/stdout_text_builtins.rs`, `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs`, `src/codegen/functions_stdlib.rs`, `src/codegen/functions_call_helpers.rs`, `src/codegen/statements/runtime_type_info.rs`, and `runtime/opal_io.c` (`stdout_terminal`, `terminal_supports_ansi`, clear/move/draw helpers).
- 2026-08-15: Time support is partially implemented: `src/type_system/checker/time_builtins.rs` registers `sleep_ms_sync`, `frame_clock_new`, `frame_clock_wait_next_sync`; runtime C backs it with `runtime/opal_io.c` + `runtime/opal_portability.h` monotonic sleep/time helpers.
- 2026-08-15: Process APIs split between `src/stdlib/system/process.rs` (trait-based `ProcessManager`, `StdProcessManager`, `MockProcessManager`) and checker/module-resolver registration in `src/type_system/checker/process_builtins.rs` + `src/type_system/module_resolver/standard_symbols_process.rs`.
- 2026-08-15: Hot-reload readiness exists via `src/hot_reload/change_detection.rs` (`FileWatcher`, `MockFileWatcher`, `PollingFileWatcher`), `src/hot_reload/loader.rs`, `src/hot_reload/abi.rs`, `src/hot_reload/classifier.rs`, and `src/hot_reload/guard.rs`; `app.rs` already uses polling watch mode.
- 2026-08-15: Native/runtime integration is centralized in `src/compiler.rs` (`RUNTIME_SOURCE`, embedded `runtime/*.c` and headers, MSVC C compile path, linker orchestration) and `runtime/opal_runtime.c` / `runtime/opal_portability.h` / `runtime/opal_runtime.h`.
- 2026-08-15: Platform/Wine handling lives in `src/build_system/targets.rs`, `src/build_system/linker.rs`, `src/compiler.rs`, `tests/integration_e2e/windows_wine.rs`, and `scripts/verify-wine-prereqs.sh`; Linux→MSVC requires XWIN_CACHE or OPAL_XWIN_SYSROOT and uses lld-link/clang-cl.
- 2026-08-15: Runtime error representation hooks: Rust side `src/runtime/errors.rs` + `src/runtime/reporting.rs`; C side `runtime/opal_error.c`, `runtime/opal_bytes.c`, `runtime/opal_string.c`, `runtime/opal_fs.c`, and `runtime/opal_runtime.h` all use stable `{value,error}` / `Fs*Result` conventions for guard/propagate lowering.
- 2026-08-15: Task 2 harness map: add terminal fixture modules under `tests/integration_e2e/` and register them in `tests/integration_e2e/tests.rs`; use `tests/integration_e2e.rs` shared helpers, `fs_helpers.rs`/`fs_state_guard.rs` for isolation, `interactive_io.rs` for piped stdin/stdout/status, `terminal_stdlib.rs` for exact terminal bytes, `string_stdlib_projects.rs::assert_project_stdout`, and `compile_failures.rs`/`time_stdlib.rs` for compile-fail patterns.
- 2026-08-15: Gated RED pattern: use dedicated ignored/env-gated integration tests modeled on `tests/integration_e2e/game_of_life_full_memory_stress.rs`; document opt-in command near traceability. Useful command shapes: `cargo test --test integration_e2e -- module::test`, `cargo test -- --ignored`, and `cargo test -- --include-ignored`.
- 2026-08-15: Compiler surface map for Tasks 6-12: AST/parser metadata in `src/ast/metadata.rs`, `src/ast/helpers.rs`, `src/ast/node_impls.rs`, `src/parser/declarations.rs`, `src/parser/imports.rs`, `src/parser/types.rs`; declaration/type handling in `src/type_system/checker/declarations.rs` and `src/module_loader.rs`; standard modules in `src/type_system/module_resolver/**`; checker builtins in `src/type_system/checker/*_builtins.rs`; codegen dispatch in `src/codegen/functions_stdlib.rs`, `src/codegen/functions.rs`, `src/codegen/functions_call.rs`, `src/codegen/statements/runtime_type_info.rs`, and error ABI in `src/codegen/error_abi.rs`.

- 2026-08-15: Task 2 added a 20-row proposal traceability matrix and an ignored/env-gated RED integration probe. Because `tests/integration_e2e.rs` is behind the `integration` feature, the exact opt-in RED command is `OPAL_TERMINAL_SESSION_RED=1 cargo test --features integration --test integration_e2e -- terminal_session_input_gated --ignored --nocapture`; it currently exits 101 as expected while selected terminal session APIs are unimplemented.

- 2026-08-15: Task 3 core terminal fixtures live under `test-projects/terminal-{key-log,text-echo-safe,size-probe,pause-counter}` and are registered only in the ignored/env-gated `terminal_session_input_gated_core_fixtures_red` harness; default `cargo test` remains green while the opt-in RED command exits 101 until selected terminal APIs/types are implemented.
- 2026-08-15: Task 3 review tightened fixture semantics: `terminal-key-log` should inspect `event.key` and `event.modifiers`, `terminal-text-echo-safe` should declassify `event.text` through `trusted_terminal_output_from_application_text`, and `terminal-pause-counter` should check `event.reason is TerminalInputResetReason.PauseBoundary`.

- 2026-08-15: Task 4 added the second ignored/env-gated terminal fixture set under `test-projects/terminal-{legacy-io-rejection,timeout-menu,cancel-demo,diagnostics-inspector}` and registered it in `terminal_session_input_gated_coordination_and_diagnostics_fixtures_red`; fixture metadata now records both deterministic input plans and fault plans.
- 2026-08-15: Task 4 RED evidence command remains `OPAL_TERMINAL_SESSION_RED=1 cargo test --features integration --test integration_e2e -- terminal_session_input_gated --ignored --nocapture`; it exits 101 with valid layouts and expected `front-end compilation failed` rejections until selected terminal/test support lands.
- 2026-08-15: Task 4 diagnostics inspector fixture statically calls every selected stable terminal diagnostic inspector and safe formatter, and treats structured fields—not formatted diagnostic text—as authority.
- 2026-08-15: Task 4 retry added Task 3-style `## Description: ... ##` blocks before every public Task 4 fixture `entry main`; direct `cargo run -- check` probes now report zero `missing_doc_comment`, leaving only missing selected symbols or proposed `is ... into` parser support as RED attribution.

- 2026-08-15: Task 5 added the remaining six terminal RED fixtures under `test-projects/terminal-{chord-quit,paste-quarantine,game-of-life-interactive,sokoban-mini,file-picker,stopwatch-pomodoro}` and registered them in a separate ignored/env-gated harness slice. Direct `cargo run -- check` probes remain RED only for selected terminal/chord symbols and future error types with zero `missing_doc_comment`; default `cargo test` passes, the no-env ignored target skips, and the opt-in RED command exits 101.

- 2026-08-15: Task 6 parser support added proposal-only declaration metadata and type forms across lexer/AST/parser: annotations (`@availability`, `@abi_type_id`, etc.), namespaces, constrained/opaque/compiler-registered/public-non-exhaustive type declarations, explicit variant IDs, hex integer literals, and hyphenated import paths. Because `@` is now valid syntax, invalid-character regression fixtures should use another character such as `$`.

- 2026-08-15: Task 6 repair extracted proposal metadata parsing into `src/parser/declaration_metadata.rs`; shared parser helpers called across parser submodules need `pub(super)` visibility, and ignored/private notepad updates should stay on disk but out of the atomic source/evidence commit.

- 2026-08-15T18:17:31: Task 7 parser work kept proposal words contextual:  dispatches only in statement position,  parses only before a type-like target, borrow modifiers are consumed for parser acceptance without ownership semantics, and broad  identifier support must still reject legacy bare  outside active guard clauses.

- 2026-08-15T18:17:48: Task 7 parser correction: using dispatches only in statement position, constrain parses only before a type-like target, borrow modifiers are consumed for parser acceptance without ownership semantics, and broad propagate identifier support must still reject legacy bare err outside active guard clauses.


## Task 7 repair notes - 2026-08-15

- Parser acceptance is insufficient for new syntax: Task 7 syntax must be represented by explicit AST nodes (`BorrowArgument`, `Constrain`, `Refinement`, `Propagate.cause`, `Stmt::Using`) so later passes cannot erase source intent.
- Conservative downstream helpers should unwrap borrow/parenthesized wrappers and inspect propagated causes for traversal/cleanup, but must not implement Tasks 8/13/14/15/16 runtime semantics.
- Documentation signature extraction must include parameter borrow modes; otherwise `ref` and `mutable ref` parameter syntax is accepted but erased from generated API text.
- Current fast verification: `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo check` passed after the repair dispatcher updates.


## Task 7 final verification - 2026-08-15

- Green verification passed after repair: `cargo check`, `cargo fmt --all -- --check`, `cargo test terminal_proposal`, `cargo test parser`, full `cargo test`, and strict `cargo clippy --all-targets -- -D warnings`.
- Expected RED terminal verification still fails under `OPAL_TERMINAL_SESSION_RED=1`, confirming Task 7 did not implement terminal runtime/API support.


## Commit-hook repair note - 2026-08-15

- The line-count hook can reject otherwise-green parser repairs when central files cross their caps. Resolve by extracting existing helpers into focused modules instead of skipping hooks or widening task scope.
- After extraction, rerun the same green and expected-RED verification; the final Task 7 repair remains parser/AST preservation only.

## Final hook-lint note - 2026-08-15

- `cargo make lint` matches the pre-commit lint profile more closely than plain `cargo clippy --all-targets -- -D warnings`; use it before committing parser repairs that touch central dispatchers.

## Task 8 formatter/doc preservation - 2026-08-15

- Formatter expression/statement support for Task 7 syntax already existed; the missing presentation path was declaration metadata/type forms. Preserve annotations before the type line, form prefixes before `type`, constrained alias `where` predicates, and opaque/resource declarations without a trailing colon.
- Explicit variant IDs with payload fields must format in the parser-supported block shape (`Variant = id:` then indented fields), not parenthesized payload syntax.
- Doc generation has no unsupported-diagnostic channel today, so public proposal type metadata should be represented in signatures rather than dropped.

## Line-count hook repair - 2026-08-15

- `src/formatter/printer.rs` crossed its explicit 1050-line hook allowance after Task 8. Moving pure renderers to `src/formatter/printer_helpers.rs` dropped it to 1006 lines while preserving the exact Task 8 formatter/doc output assertions.
- Run both `cargo make lint` and `bash scripts/check-line-count.sh` when a handoff mentions hook line-count failure; `cargo make lint` covers strict Clippy, while the hook invokes the standalone script directly.

## Task 9 module-resolution loading - 2026-08-15

- Authoritative terminal declaration loading now lives in `src/type_system/module_resolver/terminal_proposal_modules.rs`; it uses `include_str!` against the exact proposal paths and stores parsed declaration metadata on `ModuleInterface` rather than recopying declarations into Rust constants.
- `ModuleInterface` now carries availability plus parsed namespace/type declaration metadata, including annotations, declaration forms, `TypeDef`, spans, product/variant fields, and explicit variant IDs for later terminal tasks.
- Import gating is checker-side: `standard.testing.terminal` is registered but production imports return `TypeError::ModuleUnavailable` with a test-only reason; `standard.terminal` and `standard.terminal.chords` are registered for metadata but remain future-public gated.
- `module_loader.rs` must recognize these dotted stdlib proposal names as `__stdlib__/*` sentinels so project discovery reaches the checker availability diagnostic instead of failing early with `ModuleNotFound`.
- Final verification passed: LSP diagnostics on modified files, targeted resolver/typechecker tests, `cargo fmt --all -- --check`, full `cargo test`, and `cargo make lint`; the opt-in terminal RED gate still exits 101.

## Task 10 ABI validation - 2026-08-20

- Terminal ABI/history validation now lives in `src/type_system/module_resolver/terminal_proposal_abi.rs` and consumes Task 9 `ModuleInterface::type_declarations`; active `.types.op` metadata remains the active selected/chord/test-only source instead of reparsing proposal files.
- The validator enforces exactly 87 selected production type IDs, exactly 23 chord production type IDs, selected intentional gaps, selected retired variant IDs, explicit positive unique variant IDs, the uncommitted `TerminalSessionRestoreError.GenerationExhausted=14` candidate, and test-only no-production-ABI scope.
- Targeted command `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14 cargo test terminal_abi` covers positive inventory plus retired-ID, test-only authority, and uncommitted-candidate fixtures; full fmt/test/lint/line-count verification passed for Task 10.

- 2026-08-20: Task 10 retry found active variant discriminators need their own abi-history authority table: `.types.op` remains the field/payload source, but selected/chord active variant names and explicit IDs must match `abi-history.md`; large authority tables should live in a sibling helper module to keep `terminal_proposal_abi.rs` under the line-count hook.

- 2026-08-20: Task 10 non-sum retry closed a bypass where active variant history tables were only checked inside the `TypeDef::Sum` loop; always validate history-tracked declarations exist and remain sums before iterating active variants.

## Task 11 symbol/typechecker scaffold - 2026-08-20

- Terminal proposal function signatures now have one authoritative Rust inventory in `src/type_system/module_resolver/terminal_proposal_symbols.rs`; selected, chord, and test-only symbol tables register onto the proposal `ModuleInterface`s without changing production availability.
- The internal-only checker gate is `TypeChecker::enable_terminal_proposal_imports_for_tests()`: it permits future-public terminal imports for focused tests and registers prerequisite nominal terminal types, while normal production checks still reject `standard.terminal` and `standard.terminal.chords`.
- Constructor visibility metadata is tracked from `@constructor_visibility(...)` annotations and imports; sealed runtime/test-runner-issued terminal values should fail direct user construction with `TypeError::ConstructorUnavailable` rather than being treated as ordinary product constructors.
- Full Task 11 verification passed with LSP diagnostics, fmt check, targeted `cargo test terminal_proposal`, full `cargo test`, strict `cargo make lint`, `cargo build`, and `bash scripts/check-line-count.sh`.
- Task 11 core prerequisites use a synthetic `standard.system` `ModuleInterface` with `ModuleAvailability::FuturePublicApi`; this keeps core/system function signatures import-testable under the internal adoption gate without adding terminal ABI IDs, ABI-history entries, parsed terminal type declarations, or runtime/codegen behavior.

## Task 12 codegen gate - 2026-08-20

- Codegen gate checks should query Task 11 terminal proposal symbol inventory instead of duplicating proposal function names; narrow facade helpers in `type_system.rs` keep `module_resolver` private while allowing codegen to identify gated symbols.
- The Task 12 diagnostic is emitted at both stdlib import resolution and imported-function call resolution, preventing import-time LLVM extern declarations and internal test bypasses from producing unresolved terminal proposal runtime calls.
- The line-count hook leaves very little headroom in `src/type_system/module_resolver/terminal_proposal_symbols.rs`; keep new helpers compact or extract before adding more inventory logic.

## Task 13 affine ownership and borrows - 2026-08-20

- Affine ownership state lives in `src/type_system/checker/ref_rules.rs` as scoped owner/borrow tables on `TypeChecker`; lexical scope entry/exit mirrors `SymbolTable` scope handling so local owners/borrows disappear without introducing Task 14 cleanup effects.
- The checker treats nominal types registered from local `affine resource` declarations or terminal core-prerequisite metadata as noncopyable owners; owned-parameter calls mark owners moved, and later uses report use-after-move.
- Second-class call-site borrows (`ref`/`mutable ref`) are accepted only when the callee parameter metadata expects the same borrow kind, and escape analysis rejects borrow/owner storage through returns, assignments, arrays, constructor fields, and lambda captures.
- `check_value_escape_in` is the compact no-fresh-owner wrapper used from statement checking to preserve the line-count hook while keeping `let` initializers able to accept fresh owner-producing calls.
- Task 13 follow-up: borrow-mode metadata must live on `ModuleInterface` as function metadata, not only local checker state; named imports, aliases, globs, and module aliases all need to restore qualified borrow entries so imported proposal APIs still require explicit call-site `ref` / `mutable ref`.


## Task 13 retry metadata completeness - 2026-08-21

- Terminal proposal test adoption must register compiler-known affine resource names from both core prerequisites and selected/chord/test-only proposal metadata, because callers can infer owner types from imported functions without importing the types themselves.
- Borrow metadata is binding metadata, not just function metadata: local let-bound lambdas need the same `ref` / `mutable ref` parameter registration as top-level functions, and shadowing must clear/restore function borrow entries lexically to avoid stale call-site requirements.
- `CancellationSource` is a core-prerequisite affine authority; keep it in the compiler-known test-gated resource set alongside `ProcessControlSource`, `SystemWaitSet`, and timer/wait-registration resources.


## Task 14 using cleanup - 2026-08-21

- `using` cleanup typing now lives in `src/type_system/checker/using_cleanup.rs`; keep future cleanup/effect additions there instead of growing `checker/statements.rs` or `ref_rules.rs`.
- The compiler-visible cleanup registry is static and exact for Task 14: core/system resources, `TerminalSession`, `TerminalChordRouter`, and test-only terminal resources. `TerminalSessionRestoreError.CloseRestorePending` is the only cleanup-authority transfer entry.
- `Stmt::Using` codegen is intentionally a lexical scope wrapper in `src/codegen/statements/using_cleanup.rs`; it participates in existing reverse scope cleanup without opening public terminal runtime behavior.
- `codegen_propagate_expression` now runs `cleanup_return_scopes_preserving_codegen_env` on the error early-return block, matching return/break/continue scope cleanup behavior.
- Final Task 14 verification passed: fmt check, line-count, `cargo test using`, `cargo test affine`, `cargo test terminal_proposal`, full `cargo test`, `cargo check`, `cargo build`, and `cargo make lint`.


## Task 14 retry cleanup obligations - 2026-08-21

- Existing lexical scope cleanup is not sufficient for `using`; Task 14 needs explicit obligation state visible to both the checker and codegen so successful explicit close can consume cleanup, failed close can retain it, and exact cleanup-authority transfer can be modeled.
- Codegen cleanup obligations should be tracked beside scope bindings and released in reverse binding acquisition order, before ordinary RC/malloc-string cleanup, so `return`, `break`, `continue`, and propagated errors share the same cleanup path.
- `terminal_session_close_sync(mutable ref session)` is the only Task 14 explicit-close consumption shape, and `TerminalSessionRestoreError.CloseRestorePending` is the only cleanup-authority transfer; keep these recognizers exact rather than name-prefix based.
- Retry verification passed with LSP diagnostics, fmt check, line-count, `cargo test using`, `cargo test affine`, `cargo test terminal_proposal`, full `cargo test`, `cargo check`, `cargo build`, and `cargo make lint`.


## Task 14 retry 2 cleanup semantics - 2026-08-21

- Codegen must not consume `using` cleanup authority at raw call emission time. For fallible explicit close, consume only in the `propagate` success continuation after the error branch is known.
- Runtime transfer modeling needs the exact registered tuple in codegen, not only the checker: `terminal_session_close_sync` + `TerminalSessionRestoreError` + `CloseRestorePending` now drives the transfer branch that skips lexical cleanup.
- Current error ABI can forward one error pointer but has no cause/suppressed attachment constructor yet; Task 14 cleanup ordering is modeled by forwarding the first cleanup failure as primary and preserving the original body/propagate error when cleanup succeeds.
- `src/codegen/functions_call.rs` has a tight 1180-line cap; using-cleanup call helpers belong in `src/codegen/functions_call/using_cleanup.rs` to keep propagation logic maintainable and line-count clean.
- Final retry 2 verification passed with LSP diagnostics, fmt, line-count, `cargo test using`, `cargo test affine`, `cargo test terminal_proposal`, full `cargo test`, `cargo check`, `cargo build`, and `cargo make lint`.


## Task 14 retry 3 guard cleanup semantics - 2026-08-21

- Guard close handling cannot reuse the propagate compile-time consume model: guard expressions can recover through fallback paths, so cleanup consumption must be path-sensitive rather than a global `env.consume_using_cleanup_obligation`.
- Codegen now models guarded explicit close success with a runtime consumed flag stored only in guard success blocks; lexical cleanup branches on that flag so success skips duplicate close while failure/fallback paths retain cleanup.
- The checker transfer inspector should not be invoked just to discard a boolean. Removing the no-op keeps transfer recognition meaningful through existing exact helper tests and the codegen transfer branch.
- `cargo make lint` catches helper growth quickly; extracting `enter_using_cleanup_runtime_guard` kept `release_using_cleanup_obligation` below the 100-line Clippy limit without suppressing lint.

## Task 14 retry 4 guard transfer semantics - 2026-08-21

- For guards, the runtime cleanup flag means cleanup authority is gone, not only close success. Set it on guarded close success and on failure only when the exact registered `TerminalSessionRestoreError.CloseRestorePending` transfer matches; ordinary failures must leave lexical cleanup active.
- Guard statement/expression regressions should assert the transfer branch IR (`CloseRestorePending`, `strcmp`, `using.transfer.mark`, `using.transfer.keep`) as well as the cleanup run/skip flag blocks.

## Task 15 refinement/checker semantics - 2026-08-21

- Nominal variant refinement belongs in the checker as branch-local type/binding state: `is Variant into payload` narrows only the true branch and the payload binding must not escape.
- `constrain Type from value` is a fallible expression surface, not just a cast; it must require `ConstraintViolationError` on the enclosing fallible declaration and participate in normal guard/propagate handling.
- Guard-error heterogeneous unions should carry narrowed family information from refinement branches, while unrefined heterogeneous propagation must still require every possible family on the enclosing error surface.


## Task 15 follow-up refinement proof - 2026-08-21

- Proposal line 17 makes `into` optional; keep a dedicated `terminal_refinement`-filtered checker test where `if event is TerminalInputEvent.Key:` narrows `event` itself enough for payload field access.
- Heterogeneous guard-union proof must start from a multi-family guarded call such as `TerminalSessionOpenError, TerminalSessionReadError`; a single-family guard only proves ordinary family propagation.
- No checker implementation change was needed for the follow-up: `extract_variant_refinement` already handles no-`into` `BinaryOp::Is`, and `active_guard_propagation_error_types` already narrows refined guard bindings to the parent family.

## Task 16 immutable error attachments - 2026-08-21

- Immutable error attachment runtime state lives in `runtime/opal_error.c` as side-table metadata keyed by stable error strings; `opal_error_new` interns/copies primary values, while `opal_error_attach_cause` returns the primary string and records cause/suppressed metadata deterministically.
- Implemented Task 16 inspectors are the only core-prerequisite `standard.system` symbols allowed through the terminal proposal gate before the public terminal API opens; their allowlist now lives in `src/type_system/module_resolver/terminal_proposal_error_inspectors.rs`.
- Direct `propagate error_value cause prior_error` must stay on the identifier/error-value path, while `propagate new FallibleConstructor` still lowers through ordinary fallible constructor codegen and then branches on its error field.
- Final verification evidence is recorded in `.sisyphus/evidence/task-16-errors.txt`; focused negative/cycle/bounds/gating evidence is in `.sisyphus/evidence/task-16-errors-error.txt`.

## Task 16 semantic follow-up - 2026-08-21

- Checker/codegen parity for direct error-value propagation must cover every wrapper accepted by `propagated_error_value_types`: bare identifiers, parenthesized identifiers, and borrow-argument wrappers all need the immediate cleanup-aware error return path.
- `runtime/opal_error.c` now participates in `OPAL_ENABLE_INTERNAL_TESTING` allocation fault injection; attachment metadata allocation failures stay best-effort, preserve existing aliases/edges, and surface as the bytes truncation marker rather than a new public error family.

## Task 17 test-only availability boundaries - 2026-08-21

- Test-only availability is a declaration-boundary property: production checking must reject `@availability(test_only)` and sealed `@constructor_visibility(test_runner)` declarations before their signatures enter generated exports.
- Production-visible surfaces need recursive type checks through fields, function parameters/returns/errors, generic type arguments, and generic constraints; the shared helper in `src/type_system/checker/test_only_availability.rs` keeps this out of declaration registration hot paths.
- `ModuleResolver::core_type_test_only_reference` is the cross-module source of truth for generated/exported core type signatures that may indirectly reach test-only declarations.
- Final Task 17 evidence is recorded in `.sisyphus/evidence/task-17-test-only.txt`; focused negative boundary evidence is in `.sisyphus/evidence/task-17-test-only-error.txt`.

## Task 17 verification follow-up - 2026-08-21

- Atlas-style verification checks committed repository artifacts, not just transient command output. Force-add ignored `.sisyphus/evidence/task-17-test-only*.txt` files when a task explicitly requires evidence files.
- Task 17 sealed-authority coverage now includes direct construction rejection for `TerminalTestAuthority`, `TerminalTestScenario`, `TerminalTestBackendActivation`, `TerminalTestFakeBackend`, and `TerminalTestFactoryError` in test-authorized mode.

## 2026-08-21 Task 18 aggregate implementation
- EditorChordRuntime transactional aggregate support stayed private/proposal-gated: aggregate specs live in src/type_system/affine_aggregates.rs and checker/codegen hooks consume those specs without adding public owner exports or general field moves.
- The retarget/commit rule is easiest to enforce as a pending checker barrier: a successful retarget records the expected commit target, and the next fallible/observable operation or mismatched commit reports an interposition diagnostic.
- Verification for this task passed with LLVM_SYS_140_PREFIX=/usr/lib/llvm-14: cargo fmt --all -- --check, scripts/check-line-count.sh, cargo check, cargo test terminal_aggregate, cargo clippy --all-targets --all-features -- -D warnings, and cargo test.

## 2026-08-21 Task 18 Atlas retry fixes
- Keep TypeChecker transient state in a checker submodule when adding new task-specific context; moving TypeCheckContext to src/type_system/checker/context.rs kept checker.rs at 1039 lines and preserved field documentation.
- Transactional aggregate construction must preserve nominal representation: seal fields as an allocation-free SSA struct, then store that sealed value in the normal RC nominal payload and return i8* for non-Pair CoreType::Generic.
- Focused codegen tests should build a real return/store-compatible path and verify the LLVM module; checking only lowered.is_struct_value() missed the anonymous-struct/nominal-pointer mismatch.
