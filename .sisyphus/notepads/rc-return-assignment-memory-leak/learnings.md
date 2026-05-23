## 2026-05-23T00:43:19-04:00
- Task 1 regression coverage lives cleanly in `tests/integration_e2e/rc_store_leak_regressions.rs` without changing the shared C fixture because the existing harness already emits parser-compatible `rc_store_counter:*` and heap status lines.
- A mutable `board = next_generation(...)` loop over 128 iterations reproduces the leak deterministically before the compiler fix, yielding `alloc=514 free=386 live=128` and `rc_store_live_heap_bytes=5632`.
- An ignored characterization test can document the alias-return limitation by explicitly panicking with observed counters after a successful harness run, keeping the behavior discoverable without gating the normal suite.
