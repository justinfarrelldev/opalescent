//! CLI command dispatch for `opal pkg` subcommands.

extern crate alloc;

use crate::package_manager::installer::{InstallPlan, Installer, MockDownloader};
use crate::package_manager::manifest::{Manifest, parse_manifest, serialize_manifest};
use crate::package_manager::publisher::{MockUploader, Publisher};
use crate::package_manager::registry::MockRegistry;
use crate::package_manager::resolver::resolve_manifest_deps;
use alloc::string::String;
use alloc::vec::Vec;

/// Available `opal pkg` subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkgCommand {
    /// Initialise a new project with a starter manifest.
    Init {
        /// Project name for the new manifest.
        name: String,
    },
    /// Add a dependency to the manifest.
    Add {
        /// Package name to add.
        package: String,
        /// Version constraint string.
        version_constraint: String,
    },
    /// Remove a dependency from the manifest.
    Remove {
        /// Package name to remove.
        package: String,
    },
    /// Install all dependencies declared in the manifest.
    Install {
        /// Raw manifest TOML string (used in tests; real CLI reads from file).
        manifest_toml: String,
    },
    /// Publish the package described by the manifest.
    Publish {
        /// Raw manifest TOML string.
        manifest_toml: String,
    },
}

/// Result returned from a [`PkgCommand`] execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkgCommandResult {
    /// Human-readable output lines.
    pub output: Vec<String>,
    /// Whether the command succeeded.
    pub success: bool,
}

impl PkgCommandResult {
    /// Construct a successful result.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec<String> parameter cannot be used in const context on stable Rust"
    )]
    fn success(lines: Vec<String>) -> Self {
        Self {
            output: lines,
            success: true,
        }
    }

    /// Construct a failed result.
    fn failure(message: String) -> Self {
        Self {
            output: alloc::vec![message],
            success: false,
        }
    }
}

/// Dispatch an `opal pkg` subcommand and return the result.
///
/// Commands that require registry access use the provided `registry`; in tests
/// pass a [`MockRegistry`]. Commands that modify the manifest return the
/// updated TOML text as the first output line so callers can persist it.
///
/// # Errors
///
/// This function is infallible — errors are captured in [`PkgCommandResult::success`].
#[must_use]
pub fn dispatch_pkg_command(command: &PkgCommand, registry: &MockRegistry) -> PkgCommandResult {
    match *command {
        PkgCommand::Init { ref name } => cmd_init(name),
        PkgCommand::Add {
            ref package,
            ref version_constraint,
        } => cmd_add(package, version_constraint),
        PkgCommand::Remove { ref package } => cmd_remove(package),
        PkgCommand::Install { ref manifest_toml } => cmd_install(manifest_toml, registry),
        PkgCommand::Publish { ref manifest_toml } => cmd_publish(manifest_toml),
    }
}

/// Handle `opal pkg init`.
fn cmd_init(name: &str) -> PkgCommandResult {
    let manifest = Manifest {
        name: name.to_owned(),
        version: String::from("0.1.0"),
        author: None,
        description: None,
        dependencies: Vec::new(),
    };
    let toml = serialize_manifest(&manifest);
    PkgCommandResult::success(alloc::vec![
        alloc::format!("Initialized package `{name}`"),
        toml,
    ])
}

/// Handle `opal pkg add <package> <version_constraint>`.
fn cmd_add(package: &str, version_constraint: &str) -> PkgCommandResult {
    // Return a minimal manifest patch as output so callers can apply it.
    let line = alloc::format!("{package} = \"{version_constraint}\"");
    PkgCommandResult::success(alloc::vec![
        alloc::format!("Added dependency `{package} {version_constraint}`"),
        line,
    ])
}

/// Handle `opal pkg remove <package>`.
fn cmd_remove(package: &str) -> PkgCommandResult {
    PkgCommandResult::success(alloc::vec![alloc::format!(
        "Removed dependency `{package}`"
    )])
}

/// Handle `opal pkg install`.
fn cmd_install(manifest_toml: &str, registry: &MockRegistry) -> PkgCommandResult {
    let manifest = match parse_manifest(manifest_toml) {
        Ok(m) => m,
        Err(err) => {
            return PkgCommandResult::failure(alloc::format!("Manifest error: {err:?}"));
        }
    };

    let graph = match resolve_manifest_deps(&manifest, registry) {
        Ok(g) => g,
        Err(err) => {
            return PkgCommandResult::failure(alloc::format!("Resolution error: {err:?}"));
        }
    };

    let downloader = MockDownloader::new();
    let pkg_installer = Installer::new(&downloader);
    let plan: Vec<InstallPlan> = pkg_installer.plan_from_graph(&graph);

    match pkg_installer.execute(&plan) {
        Ok(names) => PkgCommandResult::success(alloc::vec![alloc::format!(
            "Installed {} package(s): {}",
            names.len(),
            names.join(", ")
        )]),
        Err(err) => PkgCommandResult::failure(alloc::format!("Install error: {err:?}")),
    }
}

/// Handle `opal pkg publish`.
fn cmd_publish(manifest_toml: &str) -> PkgCommandResult {
    let manifest = match parse_manifest(manifest_toml) {
        Ok(m) => m,
        Err(err) => {
            return PkgCommandResult::failure(alloc::format!("Manifest error: {err:?}"));
        }
    };

    let uploader = MockUploader::new();
    let publisher = Publisher::new(&uploader);

    match publisher.publish_manifest(&manifest) {
        Ok(size) => PkgCommandResult::success(alloc::vec![alloc::format!(
            "Published `{}@{}` ({size} bytes)",
            manifest.name,
            manifest.version
        )]),
        Err(err) => PkgCommandResult::failure(alloc::format!("Publish error: {err:?}")),
    }
}
