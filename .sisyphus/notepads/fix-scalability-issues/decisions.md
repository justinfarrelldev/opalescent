# Decisions

## [2026-04-17] Overflow behavior
- ALWAYS TRAP in both debug and release mode
- Trap via `opal_runtime_error("message")` — fprintf stderr + exit(1)

## [2026-04-17] LLVM upgrade target
- inkwell 0.9.0 + LLVM 18 (llvm18-1 feature)
- Requires opaque pointer migration across all codegen

## [2026-04-17] Shared helpers location
- Extract `integer_literal_bits` and `is_signed_core_type` into shared module
- Candidate: `src/codegen/types.rs` or new `src/codegen/helpers.rs`
