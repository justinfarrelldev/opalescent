# F3 Manual QA (rerun)

Reviewer: current assistant acting manually; no subagent tool is available in this environment.

Result: APPROVED for available deterministic harness coverage.

Commands run:
- `cargo test --features integration --test integration_e2e terminal_session_ --quiet` -> PASS (`4` passed, `4` ignored compile-gap probes).
- `cargo test terminal_lifecycle_api_remains_codegen_gated_until_c_abi_exists --quiet` -> PASS.
- `cargo test terminal_security --quiet` -> PASS.
- Prior Task 37 full-suite evidence remains available for:
  - `timeout 900 cargo test --all-features`
  - `cargo test --features integration`
  - `cargo make c-quality`

QA notes:
- The 14 selected terminal fixtures remain listed in the default fixture inventory and deterministic-source checks.
- Ignored opt-in probes now document generated-program lowering gaps instead of claiming terminal support has not landed.
