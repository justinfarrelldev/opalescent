# Task 5 Call-Temp Measurement

## Selected metric

**Selected metric:** the existing sanitizer path in `scripts/array_memory_sanitizer.sh` (ASAN+LSAN, with Valgrind fallback) is the primary deterministic RED/GREEN signal for leak class #2: malloc-backed interpolation and call-temporary leaks.

## Why this is the stable primary signal

- `src/codegen/expressions_string.rs:61-92` returns an interpolation buffer allocated by `allocate_interpolation_buffer(...)`, and `allocate_interpolation_buffer(...)` uses plain `malloc` at `src/codegen/expressions_string.rs:150-162`.
- `runtime/opal_string.c:9-97` allocates `*_to_string` helper results with plain `malloc`, not through `opal_rc_alloc` or `opal_rc_alloc_tracked`.
- Task 1 already established that `opal_runtime_live_heap_bytes()` is limited to RC/array allocations, so it cannot be the deterministic primary metric for these malloc-backed temporaries.
- `scripts/array_memory_sanitizer.sh:6-18` and `95-161` already define the repository's maintainable memory-verification lane for leak/use-after-free detection without adding production instrumentation.

This aligns with repo direction because it reuses the existing memory verification path instead of introducing a new production runtime accounting framework just for this bug class.

## Why existing scope/string tests are not enough

The existing green baseline in `tests/integration_e2e/scope_leak_counters.rs` is still useful, but it measures a different cleanup path:

- `src/codegen/scope_tracker.rs:150-172` and `174-218` identify owned-string-producing calls when the result becomes a binding or otherwise enters scope-tracked cleanup metadata.
- `src/codegen/scope_tracker.rs:403-448` emits `opal_rc_debug_note_free(...)` and `free(...)` for those tracked malloc-backed string bindings during scope cleanup.
- The baseline command from this task (`cargo test --features integration --test integration_e2e scope_leak_counters -- --nocapture --test-threads=1`) passes because those scenarios bind the owned strings and then release them through existing scope cleanup.

That does **not** prove direct call arguments are safe. A direct call like `writer_write_sync(writer, 'tick {i}')` can allocate a temporary interpolation buffer and pass it immediately into call lowering without ever becoming a scope-tracked binding. That is the leak class Task 6 must drive red.

## Deterministic pre-fix vs post-fix signal

### Pre-fix expected signal

For a targeted direct call-temp regression (for example, direct interpolation passed as an owned argument, or a propagated call that exits early after creating owned temporaries):

- the sanitizer-backed run should fail,
- and the failure should surface as a leak report from LSAN/ASAN, or from the Valgrind fallback when `clang` is unavailable.

That is stable because the leaked object is a concrete malloc allocation that survives process exit. The probe does not depend on allocator arena behavior, RSS jitter, or sampling timing.

### Post-fix expected signal

For the same regression after generic call-temp cleanup is implemented:

- the sanitizer-backed run should exit successfully,
- and `scripts/array_memory_sanitizer.sh` should complete without any leak markers such as `LeakSanitizer`, `detected memory leaks`, `heap-use-after-free`, or `double-free`.

This cleanly distinguishes pre-fix vs post-fix for leak class #2: the exact same scenario goes from sanitizer-detected leaked temporary to fully cleaned temporary with no leak markers.

## Why RSS is not the primary metric

RSS remains a coarse end-to-end signal, but it is not deterministic enough for the Task 6 RED tests:

- allocator reuse and libc arenas can keep RSS flat even when a temporary leak exists,
- or RSS can move for unrelated reasons while ownership behavior is correct.

So RSS can be a secondary stress signal later, but not the primary deterministic RED/GREEN probe for call-temp leak regressions.

## Generic cleanup test probe design for Task 6

The maintainable test probe should follow this shape:

1. Add targeted integration tests under `tests/integration_e2e/` for direct call-temporary scenarios.
2. Compile tiny `.op` programs that create malloc-backed owned strings via direct interpolation or `*_to_string` helpers and pass them straight into calls, especially:
   - direct owned argument on normal return,
   - owned argument before `propagate` / early return,
   - mixed borrowed + owned args,
   - nested call where a later failure must still clean earlier temporaries,
   - transferred/take-owned path proving no double-free.
3. Route those scenarios through the existing sanitizer verification lane so the same program either leaks at process exit (RED) or exits cleanly (GREEN).

### Why this probe shape is maintainable

- It matches the existing repository preference for integration-style memory verification.
- It avoids production ABI or runtime changes.
- It directly exercises the real lowering path in `functions_call` + `expressions_string`, rather than approximating the bug with a synthetic counter that does not see malloc-backed temporaries.

## Decision summary

- **Primary metric/path for leak class #2:** sanitizer-backed integration probe via `scripts/array_memory_sanitizer.sh`.
- **Reason it is stable:** it observes process-exit leaks for the exact malloc allocations that RC live-byte accounting does not cover.
- **How it distinguishes RED vs GREEN:** pre-fix targeted direct call-temp programs emit sanitizer leak markers; post-fix the same programs exit cleanly with no sanitizer markers.
- **Why the current green baseline does not invalidate the bug:** existing scope/string tests cover bound cleanup paths in `scope_tracker`, not direct call arguments that bypass binding registration.

## Baseline command captured for this task

The plan-directed baseline selector exists and is green:

```bash
cargo test --features integration --test integration_e2e scope_leak_counters -- --nocapture --test-threads=1
```

Its output is captured in `.sisyphus/evidence/task-5-existing-string-tests.txt`.
