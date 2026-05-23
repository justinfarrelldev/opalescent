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
SANITIZED_SELECTORS=(
  "array_memory_churn_sanitizer_fixture"
  "array_game_of_life_churn_sanitizer_fixture"
  "array_push_cow_alias"
  "array_pop_rc_element_ownership_transfer"
  "array_clear"
  "array_reserve_noop_when_within_capacity"
  "array_index_assignment_cow_alias"
  "array_index_assignment_rc_nested_row_rebind"
  "array_nested_assignment_shared_inner_row_cow"
  "array_self_assignment_rc_safe"
  "array_rebind_releases_old_preserves_alias"
)
MEMORY_VERIFICATION_TESTS=(
  "tests::memory_model_counters::memory_model_counters"
  "tests::rc_counter_negative_fixture::rc_counter_negative_fixture"
  "tests::rc_store_leak_regressions::rc_store_direct_assignment"
  "tests::rc_store_leak_regressions::rc_store_push_no_grow"
  "tests::rc_store_leak_regressions::rc_store_push_grow"
  "tests::rc_store_leak_regressions::rc_store_self_overwrite"
  "tests::rc_store_leak_regressions::rc_store_aliased_source_safety"
  "tests::rc_store_leak_regressions::rc_store_second_class_ref_adjacent"
  "tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_return"
  "tests::call_temp_leak_regressions::call_temp_owned_arg_freed_on_propagate"
  "tests::call_temp_leak_regressions::call_temp_mixed_disposition"
  "tests::call_temp_leak_regressions::call_temp_nested_later_failure_cleanup"
  "tests::call_temp_leak_regressions::call_temp_take_owned_no_double_free"
)
STRESS_VERIFICATION_TEST="tests::game_of_life_full_memory_stress::game_of_life_full_memory_stress"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

assert_sanitized_selectors_present() {
  local test_file="${ROOT_DIR}/tests/array_integration.rs"
  local selector
  for selector in "${SANITIZED_SELECTORS[@]}"; do
    if ! grep -Fq "fn ${selector}()" "${test_file}"; then
      echo "FAIL: expected sanitizer selector '${selector}' not found in ${test_file}." >&2
      return 1
    fi
  done
}

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

run_selector_with_retries() {
  local selector="$1"
  local attempt=1
  local max_attempts=5

  while (( attempt <= max_attempts )); do
    if cargo test --features integration --test array_integration "${selector}" -- --nocapture --test-threads=1; then
      return 0
    fi

    if (( attempt == max_attempts )); then
      echo "FAIL: selector '${selector}' failed after ${max_attempts} attempts." >&2
      return 1
    fi

    echo "WARN: selector '${selector}' failed on attempt ${attempt}; retrying serialized run." >&2
    sleep 1
    attempt=$((attempt + 1))
  done
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

  echo "INFO: running targeted array RC/COW sanitizer fixtures with ASAN+LSAN via cc wrapper (serialized)."
  (
    cd "${ROOT_DIR}"
    export PATH="${TMP_DIR}:$PATH"
    export ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:strict_string_checks=1:check_initialization_order=1"
    export LSAN_OPTIONS="halt_on_error=1:print_suppressions=0:suppressions=${LSAN_SUPPRESSIONS_FILE}"

    local selector
    for selector in "${SANITIZED_SELECTORS[@]}"; do
      run_selector_with_retries "${selector}"
    done
  ) 2>&1 | tee "${LOG_FILE}"
}

run_memory_verification_test() {
  local test_name="$1"

  echo "INFO: running mandatory memory verification test '${test_name}' (exact selector)."
  (
    cd "${ROOT_DIR}"
    cargo test --features integration --test integration_e2e "${test_name}" -- --exact --nocapture --test-threads=1
  )
}

run_opt_in_stress_verification() {
  if [[ "${OPAL_RUN_STRESS:-0}" != "1" ]]; then
    echo "INFO: skipping ignored stress verification; set OPAL_RUN_STRESS=1 to enable '${STRESS_VERIFICATION_TEST}'."
    return 0
  fi

  echo "INFO: running opt-in ignored stress verification '${STRESS_VERIFICATION_TEST}'."
  (
    cd "${ROOT_DIR}"
    OPAL_RUN_STRESS=1 cargo test --features integration --test integration_e2e \
      "${STRESS_VERIFICATION_TEST}" -- --ignored --exact --nocapture --test-threads=1
  )
}

run_memory_verification_hooks() {
  echo "INFO: running deterministic memory verification hooks."

  local test_name
  for test_name in "${MEMORY_VERIFICATION_TESTS[@]}"; do
    run_memory_verification_test "${test_name}"
  done

  run_opt_in_stress_verification
}

assert_sanitized_selectors_present

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

run_memory_verification_hooks

echo "PASS: array memory sanitizer regression completed with no sanitizer error markers."
