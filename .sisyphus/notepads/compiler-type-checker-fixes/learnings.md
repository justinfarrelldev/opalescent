# Learnings — compiler-type-checker-fixes

## [2026-04-18] Session ses_2665c0930ffexGSPKDSdVxcWUg — Plan Start

### Key Architecture Facts
- `loop_break_type_stack: Vec<Option<Vec<CoreType>>>` — outer Option is None when no typed breaks seen, inner Vec holds break-value types
- `.pop()` on the stack returns `Option<Option<Vec<CoreType>>>` — outer always Some (we just pushed), inner None = no typed breaks, Some(vec) = typed breaks
- `Stmt::Loop` in `statements.rs` is the TEMPLATE for push/pop pattern — mirror it exactly
- `Stmt::Break` handler fills the stack entry — first break sets Some(types), subsequent breaks validate against it
- `codegen_let_destructure_statement` already handles Expr::Loop initializer — it's the proven reference for codegen
- `codegen_expression(Expr::Loop)` explicitly errors — must intercept BEFORE this call in `codegen_let_statement`
- Integer literals infer as `CoreType::Int64` — use `int64_to_string` not `int32_to_string` in test programs

### Standard Module Facts
- `register_standard_module()` currently has 5 entries: print, println, take_input, string_to_int32, string_to_int64
- Do NOT add string_to_int32 or string_to_int64 again (already present)
- Target: exactly 24 entries after Task 1 (5 existing + 19 new)
- Codegen already maps all 19 under ("standard", "symbol_name") — no codegen changes needed for Task 1

### File Extension
- Source files use `.op` NOT `.opal`
- Compile/run command: `cargo run --release -- <file.op> --run`

## [2026-04-17] Task 3 — let loop initializer codegen interception

### Codegen learnings
- `Stmt::Let` uses a single `LetBinding`, and `Stmt::LetDestructure` expects `&[LetBinding]`; Option A is feasible by wrapping `binding.clone()` in a single-element array and delegating.
- Intercepting `Expr::Loop` at the top of `codegen_let_statement` cleanly avoids the `codegen_expression(Expr::Loop)` hard error while preserving that guard for general expression positions.
- `cargo build` and `cargo test` pass with the interception in place; no extra codegen duplication is needed when reusing `codegen_let_destructure_statement`.

## [2026-04-18] Scope fidelity audit learnings
- Commit-level fidelity checks must use `git show --name-status --stat <hash>` first to validate file-scope before line-level spec checks.
- Scope audits should treat `.sisyphus/evidence/*` additions as unaccounted unless explicitly listed in task specs.
- For must-not-have checks, `git diff --name-only <first>^..<last> -- <paths>` is sufficient to prove no touched files across the implementation window.
