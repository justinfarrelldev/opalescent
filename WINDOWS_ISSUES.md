# Windows File System Issues

Issues preventing Opalescent programs from running on Windows with file system operations.

## Runtime C Layer

- [ ] **1. `path_parent_directory`, `path_file_name`, `path_file_extension` ignore `\` separator**
  `runtime/opal_fs.c` — all three use `strrchr(path, '/')` only. A Windows path like `C:\Users\foo\bar.txt` returns wrong results from all three (e.g., `path_parent_directory` returns `"."` since no `/` is found).

- [ ] **2. Bare `strdup` calls in path helpers (MSVC deprecation / linker failure)**
  `runtime/opal_fs.c` — `path_parent_directory`, `path_file_name`, `path_file_extension`, and `safe_strdup` all call `strdup` directly. On MSVC, `strdup` is deprecated; with `/WX` (warnings-as-errors), these become build errors. The shim `opal_strdup` from `opal_portability.h` should be used instead.

- [ ] **3. `lex_normalize_path` uses POSIX-only absolute path detection**
  `runtime/opal_fs.c` — checks `path[0] == '/'` to detect absolute paths. Windows absolute paths (`C:\Users\...`, `\\server\share`) are never detected as absolute, so they get mangled into relative paths. Also, when collapsing to root, it hardcodes `safe_strdup("/")` instead of the platform root.

- [ ] **4. `join_path_components` only recognises `/`-rooted absolute components**
  `runtime/opal_fs.c` — `if (component[0] == '/')` is the only absolute-component check. Windows drive-letter paths (`C:\...`) and UNC paths (`\\...`) are not recognised as absolute; they get appended as relative segments instead of replacing the accumulator.

- [ ] **5. `absolute_path_sync` stores static string literals in error fields (use-after-free / crash)**
  `runtime/opal_fs.c` — Two error paths assign a string literal directly to `r.error`. `opal_fs_errors.h` explicitly forbids static literals because consumers call `free()` on every non-NULL `.error` field. Freeing a literal is undefined behaviour and crashes on Windows (where MSVC CRT validates heap pointers in `free`).

- [ ] **6. All file I/O uses ANSI narrow-char APIs — non-ASCII paths silently fail**
  Every call to `fopen`, `_stat64`, `_unlink`, `MoveFileExA`, and `opal_mkdir`/`_rmdir` passes a UTF-8 encoded `char*` path, but the Windows ANSI APIs interpret it using the system ANSI codepage (typically CP-1252), not UTF-8. Paths containing non-ASCII characters will silently open the wrong file, report "not found", or corrupt names. The Unicode conversion helpers (`opal_utf8_to_wide` / `opal_wide_to_utf8`) exist in `opal_portability.h` but are not used by any fs I/O function.

- [ ] **7. `opal_opendir` uses `FindFirstFileA` (ANSI, no Unicode support)**
  `runtime/opal_portability.h` — Directory enumeration calls `FindFirstFileA`, which applies the same ANSI codepage restriction as issue 6. Non-ASCII directory names will be mishandled or silently skipped.

- [ ] **8. `opal_opendir` doesn't set `errno` on `FindFirstFileA` failure**
  `runtime/opal_portability.h` — When `FindFirstFileA` returns `INVALID_HANDLE_VALUE`, the function frees the handle and returns NULL without calling `opal_set_errno_from_win32(GetLastError())`. Callers that check `errno` after failure will see stale errno, leading to wrong error discriminants (e.g., `"DeleteFailureError"` instead of `"FileNotFoundError"`).

- [ ] **9. `opal_closedir` doesn't propagate `errno` on `FindClose` failure**
  `runtime/opal_portability.h` — Returns `-1` on `FindClose` failure but never sets `errno`, so callers cannot distinguish the error type.

- [ ] **10. Forward declarations in `opal_fs.c` create potential ODR conflict on Windows**
  `runtime/opal_fs.c` — The `#if !OPAL_HAS_DIRENT` block (compiled on Windows) forward-declares `opal_opendir/readdir/closedir` as non-static extern functions. `opal_portability.h` already defines the same names as `static inline` in the same translation unit. Having both a `static inline` definition and a plain extern declaration for the same identifier is an ODR problem; some compilers may emit an error or silently use the wrong linkage.

- [ ] **11. `opal_stat` always reports `is_symlink = 0` on Windows**
  `runtime/opal_portability.h` — The follow-symlinks `opal_stat` unconditionally sets `out->is_symlink = 0`. `read_metadata_sync` therefore never reports a symlink on Windows, even though the `opal_stat_nofollow` path does report it via `FILE_ATTRIBUTE_REPARSE_POINT`. Programs that make decisions on `is_symlink` from `read_metadata_sync` will behave incorrectly on Windows symlinks and junctions.

- [ ] **15. `opal_runtime_init` is never called for generated programs**
  `runtime/opal_runtime.c`, `src/codegen/functions_call/tail.rs` — `opal_runtime_init()` calls `SetConsoleOutputCP(65001)` to enable UTF-8 console output on Windows. However, `opal_runtime.c` is not included in `RUNTIME_SOURCE` and the generated `main` wrapper (`emit_c_main_wrapper`) never calls `opal_runtime_init`. As a result, Unicode output from compiled programs is corrupted on Windows consoles that default to a non-UTF-8 codepage.

- [ ] **16. Filesystem path buffer capped at 260 bytes (`MAX_PATH` limit)**
  `runtime/opal_portability.h` — `OPAL_PATH_BUFFER_CAP` is defined as `260` on Windows, matching the legacy `MAX_PATH` constant. Paths longer than 260 bytes (valid on Windows 10+ with long-path support enabled) will be silently truncated or cause buffer overflows in any `fs` function that uses a stack-allocated path buffer.

## Build System

- [ ] **12. `Cargo.toml` still includes `"llvm14-0-prefer-dynamic"`**
  `Cargo.toml` — The Windows CI job works around this by stripping the feature with `sed` at CI time (a fragile scripted workaround). On a native Windows build without the CI script, `inkwell` will request a dynamically-linked LLVM. If the LLVM `.dll` is not on `PATH`, the compiler binary fails to start before executing any user code.

- [ ] **13. CI `cross-msvc-from-linux` job installs `xwin` without version pinning**
  `.github/workflows/ci.yml` — `cargo install xwin --locked` fetches the latest version each run. A breaking `xwin` release could silently break cross-compilation from Linux to Windows.

- [ ] **17. MSVC linker invocation missing `bcrypt.lib`**
  `src/build_system/linker.rs` — `msvc_shared_args()` only passes `/DEFAULTLIB:libcmt` to the linker. `opal_rng.c` calls `BCryptGenRandom`, which requires `bcrypt.lib` at link time. The MinGW path correctly passes `-lbcrypt` via `mingw_crt_libs()`, but the MSVC path does not, causing an unresolved-external link error for any program that uses the RNG stdlib.

- [ ] **18. `opal_rc.c` is absent from `RUNTIME_SOURCE`**
  `src/compiler.rs` — The `RUNTIME_SOURCE` constant concatenates all C runtime files written to the temp directory before compilation. `opal_rc.c` (reference-counting allocator) is missing from this list. `src/codegen/rc_emitter.rs` declares `opal_rc_alloc`, `opal_rc_inc`, `opal_rc_dec`, `opal_rc_drop_iterative`, `opal_weak_alloc`, `opal_weak_upgrade`, and `opal_weak_dec` as external symbols, so any compiled program that exercises RC will fail to link with unresolved-external errors on all platforms, but it is categorised here because Windows is the primary target for the MSVC toolchain path.

- [ ] **19. Missing `XWIN_CACHE` / `OPAL_XWIN_SYSROOT` panics instead of returning an error**
  `src/build_system/linker.rs` — `build_msvc()` on a non-Windows host calls `.expect("XWIN_CACHE env var required…")` when neither `XWIN_CACHE` nor `OPAL_XWIN_SYSROOT` is set. This terminates the compiler process with a panic instead of propagating a structured diagnostic. Users who omit the env var get an unformatted panic trace rather than an actionable error message.

- [ ] **20. `quote_if_needed` wraps paths in literal quote characters**
  `src/build_system/linker.rs` — `quote_if_needed()` prepends and appends `"` bytes to path strings that contain spaces, then passes the result to `std::process::Command::arg()`. `Command::arg` already handles OS-level argument quoting; adding literal quote characters produces an argument whose value on the child process's command line includes the quote bytes as part of the path, causing the linker to fail with a "file not found" error for any path that contains spaces (e.g., `/home/user name/…`).

## Hot-Reload

- [ ] **14. Windows `.dll` hot-reload lacks copy-before-load (files locked by the OS)**
  `src/hot_reload/loader.rs` — Windows locks a `.dll` while it is loaded. To hot-swap a module, the new DLL must be copied to a uniquely named temporary file before being loaded (otherwise recompiling the original `.dll` fails with a sharing-violation error). This copy-before-load mechanism is not implemented; hot-reload on Windows will fail whenever a program stays running while a module is recompiled.
