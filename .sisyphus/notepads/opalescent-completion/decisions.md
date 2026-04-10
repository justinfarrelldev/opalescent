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
