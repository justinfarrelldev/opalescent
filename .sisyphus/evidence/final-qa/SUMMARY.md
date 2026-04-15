# Final QA — Manual CLI Test Results
Date: Tue Apr 14 2026
Binary: ./target/release/opalescent (built fresh, `cargo build --release`)

## Test Results

| # | Command | Expected | Exit | Result |
|---|---------|----------|------|--------|
| 01 | `opal help` | Prints usage, all commands | 0 | PASS |
| 02 | `opal --help` | Identical to help | 0 | PASS |
| 03 | `opal pkg status` | "not yet implemented" | 1 | PASS |
| 04 | `opal fmt` (no file) | Error + exit 1 | 1 | PASS |
| 05 | `opal lsp` (no --stdio) | Error + exit 1 | 1 | PASS |
| 06 | `opal lsp --stdio` | Ready message + exit 0 | 0 | PASS |
| 07 | `opal test` | Empty suite, exit 0 | 0 | PASS |
| 08 | `opal doc` (no file) | Error + exit 1 | 1 | PASS |
| 09 | `opal bench` | Benchmarks ran, exit 0 | 0 | PASS |
| 10 | `opal run` (no file) | Error + exit 1 | 1 | PASS |
| 11 | `opal check` (no file) | Error + exit 1 | 1 | PASS |
| 12 | `opal build` (no opal.toml) | "no opal.toml" error + exit 1 | 1 | PASS |
| 13 | `opal watch` (no file) | Error + exit 1 | 1 | PASS |

## Help Content Verification

Commands present in help output:
- [x] fmt
- [x] lsp
- [x] test
- [x] doc
- [x] bench
- [x] run
- [x] check
- [x] build
- [x] watch
- [x] pkg

## VERDICT: APPROVE
