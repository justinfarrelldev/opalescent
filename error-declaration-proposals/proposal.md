# Proposal: Make Error Declarations Easier to Write

## The problem in plain language

Today, a fallible Opalescent function must write down every error it can pass to its caller. This is useful: anyone reading the function signature can see what can go wrong.

The painful part is repetition. A function that calls several imported APIs often has to copy a long list of errors into its own signature even when it does not create, transform, or inspect any of those errors. The function is only passing them upward.

### What developers write today

Imagine an application startup function. It reads a config file, creates a directory, and starts a process. Each imported API has errors of its own.

```opal
let start_application_sync = f(config_path: FilesystemPath): void errors ConfigReadError, ConfigParseError, DirectoryCreateError, PermissionDeniedError, ProcessStartError, ProcessNotFoundError =>
    let config = propagate load_config_sync(config_path)
    propagate create_directory_sync(config.workspace_path)
    propagate start_process_sync(config.command)
    return void
```

Nothing in this function decides how to handle these six errors. It only forwards them. Nevertheless, the six names must be copied into the signature.

This proposal explores ways to remove that copying **without** making errors invisible, unchecked, or automatic at call sites.

## What must stay true

Every option in this proposal keeps these parts of Opalescent's error model:

- A fallible call is still visibly handled with `guard` or `propagate`.
- Public functions still have a clear, inspectable list of errors.
- The compiler still rejects a propagated error that is not allowed by the enclosing function.
- There are no exceptions, hidden stack unwinding, wildcards, or `AnyError` escape hatches.

The question is only: **how should a function describe a large set of allowed errors without repeatedly typing every member?**

## Related work already in this repository

There is no existing Opalescent proposal that solves this specific signature-writing problem.

- `stdlib-proposals/error-strategy/open-error-set/proposal.md` describes the current explicit-list model. It already calls out signature bloat as a weakness.
- `stdlib-proposals/error-strategy/error-code-enum-module/proposal.md` suggests giving each module one error enum. That helps, but it does not provide a reusable name for a combination of errors from several modules.
- `stdlib-proposals/error-strategy/registered-error-hierarchy/proposal.md` centralizes errors in a registry. It changes where errors are organized rather than removing repeated function signatures.
- `error-handler-proposals/` focuses on what happens at a `guard` or `propagate` call site. Those proposals reduce handling boilerplate, not the list in a function signature.

A public GitHub search of this repository found no issue or pull request proposing inferred, grouped, or wildcard error declarations. As an external comparison, Catalyst has an accepted design that infers a closed error set from a function body, while requiring explicit contracts at public and recursive boundaries: [Error Returns and Inference](https://catalyst-lang.org/language-design/language/functions/error-returns-and-inference/).

## Strategy 1: Keep the language as-is, but let tooling write the list

### Idea

Do not add language syntax. Instead, the compiler and LSP calculate the errors that are missing from an `errors` clause and offer a quick fix that inserts them.

### Current experience

```opal
let start_application_sync = f(config_path: FilesystemPath): void errors ConfigReadError =>
    let config = propagate load_config_sync(config_path)
    propagate create_directory_sync(config.workspace_path)
    propagate start_process_sync(config.command)
    return void
```

The compiler reports that several propagated errors are missing. The developer must look them up and type each one.

### Proposed experience

The compiler reports the same error, but the editor offers **“Add missing errors to signature.”** Choosing it produces:

```opal
let start_application_sync = f(config_path: FilesystemPath): void errors ConfigReadError, ConfigParseError, DirectoryCreateError, PermissionDeniedError, ProcessStartError, ProcessNotFoundError =>
    let config = propagate load_config_sync(config_path)
    propagate create_directory_sync(config.workspace_path)
    propagate start_process_sync(config.command)
    return void
```

### What this fixes

It removes the most tedious typing and avoids mistakes. The source code and its public contract stay exactly as explicit as they are today.

### What it does not fix

The long list remains in the source file. This is an editor improvement, not a new way to name a group of errors.

### Recommendation

Adopt this first. It is useful regardless of which language-level strategy is chosen later.

## Strategy 2: Give each module one public error family

### Idea

A module that currently exposes many low-level errors also exposes one umbrella error type for its domain. A filesystem module exposes `FilesystemError`; a process module exposes `ProcessError`.

### Current experience

```opal
let prepare_workspace_sync = f(path: FilesystemPath): void errors DirectoryCreateError, PermissionDeniedError, DiskFullError, ProcessStartError, ProcessNotFoundError =>
    propagate create_directory_sync(path)
    propagate start_process_sync('tool')
    return void
```

The function has to know every individual error from each imported module.

### Proposed experience

```opal
let prepare_workspace_sync = f(path: FilesystemPath): void errors FilesystemError, ProcessError =>
    propagate create_directory_sync(path)
    propagate start_process_sync('tool')
    return void
```

`FilesystemError` is the module's documented family containing `DirectoryCreateError`, `PermissionDeniedError`, and `DiskFullError`. `ProcessError` is the equivalent family for process operations.

### What this fixes

A long list from one imported module becomes one stable, understandable name. It also makes documentation easier to read: `FilesystemError` is more informative than a list of implementation-level file errors.

### What it does not fix

A function that combines many modules still has one name per module. Repeated combinations remain repetitive:

```opal
errors FilesystemError, ProcessError, ConfigError
```

### Recommendation

Adopt this as a standard-library and package-author convention now. It may already be possible with the existing enum/sum-type approach, so it should not wait for a language feature.

## Strategy 3: Name a reusable combination of error families

### Idea

Let a module give a meaningful name to a group of errors that often travel together. This proposal calls that name an **error-set alias**.

An error-set alias does not create a new runtime error value. It is only a short, checked name for a fixed list of existing errors.

### Current experience

Several public functions in one application layer repeat the same combination:

```opal
let start_application_sync = f(config_path: FilesystemPath): void errors ConfigError, FilesystemError, ProcessError =>
    let config = propagate load_config_sync(config_path)
    propagate prepare_workspace_sync(config.workspace_path)
    propagate start_process_sync(config.command)
    return void

let restart_application_sync = f(config_path: FilesystemPath): void errors ConfigError, FilesystemError, ProcessError =>
    propagate stop_application_sync()
    propagate start_application_sync(config_path)
    return void
```

### Proposed experience

Define the combination once, then use its name:

```opal
type ApplicationStartError = errors ConfigError, FilesystemError, ProcessError

let start_application_sync = f(config_path: FilesystemPath): void errors ApplicationStartError =>
    let config = propagate load_config_sync(config_path)
    propagate prepare_workspace_sync(config.workspace_path)
    propagate start_process_sync(config.command)
    return void

let restart_application_sync = f(config_path: FilesystemPath): void errors ApplicationStartError =>
    propagate stop_application_sync()
    propagate start_application_sync(config_path)
    return void
```

### What the new line means

```opal
type ApplicationStartError = errors ConfigError, FilesystemError, ProcessError
```

Read it as: **“`ApplicationStartError` is the checked name for exactly these three error families.”** It is not an exception class and it is not a catch-all. The compiler expands the name back to the three listed members whenever it checks a function.

### What this fixes

- Repeated cross-module combinations have one meaningful, stable name.
- Public signatures stay explicit: readers see `ApplicationStartError`, and editor hover or generated documentation shows the three member families.
- Changing the set happens in one reviewed place instead of in every forwarding function.

### Important rules

- Error-set aliases may include individual errors or other error-set aliases.
- The compiler expands nested aliases, removes duplicates, and rejects a circular definition.
- An alias is only valid in an `errors` clause or another error-set alias. It cannot be used as an ordinary runtime value type.
- The alias lives in the normal type namespace, so it follows normal import and visibility rules.

### Recommendation

This is the recommended language feature for public APIs and for internal helpers whose error contract should remain stable.

## Strategy 4: Let private forwarding helpers infer their errors

### Idea

Some functions have no meaningful error policy at all. They are private glue that simply propagates whatever their called functions return. Let the compiler calculate the error list for these functions.

### Current experience

```opal
let read_default_config_sync = f(): Config errors FilesystemError, ConfigError =>
    let path = propagate default_config_path_sync()
    return propagate load_config_sync(path)
```

The signature is entirely mechanical: it repeats the errors from the two calls in the body.

### Proposed experience

```opal
let read_default_config_sync = f(): Config errors { infer } =>
    let path = propagate default_config_path_sync()
    return propagate load_config_sync(path)
```

Read `errors { infer }` as: **“compiler, inspect this function body and write down every error that can leave it.”** The resolved list in this example is `FilesystemError, ConfigError`.

### What counts as an escaping error

```opal
let load_optional_metadata_sync = f(path: FilesystemPath): Metadata errors { infer } =>
    guard read_text_sync(path) into text else read_error =>
        return empty_metadata()
    return propagate parse_metadata(text)
```

`read_text_sync` can fail, but the function handles that failure by returning `empty_metadata()`. Therefore its error does **not** leave the function and is not inferred. Only `ParseError` is inferred from `parse_metadata(text)`.

### Why this is private-only

Public function signatures are promises to other modules and to users. If a private implementation change silently adds a new imported error, a public caller should not receive a changed error contract by surprise.

So this is valid:

```opal
let read_default_config_sync = f(): Config errors { infer } =>
    return propagate load_config_sync(default_config_path_sync())
```

But this is rejected:

```opal
public let read_default_config_sync = f(): Config errors { infer } =>
    return propagate load_config_sync(default_config_path_sync())
```

The public version must use an explicit list or a named error-set alias.

### What this fixes

Private glue code stops needing a manually maintained list that the compiler can derive exactly.

### What it does not fix

It is intentionally not a shortcut for public APIs. It also cannot be used for callbacks, bodyless function declarations, FFI functions, entry points, or recursive groups in the first version.

### Recommendation

Adopt after error-set aliases. It is valuable, but requires more compiler work than the first three strategies.

## Strategy 5: Wildcards or a universal error type

### Idea

Allow short forms such as these:

```opal
errors filesystem::*
errors AnyError
```

### Why they look attractive

They are very short and appear to solve the typing problem immediately.

### Why this proposal rejects them

They make the public contract less exact. If a filesystem module later gains a new error, `errors filesystem::*` changes meaning without any edit to the calling function. `AnyError` is worse: callers can no longer see or exhaustively reason about what may fail.

The other strategies keep the convenience while preserving a fixed, reviewable error contract.

## Recommended plan

These options are complementary, not mutually exclusive.

1. **First: editor quick fixes.** Make the existing model faster to write without changing the language.
2. **Second: module error families.** Give standard-library and package modules one useful umbrella error name.
3. **Third: error-set aliases.** Add `type Name = errors ...` so projects can name repeated cross-module combinations.
4. **Fourth: private inference.** Add `errors { infer }` only for private, acyclic functions with bodies.
5. **Do not add wildcards or `AnyError`.** They trade away the error model's strongest property: exact contracts.

## How the compiler would keep this safe

This section is intentionally brief; it describes the safety requirements rather than prescribing code structure.

- Every explicit alias is expanded into one canonical, duplicate-free error list before the compiler compares function types, checks `propagate`, generates documentation, or records exported metadata.
- The compiler must use the same canonical list everywhere. Two aliases that name the same errors in a different order must behave as the same error contract.
- Inference needs an ordering pass. The compiler first resolves explicit functions and aliases, then resolves private inferred helpers from their callees upward. If two inferred helpers depend on each other, it reports the cycle and asks for an explicit error-set alias instead.
- Generated docs and LSP hover must always show the expanded members of an alias or inferred signature.
- If an exported function's expanded error set changes, exported-signature and hot-reload compatibility checks must treat that as an API change, even if the native calling convention is unchanged.

## Implementation and test scope

The expected implementation order is:

1. LSP missing-error quick fix.
2. Documentation and standard-library convention for module error families.
3. Parser, resolver, diagnostics, formatter, documentation, and type-checker support for `type Name = errors ...`.
4. Canonical error-set comparison in propagation and function-type compatibility.
5. Private-only `errors { infer }` with dependency ordering and cycle diagnostics.

Tests must cover simple and nested aliases, imports, duplicate members, cycles, function-type compatibility, public aliases, missing propagated members, forwarding helpers, local recovery, forward inferred calls, recursive inferred calls, public lambda-backed `let` bindings, callbacks, FFI, documentation, and exported-contract changes.

## Non-goals

This proposal does not add exceptions, automatic propagation, dynamically typed errors, wildcard error imports, or a universal error type. It only makes the existing explicit error model easier to write and maintain.
