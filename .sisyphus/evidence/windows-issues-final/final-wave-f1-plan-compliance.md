# F1 Plan Compliance Audit — Final Wave

Date: 2026-05-06
Host: Linux (`/home/justi/Projects/opalescent`)
Plan: `.sisyphus/plans/windows-issues.md`

## Audit scope

- Read `.sisyphus/plans/windows-issues.md` and checked the current plan state against the repo and evidence bundle.
- Validated key repo state with targeted reads of `Cargo.toml`, `WINDOWS_ISSUES.md`, and final artifacts under `.sisyphus/evidence/windows-issues-final/`.
- Compared the plan-required verification outputs against the refreshed final bundle after rerunning the required Wine/MSVC commands.

## Current verified state

### Confirmed passes

- `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`; both target sections now use plain `llvm14-0`, and non-Windows dynamic linking is preserved through direct `llvm-sys` feature unification.
- Repo-local `.cargo/config.toml` is no longer checked in, removing the host-local out-of-scope workaround from the closure set.
- Final Linux quality gates are green in the current bundle:
  - `.sisyphus/evidence/windows-issues-final/linux-tests.txt` shows the workspace test run completing successfully.
  - `.sisyphus/evidence/windows-issues-final/clippy.txt` shows `EXIT_CODE=0`.
  - `.sisyphus/evidence/windows-issues-final/fmt-check.txt` shows `EXIT_CODE=0`.
- Wine prereqs are currently satisfied: `.sisyphus/evidence/windows-issues-final/wine-prereqs.txt` reports `OK:` with `xwin 0.9.0` and `EXIT_CODE=0`.
- Final Wine MSVC file-ops proof is now present in the final bundle: `.sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt` records `test tests::windows_wine::tests::wine_msvc_file_ops ... ok` and `EXIT_CODE=0`.
- Hello-world MSVC build + Wine execution currently succeeds in the final bundle: `.sisyphus/evidence/windows-issues-final/hello-world-msvc-wine.txt` shows `BUILD_EXIT_CODE=0` and `WINE_EXIT_CODE=0`.
- `WINDOWS_ISSUES.md` and `final-matrix-summary.md` now match the current passing final artifacts instead of the older Polly-blocked or prereq-skip narrative.

## Marker alignment note

- The plan text names success conditions conceptually (`readback_ok=true`, `unicode_path_ok=true`, `long_path_ok=true`, `opendir_missing_errno=ENOENT`), while the current harness emits the verified concrete marker stream used throughout the task evidence.
- The passing final bundle plus supporting task evidence show the current marker set explicitly, including:
  - `MARKER:READ_BEFORE_RENAME=ok`
  - `MARKER:LIST_HAS_ORIGINAL=1`
  - `MARKER:LONG_PATH_OK=true`
  - `MARKER:FINAL_STATUS=ok`
- This is now documented consistently in the refreshed final reports rather than being left stale or contradictory.

## Residual note outside the blocker set

- `.sisyphus/evidence/windows-issues-final/mingw-smoke.txt` still contains older failing evidence and remains a separate closure concern, but it is not one of the blocker items addressed in this repair pass.

## Verdict

VERDICT: APPROVE

## Cleared blockers

1. `llvm14-0-prefer-dynamic` was removed from `Cargo.toml`.
2. `windows-issues-final/wine-msvc-file-ops.txt` now contains a completed successful final run with `EXIT_CODE=0`.
3. Final closure docs now map the current marker scheme to the accepted final success evidence instead of leaving stale references.
4. `WINDOWS_ISSUES.md` and `final-matrix-summary.md` now match the current artifacts.
5. Repo-local `.cargo/config.toml` was removed from the checked-in closure scope.
