#!/usr/bin/env bash
set -euo pipefail

# Run this after every runtime/ change touched by tasks T15+.
# Required for Regression Gate compliance.

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

if [[ -z "${XWIN_CACHE:-}" ]]; then
  fail "XWIN_CACHE is not set. Set XWIN_CACHE to your xwin sysroot (for example: export XWIN_CACHE=\$HOME/.xwin)."
fi

if [[ ! -d "$XWIN_CACHE" ]]; then
  fail "XWIN_CACHE path does not exist: $XWIN_CACHE"
fi

if ! command -v clang-cl >/dev/null 2>&1; then
  fail "clang-cl not found in PATH"
fi

if ! command -v lld-link >/dev/null 2>&1; then
  fail "lld-link not found in PATH"
fi

NM_TOOL=''
if command -v llvm-nm >/dev/null 2>&1; then
  NM_TOOL='llvm-nm'
elif command -v nm >/dev/null 2>&1; then
  NM_TOOL='nm'
else
  fail "Neither llvm-nm nor nm found in PATH"
fi

XWIN_FLAGS=(
  /imsvc "$XWIN_CACHE/crt/include"
  /imsvc "$XWIN_CACHE/sdk/include/ucrt"
  /imsvc "$XWIN_CACHE/sdk/include/um"
  /imsvc "$XWIN_CACHE/sdk/include/shared"
)

RUNTIME_OBJ='runtime/opal_fs.obj'
PROBE_OBJ='runtime/opal_msvc_link_probe.obj'
OUT_EXE='/tmp/opal_probe.exe'
PROBE_SYMBOL='read_text_sync'

rm -f "$RUNTIME_OBJ" "$PROBE_OBJ" "$OUT_EXE"

clang-cl /nologo /c "${XWIN_FLAGS[@]}" runtime/opal_fs.c -Fo:"$RUNTIME_OBJ" \
  || fail "clang-cl failed compiling runtime/opal_fs.c"

clang-cl /nologo /c "${XWIN_FLAGS[@]}" runtime/opal_msvc_link_probe.c -Fo:"$PROBE_OBJ" \
  || fail "clang-cl failed compiling runtime/opal_msvc_link_probe.c"

# Single-definition discipline checks:
# - Probe object must reference runtime symbols as undefined.
# - Runtime object must define runtime symbols.
probe_nm="$($NM_TOOL "$PROBE_OBJ" 2>/dev/null || true)"
runtime_nm="$($NM_TOOL "$RUNTIME_OBJ" 2>/dev/null || true)"

if [[ -z "$probe_nm" ]]; then
  fail "$NM_TOOL produced no output for $PROBE_OBJ"
fi
if [[ -z "$runtime_nm" ]]; then
  fail "$NM_TOOL produced no output for $RUNTIME_OBJ"
fi

probe_has_undef=0
runtime_has_def=0
probe_has_def=0

if printf '%s\n' "$probe_nm" | grep -Eq "[[:space:]]U[[:space:]].*${PROBE_SYMBOL}|^[[:space:]]*U[[:space:]].*${PROBE_SYMBOL}"; then
  probe_has_undef=1
fi
if printf '%s\n' "$runtime_nm" | grep -Eq "[[:space:]]T[[:space:]].*${PROBE_SYMBOL}|^[0-9A-Fa-f]+[[:space:]]T[[:space:]].*${PROBE_SYMBOL}"; then
  runtime_has_def=1
fi
if printf '%s\n' "$probe_nm" | grep -Eq "[[:space:]]T[[:space:]].*${PROBE_SYMBOL}|^[0-9A-Fa-f]+[[:space:]]T[[:space:]].*${PROBE_SYMBOL}"; then
  probe_has_def=1
fi

if [[ "$probe_has_undef" -ne 1 ]]; then
  fail "symbol-discipline check failed: $PROBE_SYMBOL is not undefined in $PROBE_OBJ"
fi
if [[ "$runtime_has_def" -ne 1 ]]; then
  fail "symbol-discipline check failed: $PROBE_SYMBOL is not defined in $RUNTIME_OBJ"
fi
if [[ "$probe_has_def" -eq 1 ]]; then
  fail "duplicate definition detected: $PROBE_SYMBOL is defined in probe object and runtime object"
fi

lld-link /nologo \
  /subsystem:console \
  /entry:main \
  /out:"$OUT_EXE" \
  "$PROBE_OBJ" "$RUNTIME_OBJ" \
  kernel32.lib libucrt.lib libcmt.lib \
  /libpath:"$XWIN_CACHE/crt/lib/x86_64" \
  /libpath:"$XWIN_CACHE/sdk/lib/um/x86_64" \
  /libpath:"$XWIN_CACHE/sdk/lib/ucrt/x86_64" \
  || fail "lld-link failed linking probe executable"

printf 'MSVC LINK PROBE: PASS\n'
