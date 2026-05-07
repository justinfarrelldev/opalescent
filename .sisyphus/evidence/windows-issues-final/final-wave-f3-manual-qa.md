# F3 Real Manual QA — Final Wave

Date: 2026-05-06
Host: Linux (`/home/justi/Projects/opalescent`)
Scope: Hands-on manual QA of user-facing Windows/Wine flows using freshly executed commands in this session

## Commands Executed and Observed Output

### 1) Wine prerequisite gate
- Command:
  - `bash scripts/verify-wine-prereqs.sh`
- Observed output:
  - `OK: wine=wine-8.0 (Debian 8.0~repack-4) (minimum 8+) clang-cl=Debian clang version 14.0.6 clang_cl_path=/usr/lib/llvm-14/bin/clang-cl xwin=xwin 0.9.0 (expected 0.9.0) xwin_cache=/home/justi/.xwin llvm=/usr/lib/llvm-14`
- Classification:
  - **PASS**

### 2) Canonical Windows/Wine file-ops integration flow (actual execution)
- Command:
  - `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops`
- Observed output (key lines):
  - `running 1 test`
  - `test tests::windows_wine::tests::wine_msvc_file_ops ... ok`
  - `test result: ok. 1 passed; 0 failed; ...`
- Classification:
  - **PASS**

### 3) Targeted recursive mkdir regression (failure-path policy guardrail)
- Command:
  - `cargo test --features integration --test integration_e2e mkdirp_accepts_existing_ancestor_directories -- --nocapture`
- Observed output (key lines):
  - `running 1 test`
  - `test tests::fs_directories::mkdirp_accepts_existing_ancestor_directories ... ok`
  - `test result: ok. 1 passed; 0 failed; ...`
- Classification:
  - **PASS**

### 4) Direct Wine rerun with harness env (user-facing CLI/Wine execution evidence)
- Commands:
  - `cargo run --release -- test-projects/windows-file-ops/src/main.op --target x86_64-pc-windows-msvc`
  - `WINEPREFIX=/tmp/opencode/wineprefix WINEDEBUG=-all WINEDEBUGGER=true wine target/program.exe`
- Observed output (key lines):
  - `target/program.exe`
  - `MARKER:DIR_CREATED=.../dir with spaces/café`
  - `MARKER:LIST_HAS_ORIGINAL=1`
  - `MARKER:LONG_PATH_OK=true`
  - `MARKER:LONG_PATH_LEN=481`
  - `MARKER:FINAL_STATUS=ok`
- Classification:
  - **PASS**

## Marker + Crash-Visibility Validation (grep-confirmed)

### Required success markers present
- Source checked: `.sisyphus/evidence/task-3-wine-msvc-file-ops-stdout.txt`
- Verified markers found:
  - `MARKER:DIR_CREATED=...`
  - `MARKER:FILE_CREATED=...`
  - `MARKER:LIST_HAS_ORIGINAL=1`
  - `MARKER:LONG_PATH_OK=true`
  - `MARKER:LONG_PATH_LEN=481`
  - `MARKER:FINAL_STATUS=ok`

### Crash signature policy preserved
- Source checked: `.sisyphus/evidence/task-3-wine-msvc-file-ops-stderr.txt`
- Grep scan for crash signatures:
  - `Unhandled page fault`
  - `starting debugger`
  - `fatal crash/dialog`
  - `could not load kernel32.dll`
  - `status c0000135`
- Result:
  - **No matches found** for this successful run.

Policy assessment:
- Crash signatures remain explicitly detectable by the harness policy (`tests/integration_e2e/windows_wine.rs` known-limitation matcher), and this run did not trigger that failure path.

## Command Log Summary

Executed in this QA pass:
1. `bash scripts/verify-wine-prereqs.sh` → PASS
2. `cargo test --features "integration windows-wine" --test integration_e2e -- --nocapture wine_msvc_file_ops` → PASS
3. `cargo test --features integration --test integration_e2e mkdirp_accepts_existing_ancestor_directories -- --nocapture` → PASS
4. `cargo run --release -- test-projects/windows-file-ops/src/main.op --target x86_64-pc-windows-msvc` → PASS
5. `WINEPREFIX=/tmp/opencode/wineprefix WINEDEBUG=-all WINEDEBUGGER=true wine target/program.exe` → PASS
6. Grep checks for required markers and crash signatures on fresh evidence files → PASS

## Gate Decision

VERDICT: **APPROVE**

Rationale:
1. The Windows/MSVC fixture compiles and executes under Wine in this session with full expected marker stream and `EXIT=0` evidence.
2. Critical behavior markers (`LIST_HAS_ORIGINAL=1`, long-path marker > 260, and final status) are present in fresh outputs.
3. Crash visibility policy remains intact and testable; no crash signatures appeared in this successful execution path.
