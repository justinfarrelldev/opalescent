# Issues — fmt-preserve-comments

(none yet)

## [2026-04-15] Verification gotcha
- `cargo test --features integration fmt_output_simple_quiz` can fail if `input-simple-quiz.expected.op` is stale relative to current formatter behavior (notably guard-body rendering). Regenerating the golden via formatter fixed the mismatch.
