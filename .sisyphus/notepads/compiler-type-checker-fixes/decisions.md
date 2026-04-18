# Decisions — compiler-type-checker-fixes

## [2026-04-17] Task 3 loop initializer lowering approach
- Chosen approach: Option A (delegate from `codegen_let_statement` to `codegen_let_destructure_statement` for `Expr::Loop` initializers).
- Rationale:
  - `Let` binding type is `LetBinding`, compatible with `&[LetBinding]` expected by destructure codegen.
  - Existing destructure path already contains proven loop-lowering logic (`codegen_loop_expression_into_slots`).
  - Avoids duplicated lowering code and keeps `codegen_expression(Expr::Loop)` rejection intact for non-statement contexts.
