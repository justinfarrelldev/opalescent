# Decisions — pure-keyword

## [2026-04-18] Session ses_260fd3b04ffecmESbccQV6hvfs

### No higher-order purity enforcement
- Function type syntax `f(T): U` has no purity annotation
- Enforcing purity on callback parameters would require syntax changes
- Only named-call enforcement is in scope

### `pure entry` rejected at type-check time, not parse time
- Parser still allows `pure entry` syntactically
- Type checker rejects it with `PurityViolation` error
- Reason: entry functions are implicitly impure (they interact with the OS)

### Error variant naming
- New variant: `TypeError::PurityViolation`
- NOT reusing `TypeError::InvalidOperation` — dedicated variant required
- Fields: `callee_name: String`, `reason: String`, `#[label("{reason}")] span: SourceSpan`

## [2026-04-18] Session task-2-impure-list-expanded

### Scope-constrained implementation
- Only replaced `IMPURE_STDLIB_FUNCTIONS` constant content and added tests.
- Left purity check logic at `call_resolution.rs:100-114` unchanged as required.
- Kept `size_specific_builtins.rs` and type core structures untouched.

### Allowance test strategy
- Verified pure builtin allowance with `string_to_int32` in a pure function via `propagate` and declared `ParseError`, asserting successful type-check.
- Chosen to stay aligned with existing fallible-call rules while proving pure-call permit behavior.

- Kept purity tracking on `SymbolInfo` only (not `CoreType::Function`) to separate callable type shape from symbol metadata and preserve existing type-system invariants.
- 2026-04-18: Treated `pure` as a declaration modifier (not control-flow) in TextMate grammar to keep syntax classification aligned with language semantics.

## [2026-04-18] Session task-5-transitive-purity-enforcement

- Implemented transitive purity enforcement in call resolution for identifier callees by using `SymbolInfo.is_pure` only for non-builtin symbols.
- Retained builtin denylist (`IMPURE_STDLIB_FUNCTIONS`) as authoritative for impure stdlib calls and emit `PurityViolation` with callee-specific reasons.
- Migrated all existing purity tests from `InvalidOperation` matching to `PurityViolation` matching on `callee_name`.

## [2026-04-18] Session task-6-pure-entry-rejected

- Enforced `pure entry` rejection at function-declaration type-check time via a dedicated guard in `type_check_function_declaration` (before `effective_modifiers`), returning `TypeError::PurityViolation` with `callee_name: "entry"`.
- Kept parser behavior unchanged so `pure entry` remains syntactically valid and is rejected semantically during type checking.
