# Cleanup Probe Ignore

## Overview

This proposal allows `ignore err`, but only for narrow classes of guard handlers where discarding failure is a recognized software pattern:

- best-effort cleanup
- delete-if-present operations
- fallback probes
- non-critical telemetry flushing

Everywhere else, guard errors must be returned, converted into typed wrapper variants, logged and returned, or handled through typed match recovery.

This keeps the practical usefulness of Go's `_ = cleanup()` and Rust's `let _ = cleanup()` while making the intent visible and reviewable in Opalescent.

## Assumes

- `ignore` is a new language statement.
- The compiler or linter can identify cleanup/probe contexts structurally.
- `ignore err` marks only the named guard error as consumed.
- `ignore` does not suppress type errors, runtime traps, or unrelated diagnostics.

## Handler Set

### Cleanup ignore

```opal
guard delete_file_sync(temp_path) else cleanup_err =>
    ignore cleanup_err
```

### Probe fallback

```opal
guard is_directory_sync(candidate_path) into is_dir else probe_err =>
    ignore probe_err
    return false
```

### Cleanup failure followed by primary error return

```opal
guard write_text_sync(temp_path, payload) else write_err =>
    guard delete_file_sync(temp_path) else cleanup_err =>
        ignore cleanup_err
    return write_err
```

This is the key motivating shape: the cleanup error should not hide the primary failure.

## Keywords

| Keyword | Required? | Purpose |
|---------|-----------|---------|
| `ignore` | Yes | Explicitly consumes an error value without propagation or recovery. |

No `because` keyword is included. The visible `ignore` statement is the required signal; teams can add ordinary comments above it when explanation is useful.

## Syntax Design

Only guard error names can be ignored:

```opal
ignore cleanup_err
```

These forms are not accepted:

```opal
# Not accepted: arbitrary fallible expression hidden inside ignore
ignore delete_file_sync(temp_path)

# Not accepted: ordinary value ignored as if it were an error
ignore count
```

The language can require the ignored binding name to carry context:

```opal
guard delete_file_sync(temp_path) else cleanup_err =>
    ignore cleanup_err
```

Names such as `cleanup_err`, `probe_err`, or `telemetry_err` make the discard intent easier to audit than a generic name like `err`.

## Example Applications

### Atomic write cleanup

```opal
import path_from, write_text_sync, delete_file_sync, move_path_sync from standard

let write_text_atomic_sync = f(target_text: string, payload: string): void errors WriteFailureError, MoveFailureError, InvalidPathError, PermissionDeniedError =>
    let target_path = path_from(target_text)
    let temp_path = path_from('{target_text}.tmp')

    guard write_text_sync(temp_path, payload) else write_err =>
        guard delete_file_sync(temp_path) else cleanup_err =>
            ignore cleanup_err
        return write_err

    guard move_path_sync(temp_path, target_path) else move_err =>
        guard delete_file_sync(temp_path) else cleanup_err =>
            ignore cleanup_err
        return move_err

    return void
```

### Directory-or-file fallback

```opal
import delete_directory_recursive_sync, delete_file_sync from standard

let delete_unknown_child_sync = f(child: FilesystemPath): void errors DeleteFailureError, PermissionDeniedError, InvalidPathError =>
    guard delete_directory_recursive_sync(child) else directory_err =>
        ignore directory_err

        guard delete_file_sync(child) else file_err =>
            return file_err

    return void
```

This matches current Opalescent cleanup/fallback examples while making the first failure intentionally discarded.

### Windows temporary file cleanup

```opal
import path_from, join_path_components, delete_file_sync from standard

let cleanup_temp_marker_sync = f(user_root_text: string): void =>
    let user_root = path_from(user_root_text)
    let marker_path = join_path_components(user_root, ['AppData', 'Local', 'Opalescent', 'marker.tmp'])

    guard delete_file_sync(marker_path) else cleanup_err =>
        ignore cleanup_err

    return void
```

The path is built with components, so the example is portable across Windows and non-Windows targets.

## Strengths

1. Handles real cleanup code without forcing fake logging.
2. Makes ignored errors searchable.
3. Avoids ugly `_ignored_err` local variables.
4. Prevents cleanup failures from hiding the primary error.
5. Can be kept narrow by the language instead of relying on naming conventions.
6. Maps to familiar Go and Rust best-effort cleanup idioms while being more explicit.

## Weaknesses

1. Requires precise language rules to prevent overuse.
2. Context detection may be imperfect if purely compiler-driven.
3. Teams may argue about which operations qualify as cleanup or probes.
4. Adds a new keyword.
5. If used as the default discard mechanism, it weakens the safer-than-Go goal.

## Windows Support Requirements

- Cleanup must use standard filesystem helpers rather than shell commands.
- Ignored Windows filesystem errors should still be debuggable in diagnostic or trace builds.
- The language should not special-case Windows cleanup APIs; `ignore` is platform-neutral.
- Path helpers should preserve drive prefixes, UNC roots, and long-path behavior.

## Fit

- **Ergonomics**: Good.
- **Error-model fit**: Good when narrow.
- **High-stakes fit**: High if the language keeps the form narrow, Medium if it is general-purpose.
- **Implementation effort**: Medium-High because the compiler needs an `ignore` statement and cleanup/probe classification.

## Must NOT Have

- No general-purpose `ignore` for every error.
- No hidden `ignore` through underscore-prefixed names.
- No `ignore fallible_call()` shortcut.
- No comments-as-syntax requirement such as `ignore err because '...'`.
