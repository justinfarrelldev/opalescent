# Decisions — perceus-scr-memory-model

## [2026-04-20] Architectural Decisions

### Weak<T> Representation
- Use `CoreType::Generic { name: "Weak", type_args: vec![inner] }` — same pattern as Option<T>
- Upgrade returns `Option<T>` (already registered) — NOT `?` syntax

### RC Object Header Layout
- `[refcount: size_t | weak_count: size_t | drop_children_fn: fn_ptr | payload...]`
- `drop_children_fn` called by iterative drop to enqueue child RC objects onto work-list
- ABI-stable for future module imports

### Iterative Drop
- Work-list based (stack/array), NOT recursive
- Pre-allocate reasonable stack, grow if needed
- `drop_children_fn` enqueues children, outer loop processes them

### PassingMode
- `Owned` (default, backward compat), `Ref`, `MutableRef`
- Parameter-only annotation — cannot be stored, returned, or captured in closures

## [2026-04-20] Task 13 Decisions

### RC emission architecture
- Introduced a dedicated `RcEmitter` helper in `src/codegen/rc_emitter.rs` rather than scattering ad-hoc runtime call declarations.
- `RcEmitter` owns the declaration-or-get logic for `opal_rc_inc/dec/drop` to keep signatures consistent and avoid duplicate declaration code.

### Increment/decrement strategy (initial integration)
- Emit `opal_rc_inc` at function entry for owned parameters with `CoreType::needs_rc() == true`.
- Emit `opal_rc_dec` for tracked owned RC parameters at explicit return sites.
- Emit `opal_rc_dec` at block scope exit for newly introduced owned RC locals.

### Safety gate for lowered LLVM values
- RC call emission only runs when loaded values are pointer-typed (`is_pointer_value()`), because some lowered forms (e.g. array-shaped params) are not raw pointers and would panic on pointer conversion.
