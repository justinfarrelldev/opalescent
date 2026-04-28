# T34: MSVC Compile + Link Final Verification

**Date:** 2026-04-28  
**Status:** PASS  
**Host:** Linux x86_64 (cross-compilation via clang-cl + lld-link + xwin sysroot)

---

## Environment

| Tool | Version / Path |
|------|---------------|
| `clang-cl` | `clang-cl-14` (Debian clang 14.0.6, Target: x86_64-pc-windows-msvc) |
| `lld-link` | Debian LLD 14.0.6 (`/usr/bin/lld-link`) |
| `nm` | GNU nm (GNU Binutils for Debian) 2.40 |
| `xwin` | v0.2.0 (installed via `cargo install xwin@0.2.0`) |
| `XWIN_CACHE` | `~/.xwin` (splat via `xwin --accept-license splat --output ~/.xwin`) |

### xwin sysroot structure
```text
~/.xwin/
  crt/include/        — MSVC CRT headers
  crt/lib/x86_64/     — MSVC CRT libs (libucrt.lib, libcmt.lib, ...)
  sdk/include/ucrt/
  sdk/include/um/
  sdk/include/shared/
  sdk/lib/ucrt/x86_64/
  sdk/lib/um/x86_64/
```

---

## Verified Commands

```bash
XWIN_CACHE="$HOME/.xwin" bash scripts/msvc_link_probe.sh --full
XWIN_CACHE="$HOME/.xwin" bash scripts/msvc_link_probe.sh --report-undefined
XWIN_CACHE="$HOME/.xwin" bash scripts/msvc_link_probe.sh
cargo build
```

### CI-equivalent shell snippet
```bash
export XWIN_CACHE="$HOME/.xwin"
bash scripts/msvc_link_probe.sh --full 2>&1 | tee .sisyphus/evidence/task-34-msvc.log
bash scripts/msvc_link_probe.sh --report-undefined 2>&1 | tee .sisyphus/evidence/task-34-undef.log
cargo build 2>&1 | tee .sisyphus/evidence/task-34-cargo-build.log
```

---

## Full-runtime probe behavior

`bash scripts/msvc_link_probe.sh --full` now performs these steps:

1. Resolves `clang-cl` by probing `clang-cl`, then versioned fallbacks (`clang-cl-14` through `clang-cl-18`).
2. Compiles every `runtime/opal_*.c` except `runtime/opal_msvc_link_probe.c` into `target/msvc-probe/*.obj`.
3. Links the full runtime with `lld-link /dll /noentry` into `target/msvc-probe/runtime.dll`.
4. Produces the matching import library `target/msvc-probe/runtime.lib` and exports `read_text_sync` for smoke-link validation.
5. Compiles `runtime/opal_msvc_link_probe.c` separately and links the smoke harness against `target/msvc-probe/runtime.lib`.
6. Reports unresolved symbols from the import library in machine-readable form, filtering import-library bookkeeping entries.

### Accepted output lines
```text
all object files linked: target/msvc-probe/runtime.dll
0 undefined symbols
MSVC LINK PROBE: PASS
```

---

## Result

### `bash scripts/msvc_link_probe.sh --full`
```text
all object files linked: target/msvc-probe/runtime.dll
0 undefined symbols
MSVC LINK PROBE: PASS
```

Artifacts produced:
- `target/msvc-probe/runtime.dll`
- `target/msvc-probe/runtime.lib`
- `target/msvc-probe/opal_probe.exe`

### `bash scripts/msvc_link_probe.sh --report-undefined`
```text
0 undefined symbols
```

### `bash scripts/msvc_link_probe.sh`
```text
MSVC LINK PROBE: PASS
```

### `cargo build`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

---

## Notes

- Full-runtime linking required `bcrypt.lib` because `runtime/opal_rng.c` uses `BCryptGenRandom` on Windows.
- Smoke-link validation must consume `runtime.lib`, not `runtime.dll`; `lld-link` rejects the DLL directly for that purpose.
- `nm` on MSVC import libraries reports bookkeeping symbols such as `__NULL_IMPORT_DESCRIPTOR` and `*_NULL_THUNK_DATA`; the report mode filters those so `0 undefined symbols` reflects real unresolved runtime imports.
- `clang-cl` still emits non-fatal `static_assert` extension warnings from `OPAL_STATIC_ASSERT` in `opal_rc.h` / `opal_rc.c`; these did not block compile or link success.
