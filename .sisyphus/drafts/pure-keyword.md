# Draft: Pure Keyword Implementation

## Requirements (confirmed)
- Finish implementation of `pure` keyword — must be usable and enforced with clear error messages
- TDD protocol (RED-GREEN-REFACTOR)
- Tests: `cargo test` and `cargo test --features integration`
- no_std compatible in core modules (use `alloc`/`core`)
- Files under 1000 lines
- Strict clippy configuration

## What's Already Done (Foundation)
- Lexer: `pure` → `TokenType::Pure` ✓
- AST: `FunctionModifier::Pure` ✓
- Parser: `pure` modifier parsed and stored ✓
- Type Checker: `function_modifier_stack`, `current_function_is_pure()`, enter/exit context ✓
- Basic enforcement: Calling `print`/`take_input`/`random_int32` from pure → error ✓
- Two tests for basic print/non-print ✓

## What's Missing (Needs Implementation)
1. Expand `IMPURE_STDLIB_FUNCTIONS` — all print_*, random_*, etc.
2. Dedicated `PurityViolation` error variant with clear diagnostics
3. Transitive purity — pure can't call non-pure user functions
4. Reject `let mutable` inside pure functions (NEEDS DESIGN DECISION)
5. Reject assignment statements inside pure functions (NEEDS DESIGN DECISION)
6. Reject mutating array methods (push, pop) inside pure (NEEDS DESIGN DECISION)
7. Prevent `pure entry` combination
8. Comprehensive tests for each violation type
9. VSCode syntax highlighting for `pure`
10. Purity tracking on function types for transitive checking

## Technical Decisions
- Work primarily in type checker phase
- Primary files: call_resolution.rs, statements.rs, errors.rs, tests.rs
- Use existing `function_modifier_stack` infrastructure (already working)

## Open Questions
- ~~Local mutation policy~~ **RESOLVED**: Spec examples (`automatic_regions.op:49`, `value_semantics.op:67`) show `pure` functions using `let mutable` and assignments. Pure = no external side effects (I/O, calling impure). Local mutation is ALLOWED.

## Scope Boundaries
- INCLUDE: Type checker enforcement (impure stdlib, transitive purity, pure+entry conflict), error messages, tests, VSCode highlighting
- EXCLUDE: Rejecting local mutation (spec allows it), Formatter awareness, LSP awareness
- REMOVED from gap analysis: Tasks 4 (let mutable), 5 (assignments), 6 (array mutation) — spec contradicts these
