# Decisions

- [2026-05-04T00:00:00Z] Implemented the activated broader path API fix as a targeted call-resolution compatibility bridge for `join_path_components` only, instead of broadening `types_compatible` globally, to minimize regression risk and keep diagnosis traceability.
- [2026-05-04T00:00:00Z] Kept builtin signatures/runtime ABI unchanged (`join_path_components` remains `(FilesystemPath, string[]) -> FilesystemPath`) and solved ergonomics at type-check acceptance boundary so codegen/runtime behavior remains stable.
- [2026-05-04T00:00:00Z] Updated the empty-directory workflow probe source to the intended ergonomic expression `join_path_components(base, [child_entry])` to directly validate Task 5 gate intent.
## Task 7 Decisions
- Decided to revert all out-of-scope codegen changes to ensure a clean delivery state.
- Added explicit module inclusion for fs_delete_directory_recursive to ensure test discovery.
- [2026-05-04T00:00:00Z] Baseline snapshot committed only after isolating the validated plan files and leaving follow-up work for later atomic commits.
- 2026-05-04 Decision: keep only Task-6-required implementation diff (`src/type_system/checker/call_resolution.rs`) and regenerate Task-7 evidence from a fresh seven-command run; revert all other tracked churn as out-of-scope for this remediation.
- 2026-05-04 Decision: report objective `NOT READY` because readiness is gated by command exits, not by plan-local file cleanliness.
- [2026-05-04T19:46:01-04:00] For Task 6a, applied only diagnosis-proven clippy pattern rewrites in the three flagged files and accepted rustfmt-required workspace edits; did not modify `.github/workflows/ci.yml` or `src/type_system/checker/call_resolution.rs`.

## 2026-05-04: Readiness Report
- Produced an objective readiness report in `.sisyphus/evidence/task-7-readiness-report.md` based on real command exits and verified function coverage.
- Determined that Opalescent is READY for the Linux directory-emptying workflow class of work.
