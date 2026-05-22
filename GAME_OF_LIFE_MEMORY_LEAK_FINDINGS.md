# Game of Life Memory Leak Findings

This document investigates memory-retention behavior in `test-projects/game-of-life-full`. The audit covers the app's Opalescent source, the standard-library/runtime functions it calls, and the relevant codegen/language constructs that determine ownership.

## Scope and evidence baseline

The app is a native terminal Opalescent program, not a browser or JavaScript app. In the app source under `test-projects/game-of-life-full/src`, there are no DOM nodes, event listeners, `setInterval`, `setTimeout`, `requestAnimationFrame`, canvas/WebGL/ImageData resources, React/Vue/Svelte lifecycles, Maps, or Sets.

Primary app files reviewed:

- `test-projects/game-of-life-full/src/main.op`
- `test-projects/game-of-life-full/src/render.op`
- `test-projects/game-of-life-full/src/rules.op`
- `test-projects/game-of-life-full/src/patterns.op`
- `test-projects/game-of-life-full/src/board.op`
- `test-projects/game-of-life-full/src/config.op`
- `test-projects/game-of-life-full/src/life.types.op`

Runtime/codegen files reviewed for the constructs used by the app:

- `runtime/opal_rc.c`
- `runtime/opal_rc.h`
- `runtime/opal_io.c`
- `runtime/opal_string.c`
- `src/codegen/adts.rs`
- `src/codegen/binding_store.rs`
- `src/codegen/control_flow.rs`
- `src/codegen/expressions.rs`
- `src/codegen/expressions_array.rs`
- `src/codegen/expressions_string.rs`
- `src/codegen/functions_call.rs`
- `src/codegen/functions_call/array/helpers.rs`
- `src/codegen/functions_call/array/intrinsics.rs`
- `src/codegen/functions_stdlib.rs`
- `src/codegen/scope_tracker.rs`
- `src/type_system/fallible_constructors.rs`
- `src/type_system/heap_class.rs`

Build evidence: `cargo run --manifest-path ../../Cargo.toml -- build` from `test-projects/game-of-life-full` completed successfully and produced `target/program`.

## Finding 1 — Real leak: RC array over-retention during generated board construction and board replacement

### App evidence

`test-projects/game-of-life-full/src/main.op`:

- Line 23 starts the infinite loop: `while true:`.
- Line 25 replaces the current board every frame: `board = next_generation(board, config.width, config.height)`. This is not just the caller of the leak-prone `next_generation` function; it is also its own RC-overwrite leak surface because assignment lowering stores the returned array into the mutable `board` binding.

`test-projects/game-of-life-full/src/rules.op`:

- Line 36 declares `next_generation` returning `int8[]`.
- Line 37 creates a new mutable array: `let mutable next_board: int8[] = []`.
- Lines 39-44 iterate through the board.
- Line 42 pushes one computed cell per coordinate: `next_board.push(next_cell_state(board, width, height, x, y))`.
- Line 45 returns the newly built array: `return next_board`.

With the fixed dimensions in `test-projects/game-of-life-full/src/config.op`, line 5 returns width `80` and line 11 returns height `40`, so `next_generation` performs 3,200 `push` operations per frame. At line 17, `target_frames_per_second` returns `15`, so the hot path is approximately 48,000 cell pushes per second.

### Runtime ownership evidence

`runtime/opal_rc.h` defines the RC contract:

- Lines 97-103 document `opal_rc_alloc` as allocating a new RC object and returning a payload pointer.
- Lines 157-162 define `opal_rc_inc` as incrementing the strong reference count.
- Lines 164-170 define `opal_rc_dec` as decrementing the strong reference count and dropping when the count reaches zero.

`runtime/opal_rc.c` implements that contract:

- Lines 139-172 allocate tracked RC objects.
- Line 161 initializes `header->refcount = 1` for a new object.
- Lines 201-205 increment refcount in `opal_rc_inc`.
- Lines 207-215 decrement refcount in `opal_rc_dec`, calling `opal_rc_drop_iterative` only when the refcount reaches zero.
- Lines 331-365 implement `opal_array_alloc`; line 355 allocates arrays through `opal_rc_alloc_tracked`, so each array starts with refcount 1.

### Codegen evidence for the leak

`src/codegen/expressions_array.rs`:

- Lines 520-552 implement `allocate_array_payload`; line 526 declares `opal_array_alloc`, and lines 528-538 call it.
- Lines 745-768 declare `opal_array_alloc` as the runtime allocator.

`src/codegen/functions_call/array/intrinsics.rs`:

- Lines 717-990 lower `array.push`.
- Lines 889-895 allocate a fresh array in the unique-grow path.
- Lines 939-945 allocate a fresh array in the shared-fallback path.
- Lines 923-929 store the grown array back into the receiver binding.
- Lines 973-979 store the fallback array back into the receiver binding.

`src/codegen/functions_call/array/helpers.rs`:

- Lines 69-94 implement `store_array_binding` by calling `store_binding_overwrite_rc_safe`.

`src/codegen/statements.rs`:

- Lines 629-636 lower identifier assignment by evaluating the right-hand side and passing the result to `store_binding_overwrite_rc_safe` with operation name `"assign"`. This is the path used by `main.op` line 25 for `board = next_generation(...)`.

`src/codegen/binding_store.rs`:

- Lines 44-81 implement `store_binding_overwrite_rc_safe`.
- Lines 57-65 load the old value for RC-bearing bindings.
- Line 67 calls `retain_new_binding_value_if_needed` on the new value before storing it.
- Lines 68-70 store the new value into the binding.
- Lines 71-78 decrement the previous value after the store.
- Lines 100-116 implement `retain_new_binding_value_if_needed`; line 115 emits `opal_rc_inc` for RC-bearing values.
- Lines 118-136 implement `release_binding_value_if_needed`; line 135 emits `opal_rc_dec`.

This is the leak mechanism: newly allocated arrays already start with refcount 1, but storing them into a mutable array binding increments them to refcount 2. There is no corresponding decrement for the temporary ownership of that newly allocated value. Later overwrites decrement the old binding value once, leaving the old array at refcount 1 instead of 0, so it is never dropped.

The first push into an empty array can free the original empty array, but after the first grown array is stored, that grown array has refcount 2. Subsequent pushes see the array as non-unique and go through the shared-fallback allocation path, store the replacement at refcount 2, and decrement the previous array only to refcount 1. That leaks the previous array. In `next_generation`, this happens while building every generated board.

The same root cause also applies after `next_generation` returns: `main.op` line 25 assigns the returned board into the mutable `board` binding. That assignment lowers through `src/codegen/statements.rs` lines 629-636 into the same `store_binding_overwrite_rc_safe` helper, so the returned board is retained again before storage and later decremented only once on the next frame. This broadens the leak surface of the same root cause; it does not add a separate root-cause category.

### Why this is a real unbounded leak

This is not just allocation churn. The old arrays are not merely waiting for normal scope exit; their refcounts are left above zero after they become unreachable from source-level variables.

At `rules.op` line 45, `return next_board` transfers the final board rather than cleaning it up as a local. `src/codegen/control_flow.rs` lines 694-703 collect returned identifier names as transferred values, and lines 676-690 use those transferred names during cleanup. That avoids dropping the returned board, which is correct for the return value. However, the intermediate arrays created during the 3,200 push sequence are not source-level return values. They should have reached refcount zero when replaced, but because the store path incremented newly allocated arrays, each old intermediate array is left at refcount 1.

After return, `main.op` line 25 stores that returned board into `board` through assignment lowering. Since that path also calls `store_binding_overwrite_rc_safe`, the frame-level current-board replacement has the same over-retention problem as the internal `push` replacements.

The leak repeats every frame because `main.op` line 25 calls and assigns `next_generation` inside `while true` at line 23.

### Impact

Leak count contribution: one root-cause leak class, triggered many times.

For an 80x40 board, each frame leaks many intermediate array allocations from `next_board.push(...)` and also over-retains the returned board when assigning it to `board`. The exact byte count per frame depends on the generated capacity-growth path and array header/padding, but the retention is unbounded because the loop in `main.op` never terminates on its own.

### Likely fix direction

The mutable-binding overwrite path needs to distinguish caller-owned newly allocated RC values from borrowed/aliased identifiers. `store_binding_overwrite_rc_safe` should not blindly increment a freshly allocated value whose ownership is being moved into the binding. It should retain only when storing an existing identifier/reference that needs another strong owner. This is already partially recognized in `src/codegen/statements.rs`: line 234 computes `retain_new_value = matches!(*initializer_expr, Expr::Identifier { .. })` for `let` initialization, and lines 235-242 pass that flag into `initialize_binding_value`. The assignment/array-store paths do not currently carry equivalent ownership information.

## Finding 2 — Real leak: malloc-backed string interpolation buffer passed directly to `writer_write_sync` each frame is never freed

### App evidence

`test-projects/game-of-life-full/src/render.op`:

- Line 1 imports `int64_to_string`, `writer_write_sync`, and the stdout/terminal helpers.
- Line 16 declares `write_frame`, which is called once per frame from `main.op` line 24.
- Line 24 creates a string binding: `let generation_text = int64_to_string(generation)`.
- Line 25 passes an interpolated string directly to a fallible writer call: `propagate writer_write_sync(writer, 'Generation {generation_text}\n')`.

### Runtime allocation evidence for `generation_text`

`runtime/opal_string.c`:

- Lines 36-43 implement `int64_to_string`.
- Line 38 allocates a `char*` buffer with `malloc`.
- Line 42 returns that caller-owned buffer.

`src/codegen/scope_tracker.rs`:

- Lines 195-218 classify runtime functions ending in owned string returns; line 209 includes `int64_to_string`.

`src/codegen/statements.rs`:

- Lines 221-232 mark string `let` bindings for malloc-string cleanup when their initializer requires it.

`src/codegen/scope_tracker.rs`:

- Lines 464-485 run malloc-string cleanup during scope cleanup.
- Lines 403-448 load and free marked malloc-string bindings.

This means the `generation_text` binding from `render.op` line 24 has a cleanup path on normal scope/return paths. That binding is not the success-path leak described below. It is not universally safe, however: on a `propagate` early-return path, the call-lowering code can return before normal scope cleanup runs.

### Codegen evidence for the leaked interpolation buffer

`src/codegen/expressions.rs`:

- Lines 195-197 lower every `Expr::StringInterpolation` through `codegen_string_interpolation`.

`src/codegen/expressions_string.rs`:

- Lines 56-67 build the interpolation format and allocate an output buffer.
- Lines 95-163 implement `allocate_interpolation_buffer`.
- Line 150 declares/gets `malloc`.
- Lines 151-162 call `malloc` and return `buffer_ptr`.
- Lines 81-90 free only temporary string arguments collected inside the interpolation expression.
- Line 92 returns the newly allocated interpolation buffer.
- Lines 262-275 decide which pointer arguments to free after `snprintf`; identifiers such as `generation_text` are not freed there, which is correct because the binding cleanup handles `generation_text` separately.

`src/codegen/functions_call.rs`:

- Lines 278-283 lower call arguments into `lowered_args`.
- Lines 520-524 build the runtime call with those arguments.
- Lines 525-536 return the call result.

There is no cleanup between argument lowering and the call return for a malloc-backed interpolation buffer that is used only as a direct argument. Because `render.op` line 25 passes `'Generation {generation_text}\n'` directly to `writer_write_sync`, the buffer allocated by `src/codegen/expressions_string.rs` lines 150-162 is not assigned to a marked binding and is not freed after the call.

### Propagate makes the error path worse

`src/codegen/functions_call.rs`:

- Lines 539-637 lower `propagate`.
- Lines 563-604 inspect the returned error aggregate and branch to an early return on error.
- Lines 602-604 emit the early return path without cleaning up call-argument temporaries.

The success path already leaks the interpolation buffer. If `writer_write_sync` returns an error, the early-return path also does not free the interpolation buffer.

The same error-path caveat applies to `generation_text`: normal return cleanup goes through `src/codegen/scope_tracker.rs` lines 118-148 and then the malloc-string cleanup path at lines 464-485, but `src/codegen/functions_call.rs` lines 578-604 emit an early return directly from `propagate` without invoking that cleanup. Therefore `generation_text` is cleaned on normal paths but can leak on writer-error paths.

### Why this is a real unbounded leak

`write_frame` is called once per loop iteration from `main.op` line 24, and the loop is infinite at `main.op` line 23. Therefore one malloc-backed interpolation buffer for the generation header is leaked once per frame.

On the normal path, this leak is separate from the `generation_text` binding. The binding created at `render.op` line 24 has a normal cleanup path; the direct interpolation argument at line 25 does not. On writer-error paths, both the direct interpolation buffer and the already-created `generation_text` binding can be skipped by cleanup because `propagate` emits an early return.

### Impact

Leak count contribution: one root-cause leak class, triggered once per frame.

The leaked allocation size grows slowly as `generation` gains more decimal digits, because the interpolated string is `Generation {generation_text}\n`. The leak is smaller than the array leak but still unbounded in the intentionally infinite app.

### Likely fix direction

Call lowering needs a temporary cleanup list for caller-owned string arguments produced during argument evaluation. After `writer_write_sync` returns, the generated code should free direct interpolation buffers on both success and propagate-error paths. `propagate` early-return lowering also needs to run the same cleanup stack before returning. A source-level workaround that simply binds the interpolated string to a local does **not** work today: `src/codegen/scope_tracker.rs` lines 150-171 do not classify `Expr::StringInterpolation` as requiring malloc-string cleanup, and `src/codegen/statements.rs` lines 221-232 only marks string bindings when that predicate returns true. Fixing this requires compiler support for interpolation-result ownership, not just moving the interpolation into a local variable.

## Finding 3 — Non-leak: `LifeConfig` product construction and field access are plain value operations

`test-projects/game-of-life-full/src/life.types.op`:

- Lines 4-7 define `LifeConfig` with scalar fields `width`, `height`, and `frames_per_second`.

`test-projects/game-of-life-full/src/main.op`:

- Lines 12-15 construct a `LifeConfig` value using `new LifeConfig`.
- Lines 18-19, 24-25 read fields from `config` and pass the scalar values to `FrameClock`, board creation, rendering, and next-generation logic.

`src/codegen/adts.rs`:

- Lines 641-692 lower product constructors to plain LLVM struct values: fields are lowered, a stack alloca is built, fields are stored, and the struct value is loaded.

`LifeConfig` contains only scalar numeric fields, and the product constructor lowering shown above does not allocate an RC object or register any runtime-managed handle. Field reads of those scalar values do not retain heap memory. This is a language construct used by the app, but it is not a leak.

## Finding 4 — Bounded/process-lifetime retention: `FrameClock` is runtime-managed and freed only at process exit

### App evidence

`test-projects/game-of-life-full/src/main.op`:

- Lines 17-18 construct exactly one `FrameClock`.
- Line 27 reuses that same clock every loop iteration with `frame_clock_wait_next_sync(clock)`.

### Runtime evidence

`src/codegen/adts.rs`:

- Lines 548-596 lower registered fallible constructors through their runtime symbols and error ABI.

`src/type_system/fallible_constructors.rs`:

- Lines 72-92 map `new FrameClock` to the runtime symbol `frame_clock_new`.

`src/type_system/heap_class.rs`:

- Line 39 classifies `FrameClock` as `HeapClass::RuntimeManaged`.

`runtime/opal_io.c`:

- Lines 22-25 define `OpalFrameClock`.
- Lines 35-41 define a global linked list head `OPAL_FRAME_CLOCKS` and cleanup-registration flag.
- Lines 75-84 implement `opal_frame_clock_cleanup_all`, freeing each clock and node at exit.
- Lines 86-106 implement `opal_frame_clock_register_for_cleanup`; line 87 allocates a list node, lines 94-96 link it into `OPAL_FRAME_CLOCKS`, and lines 97-105 register the cleanup function with `atexit` once.
- Lines 295-318 implement `frame_clock_new`; line 306 allocates the clock with `malloc`, and line 316 registers it for cleanup.
- Lines 320-351 implement `frame_clock_wait_next_sync`; it updates the existing clock and does not allocate.

### Classification

This is not an unbounded leak in `game-of-life-full` as written because exactly one clock is constructed. The clock is intentionally retained for the process lifetime and freed by `atexit` on normal process exit.

It would become process-lifetime accumulation if code created many `FrameClock` objects during a long-running process, because there is no per-clock destroy/unregister API in the lines reviewed. That is a runtime API limitation, not a current app leak.

## Finding 5 — Non-leak: stdout writer and terminal handles are static singletons, not heap allocations

`test-projects/game-of-life-full/src/render.op`:

- Line 8 obtains a terminal in `prepare_display`.
- Lines 17-18 obtain a terminal and writer in `write_frame`.

`runtime/opal_io.c`:

- Lines 43-44 define static `OPAL_STDOUT_WRITER` and `OPAL_STDOUT_TERMINAL` objects.
- Lines 353-356 return the address of the static stdout writer.
- Lines 366-369 return the address of the static stdout terminal.
- Lines 358-364 implement writer write/flush on the existing stream.
- Lines 371-377 implement terminal ANSI support and clear-screen write.

These calls do not allocate per frame. They do not retain old frame data.

## Finding 6 — Non-leak: per-cell writes use string literals and do not allocate app-owned buffers

`test-projects/game-of-life-full/src/render.op`:

- Lines 27-37 iterate through rows and columns.
- Line 32 writes the literal `'#'` for live cells.
- Line 34 writes the literal `'.'` for dead cells.
- Line 36 writes the literal newline.
- Line 39 flushes once per frame.

These are heavy I/O calls, but the arguments are literals. `runtime/opal_io.c` lines 358-364 write and flush; they do not allocate persistent buffers. This is performance-sensitive but not a memory leak.

## Finding 7 — Non-leak: board reads, bounds checks, numeric comparisons, casts, loops, and constants are allocation-free

`test-projects/game-of-life-full/src/board.op`:

- Lines 6-7 compute flat indices.
- Lines 12-21 perform bounds checks.
- Lines 26-29 read a board cell or return `dead_cell()`.
- Lines 34-35 compare a cell to `alive_cell()`.

`src/codegen/expressions_array.rs`:

- Lines 58-93 lower array access: the receiver type is checked, the base pointer and length are resolved, bounds checks are emitted, the element pointer is computed, and the element is loaded. This read path does not allocate or retain a new array.

`test-projects/game-of-life-full/src/config.op`:

- Lines 4-17 return numeric constants for width, height, and frame rate.
- Lines 20-23 define `board_cell_count`; this helper is present in the app source but is not called by `game-of-life-full`, so it has no runtime allocation effect in this program.
- Lines 28-35 return `int8` constants for live/dead cells.

`test-projects/game-of-life-full/src/patterns.op`:

- Lines 6-75 implement seed predicates using function calls, `if` branches, `return true`/`return false`, integer equality via `is`, range comparisons via `>=`/`<=`, boolean `and`/`or`, and parenthesized boolean expressions.
- Lines 64-75 implement the `is_seed_cell` call chain across the individual seed predicates.

`test-projects/game-of-life-full/src/rules.op`:

- Lines 7-18 count neighbors with nested `while` loops.
- Lines 23-31 apply Conway's state rules.

`test-projects/game-of-life-full/src/main.op`, `render.op`, `rules.op`, and `patterns.op` scalar mutation sites:

- `main.op` lines 20 and 26 initialize and increment the scalar `generation` counter.
- `render.op` lines 27, 29, 35, and 37 initialize/increment scalar loop counters.
- `rules.op` lines 8-11 and 15-17 initialize/increment scalar neighbor-counting locals.
- `rules.op` lines 38, 40, 43, and 44 initialize/increment scalar next-generation loop counters.
- `patterns.op` lines 82, 84, 90, and 91 initialize/increment scalar seed-board loop counters.

These constructs use stack/local scalar values, array reads, arithmetic, comparisons, boolean composition, function calls, returns, and control flow. The scalar mutable assignments update inline numeric locals, not RC-bearing heap values. They do not allocate retained heap objects by themselves.

## Finding 8 — Startup-only allocation path: `create_seed_board` uses the same array-push mechanism once

`test-projects/game-of-life-full/src/main.op`:

- Line 19 initializes `board` with `create_seed_board(config.width, config.height)`.

`test-projects/game-of-life-full/src/patterns.op`:

- Lines 80-92 implement `create_seed_board`.
- Line 81 creates `let mutable board: int8[] = []`.
- Lines 83-91 iterate over the initial board cells.
- Lines 87 and 89 push live/dead cell values.
- Line 92 returns the seed board.

This path uses the same `push` lowering mechanism described in Finding 1, so it leaks both intermediate seed-board arrays and one extra retain on the final returned seed board. The repeated `board.push(...)` calls route through the same retain-before-store path, and `patterns.op` line 92 returns the final `board` identifier without cleaning up that extra retain. `main.op` line 19 stores the returned seed board as a non-identifier initializer, so that let-store does not add another retain; however, the first later overwrite at `main.op` line 25 decrements the seed board only once, leaving the already-over-retained final seed board retained. Unlike the `next_generation` path, this all runs only once at startup. The startup leak is therefore bounded and does not explain ongoing growth by itself. The unbounded version is `rules.op` line 42 inside `next_generation`, called from `main.op` line 25 inside the infinite loop.

## Finding 9 — Not used by this app: `take_input` and input-string helpers return caller-owned malloc strings

`runtime/opal_io.c`:

- Lines 238-258 implement `duplicate_without_trailing_newline`; line 250 allocates `out` with `malloc`, line 256 frees the temporary `raw`, and line 257 returns `out`.
- Lines 260-274 implement `take_input`; line 272 gets a returned malloc string and line 274 returns it.

These functions are standard-library ownership hazards in general, but `game-of-life-full` does not import or call `take_input`. They are not part of the observed app leak.

## Total leak count

There are **2 unbounded leak root causes** in `game-of-life-full` as written:

1. **RC array over-retention during mutable array replacement**, triggered heavily by `next_board.push(...)` in `test-projects/game-of-life-full/src/rules.op` line 42 and also by assigning the returned board at `test-projects/game-of-life-full/src/main.op` line 25 inside the infinite loop at line 23.
2. **Unfreed malloc-backed string temporaries in render call lowering**, triggered once per frame by the direct interpolated argument to `writer_write_sync` in `test-projects/game-of-life-full/src/render.op` line 25, with an additional writer-error-path leak risk for `generation_text` from line 24.

There are also bounded/process-lifetime retentions that are not current unbounded leaks:

- One runtime-managed `FrameClock` allocated at `runtime/opal_io.c` line 306 and registered at line 316, corresponding to `test-projects/game-of-life-full/src/main.op` lines 17-18.
- Startup-only seed-board array retention through `test-projects/game-of-life-full/src/patterns.op` lines 80-92, including both intermediate arrays and the final returned seed board, because that path uses the same `push` ownership bug but executes only once.

## Recommended fix order

1. Fix RC ownership semantics for mutable assignment/array binding stores so newly allocated RC values are moved into bindings without an extra retain. This addresses both the internal `next_board.push(...)` leak and the `board = next_generation(...)` assignment leak.
2. Add cleanup for caller-owned temporary strings produced during function argument evaluation, including both normal return and `propagate` early-return paths. This addresses the per-frame header interpolation leak.
3. Consider adding an explicit `FrameClock` destroy/unregister API if future apps create clocks dynamically. This is not required to stop the current app's unbounded growth.
4. Optionally change the Life implementation to reuse two boards rather than allocate a fresh board per frame. This would reduce allocation pressure, but it should not be used as a substitute for fixing the compiler/runtime ownership leaks.

## TL;DR

There are **2 real unbounded memory leaks** in `game-of-life-full`.

The first and largest leak is in the board update path: `main.op` line 25 calls and assigns `next_generation` forever, and `rules.op` line 42 pushes 3,200 cells into a new board every frame. The compiler stores newly allocated RC arrays into mutable bindings by incrementing their refcount even though they already start at refcount 1. Later overwrites decrement them only once, leaving old arrays stuck at refcount 1 and never freed. This affects both the internal `next_board.push(...)` replacement path and the outer `board = next_generation(...)` assignment path.

The second leak is in rendering: `render.op` line 25 passes `'Generation {generation_text}\n'` directly into `writer_write_sync`. String interpolation allocates a malloc-backed buffer, but direct call arguments are not cleaned up after the call or on `propagate` error paths. This leaks one small string buffer per frame. `generation_text` from `render.op` line 24 is cleaned on normal paths, but can also leak if `propagate` returns early on writer failure before scope cleanup runs.

The `LifeConfig` constructor is a plain product-value operation and is not a leak. Scalar loop-counter mutations, seed-pattern comparisons, boolean composition, and board reads are non-leaking inline/control-flow operations. The `FrameClock` is retained for the process lifetime, but the app creates only one clock, and the runtime frees registered clocks at normal process exit. The stdout writer/terminal handles are static singletons and are not leaks. Per-cell `'#'`, `'.'`, and newline writes are expensive I/O but not memory leaks.
