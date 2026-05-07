# Windows File System Issues

Issues preventing Opalescent programs from running on Windows with file system operations.

## Final Closure Status (Task 12)

- Checklist closure date: 2026-05-06
- Final evidence bundle: `.sisyphus/evidence/windows-issues-final/`
- Commit history review: inspected with `git log --oneline -15`; recent history remains split into task-sized units rather than one monolithic Windows commit.
- Final host state reflected by the current closure bundle:
  - `bash scripts/verify-wine-prereqs.sh` passes on this host (`.sisyphus/evidence/windows-issues-final/wine-prereqs.txt`).
  - `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` passes and is recorded in `.sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt`.
  - `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` followed by `wine target/program.exe` succeeds and is recorded in `.sisyphus/evidence/windows-issues-final/hello-world-msvc-wine.txt`.
- Toolchain closure state:
  - `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`.
  - Repo-local `.cargo/config.toml` is not part of the checked-in closure.
  - Linux/non-Windows builds keep dynamic LLVM preference through direct `llvm-sys` feature unification while Windows keeps plain `llvm14-0`.

## Runtime C Layer

- [x] **1. `path_parent_directory`, `path_file_name`, `path_file_extension` ignore `\` separator**
  Resolution: Windows separators, drive roots, UNC roots, and mixed separators are covered by the Task 5 regression set.
  Evidence: `.sisyphus/evidence/task-5-path-tests.txt` (`fs_path_helpers_query_fixture_showcase`, `fs_path_manipulation`).

- [x] **2. Bare `strdup` calls in path helpers (MSVC deprecation / linker failure)**
  Resolution: path helper duplication now uses `opal_strdup`/safe wrappers instead of bare `strdup` in the Windows-sensitive fs paths.
  Evidence: `.sisyphus/notepads/windows-issues/issues.md` Task 5 entry and `.sisyphus/evidence/task-5-path-tests.txt`.

- [x] **3. `lex_normalize_path` uses POSIX-only absolute path detection**
  Resolution: lexical normalization now recognizes drive-letter and UNC roots and preserves platform root semantics.
  Evidence: `.sisyphus/evidence/task-5-path-tests.txt` (`normalize_windows_roots_and_mixed_separators`).

- [x] **4. `join_path_components` only recognises `/`-rooted absolute components**
  Resolution: Windows absolute components now reset the accumulator instead of being appended as relative segments.
  Evidence: `.sisyphus/evidence/task-5-path-tests.txt` (`join_windows_absolute_components_reset_accumulator`).

- [x] **5. `absolute_path_sync` stores static string literals in error fields (use-after-free / crash)**
  Resolution: error strings returned from `absolute_path_sync` are heap-allocated and safely freed by regression coverage.
  Evidence: `.sisyphus/evidence/task-6-error-allocation.txt` (`absolute_path_sync_allocates_errors_and_keeps_absolute_inputs`).

- [x] **6. All file I/O uses ANSI narrow-char APIs — non-ASCII paths silently fail**
  Resolution: Windows file I/O moved to the UTF-8↔wide boundary and the Wine file-ops fixture exercises Unicode paths.
  Evidence: `.sisyphus/evidence/task-9-long-path-wine.txt` and `.sisyphus/evidence/task-3-wine-msvc-file-ops-stdout.txt`.

- [x] **7. `opal_opendir` uses `FindFirstFileA` (ANSI, no Unicode support)**
  Resolution: directory enumeration was moved to the wide Win32 path and kept behind the portability boundary.
  Evidence: `.sisyphus/notepads/windows-issues/issues.md` Task 4 entry and `.sisyphus/evidence/task-7-dir-errno.txt`.

- [x] **8. `opal_opendir` doesn't set `errno` on `FindFirstFileA` failure**
  Resolution: missing-directory and file-as-directory probes now assert deterministic errno-driven behavior.
  Evidence: `.sisyphus/evidence/task-7-dir-errno.txt` (`list_directory_not_found`, `list_directory_rejects_file_path`).

- [x] **9. `opal_closedir` doesn't propagate `errno` on `FindClose` failure**
  Resolution: Windows dir close/open errno propagation lives in the Task 7 portability fix set.
  Evidence: `.sisyphus/notepads/windows-issues/issues.md` Task 7 entry and `.sisyphus/evidence/task-7-dir-errno.txt`.

- [x] **10. Forward declarations in `opal_fs.c` create potential ODR conflict on Windows**
  Resolution: the conflicting non-dirent forward declarations were removed and the portability-header definitions remain authoritative.
  Evidence: `.sisyphus/notepads/windows-issues/issues.md` Task 7 entry.

- [x] **11. `opal_stat` always reports `is_symlink = 0` on Windows**
  Resolution: Windows metadata now reports reparse-point symlink state in both follow and nofollow coverage.
  Evidence: `.sisyphus/evidence/task-7-symlink-metadata.txt` (`wine_msvc_symlink_metadata`; host shows explicit prereq skip when Wine/MSVC tooling is unavailable).

- [x] **15. `opal_runtime_init` is never called for generated programs**
  Resolution: `opal_runtime.c` is included in `RUNTIME_SOURCE` and generated entry wrappers call `opal_runtime_init()` before user entrypoint execution.
  Evidence: `.sisyphus/evidence/task-8-runtime-init.txt` (`test_entry_main_wrapper_calls_runtime_init_before_entrypoint`) and `.sisyphus/evidence/task-8-rc-link.txt` (`runtime_source_includes_runtime_and_rc_symbols_exactly_once`).

- [x] **16. Filesystem path buffer capped at 260 bytes (`MAX_PATH` limit)**
  Resolution: Windows fs user paths no longer depend on a `MAX_PATH`-sized buffer; remaining cap constant is no longer the legacy 260-byte value.
  Evidence: `.sisyphus/evidence/task-9-maxpath-search.txt` (`OPAL_PATH_BUFFER_CAP ((size_t)4096)`) and `.sisyphus/evidence/task-9-long-path-wine.txt`.

## Build System

- [x] **12. `Cargo.toml` still includes `"llvm14-0-prefer-dynamic"`**
  Resolution: `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`; the manifest now keeps plain `llvm14-0` and uses direct `llvm-sys` feature unification on non-Windows hosts to prefer dynamic LLVM without the old inkwell feature string.
  Evidence: `.sisyphus/evidence/windows-issues-final/toolchain-summary.txt`, `.sisyphus/evidence/windows-issues-final/linux-tests.txt`, `.sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt`.

- [x] **13. CI `cross-msvc-from-linux` job installs `xwin` without version pinning**
  Resolution: CI now pins `xwin` to `0.9.0` with `--locked`, and the prereq script reports that expectation explicitly.
  Evidence: `.sisyphus/evidence/task-10-cargo-ci-search.txt`, `.sisyphus/evidence/windows-issues-final/toolchain-summary.txt`, `.sisyphus/evidence/windows-issues-final/wine-prereqs.txt`.

- [x] **17. MSVC linker invocation missing `bcrypt.lib`**
  Resolution: MSVC shared linker args now include the required bcrypt library.
  Evidence: `.sisyphus/notepads/windows-issues/issues.md` Task 2 entry and `.sisyphus/evidence/task-5-linux-regression.txt` (`msvc_linker_shared_args_present`).

- [x] **18. `opal_rc.c` is absent from `RUNTIME_SOURCE`**
  Resolution: runtime source aggregation includes the RC runtime exactly once.
  Evidence: `.sisyphus/evidence/task-8-rc-link.txt` (`runtime_source_includes_runtime_and_rc_symbols_exactly_once`) and `.sisyphus/evidence/task-5-linux-regression.txt` (`runtime_source_includes_opal_rc_source_symbols`).

- [x] **19. Missing `XWIN_CACHE` / `OPAL_XWIN_SYSROOT` panics instead of returning an error**
  Resolution: missing xwin/sysroot configuration now surfaces as a structured linker/compiler error instead of a panic.
  Evidence: `.sisyphus/evidence/task-5-linux-regression.txt` (`msvc_linker_missing_xwin_env_surfaces_error_instead_of_panicking`) and `.sisyphus/evidence/windows-issues-final/hello-world-msvc-wine.txt`.

- [x] **20. `quote_if_needed` wraps paths in literal quote characters**
  Resolution: raw paths are now passed through `Command::arg()` without injecting literal quote bytes.
  Evidence: `.sisyphus/evidence/task-5-linux-regression.txt` (`linker_command_passes_raw_paths_with_spaces_to_command_args`).

## Hot-Reload

- [x] **14. Windows `.dll` hot-reload lacks copy-before-load (files locked by the OS)**
  Resolution: closure landed as regression verification: loader behavior already copied DLLs to unique temp paths before loading, and Task 11 locked that behavior with tests.
  Evidence: `.sisyphus/evidence/task-11-hot-reload-copy.txt`, `.sisyphus/evidence/task-11-hot-reload-tests.txt` (`windows_dll_copy_before_load_uses_dll_extension`, `fs_module_loader_repeated_loads_create_distinct_temp_copy_paths`).

## Final Verification Matrix (Task 12)

- `cargo test --all-features --workspace` → **PASS** (`EXIT_CODE=0`).
  Evidence: `.sisyphus/evidence/windows-issues-final/linux-tests.txt`
- `cargo clippy --all-targets --all-features -- -D warnings` → **PASS** (`EXIT_CODE=0`).
  Evidence: `.sisyphus/evidence/windows-issues-final/clippy.txt`
- `cargo fmt --all -- --check` → **PASS** (`EXIT_CODE=0`).
  Evidence: `.sisyphus/evidence/windows-issues-final/fmt-check.txt`
- `bash scripts/verify-wine-prereqs.sh` → **PASS** (`EXIT_CODE=0`).
  Evidence: `.sisyphus/evidence/windows-issues-final/wine-prereqs.txt`
- `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` → **PASS** (`EXIT_CODE=0`) with the completed final run recorded in the closure bundle.
  Evidence: `.sisyphus/evidence/windows-issues-final/wine-msvc-file-ops.txt`
- `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` and `wine <exe>` → **PASS** (`BUILD_EXIT_CODE=0`, `WINE_EXIT_CODE=0`).
  Evidence: `.sisyphus/evidence/windows-issues-final/hello-world-msvc-wine.txt`
- MinGW non-regression compile/link smoke → currently remains in the final bundle as prior evidence and is not part of the blocker set being cleared in this closure pass.
  Evidence: `.sisyphus/evidence/windows-issues-final/mingw-smoke.txt`
