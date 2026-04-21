# Opalescent Stdlib Proposals

This folder is the decision workspace for Opalescent’s standard-library surface. Each **concern** contains:

- a `COMPARISON.md` using the shared 6-axis matrix, and
- one or more alternative folders, each with a `proposal.md` plus `.op` examples.

## Concern Index

### 1. [error-strategy](./error-strategy/)
One model for expressing and composing failures without exceptions.

Alternatives:
- [`error-code-enum-module`](./error-strategy/error-code-enum-module/)
- [`layered-error-wrapping`](./error-strategy/layered-error-wrapping/)
- [`open-error-set`](./error-strategy/open-error-set/)
- [`registered-error-hierarchy`](./error-strategy/registered-error-hierarchy/)

### 2. [module-organization](./module-organization/)
How stdlib modules are named, grouped, and imported.

Alternatives:
- [`flat-bare-specifiers`](./module-organization/flat-bare-specifiers/)
- [`namespaced-stdlib`](./module-organization/namespaced-stdlib/)
- [`tiered-stdlib`](./module-organization/tiered-stdlib/)

### 3. [optional-representation](./optional-representation/)
How optional/absent values are represented in an errors-first language.

Alternatives:
- [`absence-via-errors`](./optional-representation/absence-via-errors/)
- [`maybe-tagged-union`](./optional-representation/maybe-tagged-union/)
- [`nullable-sentinel-types`](./optional-representation/nullable-sentinel-types/)

### 4. [byte-buffer-type](./byte-buffer-type/)
How raw binary buffers should be modeled and passed.

Alternatives:
- [`dedicated-bytes-type`](./byte-buffer-type/dedicated-bytes-type/)
- [`raw-uint8-array`](./byte-buffer-type/raw-uint8-array/)

### 5. [collections-api-shape](./collections-api-shape/)
Primary API style for arrays/maps/sets and related operations.

Alternatives:
- [`free-function-api`](./collections-api-shape/free-function-api/)
- [`method-style-api`](./collections-api-shape/method-style-api/)
- [`module-per-collection`](./collections-api-shape/module-per-collection/)
- [`pipeline-operator-api`](./collections-api-shape/pipeline-operator-api/)

### 6. [strings-text-encoding](./strings-text-encoding/)
String semantics and encoding boundaries across text APIs.

Alternatives:
- [`codepoint-first`](./strings-text-encoding/codepoint-first/)
- [`multiple-encodings`](./strings-text-encoding/multiple-encodings/)
- [`utf8-bytes-only`](./strings-text-encoding/utf8-bytes-only/)

### 7. [numeric-math-surface](./numeric-math-surface/)
How numeric utilities and math functionality are exposed.

Alternatives:
- [`expand-math-module`](./numeric-math-surface/expand-math-module/)
- [`split-math-into-modules`](./numeric-math-surface/split-math-into-modules/)
- [`typed-math-traits`](./numeric-math-surface/typed-math-traits/)

### 8. [random-rng](./random-rng/)
Deterministic RNG handling and randomness API ownership.

Alternatives:
- [`explicit-rng-handle`](./random-rng/explicit-rng-handle/)
- [`thread-local-default-rng`](./random-rng/thread-local-default-rng/)

### 9. [time-date-api](./time-date-api/)
Time modeling, clocks, and date/calendar ergonomics.

Alternatives:
- [`calendar-first`](./time-date-api/calendar-first/)
- [`monotonic-and-wall-clock-split`](./time-date-api/monotonic-and-wall-clock-split/)
- [`single-timestamp-type`](./time-date-api/single-timestamp-type/)

### 10. [file-io-surface](./file-io-surface/)
Filesystem read/write/open model and API shape.

Alternatives:
- [`handle-based`](./file-io-surface/handle-based/)
- [`path-object-centric`](./file-io-surface/path-object-centric/)
- [`whole-file-operations`](./file-io-surface/whole-file-operations/)

### 11. [network-http-layer](./network-http-layer/)
HTTP client surface for request/response interaction.

Alternatives:
- [`minimal-http-client-sync`](./network-http-layer/minimal-http-client-sync/)
- [`request-builder-sync`](./network-http-layer/request-builder-sync/)
- [`separate-client-type-sync`](./network-http-layer/separate-client-type-sync/)

### 12. [serialization](./serialization/)
Value encoding/decoding surface (JSON, TOML, typed flows, streams).

Alternatives:
- [`json-only-value-tree`](./serialization/json-only-value-tree/)
- [`json-plus-toml-uniform-api`](./serialization/json-plus-toml-uniform-api/)
- [`streaming-sync-readers-writers`](./serialization/streaming-sync-readers-writers/)
- [`typed-derive-style`](./serialization/typed-derive-style/)

### 13. [crypto-hashing](./crypto-hashing/)
Digest APIs for bytes and stream-oriented hashing.

Alternatives:
- [`hash-function-module`](./crypto-hashing/hash-function-module/)
- [`hasher-object-api`](./crypto-hashing/hasher-object-api/)
- [`typed-digest-wrappers`](./crypto-hashing/typed-digest-wrappers/)

### 14. [compression](./compression/)
Compression/decompression primitives and stream strategy.

Alternatives:
- [`compress-decompress-functions`](./compression/compress-decompress-functions/)
- [`stream-compressor-object`](./compression/stream-compressor-object/)

### 15. [logging](./logging/)
Application logging API, structure, and output handling.

Alternatives:
- [`global-logger-module`](./logging/global-logger-module/)
- [`logger-handle`](./logging/logger-handle/)
- [`structured-log-events`](./logging/structured-log-events/)

### 16. [regex](./regex/)
Pattern compilation/matching model and capture ergonomics.

Alternatives:
- [`compiled-regex-handle`](./regex/compiled-regex-handle/)
- [`pattern-type-with-captures`](./regex/pattern-type-with-captures/)
- [`regex-module-functions`](./regex/regex-module-functions/)

### 17. [uuid](./uuid/)
UUID generation/parsing API and type-level representation.

Alternatives:
- [`typed-uuid-wrappers`](./uuid/typed-uuid-wrappers/)
- [`v4-and-v7-separate`](./uuid/v4-and-v7-separate/)

### 18. [subprocess-exec](./subprocess-exec/)
Process spawning, lifecycle control, and command ergonomics.

Alternatives:
- [`command-builder`](./subprocess-exec/command-builder/)
- [`process-handle`](./subprocess-exec/process-handle/)
- [`run-command-function`](./subprocess-exec/run-command-function/)

### 19. [testing-framework](./testing-framework/)
Core testing runner/assertion/mocking strategy for stdlib users.

Alternatives:
- [`vitest-style-describe-it`](./testing-framework/vitest-style-describe-it/)

## Tier Recommendations

Recommended “most idiomatic for Opalescent” choice per concern (explicit errors, verbose names, no exceptions, Perceus model):

| Concern | Recommended alternative | Why this best fits Opalescent |
|---|---|---|
| error-strategy | `open-error-set` | It keeps error flow explicit at call boundaries without central registries or hidden conversion layers. |
| module-organization | `namespaced-stdlib` | Namespaced modules scale cleanly while preserving explicit import intent and long-form readability. |
| optional-representation | `absence-via-errors` | It aligns optionality with existing `errors`/`guard` mechanics instead of adding wrapper-driven control flow. |
| byte-buffer-type | `dedicated-bytes-type` | A distinct bytes type makes intent and API contracts clearer than reusing generic `uint8[]` everywhere. |
| collections-api-shape | `method-style-api` | Method-centric calls are expressive while still explicit and readable in expression-oriented code. |
| strings-text-encoding | `codepoint-first` | It gives predictable text semantics while keeping encoding concerns explicit at boundaries. |
| numeric-math-surface | `split-math-into-modules` | Focused modules keep APIs discoverable and prevent one oversized math namespace. |
| random-rng | `explicit-rng-handle` | Passing RNG handles keeps determinism/testability explicit and avoids hidden global state. |
| time-date-api | `monotonic-and-wall-clock-split` | Separating monotonic vs wall time prevents category mistakes and clarifies intent in signatures. |
| file-io-surface | `handle-based` | Handles map naturally to explicit error handling and resource lifecycle boundaries. |
| network-http-layer | `request-builder-sync` | Builders make request construction explicit and composable without sacrificing readability. |
| serialization | `json-plus-toml-uniform-api` | A uniform surface across common formats balances practicality with explicit format selection. |
| crypto-hashing | `hasher-object-api` | Stateful hasher objects cleanly support incremental hashing and explicit finalization. |
| compression | `stream-compressor-object` | Stream objects model stateful compression flows clearly and prepare for future deferred layering. |
| logging | `structured-log-events` | Structured events preserve machine-readability and explicit field-level intent. |
| regex | `compiled-regex-handle` | Compiled handles make cost and reuse explicit while keeping match calls straightforward. |
| uuid | `typed-uuid-wrappers` | Strong UUID wrapper types prevent accidental stringly misuse across module boundaries. |
| subprocess-exec | `command-builder` | Builders make process setup explicit, readable, and easier to validate before execution. |
| testing-framework | `vitest-style-describe-it` | It provides a full, familiar test surface while still mapping failures through explicit error paths. |

## How to Read a Proposal

Each alternative follows the same 10-section template from [`.template.proposal.md`](./.template.proposal.md):

1. **Overview** — core idea and scope.
2. **Assumes** — prerequisites/dependencies.
3. **Syntax Design** — syntax and grammar shape.
4. **Example Applications** — realistic sample usage.
5. **Strengths** — concrete benefits.
6. **Weaknesses** — trade-offs and risks.
7. **Impact on Existing Syntax** — compatibility and migration implications.
8. **Interactions with Other Concerns** — cross-concern composition/conflicts.
9. **Implementation Difficulty** — compiler/tooling/runtime effort level.
10. **Must NOT Have** — explicit anti-goals to prevent scope drift.

## Style Rules Summary (18 Binding Rules)

These are the binding cross-folder rules for `.op` examples:

1. Use `errors` clauses in signatures; do not use `Result<T, E>`, `Option<T>`, or `Either<L, R>` wrappers.
2. Use `T[]` array syntax only (never `[T]`).
3. Prefer verbose, descriptive names (no terse abbreviations like `gcd`, `clz`, `fmt`).
4. Public functions require `## ... ##` doc blocks with meaningful descriptions.
5. End functions with explicit `return <expr>` or `return void`.
6. No semicolons at statement ends.
7. Use snake_case for functions/variables/files; PascalCase for types; put types in `*.types.op` files.
8. Handle errors with `guard ... into ... else ... =>` and `propagate`; signatures must declare errors.
9. Follow canonical import grammar (`from` clauses, `./`/`../` local paths, `@scope/name` packages, explicit type imports).
10. Use canonical operators (`is`, `is not`, `^`, `band`/`bor`/`bxor`/`bnot`/`bshl`/`bshr`/`bushr`, `and`/`or`/`xor`/`not`).
11. Do not propose changes to the Perceus + second-class references memory model in these examples.
12. Use constructors as `new TypeName:` with indented fields.
13. Respect established bare-specifier module conventions (`standard`, `math`) and module-organization rules for new ones.
14. Apply `_sync` suffix to realistically deferred-capable operations; keep pure in-memory operations unsuffixed.
15. Use `#` for scenario comments; reserve `##...##` blocks for declaration doc blocks.
16. Every method in a `proposal.md` must have at least one realistic call site in `.op` examples.
17. `errors` clauses must be exhaustive and explicit; no placeholders (`...`, `etc`) or anonymous error types.
18. Every fallible call must be handled via `guard` or `propagate`, and propagated errors must be declared by the caller.

## Suggested Reading Order

1. Start with [`error-strategy/COMPARISON.md`](./error-strategy/COMPARISON.md) (it shapes every other concern).
2. Read module and type-surface foundations: `module-organization`, `optional-representation`, `byte-buffer-type`, `collections-api-shape`.
3. Move to platform-facing concerns: file/network/serialization/time/crypto/compression/subprocess.
4. Finish with developer-experience concerns: numeric math, random, logging, regex, uuid, testing framework.
