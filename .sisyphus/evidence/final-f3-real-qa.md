# Final F3 Real QA Retry (2026-05-13 02:15:23Z)

## Command Results
- `cargo build`: **PASS** (exit 0)
  - first output: `Compiling opalescent v0.1.0 (/home/justi/Projects/opalescent)`
- `cargo test`: **PASS** (exit 0)
  - first output: `running 1218 tests`
- `cargo test --features integration`: **PASS** (exit 0)
  - first output: `running 1218 tests`
- `cargo clippy --all-targets --all-features -- -D warnings`: **PASS** (exit 0)
  - first output: `Checking opalescent v0.1.0 (/home/justi/Projects/opalescent)`
- `cargo fmt --all -- --check`: **FAIL** (exit 1)
  - first output: `Diff in /home/justi/Projects/opalescent/tests/integration_e2e/fs_append_file_string.rs:41:`
  - note: failure is due to existing unrelated formatting drift across many integration test files
- `./target/release/opalescent run test-projects/delete-downloads/src/main.op`: **PASS** (exit 0)
  - first output: `target/program`
- `./target/release/opalescent run test-projects/delete-downloads-strict/src/main.op`: **PASS** (exit 0)
  - first output: `target/program`
