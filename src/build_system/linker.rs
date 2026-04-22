//! Linker detection and selection based on target triple.

use crate::build_system::targets::{Platform, TargetTriple, TripleEnv};

/// Supported linker variants for different platforms and toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linker {
    /// Microsoft Visual C++ linker (link.exe).
    Msvc,
    /// MinGW-w64 linker (x86_64-w64-mingw32-gcc).
    MinGw,
    /// Clang linker (clang).
    Clang,
    /// Generic C compiler linker (cc/gcc).
    Cc,
}

impl Linker {
    /// Return human-readable binary name for this linker.
    #[must_use]
    pub const fn binary_name(&self) -> &'static str {
        match self {
            Self::Msvc => "link.exe",
            Self::MinGw => "x86_64-w64-mingw32-gcc",
            Self::Clang => "clang",
            Self::Cc => "cc",
        }
    }
}

/// Detect the preferred linker for a given target triple.
///
/// Returns the linker variant based on the target platform and environment:
/// - `*-windows-msvc` → `Linker::Msvc`
/// - `*-windows-gnu` → `Linker::MinGw`
/// - `*-linux-gnu` / `*-linux-musl` → `Linker::Cc`
/// - `*-darwin` → `Linker::Clang`
///
/// # Examples
///
/// ```ignore
/// let msvc = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
/// assert_eq!(detect_preferred_linker(&msvc), Linker::Msvc);
///
/// let linux = parse_target_triple("x86_64-linux").unwrap();
/// assert_eq!(detect_preferred_linker(&linux), Linker::Cc);
/// ```
#[must_use]
pub fn detect_preferred_linker(target: &TargetTriple) -> Linker {
    match (target.platform, target.env) {
        (Platform::Windows, Some(TripleEnv::Gnu)) => Linker::MinGw,
        (Platform::Windows, _) => Linker::Msvc,
        (Platform::Linux, _) => Linker::Cc,
        (Platform::MacOs, _) => Linker::Clang,
    }
}
