# Draft: Bytes New to New Bytes

## Requirements (confirmed)
- Add a test project for current `bytes_new` functionality.
- Verify current `bytes_new` works as intended.
- Commit that work and fix any issues pre-commit finds.
- Add a test project for new syntax `let buffer: Bytes = new Bytes` and ensure it fails before implementation.
- Change Bytes initialization from `bytes_new` to `new Bytes` without trailing colon/properties.
- Make the new test project pass.
- Refactor after functionality works.
- Update documentation references from `bytes_new` appropriately.
- Use subagents for research because the repo is large.
- Use Serena for navigating the codebase.

## Technical Decisions
- Planning mode: produce a decision-complete execution plan rather than directly mutating source/test/docs files.
- Test approach: RED-GREEN-REFACTOR is required by project memory and matches the user's requested fail-then-pass workflow.
- Commit approach: plan includes three green commits: legacy `bytes_new` test coverage, `new Bytes` feature implementation, and docs update.
- Compatibility decision: keep source-level `bytes_new()` and runtime ABI `bytes_new` intact; docs shift users to `new Bytes`.
- Syntax decision: parser may represent propertyless `new <Ident>`, but typechecker gates this plan to `Bytes` only; non-Bytes propertyless constructors remain invalid.
- High accuracy review: Momus initially rejected invalid QA commands; plan was fixed for missing `docs/` path, cargo multi-filter usage, and pipefail RED evidence; Momus re-review returned OKAY.

## Research Findings
- Project memory: test projects live under `test-projects/<name>/` with `opal.toml`, `.gitignore`, `README.md`, and `src/main.op`.
- Project memory: integration/e2e tests use `cargo test --features integration`; full suite also uses `cargo test`.
- Project memory: mandatory TDD protocol is RED-GREEN-REFACTOR.
- Test project agent: canonical Bytes e2e is `test-projects/bytes-hex-roundtrip`; harness is `tests/integration_e2e/bytes_stdlib.rs`; existing output checks assert process success and stdout substrings.
- Implementation agent: `bytes_new` runtime ABI lives in `runtime/opal_bytes.c`/`runtime/opal_runtime.h`; type registration in `src/type_system/checker/bytes_builtins.rs`; module resolver in `src/type_system/module_resolver/standard_symbols_core_io_and_bytes.rs`; stdlib codegen declaration in `src/codegen/functions_stdlib.rs`.
- Implementation agent: `src/parser/new_expression.rs` currently requires `new <Type>:` with fields; no propertyless `new Type` form exists.
- Implementation agent: constructor typechecking/codegen paths are `src/type_system/checker/constructors.rs`, `src/type_system/checker/expressions.rs`, `src/codegen/expressions.rs`, plus ADT helpers.
- Docs agent: public/living docs with `bytes_new` include `stdlib/prelude.op`, `plan/bytes-stdlib-integration-plan.md`, `plan/bytes-type-plan.md`, and `PLAN.md`; proposal examples already show `new Bytes:` and need consistency review.
- Oracle: safest strategy is syntax sugar for `new Bytes` lowering to existing runtime `bytes_new()`; keep source-level `bytes_new()` as compatibility alias but remove it from primary docs/examples.
- Oracle: do not generalize propertyless `new Type`; accept only bare `new Bytes` without colon, keep `new Person` and `new Message.Text` invalid, and preserve `new Type:` / `new Type.Variant:` behavior.

## Open Questions
- Test strategy is effectively determined by request and project memory: RED-GREEN-REFACTOR with integration e2e and unit/codegen/typechecker tests.

## Scope Boundaries
- INCLUDE: test projects, compiler syntax/semantic changes, refactor, documentation updates, verification commands, commit/pre-commit handling.
- EXCLUDE: unrelated Bytes API changes, unrelated docs cleanup, executing implementation in this planning session.
