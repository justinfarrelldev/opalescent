//! Strict CLI options, independent from source discovery or callback execution.
use super::NativeCommandError;
use crate::build_system::targets::{TargetTriple, parse_target_triple};
use std::path::PathBuf;
use std::time::Duration;

/// Reporting formats share one canonical case/attempt data model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reporter {
    /// Human-readable behavior listing and outcomes.
    Text,
    /// Structured case/association records.
    Json,
    /// JUnit-compatible independent testcases.
    Junit,
}

/// Source selection never creates runtime fixtures or invokes setup.
pub(super) struct Options {
    /// Explicit project root, or current working directory.
    pub root: PathBuf,
    /// List checked cases without native builds.
    pub list: bool,
    /// Explicit opt-in for successful empty selection.
    pub allow_empty: bool,
    /// Exact stable encoded identity.
    pub case: Option<String>,
    /// Display-text substring.
    pub filter: Option<String>,
    /// Declared category.
    pub kind: Option<String>,
    /// Canonical subject/name selector.
    pub unit: Option<String>,
    /// Canonical World/name selector, including hook/transitive requirements.
    pub world: Option<String>,
    /// Requested native target, checked before execution.
    pub target: Option<TargetTriple>,
    /// Worker capacity override.
    pub jobs: usize,
    /// Fresh additional attempts.
    pub retries: u32,
    /// Per-worker hard deadline.
    pub timeout: Duration,
    /// Common report format.
    pub reporter: Reporter,
    /// Optional report file instead of stdout.
    pub output: Option<PathBuf>,
}

impl Options {
    /// Reject malformed/unknown options instead of silently ignoring them.
    pub fn parse(args: &[String]) -> Result<Self, NativeCommandError> {
        let mut options = Self {
            root: PathBuf::from("."),
            list: false,
            allow_empty: false,
            case: None,
            filter: None,
            kind: None,
            unit: None,
            world: None,
            target: None,
            jobs: std::thread::available_parallelism().map_or(1, core::num::NonZeroUsize::get),
            retries: 0,
            timeout: Duration::from_secs(30),
            reporter: Reporter::Text,
            output: None,
        };
        let mut args = args.iter();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--list" => options.list = true,
                "--allow-empty" => options.allow_empty = true,
                flag => {
                    let value = args.next().ok_or_else(|| {
                        NativeCommandError::argument(format!("missing value for {flag}"))
                    })?;
                    options.value(flag, value)?;
                }
            }
        }
        Ok(options)
    }

    /// Value-taking options use exact canonical spellings.
    fn value(&mut self, flag: &str, value: &str) -> Result<(), NativeCommandError> {
        match flag {
            "--root" => self.root = PathBuf::from(value),
            "--case" => self.case = Some(value.to_owned()),
            "--filter" => self.filter = Some(value.to_owned()),
            "--unit" => self.unit = Some(value.to_owned()),
            "--world" => self.world = Some(value.to_owned()),
            "--kind" if matches!(value, "unit" | "integration" | "end_to_end") => {
                self.kind = Some(value.to_owned());
            }
            "--jobs" => {
                self.jobs = value.parse().map_err(|error| {
                    NativeCommandError::argument(format!("invalid worker count: {error}"))
                })?;
                if self.jobs == 0 {
                    return Err(NativeCommandError::argument(
                        "worker count must be positive",
                    ));
                }
            }
            "--retries" => {
                self.retries = value.parse().map_err(|error| {
                    NativeCommandError::argument(format!("invalid retry count: {error}"))
                })?;
            }
            "--timeout" => self.timeout = timeout(value)?,
            "--target" => {
                self.target = Some(
                    parse_target_triple(value)
                        .map_err(|error| NativeCommandError::argument(format!("{error:?}")))?,
                );
            }
            "--reporter" => {
                self.reporter = match value {
                    "text" => Reporter::Text,
                    "json" => Reporter::Json,
                    "junit" => Reporter::Junit,
                    _ => {
                        return Err(NativeCommandError::argument(
                            "reporter must be text, json or junit",
                        ));
                    }
                }
            }
            "--output" => self.output = Some(PathBuf::from(value)),
            _ => {
                return Err(NativeCommandError::argument(format!(
                    "unknown or invalid test option: {flag} {value}"
                )));
            }
        }
        Ok(())
    }
}

/// CLI deadlines accept positive seconds or explicit ms/s/m suffixes.
fn timeout(value: &str) -> Result<Duration, NativeCommandError> {
    let (number, multiplier, divisor) = [("ms", 1_u32, 1000_u32), ("s", 1, 1), ("m", 60, 1)]
        .into_iter()
        .find_map(|(suffix, multiplier, divisor)| {
            value
                .strip_suffix(suffix)
                .map(|number| (number, multiplier, divisor))
        })
        .unwrap_or((value, 1, 1));
    let seconds = number
        .parse::<f64>()
        .map_err(|error| NativeCommandError::argument(format!("invalid timeout: {error}")))?;
    let duration = Duration::try_from_secs_f64(seconds)
        .map_err(|error| NativeCommandError::argument(format!("invalid timeout: {error}")))?
        .checked_mul(multiplier)
        .and_then(|duration| duration.checked_div(divisor))
        .ok_or_else(|| NativeCommandError::argument("timeout out of range"))?;
    if duration.is_zero() {
        return Err(NativeCommandError::argument("timeout must be positive"));
    }
    Ok(duration)
}
