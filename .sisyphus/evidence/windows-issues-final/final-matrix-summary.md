# Windows Issues Final Matrix Summary

## Commands and Status

- `cargo test --all-features --workspace` → **PASS** (`EXIT_CODE=0`)
  - Artifact: `linux-tests.txt`
  - Result: the workspace test run is green on this Linux host with the current LLVM linkage setup.
- `cargo clippy --all-targets --all-features -- -D warnings` → **PASS** (`EXIT_CODE=0`)
  - Artifact: `clippy.txt`
- `cargo fmt --all -- --check` → **PASS** (`EXIT_CODE=0`)
  - Artifact: `fmt-check.txt`
- `bash scripts/verify-wine-prereqs.sh` → **PASS** (`EXIT_CODE=0`)
  - Artifact: `wine-prereqs.txt`
  - Result: Wine, `clang-cl`, `xwin 0.9.0`, and LLVM 14 are all present for the final Wine/MSVC gate on this host.
- `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` → **PASS** (`EXIT_CODE=0`)
  - Artifact: `wine-msvc-file-ops.txt`
  - Result: the final Wine/MSVC filesystem integration gate completed successfully.
- `cargo run --release -- test-projects/hello-world/src/main.op --target x86_64-pc-windows-msvc` + `wine target/program.exe` → **PASS** (`BUILD_EXIT_CODE=0`, `WINE_EXIT_CODE=0`)
  - Artifact: `hello-world-msvc-wine.txt`
  - Result: cross-compilation and direct Wine execution both succeed.

## Toolchain Closure Checks

- `Cargo.toml` no longer contains `llvm14-0-prefer-dynamic`
- Linux/non-Windows builds still prefer dynamic LLVM 14 via direct `llvm-sys` feature unification, while Windows keeps plain `llvm14-0`
- Repo-local `.cargo/config.toml` is no longer checked in; final evidence uses explicit command environment instead of host-bound repo config
- `.github/workflows/ci.yml` still pins `xwin` to `0.9.0` with `--locked`
- `.github/workflows/ci.yml` still does not patch `Cargo.toml` with `sed`
- `Makefile.toml` still uses the `integration windows-wine` feature pair
- `scripts/verify-wine-prereqs.sh` reports configured tools deterministically and currently passes on this host

Reference artifact: `toolchain-summary.txt`

## Final Readout

The final Windows/Wine closure artifacts now match the current passing host state: prereqs pass, the canonical `wine_msvc_file_ops` gate passes, and the hello-world MSVC build runs successfully under Wine. Remaining non-final artifacts outside this closure bundle should be interpreted separately from this final matrix.
