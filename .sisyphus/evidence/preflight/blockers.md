# T0 Pre-Flight Blockers

## Status
- **FAILED** during preflight run step.
- Per instruction, scaffolding in `src/` and `runtime/` is **left in place** for diagnosis (no revert performed).

## Failing step
- Command: `target/release/opalescent run /tmp/preflight_arrays.op`
- Outcome: compiler rejected test program before runtime validation.

## Error details
```
opalescent::type_system::missing_doc_comment

× Public function 'main' is missing a documentation comment
help: Add a ## documentation block with at least 30 characters before this function
```

## Impact
- Could not validate required preflight runtime behavior:
  - `string[]` return iteration from stdlib builtin
  - `FilesystemPath[]` return iteration from stdlib builtin
  - fallible scalar `guard` success branch
  - fallible scalar `guard` error branch
- No authoritative preflight stdout/exit verification was produced due front-end rejection.

## Notes for next attempt
- The temporary `/tmp/preflight_arrays.op` program must include a valid `##` doc comment block on `entry main` before execution can proceed.

---

## [Follow-up attempt] New blocker after doc-comment fix

### Failing step
- Command: `target/release/opalescent run /tmp/preflight_arrays.op`
- Exit code: `1` (captured in `.sisyphus/evidence/preflight/exit-code.txt`)

### Error details
```
opalescent::type_system::type_mismatch
× Type mismatch: expected 'FilesystemPath', found '[FilesystemPath]'
--> /tmp/preflight_arrays.op:11:16
let paths: FilesystemPath[] = preflight_path_array()
```

### Interpretation
- The compiler currently infers/prints a mismatch indicating the `FilesystemPath[]` annotation path is not being accepted as expected for this stdlib return.
- This blocks validation of both array-iteration and guard branch runtime output assertions.

### Artifacts captured
- `.sisyphus/evidence/preflight/stdout.log`
- `.sisyphus/evidence/preflight/exit-code.txt`

### Stop condition
- Stopping here per instruction: run failed for a different reason, so blockers documented and no cleanup/revert performed.

---

## [Follow-up attempt] New blocker after removing array let annotations

### Failing step
- Command: `target/release/opalescent run /tmp/preflight_arrays.op`
- Exit code: `1` (captured in `.sisyphus/evidence/preflight/exit-code.txt`)

### Error details
```
error: compilation failed: code generation failed: unknown variable 'item'
```

### Interpretation
- Type-check phase accepted the `for item in strings:` form.
- Codegen fails because loop variable binding is not materialized for `for` iteration variables in current lowering path, causing the loop body identifier lookup (`item`) to fail.
- This blocks runtime validation of:
  - `string[]` iteration output
  - `FilesystemPath[]` iteration output
  - guard success/error branch output

### Artifacts overwritten
- `.sisyphus/evidence/preflight/stdout.log`
- `.sisyphus/evidence/preflight/exit-code.txt`

### Stop condition
- Stopping here per instruction: different failure reason encountered, blockers documented, no cleanup/revert performed.

---

## PASSED

### Final successful preflight run
- Command: `./target/release/opalescent /tmp/preflight_arrays.op --run`
- Exit code: `0`
- Captured stdout (`.sisyphus/evidence/preflight/stdout.log`):
  - `str-elem-0`
  - `str-elem-1`
  - `path-name: a.txt`
  - `path-name: b.txt`
  - `guard-ok: true`
  - `guard-err: PermissionDeniedError`

### Root cause fixed during preflight
- `for` loop codegen did not materialize iteration variable bindings into `env.variables`.
- Added loop-variable binding + per-iteration value store in codegen loop lowering path (temporary preflight fix), enabling `for item in strings` and `for p in paths` in preflight validation.

### Cleanup completed
- Reverted all preflight scaffolding under `src/` and `runtime/`.
- Removed newly-added scaffolding files:
  - `runtime/opal_fs_preflight.c`
  - `src/type_system/checker/fs_builtins.rs`
- Post-revert verification captured at `.sisyphus/evidence/preflight/git-status-post-revert.txt`.

### Post-clean verification
- `cargo build --release` succeeds after revert.
- `lsp_diagnostics` on `src/` shows no errors.
