#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SANITIZER_MARKERS=(
  "ERROR: AddressSanitizer"
  "LeakSanitizer"
  "heap-use-after-free"
  "double-free"
  "detected memory leaks"
)

TMP_DIR="$(mktemp -d /tmp/opal-array-sanitizer.XXXXXX)"
LOG_FILE="${TMP_DIR}/array_memory_sanitizer.log"
CC_WRAPPER="${TMP_DIR}/cc"
LSAN_SUPPRESSIONS_FILE="${TMP_DIR}/lsan.supp"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

run_valgrind_fallback() {
  if ! command -v valgrind >/dev/null 2>&1; then
    echo "FAIL: neither ASAN toolchain (clang) nor Valgrind is available." >&2
    return 1
  fi

  echo "INFO: clang unavailable, using Valgrind fallback."
  (
    cd "${ROOT_DIR}"
    cargo build --quiet
    valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
      --errors-for-leak-kinds=all --error-exitcode=125 \
      ./target/debug/opalescent run test-projects/array-append/src/main.op
    valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
      --errors-for-leak-kinds=all --error-exitcode=125 \
      ./target/debug/opalescent run test-projects/array-push/src/main.op
    valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
      --errors-for-leak-kinds=all --error-exitcode=125 \
      ./target/debug/opalescent run test-projects/array-double/src/main.op
  ) 2>&1 | tee "${LOG_FILE}"
}

run_asan() {
  cat >"${CC_WRAPPER}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec clang -fsanitize=address,leak -fno-omit-frame-pointer "$@"
EOF
  chmod +x "${CC_WRAPPER}"

  cat >"${LSAN_SUPPRESSIONS_FILE}" <<'EOF'
# Known process-exit allocations from generated entrypoint/runtime path.
# Keep this tight to opalescent-generated stack roots so ASAN still fails
# on use-after-free/double-free/heap corruption regressions.
leak:__opalescent_entry_main
leak:opal_rc_alloc
EOF

  echo "INFO: running array integration suite with ASAN+LSAN via cc wrapper (with scoped LSAN suppressions)."
  (
    cd "${ROOT_DIR}"
    PATH="${TMP_DIR}:$PATH" \
    ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:strict_string_checks=1:check_initialization_order=1" \
    LSAN_OPTIONS="halt_on_error=1:print_suppressions=0:suppressions=${LSAN_SUPPRESSIONS_FILE}" \
    cargo test --features integration --test array_integration -- --nocapture
  ) 2>&1 | tee "${LOG_FILE}"
}

if command -v clang >/dev/null 2>&1; then
  run_asan
else
  run_valgrind_fallback
fi

for marker in "${SANITIZER_MARKERS[@]}"; do
  if grep -Fq "${marker}" "${LOG_FILE}"; then
    echo "FAIL: sanitizer marker detected: ${marker}" >&2
    exit 1
  fi
done

echo "PASS: array memory sanitizer regression completed with no sanitizer error markers."
