//! Target selection and platform-specific build metadata.

extern crate alloc;

use crate::build_system::BuildError;

/// Supported operating-system platform for cross compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux platform.
    Linux,
    /// Apple Darwin / macOS platform.
    MacOs,
    /// Windows platform.
    Windows,
}

/// Supported CPU architecture for build target selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// `x86_64` architecture.
    X86_64,
    /// `AArch64` architecture.
    Aarch64,
}

/// Environment/toolchain variant for target triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripleEnv {
    /// Microsoft Visual C++ toolchain.
    Msvc,
    /// GNU toolchain.
    Gnu,
    /// musl libc.
    Musl,
}

/// Build target triple comprised of architecture + platform + optional environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple {
    /// Target architecture segment.
    pub arch: Architecture,
    /// Target platform segment.
    pub platform: Platform,
    /// Optional environment/toolchain variant (None for legacy 2-segment triples).
    pub env: Option<TripleEnv>,
}

/// Build target declaration wrapping a target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    /// Parsed target triple.
    pub triple: TargetTriple,
}

/// Parse a target triple from text.
///
/// Supports both legacy 2-segment format (`x86_64-linux`, `aarch64-darwin`, `x86_64-windows`)
/// and Rust 4-segment format (`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`).
///
/// # Errors
///
/// Returns [`BuildError::InvalidTarget`] when the input does not contain a
/// supported architecture/platform pair or environment variant.
pub fn parse_target_triple(input: &str) -> Result<TargetTriple, BuildError> {
    let segs: Vec<&str> = input.split('-').collect();
    match segs.len() {
        2 => parse_2_segment(input, segs[0], segs[1]),
        4 => parse_4_segment(input, segs[0], segs[1], segs[2], segs[3]),
        _ => Err(BuildError::InvalidTarget(input.to_owned())),
    }
}

/// Parse a 2-segment target triple (arch-platform).
fn parse_2_segment(input: &str, arch: &str, platform: &str) -> Result<TargetTriple, BuildError> {
    let architecture = match arch {
        "x86_64" => Architecture::X86_64,
        "aarch64" => Architecture::Aarch64,
        _ => return Err(BuildError::InvalidTarget(input.to_owned())),
    };
    let plat = match platform {
        "linux" => Platform::Linux,
        "darwin" => Platform::MacOs,
        "windows" => {
            eprintln!(
                "warning: target \"{input}\" is deprecated; use \"x86_64-pc-windows-msvc\" or \"x86_64-pc-windows-gnu\" explicitly"
            );
            Platform::Windows
        }
        _ => return Err(BuildError::InvalidTarget(input.to_owned())),
    };
    Ok(TargetTriple {
        arch: architecture,
        platform: plat,
        env: None,
    })
}

/// Parse a 4-segment target triple (arch-vendor-os-env).
fn parse_4_segment(
    input: &str,
    arch: &str,
    _vendor: &str,
    os: &str,
    env: &str,
) -> Result<TargetTriple, BuildError> {
    let architecture = match arch {
        "x86_64" => Architecture::X86_64,
        _ => return Err(BuildError::InvalidTarget(input.to_owned())),
    };
    let platform = match os {
        "linux" => Platform::Linux,
        "darwin" => Platform::MacOs,
        "windows" => Platform::Windows,
        _ => return Err(BuildError::InvalidTarget(input.to_owned())),
    };
    // aarch64-windows is out of scope
    if matches!(architecture, Architecture::Aarch64) && matches!(platform, Platform::Windows) {
        return Err(BuildError::InvalidTarget(input.to_owned()));
    }
    let triple_env =
        parse_env_segment(env).ok_or_else(|| BuildError::InvalidTarget(input.to_owned()))?;
    Ok(TargetTriple {
        arch: architecture,
        platform,
        env: Some(triple_env),
    })
}

/// Parse environment segment (msvc/gnu/musl).
fn parse_env_segment(s: &str) -> Option<TripleEnv> {
    match s {
        "msvc" => Some(TripleEnv::Msvc),
        "gnu" => Some(TripleEnv::Gnu),
        "musl" => Some(TripleEnv::Musl),
        _ => None,
    }
}

impl TargetTriple {
    /// Check if this is a Windows MSVC target (includes legacy 2-segment Windows as MSVC).
    #[must_use]
    pub fn is_windows_msvc(&self) -> bool {
        self.platform == Platform::Windows
            && (self.env == Some(TripleEnv::Msvc) || self.env.is_none())
    }

    /// Check if this is a Windows GNU target.
    #[must_use]
    pub fn is_windows_gnu(&self) -> bool {
        self.platform == Platform::Windows && self.env == Some(TripleEnv::Gnu)
    }

    /// Check if this is any Windows target.
    #[must_use]
    pub fn is_windows(&self) -> bool {
        self.platform == Platform::Windows
    }

    /// Get the host target triple based on compile-time cfg.
    #[must_use]
    pub const fn host() -> Self {
        Self {
            arch: if cfg!(target_arch = "aarch64") {
                Architecture::Aarch64
            } else {
                Architecture::X86_64
            },
            platform: if cfg!(target_os = "windows") {
                Platform::Windows
            } else if cfg!(target_os = "macos") {
                Platform::MacOs
            } else {
                Platform::Linux
            },
            env: if cfg!(target_env = "msvc") {
                Some(TripleEnv::Msvc)
            } else if cfg!(target_env = "musl") {
                Some(TripleEnv::Musl)
            } else {
                Some(TripleEnv::Gnu)
            },
        }
    }

    /// Convert to canonical Rust 4-segment triple string.
    #[must_use]
    pub fn to_rust_triple(&self) -> String {
        let arch = match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        };
        match (self.platform, self.env) {
            (Platform::Windows, Some(TripleEnv::Gnu)) => format!("{arch}-pc-windows-gnu"),
            (Platform::Windows, _) => format!("{arch}-pc-windows-msvc"),
            (Platform::Linux, Some(TripleEnv::Musl)) => format!("{arch}-unknown-linux-musl"),
            (Platform::Linux, _) => format!("{arch}-unknown-linux-gnu"),
            (Platform::MacOs, _) => format!("{arch}-apple-darwin"),
        }
    }

    /// Convert to LLVM-compatible triple string.
    ///
    /// Returns the same format as `to_rust_triple()` since LLVM uses the same
    /// 4-segment triple format (arch-vendor-os-env).
    #[must_use]
    pub fn to_llvm_string(&self) -> String {
        self.to_rust_triple()
    }
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

/// Return platform-specific object file extension.
///
/// Returns `.obj` for Windows MSVC targets, `.o` for all others.
/// Legacy 2-segment Windows triples (with `env == None`) resolve as MSVC.
///
/// # Examples
///
/// ```ignore
/// let msvc = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
/// assert_eq!(object_file_extension(&msvc), ".obj");
///
/// let linux = parse_target_triple("x86_64-linux").unwrap();
/// assert_eq!(object_file_extension(&linux), ".o");
/// ```
#[must_use]
pub fn object_file_extension(target: &TargetTriple) -> &'static str {
    if target.is_windows_msvc() {
        ".obj"
    } else {
        ".o"
    }
}

/// Return platform-specific executable filename with extension.
///
/// Appends `.exe` for Windows targets, nothing for others.
/// Legacy 2-segment Windows triples (with `env == None`) resolve as MSVC.
///
/// # Examples
///
/// ```ignore
/// let msvc = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
/// assert_eq!(executable_filename("prog", &msvc), "prog.exe");
///
/// let linux = parse_target_triple("x86_64-linux").unwrap();
/// assert_eq!(executable_filename("prog", &linux), "prog");
/// ```
#[must_use]
pub fn executable_filename(stem: &str, target: &TargetTriple) -> String {
    if target.is_windows() {
        alloc::format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}
