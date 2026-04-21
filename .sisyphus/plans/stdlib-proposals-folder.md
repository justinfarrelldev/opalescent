# Standard Library Proposals Folder

## TL;DR

> **Quick Summary**: Author a `stdlib-proposals/` folder mirroring the structure of `memory-model-proposals/`, containing design proposals for 19 standard-library concerns. Each concern gets its own folder with 1–5 alternative sub-folders, each alternative containing a `proposal.md` plus 2–3 authentic-style `.op` example files. A top-level `README.md` indexes the concerns and each concern folder has its own `COMPARISON.md` comparing only that concern's alternatives.
>
> **Deliverables**:
> - `stdlib-proposals/README.md` (index + tier recommendations)
> - 19 concern folders, each with `COMPARISON.md` + 1–5 alternative sub-folders
> - ~55–75 `proposal.md` files total (each ≤ 250 lines)
> - ~110–220 authentic-style `.op` example files (2–3 per alternative)
> - Delete the two pre-existing empty error-strategy stubs before authoring
> - Style-gate script that proves zero style violations across the folder
>
> **Estimated Effort**: XL
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: Wave 1 (foundations: template + style-gate + fixture scan) → Wave 2 (foundational concerns: error-strategy, module-organization, optional representation, byte buffer) → Wave 3 (domain concerns, max parallel) → Wave 4 (top-level README + global COMPARISON synthesis) → Wave FINAL (4 parallel reviewers + user okay)

---

## Context

### Original Request

User reviewed an earlier stdlib function recommendation and rejected its style, citing:

1. Functions used brackets `[T]` instead of Opalescent's `T[]`.
2. Names were over-abbreviated (`gcd` vs. `greatest_common_divisor`), violating Opalescent's verbose-intent principle.
3. Signatures lacked the `errors` keyword and did not follow Opalescent's error-handling grammar.

User then requested: "Please put together a folder with a similar structure to the memory-model-proposals folder which has proposals for different parts of the standard library. Where multiple options exist or are ambiguous for choices, please create multiple folders with their own individual proposals (like how the memory-model-proposals are structured, again)."

Additional directives: explore the codebase thoroughly (including `language-spec/`), use `oraios/serena`, perform a Momus high-accuracy review automatically without asking.

### Interview Summary

**Key Discussions**:
- **Scope**: All 12 core concerns + 7 additional concerns (serialization, crypto/hashing, testing, logging, regex, compression, UUID, subprocess/exec). Concurrency/async is **excluded** — user deferred it pending a separate async-surface fix.
- **Testing concern**: Must be Vitest-inspired with Opalescent flavor. Must include full mocking, stubbing, and spying if expressible in Opalescent's current mechanisms. Multiple alternatives required.
- **Verbosity mandate**: Re-emphasized — "Prefer verbosity over typing comfort. `greatest_common_denominator` over `gcd`." All `.op` examples must enforce this.
- **Opalescent fidelity constraint**: No alternative may introduce a mechanism not present in the language today. Specifically, **no exceptions**, no `try/catch`, no `Result<T,E>`, no async keywords in examples.
- **Async deferral + `_sync` discipline**: User confirmed async/await surface is being reworked separately. No concern in this folder drafts an async API. Concerns whose real-world counterparts typically offer both sync and async variants (file-io, network, subprocess, logging flush, stream hashing/compression, serialization streaming, time sleeping) must name their sync functions with a `_sync` suffix (e.g., `read_file_sync`, `http_get_sync`, `sleep_sync`) so the unsuffixed name stays reserved for the future async decision. User explicitly noted that without concurrency primitives, some concerns (file-io especially) will end up leaner than their mature counterparts in other languages — that is acceptable and must not be compensated for by faking async shapes.
- **Existing empty stubs**: `stdlib-proposals/error-strategy/per-module-errors/` and `stdlib-proposals/error-strategy/unified-std-error/` are to be **deleted** and error-strategy re-planned from scratch.
- **Alternatives cap**: 5 per concern (raised from Metis's recommendation of 3).

**Research Findings** (from codebase exploration):
- Existing stdlib is minimal: `prelude.op` (16 lines, docs only), Rust-side module files in `src/stdlib/{math.rs,strings.rs,fs.rs,io.rs,collections/,system/}`, 44 names registered in `src/codegen/functions_stdlib.rs`.
- Current bare-specifier modules: `standard` and `math`.
- `memory-model-proposals/` uses: root `COMPARISON.md`, concern folders (`borrow-checker/`, `perceus/`, `reference-counting/`, `region-based/`), each with 1–2 alternative sub-folders containing `proposal.md` + `.op` files.
- 13 concrete style rules extracted from `language-spec/*.op` (see "Opalescent Style Authority" below).

### Metis Review

Metis session `ses_251d9c871ffeyVdIYd2pAz5tyX` produced:

**Identified Gaps** (addressed in this plan):
- Fixed proposal.md template (10-section schema below).
- Per-concern `COMPARISON.md` instead of a single sprawling root doc.
- Top-level `README.md` as index-only (no duplication of per-concern comparisons).
- Cap on alternatives (user set to 5).
- Automated style-gate grep checks as a mandatory QA task.
- Empty-stub fate resolved (delete).
- Scope of concerns confirmed.

**Risks Flagged** (mitigations in plan):
1. Style drift across 100+ `.op` files → mitigated by Wave 1 style-gate + per-task lint scenario.
2. Cross-concern contradictions (e.g., error-strategy alternative A assumes monadic Option; optional-representation alternative B rejects monadic Option) → mitigated by explicit "Interactions with Other Concerns" section in every `proposal.md` and a Wave 4 cross-concern consistency audit.
3. Scope creep → capped at 5 alternatives per concern, ≤250 lines per `proposal.md`.
4. Stub zombies → Wave 1 Task 0 deletes the two empty folders before authoring.
5. Doc duplication between root and per-concern comparisons → root `README.md` is index-only.
6. Alternatives that break Opalescent identity (e.g., exceptions) → explicit "Must NOT Have" section in every proposal forbids non-Opalescent mechanisms.
7. Review fatigue with ~75 proposals → Momus reviews in batches by wave.

---

## Work Objectives

### Core Objective

Author a complete `stdlib-proposals/` folder that proposes designs for every major standard-library concern Opalescent will need, in the authentic voice of the language, with multiple alternatives wherever a design choice is genuinely open.

### Concrete Deliverables

- `stdlib-proposals/README.md` — top-level index, tier recommendations (foundational/domain/optional), links into each concern folder. Index-only; no per-alternative comparison content.
- 19 concern folders (list in Execution Strategy below), each containing:
  - `COMPARISON.md` — matrix comparing **only this concern's alternatives** across a fixed set of axes.
  - 1–5 alternative sub-folders, each containing:
    - `proposal.md` — following the fixed 10-section template below, ≤ 250 lines.
    - 2–4 `.op` example files demonstrating the proposal in authentic Opalescent style — **full realistic usage scenarios, not isolated signatures**. One file per usage scenario/domain (e.g., `read_config_file.op`, `write_atomic_log.op`, `concatenate_bytes_demo.op`). Every method proposed in `proposal.md` must have at least one call site across these files.
    - Optional `*.types.op` file(s) when the alternative introduces named error types or data types.
- Deleted folders: `stdlib-proposals/error-strategy/per-module-errors/`, `stdlib-proposals/error-strategy/unified-std-error/`.
- A `stdlib-proposals/.style-gate.sh` shell script (executable) that runs automated checks and exits non-zero on any violation.
- Evidence files under `.sisyphus/evidence/` for every QA scenario.

### Definition of Done

- [ ] `bash stdlib-proposals/.style-gate.sh` exits 0 with no violations reported.
- [ ] `find stdlib-proposals -name proposal.md | wc -l` ≥ 40 (minimum: average ~2 alternatives per concern × 19 concerns; actual will be higher).
- [ ] `find stdlib-proposals -name '*.op' | wc -l` ≥ 80 (minimum: 2 `.op` files × 40 proposals).
- [ ] `find stdlib-proposals -type d -empty` returns nothing.
- [ ] No `proposal.md` file exceeds 250 lines: `find stdlib-proposals -name proposal.md -exec wc -l {} + | awk '$1 > 250 {print; exit 1}'` passes.
- [ ] Every `proposal.md` contains all 10 required section headings (enforced by style-gate).
- [ ] `stdlib-proposals/README.md` exists and links to every concern folder.
- [ ] Every concern folder contains a `COMPARISON.md`.
- [ ] **Zero occurrences of `async` / `await` / `Promise` / `Future` anywhere in `stdlib-proposals/`.**
- [ ] **Every I/O-bearing function in file-io, network, subprocess, logging-flush, stream-hash, stream-compress, streaming-serialization, and time-sleep proposals is suffixed `_sync`.**
- [ ] **Every method proposed in any `proposal.md` has at least one realistic call site in an accompanying `.op` file.**
- [ ] **Every `errors` clause in every `.op` file enumerates all error types explicitly — no placeholders.**
- [ ] **Every fallible call in every `.op` example is handled via `guard ... into ... else err =>` or `propagate`.**
- [ ] **Every `.op` example file contains at least one `#` scenario comment describing the real-world context.**
- [ ] Momus high-accuracy review returns OKAY verdict.
- [ ] User explicitly approves the final deliverable.

### Must Have

- Mirror the **structural shape** of `memory-model-proposals/` (root summary + concern folders + alternative sub-folders + `.op` examples per alternative).
- Every `.op` example file uses authentic Opalescent style (all 13 rules below).
- Every `proposal.md` uses the fixed 10-section template (below).
- Every concern has a `COMPARISON.md` comparing only its own alternatives.
- All 19 concerns covered with at least 1 proposal each; concerns with multiple valid designs get up to 5 alternatives.
- Testing concern covers mocking, stubbing, and spying where expressible in current Opalescent mechanisms, with multiple alternatives.
- A top-level `README.md` that indexes (not duplicates) the concerns.
- All `.op` signatures use the `errors` keyword where errors can occur.
- Verbose names enforced: `greatest_common_denominator`, `count_leading_zeros`, `checked_integer_addition`, never abbreviated forms.
- Empty stubs deleted before authoring begins.

### Must NOT Have (Guardrails)

- **No `Result<T, E>`, `Option<T>`, `Either<L, R>`** in any `.op` example — Opalescent uses `errors` clauses, not monadic wrappers. (Note: an alternative proposal *about* adding a Result-like type is acceptable *as a proposal*, but the **example code** in that proposal must still compile under current Opalescent grammar using whatever syntax the proposal itself defines — no casual assumption that `Result<T,E>` exists.)
- **No `[T]` array syntax** — only `T[]`.
- **No abbreviated function names** — no `gcd`, `clz`, `itoa`, `atoi`, `mk`, `fmt`, `tmp`, `cfg`, `ctx`, `req`, `res`, `err` as names (`err` as a *local bound variable* inside `guard` is allowed since that pattern exists in `language-spec/error_handling_samples.op`; as a *function name* it is forbidden).
- **No exceptions, try/catch, throw, raise** — Opalescent does not have exceptions.
- **No async/await keywords in examples** — user deferred async pending surface fix.
- **No semicolons** at statement end.
- **No implementation code** — this folder is design proposals only; `.op` files are illustrative signatures and short bodies, not working runtime code (they should still be grammatically plausible under the current parser, but they do not need to pass type-checking against the current stdlib).
- **No Rust source code changes** — this plan only creates files under `stdlib-proposals/`.
- **No duplication** between root `README.md` and per-concern `COMPARISON.md` files — root indexes, concerns compare.
- **No concurrency/async concern folder** — explicitly deferred by user.
- **No async surface anywhere** — no `async`/`await` keywords, no futures/promises, no callback-based async, no event-loop APIs in any `.op` example across any concern. If a concern would ordinarily be lean without async (e.g., file-system, network, subprocess), it stays lean — do not compensate by inventing async-like constructs.
- **Every function whose real-world counterpart could plausibly be async in a future Opalescent release MUST be named with a `_sync` suffix.** This reserves the unsuffixed name (and any future `_async` variant) for later. Applies across: file-io-surface, network-http-layer, subprocess-exec, serialization (when reading/writing streams), crypto-hashing (when hashing streams), compression (when compressing/decompressing streams), logging (when flushing), time-date-api (when sleeping or scheduling), and testing-framework (when waiting on fake timers or awaiting mocks). Pure in-memory operations (string manipulation, numeric math, regex compile, uuid generation from a provided RNG, byte buffer slicing, collection operations) do **not** get `_sync` — they have no async counterpart. Concerns whose proposals must apply `_sync` are called out in each relevant task below.
- **No alternative proposals that introduce mechanisms absent from Opalescent today** (e.g., an alternative that "adds typed exceptions" is out of bounds; one that "adds a dedicated error-code enum module" is acceptable).
- **No `proposal.md` file exceeding 250 lines.**
- **No more than 5 alternatives per concern.**
- **No signature-only examples.** Every method proposed in a `proposal.md` must appear in at least one `.op` example file with a full usage demonstration: a realistic scenario (not `f(1, 2)` toy code), an actual call site, and explicit error handling via `guard ... into ... else ... =>` or `propagate` for every fallible call. Bare signatures in `.op` files (function declared but never called) are forbidden.
- **No `errors` clauses with placeholders or elisions.** Every function that can fail must enumerate every error type it raises. `errors FileError` is acceptable only if `FileError` is the exhaustive list; `errors SomeError, ...` or `errors /* see below */` is forbidden. Named error types must be introduced in a sibling `*.types.op` file or imported explicitly.
- **No unhandled fallible calls in example code.** If an `.op` example calls a function with an `errors` clause, the call site must either (a) use `guard ... into ... else err =>` to match and handle, or (b) use `propagate` to forward, and the enclosing function's own `errors` clause must list every forwarded error type.
- **Every example must include Opalescent-style comments** (`# …` single-line) explaining the scenario, the purpose of each step, and — critically — why each error branch matters. Comments must describe a realistic real-world scenario (e.g., "reading a config file on startup", "sending a telemetry ping", "hashing a user-uploaded file before storing"), not abstract `# do the thing`.

---

## Opalescent Style Authority (Binding Reference for All `.op` Files)

Every `.op` example file must obey these 13 rules. The style-gate script enforces the mechanical ones.

1. **Signatures**: `let name = f(params): return_type errors ErrType1, ErrType2 =>` — never `Result<T, E>` or similar.
2. **Arrays**: `T[]` (e.g., `uint8[]`, `string[]`). Never `[T]`.
3. **Verbose names**: `greatest_common_divisor` not `gcd`, `count_leading_zeros` not `clz`, `checked_integer_addition` not `checked_add`.
4. **Doc block**: `##\nDescription: …\n##` block of ≥ 30 characters on every public function.
5. **Explicit returns**: Every function ends with `return <expr>` or `return void`. No implicit last-expression returns.
6. **No semicolons** at statement ends.
7. **Casing**: `snake_case` for files, variables, functions; `PascalCase` for types; types live in `*.types.op` files.
8. **Error handling grammar**: `guard <expr> into <bind> else <err_bind> => <block>` and `propagate` keywords; signatures declare errors with the `errors` keyword.
9. **Imports** (per `language-spec/requirements/modules.md`):
   - **Bare specifiers** = stdlib only: `import sqrt from math`, `import print from standard`
   - **Local paths** must start with `./` or `../`: `import gcd from ./math`, `import config from ../shared/config`
   - **Packages** use `@scope/name`: `import leftpad from @leftpaddev/leftpad`
   - **Type imports** — both forms are valid:
     - `import PrimeFactorization from ./nums.types` (type suffix triggers type-import)
     - `import type User, Address from ./models.types` (explicit `type` keyword)
   - **Multi-import**: `import is_prime, gcd, pi from ./nums`
   - **Aliasing**: `import is_prime as is_prime_new from ./nums`, `import math as m` (whole-module alias enables `m.sqrt(9)` member access)
   - **Combined**: `import is_prime as is_prime_new, gcd as greatest_cd from ./nums`
   - Imports are case-sensitive. Only type identifiers are PascalCase; everything else is snake_case.
10. **Operators**: `is`, `is not`, `^` for power, `band`/`bor`/`bxor`/`bnot`/`bshl`/`bshr`/`bushr` for bitwise, `and`/`or`/`xor`/`not` for logical.
11. **Memory model is fixed** (Perceus + Second-Class References) — no `.op` example may propose altering it; alternatives are limited to surface API shape.
12. **Constructors**: `new TypeName:` followed by indented fields.
13. **Existing bare-specifier modules**: `standard`, `math`. New modules proposed in this folder must follow the module-organization concern's rules.
14. **`_sync` suffix discipline** (BINDING): Any function whose real-world counterpart could reasonably become asynchronous in a future Opalescent release must carry a `_sync` suffix in its name. Examples: `read_file_sync`, `write_file_sync`, `http_get_sync`, `spawn_subprocess_sync`, `sleep_sync`, `flush_log_sync`, `hash_stream_sync`, `compress_stream_sync`, `decompress_stream_sync`, `serialize_to_writer_sync`, `parse_from_reader_sync`. Pure in-memory / CPU-only operations do **not** get the suffix (e.g., `sha256_of_bytes`, `string_to_uppercase`, `regex_compile`, `greatest_common_divisor`, `uuid_v4_from_rng`, `slice_bytes`). The unsuffixed name is reserved for a future async decision. Do not invent an `_async` variant in these proposals.
15. **Comments use `#`** for single-line; `##…##` blocks are reserved for doc-blocks on public declarations. Comments inside function bodies explain intent at the scenario level, not restate code.
16. **Full usage examples mandatory**: Every method proposed in a `proposal.md` must appear in at least one `.op` file with a realistic usage scenario — actual call site with realistic arguments (not `f(1, 2)`), meaningful variable names, and error handling. Signature-only declarations (function declared but never called) are forbidden. A proposal that introduces 5 functions needs 5 call sites across its 2–3 `.op` files.
17. **Exhaustive `errors` clauses**: Every `errors` clause must list every error type the function can raise. No placeholders (`...`, `etc`), no inline comments substituting for types, no unnamed anonymous error types. Error types must either be (a) defined in a sibling `*.types.op` file in the same alternative folder, or (b) imported explicitly with a visible `import` line.
18. **Every fallible call must handle errors**: Inside `.op` example bodies, every call to a function with an `errors` clause must be wrapped in `guard ... into ... else err =>` (exhaustive handling) or followed by `propagate` (forwarding). An unhandled fallible call is a style-gate failure. When `propagate` is used, the enclosing function's `errors` clause must list every propagated error type.

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — all verification is agent-executed. Evidence saved under `.sisyphus/evidence/`.

### Test Decision
- **Infrastructure exists**: NO (Opalescent repo has Rust tests for the compiler, but this task produces documentation/proposals, not code).
- **Automated tests**: None (no code changes).
- **Framework**: N/A.
- **Verification mechanism**: Shell-based style-gate script + structural checks + per-task QA scenarios executed via `Bash` and `Read` tools.

### QA Policy

Every task includes agent-executed QA scenarios using `Bash` (grep/find/wc/awk) and `Read` (content verification). Evidence: grep output, file listings, and quoted file excerpts saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.txt`.

- **Folder structure**: `find` + `tree`-style output comparison
- **File content**: `grep -E` for required patterns + forbidden patterns
- **Line budget**: `wc -l` + `awk` threshold check
- **Template conformance**: `grep` for each of the 10 required section headings
- **Cross-concern consistency**: explicit read-and-compare scenario in Wave 4

---

## Execution Strategy

### Alternative-Count Commitment (Authoring Target)

To keep scope predictable, each concern has a **target alternative count** fixed up-front. The cap is 5; many concerns will have fewer because genuinely only 1–3 shapes make sense. Final counts may be adjusted during authoring if a proposal collapses into another, but the total must not exceed the cap.

| # | Concern | Target Alternatives |
|---|---------|---------------------|
| 1 | error-strategy | 4 |
| 2 | module-organization | 3 |
| 3 | optional-representation | 3 |
| 4 | byte-buffer-type | 2 |
| 5 | collections-api-shape | 4 |
| 6 | strings-text-encoding | 3 |
| 7 | numeric-math-surface | 3 |
| 8 | random-rng | 2 |
| 9 | time-date-api | 3 |
| 10 | file-io-surface | 3 |
| 11 | network-http-layer | 3 |
| 12 | serialization | 4 |
| 13 | crypto-hashing | 3 |
| 14 | compression | 2 |
| 15 | logging | 3 |
| 16 | regex | 3 |
| 17 | uuid | 2 |
| 18 | subprocess-exec | 3 |
| 19 | testing-framework | 5 |

Total proposals: **58**. Total `.op` files (at 2 per proposal minimum): **≥ 116**.

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundations, MAX PARALLEL):
├── Task 0:  Cleanup — delete empty error-strategy stubs [quick]
├── Task 1:  Fixed proposal.md template as authoring reference [writing]
├── Task 2:  Opalescent style-gate shell script [quick]
├── Task 3:  Reference-pattern extraction: capture canonical .op snippets from language-spec [quick]
└── Task 4:  COMPARISON.md axis schema (shared across concerns) [writing]

Wave 2 (After Wave 1 — foundational concerns, MAX PARALLEL):
├── Task 5:  Concern: error-strategy (4 alternatives) [writing]
├── Task 6:  Concern: module-organization (3 alternatives) [writing]
├── Task 7:  Concern: optional-representation (3 alternatives) [writing]
└── Task 8:  Concern: byte-buffer-type (2 alternatives) [writing]

Wave 3 (After Wave 2 — domain concerns, MAX PARALLEL):
├── Task 9:  Concern: collections-api-shape (4) [writing]
├── Task 10: Concern: strings-text-encoding (3) [writing]
├── Task 11: Concern: numeric-math-surface (3) [writing]
├── Task 12: Concern: random-rng (2) [writing]
├── Task 13: Concern: time-date-api (3) [writing]
├── Task 14: Concern: file-io-surface (3) [writing]
├── Task 15: Concern: network-http-layer (3) [writing]
├── Task 16: Concern: serialization (4) [writing]
├── Task 17: Concern: crypto-hashing (3) [writing]
├── Task 18: Concern: compression (2) [writing]
├── Task 19: Concern: logging (3) [writing]
├── Task 20: Concern: regex (3) [writing]
├── Task 21: Concern: uuid (2) [writing]
├── Task 22: Concern: subprocess-exec (3) [writing]
└── Task 23: Concern: testing-framework (5, Vitest-inspired) [writing]

Wave 4 (After Wave 3 — synthesis + cross-cutting audits):
├── Task 24: Top-level README.md (index + tier recommendations) [writing]
├── Task 25: Cross-concern consistency audit (fix contradictions) [deep]
└── Task 26: Run full style-gate script against entire folder; fix all violations [quick]

Wave FINAL (After ALL implementation — 4 parallel reviewers, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Style & doc quality review (unspecified-high)
├── Task F3: Real manual QA — exhaustive folder & content verification (unspecified-high)
└── Task F4: Scope fidelity check vs plan (deep)
→ Present results → Momus high-accuracy review → Get explicit user okay

Critical Path: Task 0 → Task 2 → Task 5 → Task 23 → Task 26 → F1–F4 → Momus → user okay
Parallel Speedup: ~70% faster than sequential (Wave 3 alone runs 15 tasks concurrently)
Max Concurrent: 15 (Wave 3)
```

### Dependency Matrix

- **0**: depends on nothing; blocks **5** (error-strategy folder re-created).
- **1**: blocks **5–23**.
- **2**: blocks **26**.
- **3**: blocks **5–23**.
- **4**: blocks **5–23**.
- **5–8**: depend on **0, 1, 3, 4**; block **9–23** only via shared style conventions (soft); block **25, 26**.
- **9–23**: depend on **1, 3, 4**; block **24, 25, 26**.
- **24**: depends on **5–23**; blocks **F1**.
- **25**: depends on **5–23**; blocks **26**.
- **26**: depends on **2, 24, 25**; blocks **F1–F4**.
- **F1–F4**: depend on **26**; block Momus review.
- **Momus**: depends on F1–F4; blocks user okay.

### Agent Dispatch Summary

- **Wave 1**: 5 tasks — T0 → `quick`, T1 → `writing`, T2 → `quick`, T3 → `quick`, T4 → `writing`.
- **Wave 2**: 4 tasks — all → `writing`.
- **Wave 3**: 15 tasks — all → `writing`.
- **Wave 4**: 3 tasks — T24 → `writing`, T25 → `deep`, T26 → `quick`.
- **Wave FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`.

---

## Fixed `proposal.md` Template (Every Alternative Uses This)

Every `proposal.md` must contain **exactly these 10 section headings in this order**. The style-gate script greps for each.

```markdown
# {Alternative Name}

## Overview
{1–2 paragraphs: what this alternative proposes, in plain English.}

## Assumes
{Bullet list of prerequisites this alternative depends on — e.g., "Assumes optional-representation chooses the `Maybe` tagged-union alternative."}

## Syntax Design
{The concrete `.op` syntax / module shape / function signatures this alternative introduces. Use fenced code blocks with `op` language tag.}

## Example Applications
{Pointer to the accompanying `.op` example files in this folder, with 1-sentence descriptions of each.}

## Strengths
{Bullet list.}

## Weaknesses
{Bullet list.}

## Impact on Existing Syntax
{What, if anything, this alternative requires changing in current Opalescent grammar or stdlib. "None" is an acceptable answer.}

## Interactions with Other Concerns
{Bullet list of dependencies on / conflicts with other concern folders' alternatives. Explicit cross-references.}

## Implementation Difficulty
{Quick / Short / Medium / Large / XL, with 1-sentence justification.}

## Must NOT Have
{Guardrails specific to this alternative — what would break its identity.}
```

---

## TODOs

### Global Authoring Rules (apply to every concern task, Tasks 5–23)

Every concern-authoring task MUST satisfy these rules. Individual tasks list only concern-specific additions; these rules are assumed.

1. **Structure**: `stdlib-proposals/<concern>/COMPARISON.md` + N alternative sub-folders (N matches target table, ≤5). Each alt folder: `proposal.md` + 2–4 `.op` files + optional `*.types.op`.
2. **Template**: Every `proposal.md` uses the 10-section template from `.template.proposal.md`. Each section must be non-empty. File ≤ 250 lines.
3. **COMPARISON axes**: Every `COMPARISON.md` uses the 6 axes from `.comparison-schema.md` (Ergonomics, Error-model fit, Opalescent-idiom fit, Implementation effort, Extensibility, Async readiness).
4. **Style**: All `.op` files obey the 18 style rules in "Opalescent Style Authority" above. Run the style-gate locally before marking the task done.
5. **Full usage examples** (rule 16): Every method declared in `proposal.md` has at least one realistic call site in an `.op` file. Scenarios must be real-world — "load a config file at startup", "send a telemetry ping", "verify a file's hash before extracting an archive" — not `f(1, 2)`.
6. **Exhaustive errors** (rule 17): Every `errors` clause lists every error type. Named error types are introduced in a sibling `*.types.op` or imported explicitly.
7. **Handled fallible calls** (rule 18): Every call to a fallible function is wrapped in `guard ... into ... else err =>` or followed by `propagate`. When using `propagate`, the enclosing function's `errors` clause lists every propagated type.
8. **`_sync` discipline** (rule 14): Applies to file-io-surface, network-http-layer, subprocess-exec, logging (flush functions only), time-date-api (sleep/schedule only), serialization (streaming functions only), crypto-hashing (streaming functions only), compression (streaming functions only). CPU-only functions in these concerns do NOT get `_sync`.
9. **Comments**: Every `.op` example has ≥ 1 `#` scenario comment at the top describing the real-world context, plus inline `#` comments on each error-handling branch explaining when that error matters.
10. **No async**: Zero `async`/`await`/`Promise`/`Future` anywhere in any file type.
11. **Interactions section**: Every `proposal.md`'s "Interactions with Other Concerns" section must explicitly reference which alternatives in other concerns this alternative assumes, composes with, or conflicts with. Use exact concern/alternative names.

### Shared QA Scenario Template (for Tasks 5–23)

Every concern task's QA block must include these three scenarios in addition to its concern-specific ones:

```
Scenario: Structure matches target
  Tool: Bash
  Steps:
    1. ls -1 stdlib-proposals/<concern>/ | grep -v '^COMPARISON.md$'
    2. Assert: exactly N folders matching target alt names
    3. For each alt: test -f proposal.md && test $(find . -maxdepth 1 -name '*.op' | wc -l) -ge 2
  Evidence: .sisyphus/evidence/task-<N>-structure.txt

Scenario: Local style-gate passes
  Tool: Bash
  Steps:
    1. Run: bash stdlib-proposals/.style-gate.sh 2>&1 | tee .sisyphus/evidence/task-<N>-gate.txt
    2. Assert: exit 0
  Evidence: .sisyphus/evidence/task-<N>-gate.txt

Scenario: Method coverage (every proposed method has a call site)
  Tool: Bash
  Steps:
    1. Run: python3 stdlib-proposals/.coverage-check.py --concern <concern>
    2. Assert: exit 0, lists every method with its call site
  Evidence: .sisyphus/evidence/task-<N>-coverage.txt
```

---

- [x] 0. Delete empty error-strategy stubs

  **What to do**:
  - Check whether `stdlib-proposals/error-strategy/per-module-errors/` and `stdlib-proposals/error-strategy/unified-std-error/` exist. If either is absent, treat Task 0 as **already satisfied** — no action needed for that path.
  - If either exists: confirm it is empty (no files, no nested folders), then remove it with `rm -r`.
  - Leave `stdlib-proposals/error-strategy/` as a non-existent (or empty) parent; it will be re-created in Task 5.
  - Use idempotent `rm -rf` so re-running the task after partial completion is safe.

  **Must NOT do**:
  - Delete anything outside `stdlib-proposals/error-strategy/`.
  - Delete the parent `stdlib-proposals/` directory.

  **Recommended Agent Profile**:
  - **Category**: `quick` — one-shot filesystem cleanup.
  - **Skills**: none.

  **Parallelization**: Wave 1. Can run immediately. Blocks Task 5. Blocked by: none.

  **References**:
  - User decision in interview: "Delete and start error-strategy fresh"
  - `/home/justi/Projects/opalescent/stdlib-proposals/error-strategy/` — may or may not exist as empty stubs; task is idempotent either way

  **Acceptance Criteria**:
  - [ ] `test ! -d stdlib-proposals/error-strategy/per-module-errors` passes
  - [ ] `test ! -d stdlib-proposals/error-strategy/unified-std-error` passes

  **QA Scenarios**:

  ```
  Scenario: Empty stubs are removed (or were never present)
    Tool: Bash
    Preconditions: None — task is idempotent. Stubs may or may not exist at start.
    Steps:
      1. If `stdlib-proposals/error-strategy/` exists, run: find stdlib-proposals/error-strategy -maxdepth 1 -mindepth 1 -type d
      2. Assert: output is empty (no per-module-errors, no unified-std-error) OR the parent error-strategy folder does not exist at all
      3. Also assert no files lost outside error-strategy: test -d stdlib-proposals/ (if other concerns exist yet, they remain untouched)
    Expected Result: No stub subfolders remain under error-strategy; task is a clean no-op if stubs were never present.
    Evidence: .sisyphus/evidence/task-0-stubs-removed.txt

  Scenario: No other folders touched
    Tool: Bash
    Preconditions: Stub deletion complete
    Steps:
      1. Run: git status --short stdlib-proposals/
      2. Assert: only the two deleted folders appear
    Expected Result: No unexpected file changes
    Evidence: .sisyphus/evidence/task-0-git-status.txt
  ```

  **Commit**: NO (groups with Wave 1 commit)

---

- [x] 1. Write fixed `proposal.md` template as authoring reference

  **What to do**:
  - Create `stdlib-proposals/.template.proposal.md` with the 10-section template defined above (Overview, Assumes, Syntax Design, Example Applications, Strengths, Weaknesses, Impact on Existing Syntax, Interactions with Other Concerns, Implementation Difficulty, Must NOT Have).
  - Include inline comments (`<!-- … -->`) in each section explaining what goes there, so authors filling it in have guidance.
  - File must itself be ≤ 250 lines.

  **Must NOT do**:
  - Add sections beyond the 10 required ones.
  - Include example content that could be mistaken for a real proposal.

  **Recommended Agent Profile**:
  - **Category**: `writing` — documentation authoring.
  - **Skills**: none.

  **Parallelization**: Wave 1. Blocks Tasks 5–23. Blocked by: none.

  **References**:
  - The 10-section schema in this plan's "Fixed `proposal.md` Template" section
  - `/home/justi/Projects/opalescent/memory-model-proposals/borrow-checker/simplified-borrow-checker/proposal.md` — reference shape

  **Acceptance Criteria**:
  - [ ] File exists at `stdlib-proposals/.template.proposal.md`
  - [ ] All 10 headings present in the correct order
  - [ ] File ≤ 250 lines

  **QA Scenarios**:

  ```
  Scenario: Template contains all 10 sections in order
    Tool: Bash
    Preconditions: Template written
    Steps:
      1. Run: grep -nE '^## ' stdlib-proposals/.template.proposal.md
      2. Assert: output lists Overview, Assumes, Syntax Design, Example Applications, Strengths, Weaknesses, Impact on Existing Syntax, Interactions with Other Concerns, Implementation Difficulty, Must NOT Have in that order
    Expected Result: 10 headings in exact order
    Evidence: .sisyphus/evidence/task-1-template-headings.txt

  Scenario: Template under line budget
    Tool: Bash
    Steps:
      1. Run: wc -l stdlib-proposals/.template.proposal.md
      2. Assert: count ≤ 250
    Expected Result: within budget
    Evidence: .sisyphus/evidence/task-1-template-lines.txt
  ```

  **Commit**: NO (groups with Wave 1 commit)

---

- [x] 2. Write Opalescent style-gate shell script + coverage checker

  **What to do**:
  - Create executable `stdlib-proposals/.style-gate.sh` that runs every check in the "Success Criteria → Verification Commands" section above (checks 1–16).
  - Create executable `stdlib-proposals/.coverage-check.py` (Python 3, stdlib only) that performs the two checks `.style-gate.sh` delegates to it. **CLI contract (both modes required, both exit 0 on clean / non-zero on violation)**:
    - **No-args mode**: `python3 stdlib-proposals/.coverage-check.py` — scans every concern folder under `stdlib-proposals/` that contains alternative subfolders.
    - **Scoped mode**: `python3 stdlib-proposals/.coverage-check.py --concern <name>` — scans only `stdlib-proposals/<name>/` and its alternative subfolders. Used by per-concern authoring QA scenarios.
    - **Method-coverage check**: For each alternative folder, parse its `proposal.md` fenced `op` code blocks, extract every `let NAME = f(...)` signature, then grep every `.op` file in the same folder for a call site `NAME(`. Every proposed method must have at least one call site.
    - **Fallible-call handling check**: For each `.op` file, parse every `let NAME = f(...): ret errors E1, E2 =>` signature to build a map of fallible-function names. Then scan every call site `NAME(` in every `.op` file in the same alternative folder; assert the enclosing statement is inside a `guard ... into ... else ... =>` construct OR is immediately followed by (or chained with) `propagate`. Walk both the current alternative's files and any sibling `.types.op` file for signatures. Calls to imported external functions (via `import`) are exempt only if the import is from a fake stdlib module (`standard`, `math`, `bytes`, `regex`, etc.) and the call is still wrapped in `guard`/`propagate` — so really exempt nothing: every call to a function whose name ends with a pattern plausibly fallible (by the plan's heuristic: ends with `_sync`, contains `parse`, `read`, `write`, `open`, `connect`, `send`, `recv`, `hash_stream`, `compress`, `decompress`, `spawn`, `kill`, `flush`) must be handled.
  - Both scripts must `set -euo pipefail` (or Python equivalent) and exit non-zero on any violation with a clear message naming the offending file and line.
  - `.style-gate.sh` checks (at minimum): forbidden patterns (`Result<`, `Option<`, `Either<`, `[T]` array syntax, abbreviated names, semicolons, `async`/`await`/`Promise`/`Future`), `_sync` suffix enforcement in the 5 I/O-bearing concerns, 10-section template conformance, ≤250-line `proposal.md` budget, `COMPARISON.md` presence per concern, ≥ 2 `.op` files per alternative, doc-block presence on every public function in `.op` files, verbose-name enforcement, placeholder-errors-clause rejection, scenario-comment presence, and delegation to `.coverage-check.py`.
  - Must emit "All style checks passed." on success.

  **Must NOT do**:
  - Check against concerns that don't exist yet (scripts must handle missing concerns gracefully during Wave 2/3 authoring).
  - Modify any files (read-only inspection).
  - Use any non-stdlib Python dependency.

  **Recommended Agent Profile**:
  - **Category**: `quick` — mechanical shell + Python.
  - **Skills**: none.

  **Parallelization**: Wave 1. Blocks Task 26 (full style-gate run). Blocked by: none.

  **References**:
  - The 16 verification commands in "Success Criteria → Verification Commands"
  - The 18 style rules in "Opalescent Style Authority"
  - `/home/justi/Projects/opalescent/language-spec/error_handling_samples.op` — source of truth for `errors` / `guard` / `propagate` grammar

  **Acceptance Criteria**:
  - [ ] `stdlib-proposals/.style-gate.sh` exists, executable, uses `set -euo pipefail`
  - [ ] `stdlib-proposals/.coverage-check.py` exists, executable, Python 3 stdlib only
  - [ ] `bash stdlib-proposals/.style-gate.sh` exits 0 against the empty folder (no concerns authored yet)
  - [ ] Injecting a file containing `Result<T, E>` into a test dir makes the script exit non-zero
  - [ ] Injecting a fallible call without `guard`/`propagate` makes the coverage check exit non-zero
  - [ ] Injecting a proposed method with no call site makes the coverage check exit non-zero

  **QA Scenarios**:

  ```
  Scenario: Style gate passes on empty folder
    Tool: Bash
    Preconditions: Only Task 0 and Task 1 completed (no concerns authored)
    Steps:
      1. Run: bash stdlib-proposals/.style-gate.sh
      2. Capture exit code and stdout
    Expected Result: exit 0, "All style checks passed." in stdout
    Evidence: .sisyphus/evidence/task-2-empty-gate.txt

  Scenario: Style gate catches forbidden Result<T,E>
    Tool: Bash
    Preconditions: Scripts written
    Steps:
      1. mkdir -p /tmp/gate-test/stdlib-proposals/fake/fake-alt
      2. cp stdlib-proposals/.style-gate.sh stdlib-proposals/.coverage-check.py /tmp/gate-test/stdlib-proposals/
      3. cat > /tmp/gate-test/stdlib-proposals/fake/fake-alt/bad.op <<'EOF'
         ##
         Description: bad example used only to verify the style gate rejects it correctly.
         ##
         let f = g(): Result<int32, string> =>
           return 1
         EOF
      4. cd /tmp/gate-test && bash stdlib-proposals/.style-gate.sh; echo "exit=$?"
    Expected Result: non-zero exit, message naming bad.op and the `Result<...>` pattern
    Evidence: .sisyphus/evidence/task-2-forbidden-pattern.txt

  Scenario: Style gate enforces _sync suffix in file-io-surface
    Tool: Bash
    Steps:
      1. Create fake concern: mkdir -p stdlib-proposals/file-io-surface/fake
      2. Create stdlib-proposals/file-io-surface/fake/a.op with a `read_file` (not `_sync`) signature and a proper doc block
      3. Run: bash stdlib-proposals/.style-gate.sh
      4. Assert: non-zero exit citing missing _sync suffix on read_file
      5. Cleanup: rm -rf stdlib-proposals/file-io-surface/fake
    Expected Result: gate rejects non-_sync I/O function
    Evidence: .sisyphus/evidence/task-2-sync-enforcement.txt

  Scenario: Coverage check rejects proposed method with no call site
    Tool: Bash
    Steps:
      1. Create fake alternative with proposal.md declaring `let foo_sync = f(path: string): void errors FileError` but no .op file calling foo_sync
      2. Run: bash stdlib-proposals/.style-gate.sh
      3. Assert: non-zero exit citing foo_sync missing a call site
      4. Cleanup
    Evidence: .sisyphus/evidence/task-2-coverage.txt

  Scenario: Coverage check rejects unhandled fallible call
    Tool: Bash
    Steps:
      1. Create fake alternative with an .op file calling a _sync function without guard or propagate
      2. Run: bash stdlib-proposals/.style-gate.sh
      3. Assert: non-zero exit citing the unhandled call with file:line
      4. Cleanup
    Evidence: .sisyphus/evidence/task-2-unhandled.txt

  Scenario: Coverage check rejects placeholder errors clause
    Tool: Bash
    Steps:
      1. Create fake file with `errors FileError, ...`
      2. Run: bash stdlib-proposals/.style-gate.sh
      3. Assert: non-zero exit
      4. Cleanup
    Evidence: .sisyphus/evidence/task-2-placeholder.txt
  ```

  **Commit**: NO (groups with Wave 1 commit)

---

- [ ] 3. Extract canonical `.op` reference snippets from language-spec

  **What to do**:
  - Read every `.op` file under `/home/justi/Projects/opalescent/language-spec/`.
  - Produce `stdlib-proposals/.reference-patterns.md` capturing 6 canonical snippets (actual copy-pasted excerpts with file:line citations):
    1. A public function with doc block + `errors` clause (from `error_handling_samples.op`)
    2. A `guard … into … else … =>` pattern (from same file)
    3. A `propagate` pattern
    4. A `*.types.op` type definition with `new TypeName:` constructor (from `types_example.types.op`)
    5. An import statement covering all three forms (bare, package, relative) — synthesize if no single file has all three
    6. An array-returning function using `T[]` syntax (from `array_helpers.op` or `partition.op`)
  - This file is **authoring reference only** — authors consult it while writing `.op` examples.

  **Must NOT do**:
  - Invent syntax not present in the language-spec files.
  - Modify any file under `language-spec/`.

  **Recommended Agent Profile**:
  - **Category**: `quick` — copy-paste extraction.
  - **Skills**: none.

  **Parallelization**: Wave 1. Blocks Tasks 5–23. Blocked by: none.

  **References**:
  - `/home/justi/Projects/opalescent/language-spec/error_handling_samples.op`
  - `/home/justi/Projects/opalescent/language-spec/simple_quiz.op`
  - `/home/justi/Projects/opalescent/language-spec/array_helpers.op`
  - `/home/justi/Projects/opalescent/language-spec/partition.op`
  - `/home/justi/Projects/opalescent/language-spec/types_example.types.op`
  - `/home/justi/Projects/opalescent/language-spec/types_usage_example.op`

  **Acceptance Criteria**:
  - [ ] File exists at `stdlib-proposals/.reference-patterns.md`
  - [ ] Contains all 6 snippets with file:line citations
  - [ ] Every snippet is quoted verbatim from a language-spec file (verifiable by grep)

  **QA Scenarios**:

  ```
  Scenario: Every cited snippet is present in the cited file
    Tool: Bash
    Steps:
      1. For each snippet in .reference-patterns.md: extract its citation (file:line) and assert the quoted code appears at that location
      2. Run: grep -F "<quoted snippet>" language-spec/<cited file>
    Expected Result: every snippet matches its cited source
    Evidence: .sisyphus/evidence/task-3-citations.txt

  Scenario: File covers all 6 required patterns
    Tool: Bash
    Steps:
      1. grep -c '^### ' stdlib-proposals/.reference-patterns.md
      2. Assert: count ≥ 6
    Evidence: .sisyphus/evidence/task-3-coverage.txt
  ```

  **Commit**: NO (groups with Wave 1 commit)

---

- [ ] 4. Write shared `COMPARISON.md` axis schema

  **What to do**:
  - Create `stdlib-proposals/.comparison-schema.md` defining a fixed table-of-axes every concern's `COMPARISON.md` must use:
    | Axis | Description |
    |------|-------------|
    | Ergonomics | How pleasant day-to-day usage is |
    | Error-model fit | How well it composes with the chosen error-strategy concern |
    | Opalescent-idiom fit | How closely it matches existing language flavor |
    | Implementation effort | Cost to land in the compiler/stdlib |
    | Extensibility | Room for future growth without breaking changes |
    | Async readiness | How cleanly an async counterpart could be added later (no async written now, but the shape must not preclude it) |
  - Include a worked mini-example (fake 2-alternative comparison) so authors see the expected filled-in shape.
  - File itself ≤ 250 lines.

  **Must NOT do**:
  - Define more than the 6 axes above.
  - Produce a real concern's comparison in this file.

  **Recommended Agent Profile**:
  - **Category**: `writing`.
  - **Skills**: none.

  **Parallelization**: Wave 1. Blocks Tasks 5–23. Blocked by: none.

  **References**:
  - `/home/justi/Projects/opalescent/memory-model-proposals/COMPARISON.md` — reference for matrix shape
  - The 6-axis list above

  **Acceptance Criteria**:
  - [ ] File exists at `stdlib-proposals/.comparison-schema.md`
  - [ ] All 6 axes defined with descriptions
  - [ ] Worked mini-example present
  - [ ] ≤ 250 lines

  **QA Scenarios**:

  ```
  Scenario: All 6 axes present
    Tool: Bash
    Steps:
      1. for axis in "Ergonomics" "Error-model fit" "Opalescent-idiom fit" "Implementation effort" "Extensibility" "Async readiness"; do grep -qF "$axis" stdlib-proposals/.comparison-schema.md || echo "MISSING: $axis"; done
      2. Assert: no output
    Evidence: .sisyphus/evidence/task-4-axes.txt

  Scenario: Worked example parseable
    Tool: Bash
    Steps:
      1. Extract markdown tables: grep -cE '^\|' stdlib-proposals/.comparison-schema.md
      2. Assert: count ≥ 4 (axis definition table + example comparison table, each with header + separator + ≥1 row)
    Evidence: .sisyphus/evidence/task-4-example.txt
  ```

  **Commit**: YES (Wave 1 commit groups Tasks 0–4)
  - Message: `docs(stdlib-proposals): scaffold template, style-gate, and comparison schema`
  - Files: `stdlib-proposals/.template.proposal.md`, `stdlib-proposals/.style-gate.sh`, `stdlib-proposals/.reference-patterns.md`, `stdlib-proposals/.comparison-schema.md`, plus stub deletions
  - Pre-commit: `bash stdlib-proposals/.style-gate.sh`

---

- [x] 5. Concern: `error-strategy` (4 alternatives)

  **What to do**:
  - Create `stdlib-proposals/error-strategy/` with `COMPARISON.md` + 4 alternative sub-folders, each with `proposal.md` + 2 `.op` examples.
  - Alternatives to author (cap 5, target 4):
    1. `open-error-set` — any function can declare any error types via `errors`; no central registry. (Status quo extrapolated.)
    2. `registered-error-hierarchy` — central `error_types` module the stdlib imports from; new errors must be registered.
    3. `error-code-enum-module` — each stdlib module exports a single enum listing its errors; callers match on enum cases.
    4. `layered-error-wrapping` — stdlib functions can attach context via a `wrap_error_context(original, message)` helper; still uses `errors` keyword, no exceptions.
  - Forbidden alternative that must NOT be written: any "add exceptions" or "add `Result<T,E>`" alternative — both violate Opalescent fidelity.
  - Every `.op` example uses `errors` clause grammar from `language-spec/error_handling_samples.op`.

  **Must NOT do**:
  - Propose exceptions, `try/catch`, `throw`, `raise`.
  - Propose `Result<T, E>` or `Option<T>` monadic wrappers.
  - Exceed 5 alternatives.

  **Recommended Agent Profile**:
  - **Category**: `writing`.
  - **Skills**: none.

  **Parallelization**: Wave 2. Blocks Tasks 25, 26 (and soft-blocks 9–23 via cross-concern dependency on error-strategy choice). Blocked by: Tasks 0, 1, 3, 4.

  **References**:
  - `/home/justi/Projects/opalescent/language-spec/error_handling_samples.op` — canonical `errors` grammar
  - `/home/justi/Projects/opalescent/language-spec/requirements/overview.md` — error-handling philosophy
  - `/home/justi/Projects/opalescent/ERROR_HANDLING_STANDARDS.md` — Rust compiler error conventions (NOT language-level; reference only to avoid confusion)
  - `stdlib-proposals/.template.proposal.md`
  - `stdlib-proposals/.comparison-schema.md`
  - `stdlib-proposals/.reference-patterns.md`

  **Acceptance Criteria**:
  - [ ] `stdlib-proposals/error-strategy/COMPARISON.md` exists and uses the 6-axis schema
  - [ ] 4 alternative folders present with `proposal.md` + ≥2 `.op` files each
  - [ ] No `proposal.md` exceeds 250 lines
  - [ ] No `.op` file contains `Result<`, `Option<`, `Either<`, exceptions, `async`, `await`
  - [ ] Every public function in `.op` files has a ≥30-char `##…##` doc block
  - [ ] Style-gate passes on this concern

  **QA Scenarios**:

  ```
  Scenario: Four alternatives authored with correct shape
    Tool: Bash
    Steps:
      1. ls -1 stdlib-proposals/error-strategy/ | grep -v COMPARISON.md
      2. Assert: exactly 4 subfolders: open-error-set, registered-error-hierarchy, error-code-enum-module, layered-error-wrapping
      3. For each: test -f proposal.md && test $(find -name '*.op' | wc -l) -ge 2
    Expected Result: all 4 conformant
    Evidence: .sisyphus/evidence/task-5-structure.txt

  Scenario: No exception or Result<T,E> mechanism introduced
    Tool: Bash
    Steps:
      1. grep -rEn 'throw|raise|try\s*{|catch\s*\(|Result<|Option<|Either<' stdlib-proposals/error-strategy/
      2. Assert: no matches
    Expected Result: clean
    Evidence: .sisyphus/evidence/task-5-forbidden.txt

  Scenario: COMPARISON.md uses shared axis schema
    Tool: Bash
    Steps:
      1. for axis in "Ergonomics" "Error-model fit" "Opalescent-idiom fit" "Implementation effort" "Extensibility" "Async readiness"; do grep -qF "$axis" stdlib-proposals/error-strategy/COMPARISON.md || echo "MISSING: $axis"; done
      2. Assert: no output
    Expected Result: all 6 axes referenced
    Evidence: .sisyphus/evidence/task-5-axes.txt
  ```

  **Commit**: NO (groups with Wave 2 commit)

---

- [x] 6. Concern: `module-organization` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/module-organization/` with `COMPARISON.md` + 3 alternative sub-folders.
  - Alternatives (target 3):
    1. `flat-bare-specifiers` — expand the current `standard` + `math` pattern: every stdlib area gets its own top-level bare specifier (`standard`, `math`, `bytes`, `strings`, `time`, `random`, `filesystem`, `network`, `serialization`, `crypto`, `compression`, `logging`, `regex`, `uuid`, `subprocess`, `testing`).
    2. `namespaced-stdlib` — single `standard` bare specifier, sub-paths like `import regex from standard/regex`.
    3. `tiered-stdlib` — three tiers: `core` (always imported), `standard` (opt-in bare-specifier), `standard_extra` (opt-in, may require build flag).
  - Each alternative's `.op` examples demonstrate import syntax for 3 different modules.

  **Must NOT do**:
  - Propose imports that would break existing grammar in `/home/justi/Projects/opalescent/language-spec/requirements/modules.md`.
  - Add a 4th alternative.

  **Recommended Agent Profile**:
  - **Category**: `writing`.

  **Parallelization**: Wave 2. Blocks 9–23 (soft — they use module imports). Blocked by: Tasks 1, 3, 4.

  **References**:
  - `/home/justi/Projects/opalescent/language-spec/requirements/modules.md`
  - `/home/justi/Projects/opalescent/stdlib/prelude.op`
  - `/home/justi/Projects/opalescent/src/codegen/functions_stdlib.rs` — current 44 registered names

  **Acceptance Criteria**:
  - [ ] 3 alternative folders with proposal.md + ≥2 .op files
  - [ ] COMPARISON.md uses 6-axis schema
  - [ ] Every .op example shows at least one `import` statement
  - [ ] No proposal exceeds 250 lines
  - [ ] Style-gate passes

  **QA Scenarios**:

  ```
  Scenario: Three alternatives with import examples
    Tool: Bash
    Steps:
      1. ls stdlib-proposals/module-organization/ | grep -v COMPARISON.md
      2. Assert: flat-bare-specifiers, namespaced-stdlib, tiered-stdlib
      3. grep -rln '^import ' stdlib-proposals/module-organization/ --include='*.op' | wc -l
      4. Assert: ≥ 6 (3 alternatives × ≥2 files)
    Evidence: .sisyphus/evidence/task-6-structure.txt

  Scenario: Import syntax matches language-spec grammar
    Tool: Bash
    Preconditions: Module-organization alt folders authored with .op files containing imports.
    Steps:
      1. Extract every line starting with `import ` across the concern:
         grep -rEn '^import ' stdlib-proposals/module-organization/ --include='*.op' > /tmp/task6-imports.txt
      2. Assert every line matches ONE of these spec forms (per modules.md lines 16-49):
         - Value import:        `^import [a-z_]+(, [a-z_]+)*( as [a-z_]+)?( , [a-z_]+( as [a-z_]+)?)* from (standard|[a-z_]+|\./[a-z_./]+|\.\./[a-z_./]+|@[a-z_]+/[a-z_-]+)$`
         - Type import (suffix):`^import [A-Z][A-Za-z0-9_]*(, [A-Z][A-Za-z0-9_]*)* from (\./[a-z_./]+|\.\./[a-z_./]+)\.types$`
         - Type import (kw):    `^import type [A-Z][A-Za-z0-9_]*(, [A-Z][A-Za-z0-9_]*)* from (\./[a-z_./]+|\.\./[a-z_./]+)\.types$`
         - Whole-module alias:  `^import [a-z_]+ as [a-z_]+$`  (e.g. `import math as m`)
         Use a small shell script with 4 regex branches; every line must match at least one, else FAIL and print the file:line.
      3. Assert: zero non-matching lines.
      4. Confirm at least one alt demonstrates a type import (either form) and at least one demonstrates aliasing, to prove examples exercise the full grammar.
    Expected Result: exit 0, all import lines match spec grammar, type-import and alias forms both present.
    Evidence: .sisyphus/evidence/task-6-imports.txt
  ```

  **Commit**: NO

---

- [x] 7. Concern: `optional-representation` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/optional-representation/` with `COMPARISON.md` + 3 alternatives.
  - Alternatives (target 3):
    1. `absence-via-errors` — no dedicated optional type; absence is an error variant like `NotFound`. Callers use `guard` to handle missing values.
    2. `maybe-tagged-union` — introduce a `Maybe T` tagged union defined in a stdlib types module; constructors `Some` / `None`.
    3. `nullable-sentinel-types` — per-type sentinel values (e.g., `empty_string`, `invalid_index`) documented by convention; no new type machinery.
  - Examples demonstrate looking up a value in a map (or similar absence-bearing operation) under each alternative.

  **Must NOT do**:
  - Import `Option<T>` from a non-existent prior art — each alternative must define its own machinery or explain why none is needed.
  - Use `null` or `nil` keywords (neither exists in Opalescent).

  **Recommended Agent Profile**:
  - **Category**: `writing`.

  **Parallelization**: Wave 2. Blocks 9 (collections), 10 (strings), 14 (fs — path lookup). Blocked by: 1, 3, 4.

  **References**:
  - `/home/justi/Projects/opalescent/language-spec/error_handling_samples.op`
  - `/home/justi/Projects/opalescent/language-spec/types_example.types.op` — how types + constructors are expressed
  - `/home/justi/Projects/opalescent/language-spec/types_usage_example.op`

  **Acceptance Criteria**:
  - [ ] 3 folders authored
  - [ ] Each alternative shows a lookup operation (e.g., map get, array find)
  - [ ] No `null` / `nil` / `Option<` / `Maybe<` (angle-bracket form) anywhere
  - [ ] Style-gate passes

  **QA Scenarios**:

  ```
  Scenario: No null/nil keywords
    Tool: Bash
    Steps:
      1. grep -rEwn 'null|nil|undefined' stdlib-proposals/optional-representation/ --include='*.op'
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-7-no-null.txt

  Scenario: Each alternative demonstrates absence handling
    Tool: Bash
    Steps:
      1. For each alternative folder: grep -l -E '(guard |NotFound|None|Maybe|invalid_|empty_)' *.op
      2. Assert: every alternative has at least one file matching
    Evidence: .sisyphus/evidence/task-7-absence.txt
  ```

  **Commit**: NO

---

- [x] 8. Concern: `byte-buffer-type` (2 alternatives)

  **What to do**:
  - Create `stdlib-proposals/byte-buffer-type/` with `COMPARISON.md` + 2 alternatives.
  - Alternatives (target 2):
    1. `raw-uint8-array` — use the existing `uint8[]` directly for all byte-level work. Stdlib provides helper functions over plain `uint8[]`. No new type.
    2. `dedicated-bytes-type` — introduce a `Bytes` type (struct wrapping `uint8[]` with length/capacity metadata) in a new `bytes` module; provides operations like `slice_bytes`, `concatenate_bytes`, `bytes_to_hex_string`.
  - Both alternatives must show: read a file's contents into bytes, concatenate two byte sequences, convert bytes to a hex string. No async — use `_sync` on the file read (depends on file-io concern but can stub the call).

  **Must NOT do**:
  - Introduce `ByteString`, `Buffer`, or `BytesView` as named types in either alternative (that would be a 3rd option; cap is 2 per the target table).
  - Use `[uint8]` syntax.

  **Recommended Agent Profile**:
  - **Category**: `writing`.

  **Parallelization**: Wave 2. Blocks 14 (fs), 15 (network), 17 (crypto), 18 (compression), 23 (testing — mocks of byte streams). Blocked by: 1, 3, 4.

  **References**:
  - `/home/justi/Projects/opalescent/language-spec/requirements/overview.md` — primitive types
  - `/home/justi/Projects/opalescent/src/stdlib/` — current Rust-side byte handling

  **Acceptance Criteria**:
  - [ ] 2 alternative folders
  - [ ] Each alternative demonstrates the 3 required operations
  - [ ] `read_file_sync` (or equivalent) used for the file-read example
  - [ ] No `[uint8]` syntax anywhere
  - [ ] Style-gate passes

  **QA Scenarios**:

  ```
  Scenario: Both alternatives cover the 3 required operations
    Tool: Bash
    Steps:
      1. grep -l -E '(read_file_sync|concatenate|hex)' stdlib-proposals/byte-buffer-type/*/*.op
      2. Assert: at least one file per alt matches each pattern
    Evidence: .sisyphus/evidence/task-8-ops.txt

  Scenario: Byte arrays use uint8[] not [uint8]
    Tool: Bash
    Steps:
      1. grep -rEn '\[uint8\]' stdlib-proposals/byte-buffer-type/
      2. Assert: no matches
      3. grep -rEn 'uint8\[\]' stdlib-proposals/byte-buffer-type/ | wc -l
      4. Assert: ≥ 4
    Evidence: .sisyphus/evidence/task-8-syntax.txt

  Scenario: File I/O function carries _sync suffix
    Tool: Bash
    Steps:
      1. grep -rEn '\bread_file\b' stdlib-proposals/byte-buffer-type/ | grep -v _sync || true
      2. Assert: no matches (all read_file calls are _sync)
    Evidence: .sisyphus/evidence/task-8-sync.txt
  ```

  **Commit**: YES (Wave 2 commit groups Tasks 5–8)
  - Message: `docs(stdlib-proposals): author foundational concerns (error, modules, optional, bytes)`
  - Files: `stdlib-proposals/error-strategy/**`, `stdlib-proposals/module-organization/**`, `stdlib-proposals/optional-representation/**`, `stdlib-proposals/byte-buffer-type/**`
  - Pre-commit: `bash stdlib-proposals/.style-gate.sh`

---

- [x] 9. Concern: `collections-api-shape` (4 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/collections-api-shape/` with 4 alts:
    1. `free-function-api` — every collection operation is a free function taking the collection as first arg (`map_over_array(arr, f)`, `filter_array(arr, pred)`).
    2. `method-style-api` — collections are types with methods (`my_array.map(f)`, `my_array.filter(pred)`) — requires introducing method-call syntax if not already present; must verify against `language-spec/requirements/` first and, if methods aren't supported today, mark alternative as requiring grammar extension in "Impact on Existing Syntax".
    3. `pipeline-operator-api` — use a pipe-like operator `|>` if present, else propose a named `pipe_into` helper: `pipe_into(my_array, [map_over_array(_, double), filter_array(_, is_positive)])`.
    4. `module-per-collection` — `array`, `map`, `set`, `list` each get their own bare-specifier module with identical operation names.
  - Every alt demonstrates: map, filter, reduce, find-first-matching (absence handling — cross-references optional-representation concern), concatenate, slice.
  - Absence-handling example must use whatever optional-representation alternative the proposal "Assumes" section names.

  **Must NOT do**: introduce mutability semantics not already in the language; propose iterator protocols unless grammar supports them.

  **Category**: `writing`. **Blocks**: 24, 25, 26. **Blocked by**: 1, 3, 4, 7 (optional-representation — soft).

  **References**: `language-spec/array_helpers.op`, `language-spec/partition.op`, `language-spec/requirements/overview.md`.

  **Concern-specific QA**:

  ```
  Scenario: Each alternative demonstrates all 6 required operations
    Tool: Bash
    Steps:
      1. For each alt: grep -l -E '(map_over|filter|reduce|find_first|concatenate|slice)' *.op
      2. Assert: every op name appears in at least one .op file in each alt
    Evidence: .sisyphus/evidence/task-9-ops.txt

  Scenario: Absence handling references optional-representation choice
    Tool: Bash
    Steps:
      1. For each alt proposal.md: grep -A3 '^## Assumes' | grep -q 'optional-representation'
      2. Assert: every alt explicitly names which optional-representation alt it assumes
    Evidence: .sisyphus/evidence/task-9-optional-link.txt
  ```

  **Commit**: NO (groups with Wave 3 "core data" commit)

---

- [x] 10. Concern: `strings-text-encoding` (3 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/strings-text-encoding/` with 3 alts:
    1. `utf8-bytes-only` — strings are always UTF-8; provide `encode_string_to_utf8_bytes`, `decode_utf8_bytes_to_string` with explicit `Utf8DecodeError` handling.
    2. `multiple-encodings` — support UTF-8 + UTF-16 + ASCII explicitly; `encode_string_to_utf16_bytes`, etc.
    3. `codepoint-first` — operate on codepoint arrays (`uint32[]`) internally; stdlib provides conversion helpers.
  - Every alt demonstrates: encode string to bytes, decode bytes to string with malformed-input handling (via `guard`), string length in bytes vs codepoints, uppercase/lowercase conversion with locale caveat, substring extraction with bounds-error handling.

  **Must NOT do**: use `Result<>`; assume UTF-8 where an alt says otherwise.

  **Category**: `writing`. **Blocks**: 24, 25, 26. **Blocked by**: 1, 3, 4, 8 (byte-buffer — soft).

  **References**: `language-spec/requirements/overview.md`, `src/stdlib/strings.rs`.

  **Concern-specific QA**:

  ```
  Scenario: Every alt shows malformed-input handling via guard
    Tool: Bash
    Steps:
      1. For each alt: grep -c 'guard' *.op
      2. Assert: each alt has ≥ 2 guard usages
    Evidence: .sisyphus/evidence/task-10-guard.txt
  ```

  **Commit**: NO

---

- [x] 11. Concern: `numeric-math-surface` (3 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/numeric-math-surface/` with 3 alts:
    1. `expand-math-module` — keep the `math` bare specifier; add `greatest_common_divisor`, `least_common_multiple`, `integer_square_root`, `modular_exponentiation`, `count_leading_zeros`, `count_trailing_zeros`, `population_count`, `checked_integer_addition`, `checked_integer_multiplication`, `saturating_integer_addition`, `floating_point_is_finite`, `floating_point_is_nan`, `floating_point_next_representable`.
    2. `split-math-into-modules` — `math/integer`, `math/floating_point`, `math/bitwise` as separate bare specifiers (depends on module-organization alt).
    3. `typed-math-traits` — math functions live as type-associated (e.g., `int32.greatest_common_divisor(a, b)`); requires dispatch mechanism, note in "Impact on Existing Syntax".
  - Every function proposed must have a realistic usage example: `greatest_common_divisor` used to reduce a fraction; `checked_integer_addition` used in a sum with overflow-reject; `count_leading_zeros` used to compute bit-width of an integer; `floating_point_is_nan` used in a validation pipeline.
  - **Enumerate `errors` explicitly**: `checked_integer_addition` returns `errors IntegerOverflow`; `integer_square_root` returns `errors NegativeInputError`; document each.

  **Must NOT do**: abbreviate names — `gcd`, `clz`, `popcnt`, `ctz` are forbidden.

  **Category**: `writing`. **Blocks**: 24, 25, 26. **Blocked by**: 1, 3, 4, 6 (module-organization — soft).

  **References**: `language-spec/requirements/math.md`, `src/stdlib/math.rs`.

  **Concern-specific QA**:

  ```
  Scenario: Verbose names enforced
    Tool: Bash
    Steps:
      1. grep -rEwn '\b(gcd|lcm|clz|ctz|popcnt|isqrt|modexp)\s*\(' stdlib-proposals/numeric-math-surface/
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-11-verbose.txt

  Scenario: Overflow-bearing functions have explicit errors clause
    Tool: Bash
    Steps:
      1. For every checked_* signature: grep -A0 'checked_' *.op | grep -q 'errors IntegerOverflow'
      2. Assert: every checked_ function declares IntegerOverflow
    Evidence: .sisyphus/evidence/task-11-checked-errors.txt
  ```

  **Commit**: NO

---

- [x] 12. Concern: `random-rng` (2 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/random-rng/` with 2 alts:
    1. `explicit-rng-handle` — callers hold a `RandomNumberGenerator` value and pass it to every function (`next_random_uint32_from(rng)`, `random_integer_in_range_from(rng, low, high)`); seeding is explicit.
    2. `thread-local-default-rng` — stdlib exposes an implicit default RNG plus explicit-handle functions as escape hatch.
  - Both alts demonstrate: generate a random 32-bit unsigned integer, generate a random integer in a bounded range (with invalid-range error handling), shuffle an array, generate a UUIDv4 by composing with the uuid concern.
  - Pure in-memory; **no `_sync` suffix** (RNG is CPU-only).

  **Must NOT do**: introduce `rand()` or other abbreviations; assume cryptographic strength unless proposal explicitly says so.

  **Category**: `writing`. **Blocks**: 24, 25, 26; soft-blocks 21 (uuid). **Blocked by**: 1, 3, 4.

  **Concern-specific QA**:

  ```
  Scenario: Invalid-range error explicitly handled
    Tool: Bash
    Steps:
      1. grep -rEn 'random_integer_in_range_from' stdlib-proposals/random-rng/ --include='*.op'
      2. grep -B2 -A6 that call site in every .op file
      3. Assert: every call is wrapped in guard / propagate with a named error type (e.g., InvalidRangeError)
    Evidence: .sisyphus/evidence/task-12-range-handling.txt
  ```

  **Commit**: NO

---

- [x] 13. Concern: `time-date-api` (3 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/time-date-api/` with 3 alts:
    1. `monotonic-and-wall-clock-split` — separate types `MonotonicInstant` and `WallClockDateTime`; `now_monotonic_sync()`, `now_wall_clock_sync()` (both I/O-bearing → `_sync`).
    2. `single-timestamp-type` — unified `Timestamp` (nanoseconds since epoch) with conversion helpers to/from calendar breakdowns.
    3. `calendar-first` — primary type is `CalendarDateTime` with year/month/day/hour/minute/second/nanosecond fields; duration arithmetic returns a `Duration` type.
  - Every alt demonstrates: read the current time, compute elapsed time between two instants, format a timestamp as ISO-8601, parse an ISO-8601 string with malformed-input handling, sleep for a duration (`sleep_sync`).
  - `_sync` suffix REQUIRED on: `now_monotonic_sync`, `now_wall_clock_sync`, `sleep_sync`, any function that consults the OS clock or blocks.
  - Pure conversions (`format_timestamp_as_iso8601`, `parse_iso8601_string`, `duration_from_seconds`) do NOT get `_sync`.

  **Must NOT do**: propose timezone database bundling unless alt explicitly addresses it; abbreviate (`now`, `ts`, `dt` are all forbidden as function names).

  **Category**: `writing`. **Blocks**: 24, 25, 26. **Blocked by**: 1, 3, 4.

  **Concern-specific QA**:

  ```
  Scenario: OS-clock and sleep functions are _sync suffixed
    Tool: Bash
    Steps:
      1. grep -rEn '\b(now_monotonic|now_wall_clock|sleep)\b' stdlib-proposals/time-date-api/ --include='*.op' | grep -v '_sync'
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-13-sync.txt

  Scenario: Parse function uses guard for malformed input
    Tool: Bash
    Steps:
      1. For every parse_iso8601_string call: grep -B1 -A3 and assert guard or propagate present
    Evidence: .sisyphus/evidence/task-13-parse.txt
  ```

  **Commit**: NO

---

- [x] 14. Concern: `file-io-surface` (3 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/file-io-surface/` with 3 alts:
    1. `whole-file-operations` — only `read_file_to_bytes_sync`, `read_file_to_string_sync`, `write_bytes_to_file_sync`, `write_string_to_file_sync`, `append_bytes_to_file_sync`, `delete_file_sync`, `file_exists_sync`, `create_directory_sync`, `list_directory_entries_sync`. No streaming, no file handles. User noted this concern will be lean without async — lean is correct.
    2. `handle-based` — `open_file_sync(path, mode)` returns a `FileHandle`; then `read_from_file_handle_sync`, `write_to_file_handle_sync`, `close_file_handle_sync`. Still no streaming semantics beyond sequential reads.
    3. `path-object-centric` — introduce a `FilesystemPath` type with methods (or free functions) like `read_contents_sync(path)`, `write_contents_sync(path, bytes)`; emphasizes path manipulation (`join_path_components`, `path_parent_directory`, `path_file_name`, `path_file_extension`) which are CPU-only (no `_sync`).
  - Every function that touches the filesystem gets `_sync`. Every path-manipulation helper does NOT.
  - `.op` examples must include: reading a config file on startup with `guard` for `FileNotFound`, `PermissionDenied`, `ReadFailure`; atomically writing a log entry (write to temp + rename, each step error-handled); listing directory entries and filtering by extension.
  - **Errors** must be enumerated exhaustively. Define in `file-io-surface/<alt>/filesystem_errors.types.op`: `FileNotFoundError`, `PermissionDeniedError`, `FileAlreadyExistsError`, `ReadFailureError`, `WriteFailureError`, `InvalidPathError`, `FilesystemFullError`, `IsADirectoryError`, `IsNotADirectoryError`.

  **Must NOT do**: propose streaming/chunked reads (that is an async-shaped surface, deferred); introduce iterators unless grammar supports them; skip any error type in signatures.

  **Category**: `writing`. **Blocks**: 24, 25, 26; soft-blocks 18 (compression), 23 (testing — uses fs mocks). **Blocked by**: 1, 3, 4, 5 (error-strategy), 8 (byte-buffer).

  **Concern-specific QA**:

  ```
  Scenario: Every filesystem-touching function is _sync
    Tool: Bash
    Steps:
      1. The style-gate enforces this for the file-io-surface concern; run it and capture output
      2. Assert: gate passes for this concern
    Evidence: .sisyphus/evidence/task-14-sync.txt

  Scenario: All 9 error types enumerated and used
    Tool: Bash
    Steps:
      1. For each alt: grep -l 'FileNotFoundError' *.types.op && grep -l 'PermissionDeniedError' *.types.op && ...  (all 9)
      2. Assert: every error type appears in at least one types file per alt
      3. grep -rc 'guard.*else' stdlib-proposals/file-io-surface/ --include='*.op'
      4. Assert: ≥ 3 guard usages per alt
    Evidence: .sisyphus/evidence/task-14-errors.txt

  Scenario: Atomic-write scenario present and correctly handled
    Tool: Bash
    Steps:
      1. grep -rln 'atomic' stdlib-proposals/file-io-surface/
      2. For each match: verify write-temp + rename pattern with error handling on both steps
    Evidence: .sisyphus/evidence/task-14-atomic.txt
  ```

  **Commit**: NO

---

- [x] 15. Concern: `network-http-layer` (3 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/network-http-layer/` with 3 alts:
    1. `minimal-http-client-sync` — only `http_get_sync`, `http_post_sync`, `http_request_sync(HttpRequest)` returning `HttpResponse`. No server. No streaming. Very lean.
    2. `request-builder-sync` — introduce `HttpRequestBuilder` value type; chain via `.with_header`, `.with_body`, terminated by `.send_sync()`. Still purely synchronous.
    3. `separate-client-type-sync` — an `HttpClient` value holds config (base URL, default headers, timeout); `http_client.execute_sync(request)` returns response.
  - Every alt demonstrates: GET a JSON endpoint and parse the body (cross-references serialization concern), POST form data, handle timeout error, handle non-2xx status code as an error, set a custom User-Agent header.
  - `_sync` suffix REQUIRED on every network-touching call.
  - **Errors** enumerated: `NetworkUnreachableError`, `ConnectionRefusedError`, `ConnectionTimeoutError`, `TlsHandshakeError`, `HttpProtocolError`, `UnexpectedHttpStatusError` (with status code field), `ResponseBodyReadError`, `RequestBodyWriteError`, `MalformedUrlError`.

  **Must NOT do**: propose server-side APIs (out of scope); propose WebSocket or SSE (both async-shaped); propose streaming request/response bodies.

  **Category**: `writing`. **Blocks**: 24, 25, 26. **Blocked by**: 1, 3, 4, 5, 8, 16 (serialization — soft).

  **Concern-specific QA**:

  ```
  Scenario: All 9 network error types enumerated
    Tool: Bash
    Steps:
      1. For each alt: verify every listed error type appears in at least one .types.op or import
    Evidence: .sisyphus/evidence/task-15-errors.txt

  Scenario: Timeout and non-2xx scenarios explicitly demonstrated
    Tool: Bash
    Steps:
      1. grep -rln 'ConnectionTimeoutError' stdlib-proposals/network-http-layer/ --include='*.op'
      2. grep -rln 'UnexpectedHttpStatusError' stdlib-proposals/network-http-layer/ --include='*.op'
      3. Assert: both error types used in real call sites (with guard) in every alt
    Evidence: .sisyphus/evidence/task-15-scenarios.txt
  ```

  **Commit**: NO

---

- [x] 16. Concern: `serialization` (4 alternatives)

  **What to do**:
  - Folder `stdlib-proposals/serialization/` with 4 alts:
    1. `json-only-value-tree` — single `JsonValue` tagged union (Null/Boolean/Number/String/Array/Object); `parse_json_string`, `serialize_json_value_to_string`. No streaming.
    2. `json-plus-toml-uniform-api` — both formats expose identical shape: `parse_<format>_string`, `serialize_to_<format>_string`, both producing/consuming a generic `StructuredValue`.
    3. `typed-derive-style` — proposal document the need for a future `derive Serialize/Deserialize` annotation; for now, provide hand-written `*_to_json_value` / `*_from_json_value` per user type. "Impact on Existing Syntax" notes a derive macro extension is a prerequisite for full ergonomics.
    4. `streaming-sync-readers-writers` — `JsonStreamReader`, `JsonStreamWriter` with `read_next_token_sync`, `write_value_sync` for memory-bounded processing. Streaming functions get `_sync`.
  - Every alt demonstrates: parse a config JSON, emit a structured log line, round-trip a record through serialize+parse, handle malformed input with exhaustive error enumeration.
  - **Errors**: `MalformedJsonError`, `MalformedTomlError`, `UnexpectedJsonShapeError`, `NumericRangeError`, `MissingRequiredFieldError`, `UnknownFieldError`.

  **Must NOT do**: propose YAML, CBOR, MsgPack (not in user's list); invent derive syntax — only *reference* it as prerequisite in alt 3.

  **Category**: `writing`. **Blocks**: 24, 25, 26; soft-blocks 15 (network). **Blocked by**: 1, 3, 4, 5, 7 (optional — soft), 8.

  **Concern-specific QA**:

  ```
  Scenario: Streaming alt uses _sync suffix
    Tool: Bash
    Steps:
      1. grep -rEn '\b(read_next_token|write_value)\b' stdlib-proposals/serialization/streaming-sync-readers-writers/ --include='*.op' | grep -v '_sync'
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-16-sync.txt

  Scenario: Round-trip example present in every alt
    Tool: Bash
    Steps:
      1. For each alt: grep -l 'round_trip\|roundtrip\|parse.*serialize\|serialize.*parse' *.op
      2. Assert: every alt has ≥ 1 matching file
    Evidence: .sisyphus/evidence/task-16-roundtrip.txt
  ```

  **Commit**: NO (groups with Wave 3 commits — split by cluster; see Commit Strategy)

- [x] 17. Author `crypto-hashing/` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/crypto-hashing/` with `COMPARISON.md` + 3 alternative folders:
    - `hash-functions-only/` — SHA-256, SHA-512, BLAKE3 as pure functions over `byte[]`; HMAC as separate `hmac_sign(key, message, algorithm)`; constant-time compare as `constant_time_equals(a, b)`. One-shot only.
    - `hasher-object-streaming/` — `Hasher` type with `new_hasher`, `update(chunk)`, `finalize()`; streaming API; `finalize_sync` suffix when backed by finalizing a file-fed stream, but CPU-only `update`/`finalize` do NOT carry `_sync`.
    - `typed-digest-wrappers/` — strongly-typed wrappers (`Sha256Digest`, `Sha512Digest`, `Blake3Digest`, `HmacSignature`) with `to_hex()`, `to_base64()`; prevents mixing digest types at call sites.
  - Every alt: 3 `.op` files — `hashes.op` (one-shot), `streaming.op` (streaming/Hasher), `hmac_and_compare.op` (HMAC + constant-time).
  - Every example MUST use `constant_time_equals` for signature verification, never raw `==`.
  - Bulk CPU hashing (`sha256_of_bytes`) is pure: no `_sync` suffix. Only functions that could plausibly gain an async streaming counterpart (e.g., `hash_file_sync` if ever added as a convenience) get suffixed.
  - Full-usage rule + exhaustive-errors rule + guard-handling rule all apply.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — security-correctness sensitivity
  - **Skills**: `[]` — no specialized skills apply
  - **Skills Evaluated but Omitted**: `frontend-ui-ux` (no UI)

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4; Blocks: Task 24 (README), Task 25 (audit).

  **References**:
  - Pattern: `language-spec/error_handling_samples.op` — guard/propagate idioms for fallible verify
  - Pattern: `stdlib-proposals/byte-buffer-type/*/proposal.md` — byte representation decided in Task 8 (use it)
  - API: `language-spec/requirements/overview.md` — no mutation of inputs
  - External: `https://doc.rust-lang.org/std/primitive.slice.html#method.eq` (constant-time comparison context; implement in Opalescent style)

  **Acceptance Criteria + QA Scenarios**: Apply Shared QA Scenario Template. Additionally:
  ```
  Scenario: Constant-time compare used in verify examples
    Tool: Bash
    Steps:
      1. grep -rE '==.*hmac|hmac.*==' stdlib-proposals/crypto-hashing/*/*.op
      2. Assert: no matches (raw == on HMAC output is forbidden)
      3. grep -rE 'constant_time_equals' stdlib-proposals/crypto-hashing/*/*.op
      4. Assert: ≥ 3 matches (every alt demonstrates it)
    Evidence: .sisyphus/evidence/task-17-ct-compare.txt

  Scenario: Digest errors exhaustively listed
    Tool: Bash
    Steps:
      1. grep -nE 'errors\s+[A-Za-z]' stdlib-proposals/crypto-hashing/*/*.op
      2. Assert: each fallible fn lists ≥ 1 error type; no ellipsis
    Evidence: .sisyphus/evidence/task-17-errors.txt
  ```

  **Commit**: NO (Wave 3 dev-experience cluster).

- [x] 18. Author `compression/` (2 alternatives)

  **What to do**:
  - Create `stdlib-proposals/compression/` with `COMPARISON.md` + 2 alternative folders:
    - `codec-functions/` — `gzip_compress(bytes) / gzip_decompress(bytes)`, `deflate_compress`, `zstd_compress`; one-shot functions taking full buffers. No `_sync` on pure CPU one-shots UNLESS the alt also offers a `compress_file_sync(source_path, dest_path)` convenience — that variant gets `_sync`.
    - `stream-codec-objects/` — `Compressor`/`Decompressor` objects with `write_chunk(bytes)` + `finish()`; `finish_sync` ONLY if it flushes a file-backed sink; pure in-memory `finish` stays unsuffixed.
  - Every alt: 2 `.op` files — `one_shot.op`, `streaming.op`. Show round-trip example (compress then decompress → assert equality).
  - Enumerate errors: `InvalidDataError`, `UnsupportedAlgorithmError`, `TruncatedInputError`, `IoError` (streaming).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4, 8 (byte-buffer-type); Blocks: 24, 25.

  **References**:
  - Pattern: Task 17 streaming alt (same object-lifecycle shape)
  - API: Task 8 byte-buffer decision
  - External: `https://www.rfc-editor.org/rfc/rfc1952` (gzip), `https://datatracker.ietf.org/doc/html/rfc8478` (zstd) — reference only, do not re-explain the spec in proposals

  **Acceptance Criteria + QA Scenarios**: Shared template + round-trip demonstration scenario:
  ```
  Scenario: Round-trip example present
    Tool: Bash
    Steps:
      1. grep -l 'compress.*decompress\|decompress.*compress' stdlib-proposals/compression/*/*.op
      2. Assert: ≥ 1 matching file per alt (2 alts → ≥ 2 files matched total; verify one match per alt folder)
    Evidence: .sisyphus/evidence/task-18-roundtrip.txt
  ```

  **Commit**: NO (Wave 3 platform cluster).

- [x] 19. Author `logging/` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/logging/` with `COMPARISON.md` + 3 alternative folders:
    - `structured-key-value/` — `log_info(message, fields: (string, any)[])`, `log_warn`, `log_error`, `log_debug`; fields as key-value pairs; destructuring via pattern match.
    - `printf-style-with-levels/` — `log(level: LogLevel, format: string, args: any[])`; interpolation-style; level filter via global config.
    - `tracing-spans-and-events/` — `span_start(name)` returns `Span`, `span_end(span)`, `event_in_span(span, message)`; nested spans; elapsed-time capture; sink interface for pluggable backends.
  - Every alt: 3 `.op` files — `basic_usage.op`, `with_context.op` (request ID / user ID threaded through), `flush_and_sinks.op` (sink registration + flush).
  - `flush_sync()` carries suffix (may block on stderr/file/network sink). `log_info/log_warn/...` themselves do NOT get `_sync` — they enqueue to an in-memory buffer.
  - Alts must NOT introduce thread-local state (no concurrency primitives yet) — use explicit context parameters passed into log calls.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4; Blocks: 24, 25.

  **References**:
  - Pattern: `language-spec/types_example.types.op` — sum types for `LogLevel`
  - Pattern: Task 5 (error-strategy) — how sinks report fallible flush
  - External: `https://docs.rs/tracing/latest/tracing/` (spans concept), but translate to Opalescent conventions — no macros, no attributes

  **Acceptance Criteria + QA Scenarios**: Shared template + flush-suffix check:
  ```
  Scenario: flush_sync suffix enforced
    Tool: Bash
    Steps:
      1. grep -nE 'let\s+flush[^_]' stdlib-proposals/logging/*/*.op
      2. Assert: no matches (all flush fns are flush_sync)
      3. grep -nE 'flush_sync' stdlib-proposals/logging/*/*.op
      4. Assert: ≥ 3 matches (every alt demonstrates it)
    Evidence: .sisyphus/evidence/task-19-flush.txt

  Scenario: No thread-local/concurrency constructs
    Tool: Bash
    Steps:
      1. grep -rnE 'thread_local|Mutex|RwLock|atomic|channel' stdlib-proposals/logging/
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-19-no-concurrency.txt
  ```

  **Commit**: NO (Wave 3 dev-experience cluster).

- [x] 20. Author `regex/` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/regex/` with `COMPARISON.md` + 3 alternative folders:
    - `compile-once-match-many/` — `let pattern = compile_pattern(source)` returns `Pattern`; `pattern.find(haystack)`, `pattern.find_all(haystack)`, `pattern.replace(haystack, replacement)`. Compilation is fallible (`RegexSyntaxError`).
    - `string-method-convenience/` — `string.matches(pattern_source)`, `string.replace_matching(pattern_source, replacement)`; compiles on every call; simpler, slower.
    - `typed-capture-groups/` — `Pattern<Captures>` generic; named captures returned as a typed product type; avoids stringly-typed group access.
  - Every alt: 3 `.op` files — `find_and_match.op`, `replace_and_split.op`, `capture_groups.op`.
  - All regex operations are CPU-only → NO `_sync` suffix anywhere.
  - Errors: `RegexSyntaxError`, `UnsupportedFeatureError` (e.g., backrefs if not supported).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4, 10 (strings); Blocks: 24, 25.

  **References**:
  - Pattern: Task 10 strings-text-encoding (string representation)
  - External: `https://docs.rs/regex/latest/regex/` — but translate to Opalescent method style, not Rust type chains

  **Acceptance Criteria + QA Scenarios**: Shared template + no-sync-suffix check:
  ```
  Scenario: No _sync suffix in regex (CPU-only)
    Tool: Bash
    Steps:
      1. grep -nE 'let\s+\w+_sync\s*=' stdlib-proposals/regex/*/*.op
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-20-no-sync.txt
  ```

  **Commit**: NO (Wave 3 dev-experience cluster).

- [x] 21. Author `uuid/` (2 alternatives)

  **What to do**:
  - Create `stdlib-proposals/uuid/` with `COMPARISON.md` + 2 alternative folders:
    - `v4-and-v7-separate/` — `generate_uuid_v4(rng: Rng)`, `generate_uuid_v7(rng: Rng, timestamp_ms: int64)`; two explicit functions; caller chooses version.
    - `typed-uuid-wrappers/` — `UuidV4` and `UuidV7` as distinct product types; `to_string()`, `to_bytes()`, `parse_uuid(string)` (returns sum type `UuidV4 | UuidV7`).
  - Composes with `random-rng/` concern (Task 12): the chosen RNG alt's `Rng` type threads through. Reference it explicitly in the comparison doc.
  - Every alt: 2 `.op` files — `generate.op` (both versions), `parse_and_display.op` (round-trip parse/to_string).
  - No `_sync` — generation is pure given an RNG + timestamp.
  - Errors: `UuidParseError`.

  **Recommended Agent Profile**:
  - **Category**: `quick` — small, well-scoped concern
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4, 12 (random-rng), 13 (time-date); Blocks: 24, 25.

  **References**:
  - Pattern: Task 12 random-rng — `Rng` type usage
  - Pattern: Task 13 time-date — `timestamp_ms` representation
  - External: `https://datatracker.ietf.org/doc/html/rfc9562` (UUID v7)

  **Acceptance Criteria + QA Scenarios**: Shared template + RNG-composition check:
  ```
  Scenario: Examples use Rng from random-rng concern
    Tool: Bash
    Steps:
      1. grep -rnE 'Rng|new_rng|seed' stdlib-proposals/uuid/*/*.op
      2. Assert: ≥ 2 matches per alt (generation requires RNG)
    Evidence: .sisyphus/evidence/task-21-rng-compose.txt
  ```

  **Commit**: NO (Wave 3 dev-experience cluster).

- [x] 22. Author `subprocess-exec/` (3 alternatives)

  **What to do**:
  - Create `stdlib-proposals/subprocess-exec/` with `COMPARISON.md` + 3 alternative folders:
    - `run-and-wait/` — `run_command_sync(program, args, options): CommandOutput errors SpawnError, TimeoutError, IoError`; fully blocking; captures stdout/stderr/exit-code into a product type.
    - `spawn-handle-with-methods/` — `spawn_command_sync(program, args, options): Process errors SpawnError`; `process.wait_sync()`, `process.kill_sync(signal)`, `process.stdin_write_sync(bytes)`, `process.stdout_read_sync()`; lifecycle-object style.
    - `pipeline-composition/` — `pipe_sync(commands: Command[])` composes multiple commands with stdout→stdin chaining; returns final output.
  - ALL operations that touch OS get `_sync` (spawn, wait, kill, stdin_write, stdout_read, pipe).
  - Every alt: 3 `.op` files — `basic_run.op`, `with_timeout_and_env.op` (env vars + timeout), `capture_and_stream.op` (output capture + incremental reading).
  - Enumerate errors: `SpawnError`, `TimeoutError`, `IoError`, `InvalidArgumentError`, `ProcessKilledError`, `PipelineError` (pipeline alt).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — OS-interaction correctness (PATH, signals, zombies)
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4, 8 (byte-buffer); Blocks: 24, 25.

  **References**:
  - Pattern: Task 14 file-io-surface (handle-object vs function-first tradeoff)
  - External: `https://doc.rust-lang.org/std/process/struct.Command.html` — for capability checklist only, not style

  **Acceptance Criteria + QA Scenarios**: Shared template + universal _sync check:
  ```
  Scenario: Every process-touching fn ends in _sync
    Tool: Bash
    Steps:
      1. grep -nE 'let\s+(run|spawn|wait|kill|stdin_write|stdout_read|pipe)[^_]' stdlib-proposals/subprocess-exec/*/*.op
      2. Assert: no matches (every process fn is _sync)
    Evidence: .sisyphus/evidence/task-22-sync.txt

  Scenario: Timeout demonstrated
    Tool: Bash
    Steps:
      1. grep -rnE 'TimeoutError|timeout_ms|timeout:' stdlib-proposals/subprocess-exec/*/*.op
      2. Assert: ≥ 3 matches (every alt shows timeout handling)
    Evidence: .sisyphus/evidence/task-22-timeout.txt
  ```

  **Commit**: NO (Wave 3 dev-experience cluster).

- [x] 23. Author `testing-framework/` (5 alternatives) — Vitest-inspired, FULL mocking/stubbing/spying

  **What to do**:
  - Create `stdlib-proposals/testing-framework/` with `COMPARISON.md` + 5 alternative folders:
    - `describe-it-assertions-functional/` — `describe(name, body_fn)`, `it(name, body_fn)`, `expect(actual).to_equal(expected)`; nested suites; lifecycle hooks `before_each`, `after_each`, `before_all`, `after_all`; assertions as method chains on `Expectation<T>` type.
    - `test-table-driven/` — `test_case(name, inputs, expected, assertion_fn)`; table-driven style; every case is a product-type row; good for parameterized tests.
    - `mock-first-dependency-injection/` — Every dependency explicitly passed; `create_mock<T>(default_impl): Mock<T>` returns an object with `.set_return(fn_name, value)`, `.set_error(fn_name, error)`, `.call_count(fn_name)`, `.called_with(fn_name, args)`; mocks are values, not globals.
    - `spy-and-stub-wrappers/` — `spy_on(object, method_name): Spy` wraps an existing method recording calls while preserving original behavior; `stub(object, method_name, replacement_fn)` swaps implementation; both return handles with `.restore()` for cleanup in `after_each`.
    - `property-based-generative/` — `property(name, generators, body_fn)`; `generate_int32(min, max)`, `generate_string(length_range)`, `generate_array(element_gen, length_range)`; shrinking on failure; seed captured in evidence.
  - **Mocking/stubbing/spying coverage (MUST appear somewhere in the set of alternatives, not just one)**:
    - Mock entire dependency object — alt 3
    - Spy on a real method (record calls, preserve impl) — alt 4
    - Stub one method (replace impl) — alt 4
    - Assert call count / call args — alts 3 and 4
    - Restore cleanup — alt 4
    - Generative input — alt 5
  - Every alt: 4 `.op` files — `basic.op` (smallest workable test), `with_lifecycle.op` (before/after hooks or equivalent setup), `mocks_stubs_spies.op` (the feature each alt emphasizes), `failure_and_skip.op` (how failures are reported + skipping a test).
  - No `_sync` — test runner sees I/O through user code; the framework itself is CPU-only dispatch.
  - Must fit Opalescent mechanisms: no reflection, no decorators, no macros, no attributes. Alts that need dispatch-by-name must register via explicit function-value arguments (no stringly-typed magic).
  - COMPARISON.md MUST include a "Mocking capability matrix" with rows [full-mock, spy, stub, call-count, call-args, restore, generative] and columns [alt1..alt5] marking ✓/✗ per alt.
  - Errors: `AssertionFailedError`, `TestSkippedError`, `LifecycleHookError`, `GenerationExhaustedError` (property alt).

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high` — nuanced framework design; requires careful API taste
  - **Skills**: `[]`

  **Parallelization**: Wave 3; Blocked By: Tasks 1–4, 5 (error-strategy), 6 (module-org); Blocks: 24 (README), 25 (audit). **This task carries the Wave 3 final commit** (see Commit Strategy).

  **References**:
  - Pattern: `language-spec/error_handling_samples.op` — how assertion failures surface as errors, not panics
  - Pattern: Task 5 (error-strategy) — how test errors roll up
  - External: `https://vitest.dev/api/` — API surface inspiration, translate idioms to Opalescent (no chained arrow-fns, no decorators, no `vi.mock` globals)
  - External: `https://hypothesis.readthedocs.io/` (property-based generation concept)

  **Acceptance Criteria + QA Scenarios**: Shared template + full mocking-coverage scenarios:
  ```
  Scenario: Mocking capability matrix present
    Tool: Bash
    Steps:
      1. grep -E 'full-mock|spy|stub|call-count|call-args|restore|generative' stdlib-proposals/testing-framework/COMPARISON.md
      2. Assert: ≥ 7 matches (row labels present)
    Evidence: .sisyphus/evidence/task-23-matrix.txt

  Scenario: Every alt demonstrates assertions
    Tool: Bash
    Steps:
      1. grep -l 'expect\|assert_equal\|assert_that\|to_equal' stdlib-proposals/testing-framework/*/basic.op
      2. Assert: 5 matches (one per alt)
    Evidence: .sisyphus/evidence/task-23-assertions.txt

  Scenario: Mock/spy/stub demonstrated where promised
    Tool: Bash
    Steps:
      1. grep -l 'create_mock\|set_return\|call_count' stdlib-proposals/testing-framework/mock-first-dependency-injection/*.op
      2. Assert: ≥ 1 match
      3. grep -l 'spy_on\|stub\|\.restore' stdlib-proposals/testing-framework/spy-and-stub-wrappers/*.op
      4. Assert: ≥ 1 match
    Evidence: .sisyphus/evidence/task-23-msr.txt

  Scenario: No reflection/decorator/macro leakage
    Tool: Bash
    Steps:
      1. grep -rnE '@[a-z]+\(|#\[.*\]|reflect|__proto__|Object\.keys' stdlib-proposals/testing-framework/
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-23-no-magic.txt

  Scenario: No _sync suffix in framework core
    Tool: Bash
    Steps:
      1. grep -nE 'let\s+\w+_sync\s*=' stdlib-proposals/testing-framework/*/*.op
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-23-no-sync.txt
  ```

  **Commit**: YES — closes Wave 3. Message:
  ```
  docs(stdlib-proposals): add waves 1-3 stdlib design proposals (19 concerns, 58 alternatives)

  Delivers full stdlib proposal set: error-strategy, module-organization,
  optional-representation, byte-buffer-type, collections-api-shape,
  strings-text-encoding, numeric-math-surface, random-rng, time-date-api,
  file-io-surface, network-http-layer, serialization, crypto-hashing,
  compression, logging, regex, uuid, subprocess-exec, testing-framework.

  Each concern contains COMPARISON.md and 2-5 alternatives, each with
  proposal.md + authentic .op examples demonstrating full real-world
  usage with exhaustive error clauses and guard/propagate handling.

  Async surface deferred; every fn with plausible future async counterpart
  carries _sync suffix.
  ```
  Files: `stdlib-proposals/**`
  Pre-commit: `bash stdlib-proposals/.style-gate.sh && python3 stdlib-proposals/.coverage-check.py`

- [x] 24. Author top-level `stdlib-proposals/README.md` — index + tier recommendations

  **What to do**:
  - Create `stdlib-proposals/README.md` with:
    - 2-paragraph intro: what this folder is, how to navigate, link to `memory-model-proposals/` for structural precedent.
    - Table of all 19 concerns: `| Concern | Alternatives | Complexity | Status |` with a link from concern name to its folder.
    - "How to read a proposal" section: points to the 10-section template (Task 1).
    - "Recommended tiers" section: groups concerns into **Tier 1 — critical foundations** (error-strategy, module-organization, optional-representation, byte-buffer-type, collections-api-shape, strings-text-encoding), **Tier 2 — platform** (file-io, network-http, serialization, time-date, crypto, compression, subprocess-exec), **Tier 3 — developer experience** (numeric-math, random-rng, logging, regex, uuid, testing-framework). Note which tier must be decided before stdlib authoring begins.
    - Cross-concern dependency diagram (ASCII or textual): `collections → strings → regex`, `random-rng → uuid`, `byte-buffer → serialization/crypto/compression/net`, `error-strategy → everything`.
    - Explicit disclaimer: **"Async surface is intentionally deferred. Every function that will plausibly gain an async counterpart carries `_sync` in this proposal set."**
  - Must NOT recommend a specific alternative within any concern — that is the user's decision. Tiers are about ordering, not choice.
  - ≤ 300 lines; index-only (no deep design prose).

  **Recommended Agent Profile**:
  - **Category**: `writing`
  - **Skills**: `[]`

  **Parallelization**: Wave 4; Blocked By: Tasks 5–23 (all concerns authored); Blocks: 25, 26, F1–F5.

  **References**:
  - Pattern: `memory-model-proposals/COMPARISON.md` (root-level concern index + comparison style — there is no README.md at that root; mirror the COMPARISON.md's high-level framing and extend with a concern-by-concern index table as described above)
  - Pattern: This plan's Execution Strategy table (for tier groupings)

  **Acceptance Criteria + QA Scenarios**:
  ```
  Scenario: README links resolve to every concern
    Tool: Bash
    Preconditions: stdlib-proposals/README.md authored with markdown links `[text](./concern-name/)` for every concern folder.
    Steps:
      1. Extract concern folder names from markdown link targets (strip parens, leading `./`, trailing `/`):
         grep -oE '\]\(\./[a-z-]+/\)' stdlib-proposals/README.md \
           | sed -E 's|^\]\(\./||; s|/\)$||' \
           | sort -u > /tmp/task24-linked-concerns.txt
      2. Count unique concern links:
         test "$(wc -l < /tmp/task24-linked-concerns.txt)" -eq 19
      3. For each concern name in /tmp/task24-linked-concerns.txt: verify the directory exists:
         while read concern; do test -d "stdlib-proposals/${concern}" || { echo "MISSING: ${concern}"; exit 1; }; done < /tmp/task24-linked-concerns.txt
      4. Reverse check: every alt folder sibling-list in stdlib-proposals/ (excluding hidden files and README.md) appears in the linked set:
         ls -1 stdlib-proposals/ | grep -vE '^(\.|README\.md$)' | sort -u > /tmp/task24-actual-concerns.txt
         diff /tmp/task24-linked-concerns.txt /tmp/task24-actual-concerns.txt
      5. Assert: diff is empty (links ↔ folders are 1:1).
    Expected Result: exit 0, all 19 concerns both linked and present on disk.
    Evidence: .sisyphus/evidence/task-24-links.txt

  Scenario: No alternative is "recommended" over others
    Tool: Bash
    Steps:
      1. grep -nE 'recommended alternative|prefer|our choice|we recommend' stdlib-proposals/README.md
      2. Assert: no matches
    Evidence: .sisyphus/evidence/task-24-neutral.txt

  Scenario: Async-deferral disclaimer present
    Tool: Bash
    Steps:
      1. grep -n 'async' stdlib-proposals/README.md
      2. Assert: ≥ 1 match mentioning deferral
      3. grep -n '_sync' stdlib-proposals/README.md
      4. Assert: ≥ 1 match explaining the suffix convention
    Evidence: .sisyphus/evidence/task-24-async-note.txt
  ```

  **Commit**: NO (groups with Task 26 final commit).

- [x] 25. Cross-concern consistency audit

  **What to do**:
  - Read every `proposal.md` and `.op` file in `stdlib-proposals/`.
  - Produce `.sisyphus/evidence/task-25-audit.md` with findings in these categories:
    - **Naming consistency**: same concept (e.g., "byte buffer") referred to by the same name across concerns; flag divergences with file:line pairs.
    - **Error-type reuse**: if `IoError` is defined in Task 14 alt X, Tasks 15/18/19/22 must reference the same type (per whichever error-strategy alt is scoped); flag unreferenced duplicates.
    - **`_sync` discipline**: every async-plausible fn has it; no CPU-only fn has it. Enumerate violations.
    - **Module import style**: all examples use one of the 3 documented forms (bare `standard`, scoped `@scope/name`, relative `./rel`); no fourth style introduced.
    - **Doc-block length**: every public fn has `##…##` ≥ 30 chars. List every violation.
    - **Verbose-names enforcement**: no `gcd`, `clz`, `ctz`, `pcm`, `utf`, `enc`, `dec` abbreviations. List every hit.
    - **Full-usage coverage**: every proposed method appears in ≥ 1 call-site with realistic inputs and `#` comments.
    - **Exhaustive-errors coverage**: every fallible signature's `errors` clause lists concrete types (no `...`, no placeholders).
    - **Guard-handling coverage**: every fallible call in an example is either `guard … into … else` or `propagate`; no bare fallible call ignored.
  - Fixes must be applied in the same task (not deferred). Re-run style-gate after fixes.
  - Output a final clean audit report (zero findings) to `.sisyphus/evidence/task-25-audit-final.md`.

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: `[]`

  **Parallelization**: Wave 4; Blocked By: Tasks 5–24; Blocks: 26, F1–F5.

  **References**:
  - Pattern: Task 2 style-gate & coverage-check scripts (run both)
  - Pattern: Task 4 comparison schema (verify conformance)

  **Acceptance Criteria + QA Scenarios**:
  ```
  Scenario: Audit final report has zero findings
    Tool: Bash
    Steps:
      1. grep -cE '^- \[ \]|FINDING:|VIOLATION:' .sisyphus/evidence/task-25-audit-final.md
      2. Assert: output is 0
    Evidence: .sisyphus/evidence/task-25-audit-final.md

  Scenario: Style-gate still passes after fixes
    Tool: Bash
    Steps:
      1. bash stdlib-proposals/.style-gate.sh
      2. Assert: exit 0
      3. python3 stdlib-proposals/.coverage-check.py
      4. Assert: exit 0
    Evidence: .sisyphus/evidence/task-25-gates.txt
  ```

  **Commit**: NO (groups with Task 26 final commit).

- [x] 26. Full style-gate run + fix any remaining violations

  **What to do**:
  - Run `bash stdlib-proposals/.style-gate.sh` over the complete folder; capture output.
  - Run `python3 stdlib-proposals/.coverage-check.py` over the complete folder; capture output.
  - If any violations: fix them in-place, re-run, capture the clean run.
  - Regenerate `find stdlib-proposals -type f | sort > .sisyphus/evidence/final-structure.txt` for F3.
  - Produce commit.

  **Recommended Agent Profile**:
  - **Category**: `quick` — mechanical gate run + small fixes
  - **Skills**: `[]`

  **Parallelization**: Wave 4; Blocked By: Tasks 24, 25; Blocks: F1–F5.

  **References**:
  - Pattern: Task 2 gate scripts

  **Acceptance Criteria + QA Scenarios**:
  ```
  Scenario: Gates pass cleanly
    Tool: Bash
    Steps:
      1. bash stdlib-proposals/.style-gate.sh > .sisyphus/evidence/task-26-style.txt 2>&1
      2. Assert: exit 0
      3. python3 stdlib-proposals/.coverage-check.py > .sisyphus/evidence/task-26-coverage.txt 2>&1
      4. Assert: exit 0
    Evidence: .sisyphus/evidence/task-26-style.txt, .sisyphus/evidence/task-26-coverage.txt

  Scenario: Structure snapshot captured
    Tool: Bash
    Steps:
      1. find stdlib-proposals -type f | sort > .sisyphus/evidence/final-structure.txt
      2. wc -l .sisyphus/evidence/final-structure.txt
      3. Assert: ≥ 150 lines (19 concerns × avg 8 files ≈ 150+)
    Evidence: .sisyphus/evidence/final-structure.txt
  ```

  **Commit**: YES — closes Wave 4. Message:
  ```
  docs(stdlib-proposals): cross-concern audit and final style-gate pass

  Adds top-level README index with tier recommendations, applies
  cross-concern consistency audit fixes, and verifies all proposals
  pass automated style and coverage gates.
  ```
  Files: `stdlib-proposals/README.md`, `stdlib-proposals/**` (audit fixes), `.sisyphus/evidence/**`
  Pre-commit: `bash stdlib-proposals/.style-gate.sh && python3 stdlib-proposals/.coverage-check.py`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Then Momus high-accuracy review. Then present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read this plan end-to-end. For each "Must Have": verify the deliverable exists (read file, count files, grep content). For each "Must NOT Have": search the folder for forbidden patterns — reject with file:line if found. Verify the 19 concern folders and target alternative counts match the table. Verify the empty stubs were deleted. Check evidence files in `.sisyphus/evidence/`. Compare deliverables against plan line-by-line.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Concerns [19/19] | Alternatives total [58±] | Deleted stubs [2/2] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Style & Doc Quality Review** — `unspecified-high`
  Run `bash stdlib-proposals/.style-gate.sh`. Read every `proposal.md`: verify 10 required section headings present, ≤ 250 lines, internal links resolve. Read a random sample of 15 `.op` files: verify every public function has a ≥ 30-char `##…##` doc block, signatures use `errors` keyword where they can fail, no `Result<`, no `[T]`, no abbreviated names, no semicolons, explicit `return`. Check that verbose names are enforced (no `gcd`/`clz`/etc.).
  Output: `Style-gate [PASS/FAIL] | Template-conformance [N/N] | Sample .op clean [15/15] | Verbose-names [PASS/FAIL] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  From a clean state: `find stdlib-proposals -type f | sort` to enumerate every file. Verify `README.md` at root, `COMPARISON.md` in every concern folder, `proposal.md` + at least 2 `.op` files in every alternative. Read `README.md` and confirm it links to every concern folder. Read 5 random `COMPARISON.md` files and confirm they use the shared axis schema from Task 4 and only compare their own concern's alternatives. Read the testing-framework concern and confirm mocking/stubbing/spying coverage. Save full `find` output to `.sisyphus/evidence/final-qa/structure.txt`.
  Output: `Total files [N] | Structure complete [YES/NO] | README links [N/19] | COMPARISONs conformant [N/19] | Testing coverage [PASS/FAIL] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each of the 19 concerns: read its `COMPARISON.md` + every alternative's `proposal.md`. Verify alternative count matches the target table (±1 allowed with justification note in COMPARISON). Verify no concern introduces a non-Opalescent mechanism (no exceptions, no async in examples, no `Result<T,E>` used casually). Verify no folder contains files outside its scope (e.g., no error-strategy content in the testing-framework folder). Check git diff vs pre-plan baseline to detect contamination: any file created outside `stdlib-proposals/` is a violation unless it is `.sisyphus/evidence/…` or the deleted stub operation.
  Output: `Concerns [19/19 compliant] | Alternative counts [match/mismatch list] | Non-Opalescent mechanisms [CLEAN/N violations] | Contamination [CLEAN/N files] | VERDICT`

- [ ] F5. **Momus High-Accuracy Review** (pre-authorized by user — do not ask again)
  Invoke Momus with the plan file path. Iterate fix-and-resubmit until OKAY verdict. No maximum retry.
  Output: `Momus verdict: OKAY | Rounds: N`

- [ ] F6. **User Okay Gate**
  Present consolidated F1–F5 results. Wait for explicit user approval. Do not mark F1–F6 complete until user approves.

---

## Commit Strategy

This plan creates documentation only. Commits are grouped by wave:

- **After Wave 1**: `docs(stdlib-proposals): scaffold template, style-gate, and comparison schema`
  - Files: `stdlib-proposals/.style-gate.sh`, `stdlib-proposals/.template.md` (if kept as reference), plus stub deletion.
- **After Wave 2**: `docs(stdlib-proposals): author foundational concerns (error, modules, optional, bytes)`
- **After Wave 3** (split into 3 commits by domain cluster to keep diffs reviewable):
  - `docs(stdlib-proposals): author core data concerns (collections, strings, numeric, random)`
  - `docs(stdlib-proposals): author platform concerns (time, fs, net, serialization, crypto, compression)`
  - `docs(stdlib-proposals): author developer-experience concerns (logging, regex, uuid, subprocess, testing)`
- **After Wave 4**: `docs(stdlib-proposals): add top-level README and fix cross-concern inconsistencies`

Each commit must pass the style-gate. Pre-commit check: `bash stdlib-proposals/.style-gate.sh`.

---

## Success Criteria

### Verification Commands

```bash
# 1. Style gate passes
bash stdlib-proposals/.style-gate.sh
# Expected: exits 0, prints "All style checks passed."

# 2. Structural completeness
find stdlib-proposals -maxdepth 1 -mindepth 1 -type d | wc -l
# Expected: 19 (one per concern) + 0 stubs

# 3. No empty directories
find stdlib-proposals -type d -empty
# Expected: (no output)

# 4. Every concern has a COMPARISON.md
for d in stdlib-proposals/*/; do test -f "$d/COMPARISON.md" || echo "MISSING: $d"; done
# Expected: (no output)

# 5. Every alternative has proposal.md + ≥ 2 .op files
for d in stdlib-proposals/*/*/; do
  test -f "$d/proposal.md" || echo "MISSING proposal.md: $d"
  n=$(find "$d" -maxdepth 1 -name '*.op' | wc -l)
  test "$n" -ge 2 || echo "TOO FEW .op: $d ($n)"
done
# Expected: (no output)

# 6. No proposal.md exceeds 250 lines
find stdlib-proposals -name proposal.md -exec wc -l {} + | awk 'NF==2 && $1 > 250 {print; bad=1} END {exit bad+0}'
# Expected: exits 0

# 7. Top-level README links to every concern
for d in stdlib-proposals/*/; do
  name=$(basename "$d")
  grep -q "$name" stdlib-proposals/README.md || echo "README missing link: $name"
done
# Expected: (no output)

# 8. Empty stubs deleted
test ! -d stdlib-proposals/error-strategy/per-module-errors && test ! -d stdlib-proposals/error-strategy/unified-std-error && echo OK
# Expected: OK

# 9. Forbidden patterns absent from .op files
! grep -rEn 'Result<|Option<|Either<' stdlib-proposals --include='*.op'
! grep -rEn '\[(uint8|int8|int16|int32|int64|uint16|uint32|uint64|float32|float64|string|bool|T|U|K|V)\]' stdlib-proposals --include='*.op'
! grep -rEn '\b(gcd|clz|itoa|atoi|fmt|cfg|ctx|req|res)\s*\(' stdlib-proposals --include='*.op'
! grep -rEn ';\s*$' stdlib-proposals --include='*.op'
! grep -rEnw 'async|await|Promise|Future|spawn_async|then' stdlib-proposals --include='*.op'
# Expected: all exit 0 (no matches)

# 11. _sync suffix enforcement in I/O-bearing concerns
# For each of these folders, every function in .op files whose name suggests I/O
# (read_, write_, open_, close_, send_, recv_, connect_, listen_, spawn_, sleep_,
#  flush_, compress_stream, decompress_stream, hash_stream, serialize_to_writer,
#  parse_from_reader) must end in _sync.
for concern in file-io-surface network-http-layer subprocess-exec logging time-date-api; do
  bad=$(grep -rEn '\blet\s+(read|write|open|close|send|recv|connect|listen|spawn|sleep|flush)[a-z_]*\s*=\s*f\(' "stdlib-proposals/$concern" --include='*.op' | grep -vE '_sync\s*=\s*f\(' || true)
  test -z "$bad" || { echo "MISSING _sync suffix in $concern:"; echo "$bad"; exit 1; }
done
# Expected: exits 0

# 12. No async surface anywhere
! grep -rEnw 'async|await' stdlib-proposals
# Expected: exits 0 (no matches across any file type)

# 13. Every proposed method has a realistic call site (coverage check)
# For each alternative, extract function names from proposal.md signature blocks,
# then assert each appears as a call (name followed by `(`) in at least one .op file in that alt.
python3 stdlib-proposals/.coverage-check.py
# Expected: exits 0, prints "All N proposed methods have usage examples."
# (Script authored in Task 2 alongside the style gate.)

# 14. No placeholder errors clauses
! grep -rEn 'errors\s+\.\.\.|errors\s+/\*' stdlib-proposals --include='*.op'
! grep -rEn 'errors\s+[A-Za-z_, ]+,\s*\.\.\.' stdlib-proposals --include='*.op'
# Expected: no matches

# 15. Every fallible call is handled (guard/propagate coverage)
# Algorithm: find every call of the form `name(` where `name` appears in a signature with an
# `errors` clause. For each such call site, assert the enclosing statement or the
# surrounding 3 lines contain one of: `guard `, `propagate`, or the call is on the RHS of
# `guard X into Y else Z =>`. Implemented in .coverage-check.py (Task 2).
# Expected: exits 0

# 16. Every .op file contains at least one `#` comment explaining the scenario
for f in $(find stdlib-proposals -name '*.op' -not -path '*/node_modules/*'); do
  grep -qE '^\s*#[^#]' "$f" || { echo "MISSING scenario comment: $f"; exit 1; }
done
# Expected: exits 0

# 10. Momus verdict
# Manual: last Momus response must contain "OKAY"
```

### Final Checklist

- [ ] All "Must Have" items present
- [ ] All "Must NOT Have" items absent
- [ ] Style-gate passes
- [ ] 19 concern folders with target alternative counts (±1 allowed with justification)
- [ ] Empty error-strategy stubs deleted
- [ ] Top-level README.md indexes every concern
- [ ] Every concern has COMPARISON.md
- [ ] Every alternative has proposal.md + ≥ 2 .op files
- [ ] Momus returns OKAY
- [ ] User approves
