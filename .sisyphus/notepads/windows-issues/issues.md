
## 2026-05-07T00:20:00Z — List marker mismatch resolved

- `MARKER:LIST_HAS_ORIGINAL` is now aligned with the established `list_directory_sync(...)` contract: the Wine fixture compares listed entry names against `path_file_name(original_file)` and now emits `MARKER:LIST_HAS_ORIGINAL=1`.
- The targeted Wine integration slice also needed the long-path assertion aligned to the real cross-environment contract; it now parses `MARKER:LONG_PATH_LEN` numerically and requires a value greater than 260 while still verifying the host-visible deep file and contents.
- Fresh evidence in `.sisyphus/evidence/task-crash-fix-wine-{stdout,stderr,exit}.txt` now shows a clean Wine exit with `LIST_HAS_ORIGINAL=1`, `LONG_PATH_LEN=481`, and no stderr output.

## 2026-05-06T23:59:00Z — Final blocker set cleared

- Removed the remaining `llvm14-0-prefer-dynamic` reference from `Cargo.toml` and replaced the host-preserving behavior with direct `llvm-sys` feature unification on non-Windows builds.
- Deleted the checked-in host-local `.cargo/config.toml` so the closure bundle no longer depends on one machine layout.
- Refreshed `wine-prereqs.txt`, `wine-msvc-file-ops.txt`, `hello-world-msvc-wine.txt`, `final-matrix-summary.md`, `final-wave-f1-plan-compliance.md`, `final-wave-f4-scope-fidelity.md`, and `WINDOWS_ISSUES.md` so they now match the current passing Wine/MSVC evidence.

## 2026-05-07T00:30:00Z — Final verification environment requirement

- The ABI/codegen regression fix itself was complete once the targeted Rust tests passed, but the mandated `cargo test --all-features --workspace` run on this Linux host also required exporting the existing MSVC cross-compilation environment (`PATH=/usr/lib/llvm-14/bin:$PATH`, `LLVM_SYS_140_PREFIX=/usr/lib/llvm-14`, `XWIN_CACHE=/home/justi/.xwin`, `OPAL_XWIN_SYSROOT=/home/justi/.xwin`, `OPAL_MSVC_CC=/usr/lib/llvm-14/bin/clang-cl`, and matching `CFLAGS_x86_64_pc_windows_msvc`).
- Without that shell environment, the Windows/Wine integration gate fails before execution with missing xwin or missing `clang-cl` process errors, which is a host verification setup issue rather than a new filesystem ABI regression.
