# Problems


- Task 6 gating remains blocked by repository-wide non-GoL failures: baseline-existing integration_e2e failures (fs_markdown_roundtrip, fs_rerunnability) plus out-of-scope clippy/fmt hygiene drift in type_system/codegen/tests files.
- These blockers are unrelated to the GoL probe-bin removal and should be tracked as inherited global gate debt while allowing GoL-removal verification to proceed with documented context.
