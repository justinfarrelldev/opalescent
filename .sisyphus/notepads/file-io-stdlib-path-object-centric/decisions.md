# Decisions — file-io-stdlib-path-object-centric

## [2026-04-22] Locked design decisions (from user Q&A)

1. Path construction: BOTH `path_from(raw)` builtin AND `new FilesystemPath:` record constructor
2. Text encoding: UTF-8 strict with `InvalidUtf8Error`; byte-oriented escape hatches retained
3. Line endings: Split on `\n`, strip single trailing `\r`
4. Write atomicity: BOTH naive (`write_*_sync`) AND atomic (`write_*_atomic_sync`)
5. Symlinks: Content ops follow target; metadata/inspection have `_nofollow_sync` variants; path manipulation lexical-only
6. Permissions: Abstract triple `{readable, writable, executable}`. Mapping: read→0400, write→0200, execute→0100 (owner only; group/others preserved)
7. Platform scope: Linux-only CI; Windows parity → `.sisyphus/followups.md`
8. Pre-Flight Validation: MANDATORY T0 — verify `FilesystemPath[]`, `string[]` returns + scalar fallible-ABI work from stdlib builtins

## [2026-04-22] T0 evidence capture decision
- For acceptance-style stdout matching, capture canonical output from `./target/program` after successful `opal <file>` compile step, because `opal --run` prepends `target/program` line that is outside the expected program stdout payload.
