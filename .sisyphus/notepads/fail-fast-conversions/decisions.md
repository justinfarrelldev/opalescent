# Decisions

## [2026-04-16] Architecture decisions from plan
- Parse functions return `{value, const char* error}` structs (not bool+value)
- NULL error pointer = success; non-NULL = failure with message
- Error messages must be specific: "null input", "empty input", "invalid digit 'X' in input", "overflow: value exceeds intN range"
- `*_to_string` functions are infallible (no error return)
- Bare calls to parse functions without guard/propagate = compile-time error
- TDD approach: write failing test first, then implement
