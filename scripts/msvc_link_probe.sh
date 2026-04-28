#!/usr/bin/env bash
set -euo pipefail

# Run this after every runtime/ change touched by tasks T15+.
# Required for Regression Gate compliance.

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

usage() {
  printf 'Usage: %s [--full|--report-undefined]\n' "$0" >&2
  exit 1
}

MODE='quick'
case "${1:-}" in
  '')
    ;;
  --full)
    MODE='full'
    ;;
  --report-undefined)
    MODE='report_undefined'
    ;;
  -h|--help)
    usage
    ;;
  *)
    usage
    ;;
esac

if [[ -z "${XWIN_CACHE:-}" ]]; then
  fail "XWIN_CACHE is not set. Set XWIN_CACHE to your xwin sysroot (for example: export XWIN_CACHE=\$HOME/.xwin)."
fi

if [[ ! -d "$XWIN_CACHE" ]]; then
  fail "XWIN_CACHE path does not exist: $XWIN_CACHE"
fi

CLANG_CL=''
for candidate in clang-cl clang-cl-14 clang-cl-15 clang-cl-16 clang-cl-17 clang-cl-18; do
  if command -v "$candidate" >/dev/null 2>&1; then
    CLANG_CL="$candidate"
    break
  fi
done
if [[ -z "$CLANG_CL" ]]; then
  fail "clang-cl not found in PATH (tried: clang-cl clang-cl-14 through clang-cl-18)"
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
  /D_CRT_SECURE_NO_WARNINGS
  /D_CRT_NONSTDC_NO_WARNINGS
)

LIB_FLAGS=(
  kernel32.lib
  bcrypt.lib
  libucrt.lib
  libcmt.lib
  /libpath:"$XWIN_CACHE/crt/lib/x86_64"
  /libpath:"$XWIN_CACHE/sdk/lib/um/x86_64"
  /libpath:"$XWIN_CACHE/sdk/lib/ucrt/x86_64"
)

PROBE_DIR='target/msvc-probe'
RUNTIME_OBJ='runtime/opal_fs.obj'
PROBE_OBJ='runtime/opal_msvc_link_probe.obj'
OUT_EXE='/tmp/opal_probe.exe'
PROBE_SYMBOL='read_text_sync'
FULL_RUNTIME_DLL="$PROBE_DIR/runtime.dll"
FULL_RUNTIME_LIB="$PROBE_DIR/runtime.lib"
FULL_SMOKE_EXE="$PROBE_DIR/opal_probe.exe"

compile_object() {
  local source_path="$1"
  local object_path="$2"

  "$CLANG_CL" /nologo /c "${XWIN_FLAGS[@]}" "$source_path" "/Fo${object_path}" \
    || fail "clang-cl failed compiling $source_path"
}

read_nm() {
  local object_path="$1"
  local nm_output

  nm_output="$($NM_TOOL "$object_path" 2>/dev/null || true)"
  if [[ -z "$nm_output" ]]; then
    fail "$NM_TOOL produced no output for $object_path"
  fi

  printf '%s\n' "$nm_output"
}

check_symbol_discipline() {
  local probe_nm
  local runtime_nm
  local probe_has_undef=0
  local runtime_has_def=0
  local probe_has_def=0

  probe_nm="$(read_nm "$PROBE_OBJ")"
  runtime_nm="$(read_nm "$RUNTIME_OBJ")"

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
}

report_undefined_symbols() {
  local artifact_path="$1"
  local nm_output
  local undefined_count=0
  local line
  local symbol_name

  nm_output="$($NM_TOOL "$artifact_path" 2>/dev/null || true)"
  if [[ -z "$nm_output" ]]; then
    fail "$NM_TOOL produced no output for $artifact_path"
  fi

  while IFS= read -r line; do
    if [[ ! "$line" =~ (^|[[:space:]])U[[:space:]] ]]; then
      continue
    fi

    symbol_name="${line##* }"
    case "$symbol_name" in
      __NULL_IMPORT_DESCRIPTOR|__IMPORT_DESCRIPTOR_*|*_NULL_THUNK_DATA)
        continue
        ;;
    esac

    undefined_count=$((undefined_count + 1))
  done <<< "$nm_output"

  printf '%s undefined symbols\n' "$undefined_count"

  if [[ "$undefined_count" -ne 0 ]]; then
    fail "undefined symbols remain in $artifact_path"
  fi
}

run_quick_probe() {
  rm -f "$RUNTIME_OBJ" "$PROBE_OBJ" "$OUT_EXE"

  compile_object 'runtime/opal_fs.c' "$RUNTIME_OBJ"
  compile_object 'runtime/opal_msvc_link_probe.c' "$PROBE_OBJ"
  check_symbol_discipline

  lld-link /nologo \
    /subsystem:console \
    /entry:main \
    /out:"$OUT_EXE" \
    "$PROBE_OBJ" "$RUNTIME_OBJ" \
    "${LIB_FLAGS[@]}" \
    || fail "lld-link failed linking probe executable"

  printf 'MSVC LINK PROBE: PASS\n'
}

compile_full_runtime() {
  local runtime_sources=()
  local source_path
  local object_path
  local base_name

  mkdir -p "$PROBE_DIR"
  rm -f "$PROBE_DIR"/*.obj "$FULL_RUNTIME_DLL" "$FULL_RUNTIME_LIB" "$FULL_SMOKE_EXE"

  for source_path in runtime/opal_*.c; do
    if [[ "$source_path" == 'runtime/opal_msvc_link_probe.c' ]]; then
      continue
    fi

    runtime_sources+=("$source_path")
    base_name="${source_path##*/}"
    object_path="$PROBE_DIR/${base_name%.c}.obj"
    compile_object "$source_path" "$object_path"
  done

  if [[ "${#runtime_sources[@]}" -eq 0 ]]; then
    fail "no runtime sources matched runtime/opal_*.c"
  fi
}

link_full_runtime() {
  local runtime_objects=("$PROBE_DIR"/*.obj)

  if [[ ! -e "${runtime_objects[0]}" ]]; then
    fail "full runtime build produced no object files"
  fi

  lld-link /nologo \
    /dll \
    /noentry \
    "/export:${PROBE_SYMBOL}" \
    /out:"$FULL_RUNTIME_DLL" \
    "${runtime_objects[@]}" \
    "${LIB_FLAGS[@]}" \
    || fail "lld-link failed linking full runtime artifact"

  if [[ ! -f "$FULL_RUNTIME_DLL" ]]; then
    fail "full runtime link did not produce $FULL_RUNTIME_DLL"
  fi
  if [[ ! -f "$FULL_RUNTIME_LIB" ]]; then
    fail "full runtime link did not produce $FULL_RUNTIME_LIB"
  fi

  printf 'all object files linked: %s\n' "$FULL_RUNTIME_DLL"
}

link_full_smoke_harness() {
  compile_object 'runtime/opal_msvc_link_probe.c' "$PROBE_OBJ"

  lld-link /nologo \
    /subsystem:console \
    /entry:main \
    /out:"$FULL_SMOKE_EXE" \
    "$PROBE_OBJ" "$FULL_RUNTIME_LIB" \
    "${LIB_FLAGS[@]}" \
    || fail "lld-link failed linking full smoke harness"

  if [[ ! -f "$FULL_SMOKE_EXE" ]]; then
    fail "full smoke harness link did not produce $FULL_SMOKE_EXE"
  fi
}

run_full_probe() {
  rm -f "$PROBE_OBJ"
  compile_full_runtime
  link_full_runtime
  link_full_smoke_harness
  report_undefined_symbols "$FULL_RUNTIME_LIB"
  printf 'MSVC LINK PROBE: PASS\n'
}

case "$MODE" in
  quick)
    run_quick_probe
    ;;
  full)
    run_full_probe
    ;;
  report_undefined)
    if [[ ! -f "$FULL_RUNTIME_LIB" ]]; then
      fail "$FULL_RUNTIME_LIB does not exist. Run $0 --full first"
    fi
    report_undefined_symbols "$FULL_RUNTIME_LIB"
    ;;
  *)
    fail "unsupported mode: $MODE"
    ;;
esac
