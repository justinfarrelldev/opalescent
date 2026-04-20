# Decisions

## 2026-04-20 Session ses_2547b9221ffecHt6C3Ua0J6XH1

### Dispatch Location
Insert dispatch AFTER lowered_args loop (~line 85) but BEFORE build_call (~line 123) in codegen_call_expression.
Early return after dispatched call — do NOT fall through to generic build_call.

### Signed-Only Integer Dispatch
All integers dispatch to print_int* (signed variants). LLVM i32 cannot distinguish signed from unsigned.
This matches existing string interpolation behavior. Known limitation, documented.

### free() Declaration Strategy
Declare free() inline in the dispatch code, NOT added to STDLIB_NAMES.
Signature: void(i8*). Use module.get_function("free").unwrap_or_else(|| declare inline).

### Float Literals in Test Project
Include print(3.14) in test project — executor will verify if float literals compile and adjust if needed.

## 2026-04-20 F3 Evidence Decision
- Saved scenario evidence under `.sisyphus/evidence/final-qa/` using dedicated files per scenario plus integration regression tail output for traceability.
