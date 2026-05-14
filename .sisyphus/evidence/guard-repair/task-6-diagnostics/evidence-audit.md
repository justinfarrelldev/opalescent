# Evidence Audit — task 6 diagnostics

## Scope
Hardening-only change. No checker semantics were changed.

## Verified
- Guard unit tests now assert exact TypeError variants plus source spans.
- Delete-download integration tests assert missing-terminal variant and exact span.
- Focused test slices passed with `--nocapture`.
- Rendered diagnostics were captured for both delete-download compile-fail fixtures.

## Notes
- Diagnostic rendering uses the existing `render_diagnostic` / `render_report` code paths.
- The evidence bundle records the diagnostic text, git snapshot, and verification results.
