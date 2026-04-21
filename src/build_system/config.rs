//! Project configuration parsing for `opal.toml`-style input strings.

extern crate alloc;

use crate::build_system::BuildError;
use crate::build_system::targets::BuildTarget;
use crate::build_system::targets::parse_target_triple;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;

/// Semantic version value parsed from `major.minor.patch`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major version segment.
    pub major: u64,
    /// Minor version segment.
    pub minor: u64,
    /// Patch version segment.
    pub patch: u64,
}

/// Comparator kind in a semantic version range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionComparator {
    /// Strictly greater-than.
    Greater,
    /// Greater-than-or-equal.
    GreaterEq,
    /// Strictly less-than.
    Less,
    /// Less-than-or-equal.
    LessEq,
    /// Exactly equal.
    Eq,
}

/// One comparator clause in a version constraint expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionClause {
    /// Comparator operator.
    pub comparator: VersionComparator,
    /// Comparator semantic version.
    pub version: Version,
}

/// Parsed semantic-version constraint list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConstraint {
    /// All clauses that must be satisfied.
    pub clauses: Vec<VersionClause>,
}

/// One declared package dependency from project configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Dependency package name.
    pub name: String,
    /// Required version range.
    pub version_constraint: VersionConstraint,
}

/// Parsed project configuration fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Project package name.
    pub name: String,
    /// Project package version.
    pub version: Version,
    /// Direct dependencies.
    pub dependencies: Vec<Dependency>,
    /// Declared build targets.
    pub build_targets: Vec<BuildTarget>,
}

/// Parse `opal.toml`-style project configuration from in-memory string input.
///
/// # Errors
///
/// Returns [`BuildError`] when required fields are missing or when a
/// configuration line cannot be parsed.
pub fn parse_config(input: &str) -> Result<ProjectConfig, BuildError> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Section {
        /// Top-level key-value field section.
        Root,
        /// Dependency declaration section.
        Dependencies,
        /// Build configuration declaration section.
        Build,
    }

    let mut section = Section::Root;
    let mut name: Option<String> = None;
    let mut version: Option<Version> = None;
    let mut dependencies = Vec::new();
    let mut build_targets = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "[dependencies]" {
            section = Section::Dependencies;
            continue;
        }

        if line == "[build]" {
            section = Section::Build;
            continue;
        }

        match section {
            Section::Root => parse_root_line(line, &mut name, &mut version)?,
            Section::Dependencies => parse_dependency_line(line, &mut dependencies)?,
            Section::Build => parse_build_line(line, &mut build_targets)?,
        }
    }

    let final_name = name.ok_or_else(|| BuildError::MissingField(String::from("name")))?;
    let final_version = version.ok_or_else(|| BuildError::MissingField(String::from("version")))?;

    Ok(ProjectConfig {
        name: final_name,
        version: final_version,
        dependencies,
        build_targets,
    })
}

/// Parse a semantic version from text.
///
/// # Errors
///
/// Returns [`BuildError::InvalidVersion`] when the string does not contain
/// exactly three numeric segments.
pub fn parse_version(input: &str) -> Result<Version, BuildError> {
    let trimmed = input.trim();
    let mut segments = trimmed.split('.');
    let major_segment = segments
        .next()
        .ok_or_else(|| BuildError::InvalidVersion(input.to_owned()))?;
    let minor_segment = segments
        .next()
        .ok_or_else(|| BuildError::InvalidVersion(input.to_owned()))?;
    let patch_segment = segments
        .next()
        .ok_or_else(|| BuildError::InvalidVersion(input.to_owned()))?;
    if segments.next().is_some() {
        return Err(BuildError::InvalidVersion(input.to_owned()));
    }

    let major = major_segment
        .parse::<u64>()
        .map_err(|_parse_error| BuildError::InvalidVersion(input.to_owned()))?;
    let minor = minor_segment
        .parse::<u64>()
        .map_err(|_parse_error| BuildError::InvalidVersion(input.to_owned()))?;
    let patch = patch_segment
        .parse::<u64>()
        .map_err(|_parse_error| BuildError::InvalidVersion(input.to_owned()))?;

    Ok(Version {
        major,
        minor,
        patch,
    })
}

/// Parse a semantic version constraint expression.
///
/// # Errors
///
/// Returns [`BuildError::InvalidConstraint`] when any clause is malformed or
/// includes an invalid semantic version.
pub fn parse_version_constraint(input: &str) -> Result<VersionConstraint, BuildError> {
    let normalized = input.replace(',', " ");
    let mut clauses = Vec::new();

    for token in normalized.split_whitespace() {
        let (operator, version_text) = parse_constraint_token(token);

        let parsed_version = parse_version(version_text)
            .map_err(|_parse_error| BuildError::InvalidConstraint(input.to_owned()))?;
        clauses.push(VersionClause {
            comparator: operator,
            version: parsed_version,
        });
    }

    if clauses.is_empty() {
        return Err(BuildError::InvalidConstraint(input.to_owned()));
    }

    Ok(VersionConstraint { clauses })
}

/// Parse one version-constraint token into `(comparator, version_text)`.
fn parse_constraint_token(token: &str) -> (VersionComparator, &str) {
    token.strip_prefix(">=").map_or_else(
        || {
            token.strip_prefix("<=").map_or_else(
                || {
                    token.strip_prefix('>').map_or_else(
                        || {
                            token.strip_prefix('<').map_or_else(
                                || {
                                    token.strip_prefix('=').map_or_else(
                                        || (VersionComparator::Eq, token),
                                        |rest| (VersionComparator::Eq, rest),
                                    )
                                },
                                |rest| (VersionComparator::Less, rest),
                            )
                        },
                        |rest| (VersionComparator::Greater, rest),
                    )
                },
                |rest| (VersionComparator::LessEq, rest),
            )
        },
        |rest| (VersionComparator::GreaterEq, rest),
    )
}

/// Evaluate whether a version satisfies all clauses in a constraint.
#[must_use]
pub fn version_satisfies_constraint(version: &Version, constraint: &VersionConstraint) -> bool {
    for clause in &constraint.clauses {
        let satisfied = match clause.comparator {
            VersionComparator::Greater => version > &clause.version,
            VersionComparator::GreaterEq => version >= &clause.version,
            VersionComparator::Less => version < &clause.version,
            VersionComparator::LessEq => version <= &clause.version,
            VersionComparator::Eq => version == &clause.version,
        };

        if !satisfied {
            return false;
        }
    }

    true
}

/// Parse one root key/value configuration line.
fn parse_root_line(
    line: &str,
    name: &mut Option<String>,
    version: &mut Option<Version>,
) -> Result<(), BuildError> {
    let (key, value) = split_key_value(line)?;
    let value_text = parse_quoted_value(value)?;

    if key == "name" {
        *name = Some(value_text.to_owned());
        return Ok(());
    }

    if key == "version" {
        *version = Some(parse_version(value_text)?);
        return Ok(());
    }

    Err(BuildError::ParseError(String::from(
        "unknown root configuration key",
    )))
}

/// Parse one dependency declaration line.
fn parse_dependency_line(line: &str, dependencies: &mut Vec<Dependency>) -> Result<(), BuildError> {
    let (name, value) = split_key_value(line)?;
    let constraint_text = parse_quoted_value(value)?;
    let version_constraint = parse_version_constraint(constraint_text)?;
    dependencies.push(Dependency {
        name: name.to_owned(),
        version_constraint,
    });
    Ok(())
}

/// Parse one build-section declaration line.
fn parse_build_line(line: &str, build_targets: &mut Vec<BuildTarget>) -> Result<(), BuildError> {
    let (key, value) = split_key_value(line)?;
    if key != "targets" {
        return Err(BuildError::ParseError(String::from(
            "unknown build configuration key",
        )));
    }

    let target_list = parse_string_array(value)?;
    for target_text in target_list {
        let triple = parse_target_triple(target_text)?;
        build_targets.push(BuildTarget { triple });
    }
    Ok(())
}

/// Split a configuration line into key/value segments.
fn split_key_value(line: &str) -> Result<(&str, &str), BuildError> {
    let mut parts = line.splitn(2, '=');
    let key = parts
        .next()
        .map(str::trim)
        .ok_or_else(|| BuildError::ParseError(String::from("missing key")))?;
    let value = parts
        .next()
        .map(str::trim)
        .ok_or_else(|| BuildError::ParseError(String::from("missing value")))?;
    if key.is_empty() {
        return Err(BuildError::ParseError(String::from("empty key")));
    }
    Ok((key, value))
}

/// Parse one quoted string value (`"text"`).
fn parse_quoted_value(input: &str) -> Result<&str, BuildError> {
    let without_prefix = input.strip_prefix('"');
    let Some(without_prefix) = without_prefix else {
        return Err(BuildError::ParseError(String::from(
            "expected quoted string value",
        )));
    };

    let without_suffix = without_prefix.strip_suffix('"');
    let Some(without_suffix) = without_suffix else {
        return Err(BuildError::ParseError(String::from(
            "expected quoted string value",
        )));
    };

    Ok(without_suffix)
}

/// Parse an array of quoted strings (`["a", "b"]`).
fn parse_string_array(input: &str) -> Result<Vec<&str>, BuildError> {
    let trimmed = input.trim();
    let without_prefix = trimmed.strip_prefix('[');
    let Some(without_prefix) = without_prefix else {
        return Err(BuildError::ParseError(String::from("expected array value")));
    };

    let without_suffix = without_prefix.strip_suffix(']');
    let Some(inner) = without_suffix else {
        return Err(BuildError::ParseError(String::from("expected array value")));
    };
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::new();
    for raw in inner.split(',') {
        let item = parse_quoted_value(raw.trim())?;
        output.push(item);
    }
    Ok(output)
}
