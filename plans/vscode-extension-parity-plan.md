# VS Code Extension Language Parity Plan

## Goal

Bring `vscode-extension/` from grammar-only scaffolding to a usable Opalescent editor extension that delegates formatting, linting/checking, building, and running to the Opalescent compiler wherever possible. The extension should feel close to TypeScript/Go ergonomics: automatic diagnostics, command-palette actions, code lenses above `entry` functions, formatting, navigation, and project-root awareness.

## Constraints and guardrails

- Follow red-green-refactor for every implemented slice.
- Keep an up-to-date checklist in `plans/checklists/`.
- Do not add braces-around-block snippets or suggest brace-delimited Opalescent bodies.
- Use the compiler/formatter/checker as the source of truth for diagnostics and formatting.
- Keep navigation helpers conservative and text-index based only where the compiler currently exposes no stable query API.
- Add a `cargo make` task that packages a `.vsix` artifact.
- Keep commits atomic.

## Current gaps

- The extension contributes only language metadata and TextMate grammar.
- No extension host entry point exists.
- No binary discovery or prompting exists.
- No command palette commands exist for linting/formatting/build/run.
- `opal check` emits human-readable diagnostics only, making exact editor ranges brittle.
- Existing `opal lsp --stdio` is not a usable JSON-RPC server yet, so the extension must invoke the CLI directly for now.
- Project detection is absent in the extension.
- Syntax highlighting is stale (`match`, `char`, `module`, `export` surfaces are misleading for current user-facing syntax).

## Implementation phases

### Phase 1: Compiler-backed machine diagnostics

1. Add tests for machine-readable diagnostic conversion.
2. Add an editor diagnostic JSON model that maps compiler errors and type-checker warnings to exact zero-based ranges.
3. Extend `opal check` with `--json` and `--project [path]`.
4. Implement project checking without code generation by reusing module discovery and type checking.
5. Preserve existing human CLI behavior.

### Phase 2: Extension TypeScript foundation

1. Add TypeScript project scaffolding, unit tests, and npm scripts.
2. Add pure helper modules for Opalescent file detection, project-root discovery, CLI arg construction, compiler JSON parsing, and symbol indexing.
3. Write red tests for those helpers before implementing.

### Phase 3: VS Code integration

1. Add activation entry point.
2. Add Opalescent binary resolution from settings, workspace build outputs, `PATH`, and user prompt fallback.
3. Register diagnostics collection that runs `opal check --json`, project-aware when inside `opal.toml`.
4. Register formatter provider and command that runs `opal fmt` through temporary files.
5. Register command-palette commands for format, lint/check, build, run, and binary selection.
6. Register code lenses above `entry ... = f(...) =>` declarations for Build and Run.
7. Register conservative definition and implementation providers using a project file index.

### Phase 4: Packaging and grammar parity

1. Refresh package contributions, configuration, activation events, and VS Code categories.
2. Update syntax grammar and language configuration for current syntax.
3. Add `.vscodeignore` and cargo-make tasks for installing extension deps, compiling, testing, and packaging a `.vsix`.
4. Run extension tests, Rust tests for touched compiler code, cargo-make package task, and a review pass.

## Test strategy

- Rust unit tests for JSON diagnostics and project check helper behavior.
- Extension unit tests with Node's built-in test runner for pure helpers.
- TypeScript compile as a CI gate for VS Code API integration.
- Manual smoke via `cargo make vscode-extension-vsix` to produce a `.vsix`.

## Known limitations after this pass

- Navigation will use a conservative project text index until the compiler exposes stable symbol-query/LSP APIs.
- Automatic diagnostics are source-of-truth compiler diagnostics, but dirty unsaved buffers can only be checked as standalone temporary content; project-wide diagnostics become authoritative after save.
- The extension prompts for an Opalescent binary when it cannot resolve one automatically.
