# Learnings — windows-support

## [2026-04-21] Session start

### Cargo.toml state
- inkwell = { version = "0.8.0", features = ["llvm14-0", "llvm14-0-prefer-dynamic"] }
- No build.rs, no rust-toolchain.toml, no .cargo/config.toml
- .github/workflows/ does NOT exist yet

### Key file locations (ground-truth verified in planning sessions)
- `src/build_system.rs` is the module file (NOT `src/build_system/mod.rs`)
- `src/build_system/targets.rs` — TargetTriple, parse_target_triple, dynamic_lib_extension (92 lines)
- `src/build_system/tests.rs` — existing tests including parse_legacy_2_segment_still_works
- `src/compiler.rs` — build_linker_command at lines 289-342; hardcoded .o at 419-420
- `src/hot_reload/version.rs:41-55` — shared_library_extension() is param-less const fn using HOST cfg
- `scripts/check-line-count.sh` exists; Makefile.toml has NO check-line-count task

### Architecture decisions (locked)
- MSVC primary + MinGW best-effort
- Linux→Windows cross: clang-cl + lld-link + xwin sysroot (NOT xwin-as-toolchain)
- xwin splat layout: $XWIN_CACHE/crt/{include,lib/x86_64}, $XWIN_CACHE/sdk/{include,lib}/{ucrt,um,shared}/x86_64
- Static LLVM on all platforms
- Hot-reload: .so Linux, .dll Windows, LoadLibraryW
- zig cc BANNED
- aarch64-windows OUT OF SCOPE

## [2026-04-21] Task 0 spike implementation learnings

- Added `.github/workflows/ci.yml` with `build-windows-spike` on `windows-latest`.
- Using `KyleMayes/install-llvm-action@v2` with `version: "14.0"` works as a minimal LLVM install path for the spike.
- Setting `LLVM_SYS_140_PREFIX` from `${{ steps.llvm.outputs.llvm-path }}` in `$GITHUB_ENV` is the critical wiring step.
- Also exported `LLVM_SYS_140_USE_DEBUG_MSVCRT=NO` and `LLVM_SYS_140_FFI_WORKAROUND=YES` in workflow env setup for llvm-sys compatibility hardening.
- Artifact upload configured with `actions/upload-artifact@v4` for `target/release/opalescent.exe` and `if-no-files-found: error`.
- Kept `Cargo.toml` unchanged in this spike (did NOT remove `llvm14-0-prefer-dynamic` yet).
- Local `cargo test --all-features` currently fails due existing formatter integration golden mismatch in this environment (unrelated pre-existing failures).
- Current environment lacked `gh` CLI and GitHub credentials, so remote push / workflow run verification had to be deferred outside this shell.
