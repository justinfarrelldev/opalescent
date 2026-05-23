#![cfg(feature = "integration")]

use super::fs_helpers::{FsStateGuard, fs_project_root};
use super::*;
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const STRESS_WINDOW: Duration = Duration::from_secs(15);
const HARD_TIMEOUT: Duration = Duration::from_secs(20);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);
const MIN_SAMPLES: usize = 30;
const WARMUP_SAMPLES: usize = 8;
const MAX_POST_WARMUP_GROWTH_BYTES: i64 = 8 * 1024 * 1024;
const MAX_POST_WARMUP_SPREAD_BYTES: i64 = 10 * 1024 * 1024;

#[derive(Clone, Debug)]
struct MemorySample {
    elapsed: Duration,
    rss_bytes: i64,
}

#[derive(Clone, Debug)]
struct StressReport {
    samples: Vec<MemorySample>,
    warmup_samples: usize,
    min_samples: usize,
    sample_interval: Duration,
    stress_window: Duration,
    hard_timeout: Duration,
    max_post_warmup_growth_bytes: i64,
    max_post_warmup_spread_bytes: i64,
    timed_out: bool,
    final_status: Option<ExitStatus>,
    kill_attempted: bool,
    kill_result: Option<String>,
}

fn should_run_stress() -> bool {
    std::env::var("OPAL_RUN_STRESS")
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

fn format_bytes(value: i64) -> String {
    format!("{value}B")
}

fn format_duration_ms(duration: Duration) -> String {
    format!("{}ms", duration.as_millis())
}

fn format_series(samples: &[MemorySample]) -> String {
    if samples.is_empty() {
        return String::from("<no samples>");
    }

    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "#{index}: t={} rss={}",
                format_duration_ms(sample.elapsed),
                format_bytes(sample.rss_bytes)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(target_os = "linux")]
fn read_linux_rss_bytes(pid: u32) -> Result<i64, String> {
    let status_path = format!("/proc/{pid}/status");
    let content = fs::read_to_string(&status_path)
        .map_err(|error| format!("failed to read {status_path}: {error}"))?;

    let vmrss_line = content
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .ok_or_else(|| format!("{status_path} does not contain VmRSS line"))?;

    let mut parts = vmrss_line.split_whitespace();
    let _label = parts
        .next()
        .ok_or_else(|| format!("malformed VmRSS line in {status_path}: {vmrss_line}"))?;
    let value_kib = parts
        .next()
        .ok_or_else(|| format!("missing VmRSS value in {status_path}: {vmrss_line}"))?
        .parse::<i64>()
        .map_err(|error| format!("failed to parse VmRSS value from {status_path}: {error}"))?;

    value_kib
        .checked_mul(1024)
        .ok_or_else(|| format!("VmRSS bytes overflow for pid={pid}, kib={value_kib}"))
}

#[cfg(not(target_os = "linux"))]
fn read_linux_rss_bytes(pid: u32) -> Result<i64, String> {
    Err(format!(
        "game_of_life_full_memory_stress currently requires linux /proc RSS sampling; unsupported pid={pid}"
    ))
}

fn kill_and_reap_child(child: &mut Child) -> (bool, Option<String>, Option<ExitStatus>) {
    let mut kill_result: Option<String> = None;

    match child.try_wait() {
        Ok(Some(status)) => {
            return (false, Some(String::from("not-needed-already-exited")), Some(status));
        }
        Ok(None) => {}
        Err(error) => {
            kill_result = Some(format!("try_wait_before_kill_error:{error}"));
        }
    }

    let kill_attempted = true;
    if kill_result.is_none() {
        kill_result = Some(match child.kill() {
            Ok(()) => String::from("ok"),
            Err(error) => format!("kill_error:{error}"),
        });
    }

    let final_status = match child.wait() {
        Ok(status) => Some(status),
        Err(error) => {
            kill_result = Some(kill_result.map_or_else(
                || format!("wait_error:{error}"),
                |existing| format!("{existing}; wait_error:{error}"),
            ));
            None
        }
    };

    (kill_attempted, kill_result, final_status)
}

fn run_memory_stress(binary_path: &Path) -> Result<StressReport, String> {
    let mut child = Command::new(binary_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "game-of-life-full compiled binary should spawn for memory stress: {error}"
            )
        })?;

    let pid = child.id();
    let start = Instant::now();
    let mut samples: Vec<MemorySample> = Vec::new();
    let mut timed_out = false;

    loop {
        let elapsed = start.elapsed();
        if elapsed >= HARD_TIMEOUT {
            timed_out = true;
            break;
        }

        if elapsed >= STRESS_WINDOW {
            break;
        }

        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {}
            Err(error) => {
                let (kill_attempted, kill_result, final_status) = kill_and_reap_child(&mut child);
                return Err(format!(
                    "game-of-life-full stress child try_wait should succeed, but failed: {error}; kill_attempted={kill_attempted}; kill_result={kill_result:?}; final_status={final_status:?}; samples=[{}]",
                    format_series(&samples)
                ));
            }
        }

        let rss_bytes = read_linux_rss_bytes(pid)?;
        samples.push(MemorySample { elapsed, rss_bytes });
        thread::sleep(SAMPLE_INTERVAL);
    }

    let (kill_attempted, kill_result, final_status) = kill_and_reap_child(&mut child);

    Ok(StressReport {
        samples,
        warmup_samples: WARMUP_SAMPLES,
        min_samples: MIN_SAMPLES,
        sample_interval: SAMPLE_INTERVAL,
        stress_window: STRESS_WINDOW,
        hard_timeout: HARD_TIMEOUT,
        max_post_warmup_growth_bytes: MAX_POST_WARMUP_GROWTH_BYTES,
        max_post_warmup_spread_bytes: MAX_POST_WARMUP_SPREAD_BYTES,
        timed_out,
        final_status,
        kill_attempted,
        kill_result,
    })
}

fn stress_context(report: &StressReport) -> String {
    format!(
        "sample_interval={}; stress_window={}; hard_timeout={}; warmup_samples={}; min_samples={}; kill_attempted={}; kill_result={:?}; final_status={:?}; timed_out={}; samples=[{}]",
        format_duration_ms(report.sample_interval),
        format_duration_ms(report.stress_window),
        format_duration_ms(report.hard_timeout),
        report.warmup_samples,
        report.min_samples,
        report.kill_attempted,
        report.kill_result,
        report.final_status,
        report.timed_out,
        format_series(&report.samples)
    )
}

fn extract_post_warmup_stats(report: &StressReport) -> Result<(i64, i64, i64, i64, usize), String> {
    let post_warmup = &report.samples[report.warmup_samples..];
    let Some(first_sample) = post_warmup.first() else {
        return Err(String::from(
            "game-of-life-full memory stress expected at least one post-warmup sample",
        ));
    };
    let Some(last_sample) = post_warmup.last() else {
        return Err(String::from(
            "game-of-life-full memory stress expected a trailing post-warmup sample",
        ));
    };

    let first = first_sample.rss_bytes;
    let last = last_sample.rss_bytes;
    let min_post = post_warmup
        .iter()
        .map(|sample| sample.rss_bytes)
        .min()
        .unwrap_or(first);
    let max_post = post_warmup
        .iter()
        .map(|sample| sample.rss_bytes)
        .max()
        .unwrap_or(last);

    Ok((first, last, min_post, max_post, post_warmup.len()))
}

fn assert_bounded_memory(report: &StressReport) -> Result<(), String> {
    if report.samples.len() < report.min_samples {
        return Err(format!(
            "game-of-life-full memory stress should collect at least {} samples, got {}; {}",
            report.min_samples,
            report.samples.len(),
            stress_context(report)
        ));
    }

    if report.samples.len() <= report.warmup_samples {
        return Err(format!(
            "game-of-life-full memory stress requires post-warmup samples; warmup_samples={} but total_samples={}; {}",
            report.warmup_samples,
            report.samples.len(),
            stress_context(report)
        ));
    }

    let (first, last, min_post, max_post, post_warmup_len) = extract_post_warmup_stats(report)?;

    let growth = last.checked_sub(first).ok_or_else(|| {
        format!(
            "game-of-life-full growth underflow: first={} last={}",
            format_bytes(first),
            format_bytes(last)
        )
    })?;
    let spread = max_post.checked_sub(min_post).ok_or_else(|| {
        format!(
            "game-of-life-full spread underflow: min={} max={}",
            format_bytes(min_post),
            format_bytes(max_post)
        )
    })?;

    if growth > report.max_post_warmup_growth_bytes {
        return Err(format!(
            "game-of-life-full post-warmup RSS growth should stay <= {} but was {}; warmup_samples={}; post_warmup_samples={}; spread={} (limit={}); {}",
            format_bytes(report.max_post_warmup_growth_bytes),
            format_bytes(growth),
            report.warmup_samples,
            post_warmup_len,
            format_bytes(spread),
            format_bytes(report.max_post_warmup_spread_bytes),
            stress_context(report)
        ));
    }

    if spread > report.max_post_warmup_spread_bytes {
        return Err(format!(
            "game-of-life-full post-warmup RSS spread should stay <= {} but was {}; warmup_samples={}; post_warmup_samples={}; growth={} (limit={}); {}",
            format_bytes(report.max_post_warmup_spread_bytes),
            format_bytes(spread),
            report.warmup_samples,
            post_warmup_len,
            format_bytes(growth),
            format_bytes(report.max_post_warmup_growth_bytes),
            stress_context(report)
        ));
    }

    Ok(())
}

#[test]
#[ignore = "stress test: opt-in via --ignored and OPAL_RUN_STRESS=1"]
#[serial(fs)]
fn game_of_life_full_memory_stress() {
    if !should_run_stress() {
        eprintln!("skipping game_of_life_full_memory_stress: OPAL_RUN_STRESS != 1");
        return;
    }

    let project_name = "game-of-life-full";
    let project_dir = fs_project_root(project_name);

    let execution_result: Result<(), String> = (|| {
        let _guard = FsStateGuard::new("test-projects/game-of-life-full")
            .map_err(|error| format!("game-of-life-full guard should initialize: {error}"))?;

        let temp_dir = super::fs_helpers::unique_probe_target_dir("game-of-life-full-stress");
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("game-of-life-full fixture should compile into a binary: {error}")
            })?;

        let report = run_memory_stress(&binary_path)?;
        assert_bounded_memory(&report)
    })();

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "game-of-life-full stress executable should run under bounded timeout, collect sufficient samples, remain memory-bounded post-warmup, and always be reaped: {failure_message}"
    );
}
