# Opalescent Completion — Decisions

## [2026-04-10] Session Start

### Architecture Decisions (from plan)
- Multiple return values: `Vec<Type>` replacing single return type (backward compat: vec of 1)
- Warning system: parallel to TypeError, collects alongside errors
- Built-in registration: happens in `TypeChecker::new()`
- If expressions: both Rust-style (value-returning) with branch type unification

### Plan-Mandated Decisions
- No monomorphization until Phase 5 (Task 6 does constraint solving only)
- No runtime trap code gen until Phase 5 (Tasks 8, 9 do compile-time detection only)
- No full ADT field access until Phase 3 (Task 7 handles module/basic struct)
- stdlib/prelude.op is documentation only (Task 2 — no runtime behavior)

## [2026-04-10] Task 2 Built-ins Decisions
- Built-in functions are represented as symbol-table pre-registrations plus type-environment built-in signatures; no runtime implementation in this phase.
- ParseError is registered as a nominal core type in TypeEnvironment bootstrap to satisfy built-in error signature resolution.
- Generic built-in specialization uses per-call fresh type-variable instantiation, isolated in checker/call_resolution.rs to satisfy line-count constraints.
- hello_world spec validation in tests uses tab-to-spaces normalization helper to avoid lexer mixed-whitespace rejection while preserving program semantics.
## [2026-04-11] Task 7: Member Access Resolution Strategy
- Chose symbol-table-driven member lookup for Phase 2 instead of introducing new `CoreType` variants or full HasField constraints.
- Resolution order for `Expr::Member`: first check `identifier.member` (module-style access), then fallback to `GenericTypeName.member` for basic nominal field access.
- Deferred full ADT/HasField constraint enforcement to Phase 3 per plan; current approach keeps member typing functional without expanding constraint solver scope.
