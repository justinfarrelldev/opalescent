# Issues — cli-commands-wiring

## Known Issues / Gotchas
- 6 existing tests assert `Err(1)` for unimplemented commands — Task 1 updates these (RED phase)
- `opal build` test writes to filesystem — use temp dirs to avoid test pollution
- `opal run` must be intercepted BEFORE the filename-fallback path or "run" gets treated as a filename
- Watch loop must NOT be entered during unit tests — test only arg parsing + file validation
- `FileWatcher` trait must be imported alongside `PollingFileWatcher` for method calls to work
