# Compiler-Owned String Semantics Implementation Checklist

- [x] Read AGENTS.md project rules and required language documentation.
- [x] Read the selected compiler-owned string semantics proposal and comparison context.
- [x] Add red fixtures/tests for returning literals, returning parameters, ADT payload storage, field access returns, product field access, and string arrays without defensive copies.
- [x] Implement safe compiler/runtime ownership handling for string returns, parameters, constructors, field access, and arrays.
- [x] Remove terminal editor defensive `copy_text` usage after ownership fixtures pass.
- [x] Update public documentation for compiler-owned string semantics.
- [x] Run targeted tests and at least one generated-program build/run ladder.
- [x] Review implementation for memory-safety, maintainability, and proposal completeness.
