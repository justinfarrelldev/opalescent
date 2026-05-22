# F3 Real Manual QA — remove-gol-hardcoding

Date: 2026-05-21
Plan: `.sisyphus/plans/remove-gol-hardcoding.md`

## Required Gate Status (current workspace, real execution)

- `cargo build --all-features` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-cargo-build-all-features.txt`
- `cargo test --all-features` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-cargo-test-all-features.txt`
  - Summary: `1272 passed; 0 failed; 5 ignored` (crate tests), plus integration/doc-test suites all green.
- `cargo clippy --all-features --all-targets -- -D warnings` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-cargo-clippy-all-features-all-targets.txt`
- `cargo fmt --all -- --check` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-cargo-fmt-check.txt`
- `bash scripts/array_memory_sanitizer.sh` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-array-memory-sanitizer.txt`
  - Summary: Script reports `PASS: array memory sanitizer regression completed with no sanitizer error markers.`

## Targeted Tests Required by Plan Context

- `cargo test --features integration --test integration_e2e "game_of_life_ten_frames" -- --nocapture --test-threads=1` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-targeted-game-of-life-ten-frames.txt`
  - Output includes: `test tests::game_of_life::game_of_life_ten_frames ... ok`
- `cargo test --features integration --test integration_e2e "memory_model_counters" -- --nocapture --test-threads=1` → **PASS** (exit `0`)
  - Evidence: `.sisyphus/evidence/f3-targeted-memory-model-counters.txt`
  - Output includes: `test tests::memory_model_counters::memory_model_counters ... ok`

## Forbidden-Token Exact Sweeps (plan globs)

Commands executed exactly with:
- `--hidden --glob '!target' --glob '!.sisyphus/plans/remove-gol-hardcoding.md' --glob '!.sisyphus/evidence/**' .`

Results:
- `\bPEAK_LIVE_BYTES_LIMIT\b` → **NO HIT** (`rg` exit `1`) ✅
  - Evidence: `.sisyphus/evidence/f3-sweep-peak-limit.txt` (empty/no matches)
- `\bSTEADY_STATE_SPREAD_LIMIT\b` → **NO HIT** (`rg` exit `1`) ✅
  - Evidence: `.sisyphus/evidence/f3-sweep-spread-limit.txt` (empty/no matches)
- `\bgol_memory_probe\b` → **NO HIT** (`rg` exit `1`) ✅
  - Evidence: `.sisyphus/evidence/f3-sweep-gol-memory-probe.txt` (empty/no matches)

## Explicit Exit Code Ledger

- BUILD=0
- TEST=0
- CLIPPY=0
- FMT=0
- SANITIZER=0
- GOL10=0
- MEM_COUNTERS=0
- SWEEP_PEAK=1 (expected no-hit)
- SWEEP_SPREAD=1 (expected no-hit)
- SWEEP_PROBE=1 (expected no-hit)

VERDICT: APPROVE
