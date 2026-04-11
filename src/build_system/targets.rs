//! Target selection and platform-specific build metadata.

extern crate alloc;

use crate::build_system::BuildError;

/// Supported operating-system platform for cross compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    /// Linux platform.
    Linux,
    /// Apple Darwin / macOS platform.
    MacOs,
    /// Windows platform.
    Windows,
}

/// Supported CPU architecture for build target selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Architecture {
    /// `x86_64` architecture.
    X86_64,
    /// `AArch64` architecture.
    Aarch64,
}

/// Build target triple comprised of architecture + platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple {
    /// Target architecture segment.
    pub arch: Architecture,
    /// Target platform segment.
    pub platform: Platform,
}

/// Build target declaration wrapping a target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    /// Parsed target triple.
    pub triple: TargetTriple,
}

/// Parse a target triple from text (`x86_64-linux`, `aarch64-darwin`, `x86_64-windows`).
///
/// # Errors
///
/// Returns [`BuildError::InvalidTarget`] when the input does not contain a
/// supported architecture/platform pair.
pub fn parse_target_triple(input: &str) -> Result<TargetTriple, BuildError> {
    let mut segments = input.split('-');
    let arch_segment = segments
        .next()
        .ok_or_else(|| BuildError::InvalidTarget(input.to_owned()))?;
    let platform_segment = segments
        .next()
        .ok_or_else(|| BuildError::InvalidTarget(input.to_owned()))?;
    if segments.next().is_some() {
        return Err(BuildError::InvalidTarget(input.to_owned()));
    }

    let architecture = match arch_segment {
        "x86_64" => Architecture::X86_64,
        "aarch64" => Architecture::Aarch64,
        _ => {
            return Err(BuildError::InvalidTarget(input.to_owned()));
        }
    };

    let platform = match platform_segment {
        "linux" => Platform::Linux,
        "darwin" => Platform::MacOs,
        "windows" => Platform::Windows,
        _ => {
            return Err(BuildError::InvalidTarget(input.to_owned()));
        }
    };

    Ok(TargetTriple {
        arch: architecture,
        platform,
    })
}

/// Return platform-specific dynamic library extension.
#[must_use]
pub const fn dynamic_lib_extension(target: &TargetTriple) -> &'static str {
    match target.platform {
        Platform::Linux => ".so",
        Platform::MacOs => ".dylib",
        Platform::Windows => ".dll",
    }
}
