# Issues — CLI Help Alignment

(none yet)

## Task 3 blocker
- cargo make lint failed initially due to strict test lint rules in src/app.rs test module (`clippy::str_to_string`, `clippy::default_numeric_fallback`).
- Resolved by adding file-level `cfg_attr(test, allow(..., reason = ...))` in src/app.rs without editing test bodies.
