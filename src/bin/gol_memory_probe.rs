//! Game-of-Life heap-accounting probe harness compiler/runner.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Peak live-heap-bytes threshold enforced by this probe run.
const PEAK_LIVE_BYTES_LIMIT: usize = 102_400;
/// Allowed max spread between steady-state min/max live heap bytes.
const STEADY_STATE_SPREAD_LIMIT: usize = 1_024;

/// C harness template compiled and executed to sample runtime heap usage.
const HARNESS_TEMPLATE: &str = r#"#include "opal_rc.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void drop_bool_rows(void *obj, void ***stack, size_t *stack_top, size_t *stack_cap) {
    size_t len = opal_array_len(obj);
    void **rows = (void **)opal_array_data(obj, _Alignof(void *));
    size_t index = 0;
    for (index = 0; index < len; ++index) {
        opal_rc_drop_child(rows[index], stack, stack_top, stack_cap);
    }
}

static void set_cell(void *row, size_t column, uint8_t value) {
    uint8_t *cells = (uint8_t *)opal_array_data(row, _Alignof(uint8_t));
    cells[column] = value;
}

static uint8_t get_cell(const void *row, size_t column) {
    const uint8_t *cells = (const uint8_t *)opal_array_data_const(row, _Alignof(uint8_t));
    return cells[column];
}

static void *alloc_bool_row(size_t size) {
    return opal_array_alloc(sizeof(uint8_t), _Alignof(uint8_t), size, size, NULL);
}

static void *alloc_bool_board(size_t size) {
    size_t row_index = 0;
    void *board = opal_array_alloc(sizeof(void *), _Alignof(void *), size, size, drop_bool_rows);
    void **rows = NULL;
    if (board == NULL) {
        return NULL;
    }
    rows = (void **)opal_array_data(board, _Alignof(void *));
    for (row_index = 0; row_index < size; ++row_index) {
        rows[row_index] = alloc_bool_row(size);
        if (rows[row_index] == NULL) {
            opal_rc_dec(board);
            return NULL;
        }
    }
    return board;
}

static void seed_glider(void *board, size_t size) {
    void **rows = (void **)opal_array_data(board, _Alignof(void *));
    if (size < 3) {
        return;
    }
    set_cell(rows[0], 1, 1);
    set_cell(rows[1], 2, 1);
    set_cell(rows[2], 0, 1);
    set_cell(rows[2], 1, 1);
    set_cell(rows[2], 2, 1);
}

static uint8_t count_neighbors(void *board, size_t size, size_t row, size_t column) {
    static const int offsets[3] = {-1, 0, 1};
    void **rows = (void **)opal_array_data(board, _Alignof(void *));
    uint8_t total = 0;
    size_t dr_index = 0;
    for (dr_index = 0; dr_index < 3; ++dr_index) {
        size_t dc_index = 0;
        for (dc_index = 0; dc_index < 3; ++dc_index) {
            int dr = offsets[dr_index];
            int dc = offsets[dc_index];
            size_t neighbor_row = 0;
            size_t neighbor_column = 0;
            if (dr == 0 && dc == 0) {
                continue;
            }
            if ((dr < 0 && row == 0) || (dc < 0 && column == 0)) {
                continue;
            }
            neighbor_row = (size_t)((int)row + dr);
            neighbor_column = (size_t)((int)column + dc);
            if (neighbor_row >= size || neighbor_column >= size) {
                continue;
            }
            total = (uint8_t)(total + get_cell(rows[neighbor_row], neighbor_column));
        }
    }
    return total;
}

static int run_probe(size_t size, size_t ticks, int report_per_tick) {
    size_t tick = 0;
    size_t steady_min = 0;
    size_t steady_max = 0;
    size_t peak_live_bytes = 0;
    size_t steady_state_spread_bytes = 0;
    int steady_initialized = 0;
    void *current = NULL;
    void *next = NULL;
    void **current_rows = NULL;
    void **next_rows = NULL;

    opal_runtime_reset_heap_accounting();
    current = alloc_bool_board(size);
    next = alloc_bool_board(size);
    if (current == NULL || next == NULL) {
        fprintf(stderr, "allocation failure while building Game of Life boards\n");
        if (next != NULL) { opal_rc_dec(next); }
        if (current != NULL) { opal_rc_dec(current); }
        return 2;
    }

    seed_glider(current, size);

    for (tick = 0; tick < ticks; ++tick) {
        size_t row = 0;
        current_rows = (void **)opal_array_data(current, _Alignof(void *));
        next_rows = (void **)opal_array_data(next, _Alignof(void *));
        for (row = 0; row < size; ++row) {
            size_t column = 0;
            for (column = 0; column < size; ++column) {
                uint8_t alive = get_cell(current_rows[row], column);
                uint8_t neighbors = count_neighbors(current, size, row, column);
                uint8_t next_alive = (neighbors == 3 || (alive == 1 && neighbors == 2)) ? 1 : 0;
                set_cell(next_rows[row], column, next_alive);
            }
        }

        if (report_per_tick) {
            size_t live = opal_runtime_live_heap_bytes();
            printf("tick_%zu_live_bytes: %zu\n", tick + 1, live);
            if (tick + 1 >= 50) {
                if (!steady_initialized) {
                    steady_min = live;
                    steady_max = live;
                    steady_initialized = 1;
                } else {
                    if (live < steady_min) {
                        steady_min = live;
                    }
                    if (live > steady_max) {
                        steady_max = live;
                    }
                }
            }
        }

        {
            void *swap = current;
            current = next;
            next = swap;
        }
    }

    peak_live_bytes = opal_runtime_peak_heap_bytes();
    if (report_per_tick) {
        steady_state_spread_bytes = steady_initialized ? (steady_max - steady_min) : 0;
    }

    printf("peak_live_bytes: %zu\n", peak_live_bytes);
    if (report_per_tick) {
        printf("steady_state_spread_bytes: %zu\n", steady_state_spread_bytes);
    }

    opal_rc_dec(next);
    opal_rc_dec(current);

    if (peak_live_bytes >= __PEAK_LIVE_BYTES_LIMIT__) {
        fprintf(
            stderr,
            "peak_live_bytes threshold violated: %zu >= __PEAK_LIVE_BYTES_LIMIT__\n",
            peak_live_bytes
        );
        return 3;
    }
    if (report_per_tick && steady_state_spread_bytes > __STEADY_STATE_SPREAD_LIMIT__) {
        fprintf(
            stderr,
            "steady_state_spread_bytes threshold violated: %zu > __STEADY_STATE_SPREAD_LIMIT__\n",
            steady_state_spread_bytes
        );
        return 4;
    }
    return 0;
}

int main(void) {
    return run_probe((size_t)__SIZE__, (size_t)__TICKS__, __REPORT_PER_TICK__);
}
"#;

/// Parsed CLI configuration for the memory probe harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeArgs {
    /// Square board side length in cells.
    size: usize,
    /// Number of simulation ticks to execute.
    ticks: usize,
    /// Whether to print per-tick live-heap samples.
    report_per_tick: bool,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::ExitCode::from(1)
        }
    }
}

/// Generate, compile, and execute the probe harness for requested settings.
fn run() -> Result<(), String> {
    let collected_args: Vec<String> = env::args().skip(1).collect();
    let args = parse_args(collected_args.as_slice())?;
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_dir = make_probe_output_dir(&workspace_root)?;
    let source_path = output_dir.join("gol_memory_probe_harness.c");
    let binary_path = output_dir.join("gol_memory_probe_harness");
    let harness_source = HARNESS_TEMPLATE
        .replace("__SIZE__", &args.size.to_string())
        .replace("__TICKS__", &args.ticks.to_string())
        .replace(
            "__REPORT_PER_TICK__",
            if args.report_per_tick { "1" } else { "0" },
        )
        .replace(
            "__PEAK_LIVE_BYTES_LIMIT__",
            &PEAK_LIVE_BYTES_LIMIT.to_string(),
        )
        .replace(
            "__STEADY_STATE_SPREAD_LIMIT__",
            &STEADY_STATE_SPREAD_LIMIT.to_string(),
        );

    fs::write(&source_path, harness_source).map_err(|error| {
        format!(
            "failed to write probe harness '{}': {error}",
            source_path.display()
        )
    })?;

    compile_c_harness(&workspace_root, &source_path, &binary_path)?;
    let output = Command::new(&binary_path).output().map_err(|error| {
        format!(
            "failed to execute probe harness '{}': {error}",
            binary_path.display()
        )
    })?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "probe harness exited with status {}",
            output.status.code().unwrap_or(1)
        ))
    }
}

/// Parse probe CLI flags into [`ProbeArgs`].
fn parse_args(args: &[String]) -> Result<ProbeArgs, String> {
    let mut size: Option<usize> = None;
    let mut ticks: Option<usize> = None;
    let mut report_per_tick = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--size" => {
                let value_index = index.saturating_add(1);
                let value = args
                    .get(value_index)
                    .ok_or_else(|| String::from("missing value for --size"))?;
                size = Some(parse_positive_usize("size", value)?);
                index = index.saturating_add(2);
            }
            "--ticks" => {
                let value_index = index.saturating_add(1);
                let value = args
                    .get(value_index)
                    .ok_or_else(|| String::from("missing value for --ticks"))?;
                ticks = Some(parse_positive_usize("ticks", value)?);
                index = index.saturating_add(2);
            }
            "--report-per-tick" => {
                report_per_tick = true;
                index = index.saturating_add(1);
            }
            flag => {
                return Err(format!("unknown argument: {flag}"));
            }
        }
    }

    let size = size.ok_or_else(|| String::from("--size is required"))?;
    let ticks = ticks.ok_or_else(|| String::from("--ticks is required"))?;
    Ok(ProbeArgs {
        size,
        ticks,
        report_per_tick,
    })
}

/// Parse a strictly-positive usize argument with contextual error text.
fn parse_positive_usize(name: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|parse_error| format!("{name} must be a positive integer: {parse_error}"))?;
    if parsed == 0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(parsed)
}

/// Create a unique output directory for generated probe artifacts.
fn make_probe_output_dir(workspace_root: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error while creating probe dir: {error}"))?
        .as_nanos();
    let dir = workspace_root
        .join("target")
        .join("gol_memory_probe")
        .join(format!("{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create probe output dir '{}': {error}",
            dir.display()
        )
    })?;
    Ok(dir)
}

/// Compile the generated C harness against the runtime C sources.
fn compile_c_harness(
    workspace_root: &Path,
    source_path: &Path,
    binary_path: &Path,
) -> Result<(), String> {
    let runtime_path = workspace_root.join("runtime").join("opal_rc.c");
    let include_dir = workspace_root.join("runtime");
    let compiler = ["cc", "gcc", "clang"]
        .into_iter()
        .find(|candidate| Command::new(candidate).arg("--version").output().is_ok())
        .ok_or_else(|| String::from("no system C compiler found (tried cc, gcc, clang)"))?;

    let output = Command::new(compiler)
        .arg("-std=c11")
        .arg("-D_POSIX_C_SOURCE=200809L")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg(source_path)
        .arg(&runtime_path)
        .arg("-I")
        .arg(&include_dir)
        .arg("-o")
        .arg(binary_path)
        .output()
        .map_err(|error| format!("failed to invoke {compiler} for probe harness: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "probe harness compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProbeArgs, parse_args};

    #[test]
    fn parse_args_accepts_required_probe_flags() {
        let args = vec![
            String::from("--size"),
            String::from("100"),
            String::from("--ticks"),
            String::from("10"),
            String::from("--report-per-tick"),
        ];
        let parsed = parse_args(args.as_slice()).expect("probe args should parse");

        assert_eq!(
            parsed,
            ProbeArgs {
                size: 100,
                ticks: 10,
                report_per_tick: true,
            }
        );
    }

    #[test]
    fn parse_args_rejects_zero_size_cleanly() {
        let args = vec![
            String::from("--size"),
            String::from("0"),
            String::from("--ticks"),
            String::from("10"),
        ];
        let error = parse_args(args.as_slice()).expect_err("zero size should be rejected");

        assert_eq!(error, String::from("size must be positive"));
    }
}
