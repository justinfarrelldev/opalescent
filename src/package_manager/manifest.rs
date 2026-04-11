//! Package manifest parsing for `opal.pkg.toml`-style project descriptors.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

/// Error returned when manifest parsing fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// A required field was absent from the manifest text.
    MissingField(String),
    /// A field value could not be interpreted.
    InvalidField(String),
}

/// One dependency declared in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDependency {
    /// Package name.
    pub name: String,
    /// Version constraint string (e.g. `>=1.0.0`).
    pub version_constraint: String,
}

/// Parsed package manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Package name.
    pub name: String,
    /// Package version string.
    pub version: String,
    /// Author name (optional).
    pub author: Option<String>,
    /// Short description (optional).
    pub description: Option<String>,
    /// Direct dependencies.
    pub dependencies: Vec<ManifestDependency>,
}

/// Parse a package manifest from an in-memory TOML-style string.
///
/// Supported keys:
/// - `name = "…"` (required)
/// - `version = "…"` (required)
/// - `author = "…"` (optional)
/// - `description = "…"` (optional)
/// - `[dependencies]` section with `name = "constraint"` entries
///
/// # Errors
///
/// Returns [`ManifestError`] when required fields are absent or invalid.
pub fn parse_manifest(input: &str) -> Result<Manifest, ManifestError> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut author: Option<String> = None;
    let mut description: Option<String> = None;
    let mut dependencies: Vec<ManifestDependency> = Vec::new();
    let mut in_deps = false;

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if line.starts_with('[') {
            in_deps = false;
            continue;
        }

        if let Some((key, value)) = parse_kv(line) {
            if in_deps {
                dependencies.push(ManifestDependency {
                    name: key,
                    version_constraint: value,
                });
            } else {
                match key.as_str() {
                    "name" => name = Some(value),
                    "version" => version = Some(value),
                    "author" => author = Some(value),
                    "description" => description = Some(value),
                    _ => {}
                }
            }
        }
    }

    let name = name.ok_or_else(|| ManifestError::MissingField(String::from("name")))?;
    let version = version.ok_or_else(|| ManifestError::MissingField(String::from("version")))?;

    Ok(Manifest {
        name,
        version,
        author,
        description,
        dependencies,
    })
}

/// Parse a `key = "value"` line, stripping surrounding quotes from the value.
fn parse_kv(line: &str) -> Option<(String, String)> {
    let equals = line.find('=')?;
    let key = line.get(..equals)?.trim().to_owned();
    let raw_val = line.get(equals.saturating_add(1)..)?.trim();
    let value = raw_val
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw_val)
        .to_owned();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

/// Serialize a [`Manifest`] back to TOML-style text.
#[must_use]
pub fn serialize_manifest(manifest: &Manifest) -> String {
    let mut out = String::new();
    let _name_err = writeln!(out, "name = \"{}\"", manifest.name);
    let _ver_err = writeln!(out, "version = \"{}\"", manifest.version);
    if let Some(ref author) = manifest.author {
        let _auth_err = writeln!(out, "author = \"{author}\"");
    }
    if let Some(ref desc) = manifest.description {
        let _desc_err = writeln!(out, "description = \"{desc}\"");
    }
    if !manifest.dependencies.is_empty() {
        out.push_str("\n[dependencies]\n");
        for dep in &manifest.dependencies {
            let _dep_err = writeln!(out, "{} = \"{}\"", dep.name, dep.version_constraint);
        }
    }
    out
}
