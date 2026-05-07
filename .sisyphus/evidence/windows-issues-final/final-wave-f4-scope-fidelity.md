# F4 Scope Fidelity Check — Final Wave

Date: 2026-05-06
Host: Linux (`/home/justi/Projects/opalescent`)
Plan: `.sisyphus/plans/windows-issues.md`

## Scope Reviewed

This review checked the delivered change set against the plan's scope rules, using:
- `.sisyphus/plans/windows-issues.md`
- `.sisyphus/notepads/windows-issues/learnings.md`
- `.sisyphus/notepads/windows-issues/issues.md`
- current final artifacts under `.sisyphus/evidence/windows-issues-final/`
- focused closure files: `Cargo.toml`, `WINDOWS_ISSUES.md`, and the final evidence bundle

## Plan Scope Guardrails Applied

From the plan, scope is acceptable only if all of the following remain true:
- Changes stay within the Windows issues listed in `WINDOWS_ISSUES.md`, plus tests/evidence needed to verify them.
- No broad new path abstraction or unrelated architectural refactor is introduced.
- No extra source/config changes are added outside the listed issue scope.
- Final closure must include accurate evidence-backed completion of required scope, including missing-scope analysis as well as extra-scope analysis.
- Documented Wine caveats are allowed where the plan explicitly permits them.

## What Stayed In Scope

The final repair pass stayed tightly scoped to closure artifacts and the one manifest/toolchain requirement tied directly to the blocker set:
- `Cargo.toml` was updated only to remove the forbidden `llvm14-0-prefer-dynamic` string while preserving host viability through direct `llvm-sys` feature unification.
- The checked-in host-local `.cargo/config.toml` was removed, shrinking scope rather than expanding it.
- Final evidence files were refreshed by rerunning the required Wine/MSVC commands.
- `WINDOWS_ISSUES.md`, `final-matrix-summary.md`, and the final-wave audit notes were updated only where they contradicted the current evidence bundle.
- No runtime/compiler logic beyond the LLVM linkage configuration needed for the existing verification path was changed in this closure pass.

## Out-of-Scope Additions

- No new out-of-scope additions remain in the repaired closure set.
- The previously flagged host-local `.cargo/config.toml` has been removed from the repository.

## Required Scope Now Closed Accurately

### 1. Task 10 manifest requirement
**Status:** Closed.

- `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`.
- The remaining non-Windows dynamic-link preference is implemented through direct `llvm-sys` feature selection instead of the old inkwell feature string.

### 2. Final Wine MSVC file-ops gate
**Status:** Closed.

- `.sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt` now contains a completed successful run.
- The artifact records `test tests::windows_wine::tests::wine_msvc_file_ops ... ok` and `EXIT_CODE=0`.

### 3. Final closure reporting accuracy
**Status:** Closed.

- `WINDOWS_ISSUES.md` now matches the current passing final bundle.
- `final-matrix-summary.md` now reflects current PASS results for prereqs, Wine file ops, and hello-world under Wine.
- Toolchain closure notes now accurately describe the removal of repo-local host config and the retained dynamic-link behavior on non-Windows hosts.

## Residual note outside this blocker repair

- `mingw-smoke.txt` remains older evidence and should be handled separately if the orchestrator wants the entire final bundle normalized beyond the F1/F4 blocker set.

## Scope Fidelity Conclusion

The repaired closure set now stays within plan scope and corrects the exact fidelity failures that caused rejection:
1. The unauthorized checked-in host-local config is gone.
2. The manifest criterion is satisfied as written.
3. The required final Wine/MSVC evidence is complete and current.
4. The closure docs now faithfully match the evidence bundle.

## VERDICT: APPROVE
