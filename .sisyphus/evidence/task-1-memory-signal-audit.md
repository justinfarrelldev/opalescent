# Task 1 Memory Signal Audit

## RC leak metric decision

**Primary metric for RC-array over-retention leaks:** `opal_runtime_live_heap_bytes()` with the existing RC debug counters as supporting evidence.

### Why this is the right primary signal
- `runtime/opal_rc.h:182-197` documents that runtime heap accounting tracks only Opalescent RC/array heap bytes allocated via `opal_rc_alloc`.
- `runtime/opal_rc.c:78-90` updates `opal_runtime_live_bytes` only through `opal_runtime_account_alloc` / `opal_runtime_account_free`.
- `runtime/opal_rc.c:139-169` shows `opal_rc_alloc_tracked(...)` calls `malloc(...)`, initializes the RC header, then records `tracked_bytes` via `opal_runtime_account_alloc(tracked_bytes)`.
- `runtime/opal_rc.c:314-317` and `runtime/opal_rc.c:355` show the matching free/accounting path for RC objects and arrays, including `opal_array_alloc(...)` going through `opal_rc_alloc_tracked(..., OPAL_RC_DEBUG_COUNTER_ARRAYS)`.

That makes `opal_runtime_live_heap_bytes()` deterministic for the RC-array leak class: if a fresh RC array is over-retained, tracked live bytes stay non-zero after the scenario should have released the object.

### Why raw RSS alone is insufficient
Raw RSS is process-level memory, not ownership-model memory. It includes unrelated allocator behavior, libc arenas, test harness overhead, code pages, and timing noise. The RC regression we need is about whether a specific RC-owned allocation was balanced. `opal_runtime_live_heap_bytes()` directly measures the tracked RC/array allocation set, so it can fail on one leaked retain even when RSS does not move predictably.

## Call-temp leak metric decision

**Primary metric for malloc-backed call-argument/interpolation temporary leaks:** the existing sanitizer-based path in `scripts/array_memory_sanitizer.sh` (ASAN+LSAN, with Valgrind fallback), not `opal_runtime_live_heap_bytes()`.

### Why this is the right primary signal
- `src/codegen/expressions_string.rs:97-162` allocates interpolation result buffers with direct `malloc` in `allocate_interpolation_buffer(...)`.
- `runtime/opal_string.c:36-42` (`int64_to_string`) and the other `*_to_string` helpers allocate caller-owned strings with direct `malloc(...)`.
- Those allocations do **not** go through `opal_rc_alloc` / `opal_rc_alloc_tracked`, so they are outside the RC live-byte accounting described in `runtime/opal_rc.h:182-197`.
- `scripts/array_memory_sanitizer.sh:6-34` and `95-161` already define the repository's memory-verification path for leak/use-after-free detection using ASAN+LSAN markers, plus Valgrind fallback when `clang` is unavailable.

This fits the current repo direction because it reuses the existing verification lane instead of adding new production runtime instrumentation for non-RC malloc traffic.

## Coverage verdict for opal_runtime_live_heap_bytes

**Verdict:** `opal_runtime_live_heap_bytes()` does **not** cover malloc-backed interpolation temporaries.

### Direct evidence
- `runtime/opal_rc.h:184-185` explicitly says the measurement tracks only RC/array heap bytes allocated via `opal_rc_alloc`.
- `runtime/opal_rc.c:139-169` records tracked bytes only inside `opal_rc_alloc_tracked(...)`.
- `src/codegen/expressions_string.rs:150-162` allocates interpolation output buffers with a plain `malloc` declaration retrieved by `ensure_malloc_function(...)`, not `opal_rc_alloc(...)`.
- `runtime/opal_string.c:9-96` allocates `*_to_string` buffers with plain `malloc(...)`.

### Important nuance
Some malloc-backed string paths do participate in **debug object counters**:
- `runtime/opal_string.c:13,22,31,40,...` calls `opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS)` for `*_to_string` allocations.
- `src/codegen/scope_tracker.rs:419-447` emits `opal_rc_debug_note_free(...)` plus `free(...)` when a bound owned string is released through scope cleanup.

But that still does not make them visible to `opal_runtime_live_heap_bytes()`, and it does not prove direct call-argument temporaries are always cleaned up. The interpolation result buffer from `src/codegen/expressions_string.rs:150-162` is plain malloc with no RC live-byte accounting.

## Chosen RED/GREEN evidence workflow

### Leak class 1: RC-array over-retention
- **RED metric:** targeted integration/counter harness assertions on `opal_runtime_live_heap_bytes()` after the scenario completes, optionally supported by array debug counters.
- **GREEN metric:** the same targeted test returns to zero live tracked bytes and balanced relevant counters.
- **Why:** this leak class is specifically about RC/array ownership imbalance, and the runtime already exposes deterministic accounting for exactly that allocation family.

### Leak class 2: malloc-backed call-argument/interpolation temporaries
- **RED metric:** targeted integration regression routed through the existing sanitizer path (`scripts/array_memory_sanitizer.sh`) so LSAN/ASAN or Valgrind reports leaked temporaries on the pre-fix tree.
- **GREEN metric:** the same targeted regression runs clean under the sanitizer path with no leak markers.
- **Why:** the leak source is plain `malloc` outside RC live-byte tracking, so pretending `opal_runtime_live_heap_bytes()` covers it would create a false-green test. Reusing the existing sanitizer hook is the maintainable alternate path already present in the repo.

### Existing baseline evidence
- `tests/integration_e2e/memory_model_counters.rs:10-166` compiles and runs `tests/integration_e2e/fixtures/memory_model_counters.c` with `-DOPAL_ENABLE_INTERNAL_TESTING` and expects `counter_status=balanced`.
- `tests/integration_e2e/fixtures/memory_model_counters.c:40-59` prints per-category alloc/free/live counts and `counter_status=balanced` when all exercised counters return to zero.
- `scripts/array_memory_sanitizer.sh:31-34` already includes `memory_model_counters` in `MEMORY_VERIFICATION_TESTS`.

## Commands executed

```bash
cargo test --features integration --test integration_e2e memory_model_counters -- --nocapture --test-threads=1

cc -std=gnu11 -DOPAL_ENABLE_INTERNAL_TESTING -I. \
  runtime/opal_rc.c runtime/opal_error.c runtime/opal_string.c runtime/opal_bytes.c runtime/opal_fs.c \
  tests/integration_e2e/fixtures/memory_model_counters.c \
  -o /tmp/opalescent-task1-memory-model-counters/memory_model_counters_harness

/tmp/opalescent-task1-memory-model-counters/memory_model_counters_harness \
  /tmp/opalescent-task1-memory-model-counters/workspace
```
