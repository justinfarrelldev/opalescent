
- 2026-04-17: Implemented array-parameter length propagation in codegen only (no parser/AST/type-system changes) by expanding LLVM function parameter lists with implicit `*_len` i64 arguments for `CoreType::Array` parameters.
- 2026-04-17: Call-site lowering now appends inferred array length metadata only for identifier callees (regular function calls), avoiding lambda-call argument shape regressions.
