# Pattern Type with Captures

## Overview
This alternative introduces a `Pattern` type that retains named capture metadata and yields typed `CaptureMap` match results. Instead of returning only booleans or raw strings, regex operations expose structured capture values that can be validated and consumed safely.

The goal is correctness and expressiveness for applications that depend heavily on extracted fields, such as URL parsing, log processing, and protocol decoding.

## Assumes
- Opalescent type system can represent structured capture maps as explicit types
- Named capture metadata can be stored with compiled pattern values
- Standard library may include helper conversion functions for capture extraction
- Existing guard/propagate error flow remains mandatory

## Syntax Design
The design uses `Pattern` as the compiled object and typed capture outputs:

```opal
let compile_pattern = f(pattern_text: string): Pattern errors InvalidPattern =>
    # implementation detail
    return compiled

let match_captures = f(self: Pattern, input_text: string): CaptureMap errors MatchError, MissingCapture =>
    # implementation detail
    return captures
```

`CaptureMap` is explicit in signatures so callers can reason about missing names, conversion failures, and required keys with compile-time guidance.

## Example Applications
A parser can compile a route pattern once and then read `user_id` and `team_id` by name from `CaptureMap`, reducing fragile positional indexing. Similar gains apply to ingest pipelines that parse timestamps, levels, and request IDs from logs.

Typed captures also improve maintainability in long-lived systems because schema drift appears as clear compilation or guard-handling updates rather than silent index mistakes.

## Strengths
- Highest clarity for capture-heavy workflows
- Strong static contracts around named extraction
- Excellent foundation for future typed parsing utilities
- Error handling can encode capture-specific failure modes explicitly

## Weaknesses
- More concepts for users to learn (`Pattern`, `CaptureMap`, capture errors)
- Higher implementation and compiler integration cost
- Slightly verbose for simple boolean match checks

## Impact on Existing Syntax
Core language syntax can remain unchanged, but type-checker and standard library surfaces become richer. Existing regex use can coexist with this model through additive APIs.

Teams doing simple matches may keep using lighter forms, while capture-centric code adopts `Pattern` and `CaptureMap` for safety.

## Interactions with Other Concerns
This alternative aligns tightly with error strategy work because capture absence and conversion issues become first-class enum variants. It also interacts with module organization by encouraging paired `.types.op` definitions and operation modules.

In a future deferred landscape, a typed pattern object remains stable while only input transport changes, preserving capture contracts across synchronous and concurrent callers.

## Implementation Difficulty
High. The runtime must preserve named capture metadata, and checker/tooling must present precise types and diagnostics for `CaptureMap` usage. LSP support for capture names and type hints is likely needed for good developer experience.

## Must NOT Have
- Dynamic untyped bag returns for capture access
- Exception-based lookup failures
- Async-first API requirements in this concern
- `_sync` suffix naming for CPU-only regex operations
