//! Platform detection — operating system and CPU architecture identification.
//!
//! All detection is done at compile time via Rust's `cfg` machinery.  No
//! dynamic dispatch or runtime system calls are made here; the values are
//! baked into the binary at compile time and exposed as enum variants.

/// Operating system family detected at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum OsKind {
    /// Linux (including Android).
    Linux,
    /// macOS / iOS / tvOS / watchOS.
    MacOs,
    /// Windows.
    Windows,
    /// FreeBSD.
    FreeBsd,
    /// OpenBSD.
    OpenBsd,
    /// NetBSD.
    NetBsd,
    /// Any other OS not explicitly listed above.
    Other,
}

/// CPU architecture detected at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Arch {
    /// 64-bit x86 (AMD64 / Intel 64).
    X86_64,
    /// 32-bit x86.
    X86,
    /// 64-bit ARM (`AArch64`).
    Aarch64,
    /// 32-bit ARM.
    Arm,
    /// 64-bit RISC-V.
    Riscv64,
    /// 64-bit MIPS.
    Mips64,
    /// WebAssembly.
    Wasm32,
    /// Any other architecture not explicitly listed above.
    Other,
}

/// Full platform description combining OS and architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// Operating system.
    pub os: OsKind,
    /// CPU architecture.
    pub arch: Arch,
}

impl Platform {
    /// Returns the platform the binary was compiled for.
    ///
    /// All values are resolved entirely at compile time.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            os: current_os(),
            arch: current_arch(),
        }
    }

    /// Returns `true` when running on Linux.
    #[must_use]
    pub const fn is_linux(self) -> bool {
        matches!(self.os, OsKind::Linux)
    }

    /// Returns `true` when running on macOS.
    #[must_use]
    pub const fn is_macos(self) -> bool {
        matches!(self.os, OsKind::MacOs)
    }

    /// Returns `true` when running on Windows.
    #[must_use]
    pub const fn is_windows(self) -> bool {
        matches!(self.os, OsKind::Windows)
    }

    /// Returns `true` when running on a 64-bit x86 CPU.
    #[must_use]
    pub const fn is_x86_64(self) -> bool {
        matches!(self.arch, Arch::X86_64)
    }

    /// Returns `true` when running on a 64-bit ARM CPU.
    #[must_use]
    pub const fn is_aarch64(self) -> bool {
        matches!(self.arch, Arch::Aarch64)
    }
}

/// Compile-time OS detection.
#[must_use]
const fn current_os() -> OsKind {
    #[cfg(target_os = "linux")]
    return OsKind::Linux;
    #[cfg(target_os = "macos")]
    return OsKind::MacOs;
    #[cfg(target_os = "windows")]
    return OsKind::Windows;
    #[cfg(target_os = "freebsd")]
    return OsKind::FreeBsd;
    #[cfg(target_os = "openbsd")]
    return OsKind::OpenBsd;
    #[cfg(target_os = "netbsd")]
    return OsKind::NetBsd;
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
    )))]
    return OsKind::Other;
}

/// Compile-time architecture detection.
#[must_use]
const fn current_arch() -> Arch {
    #[cfg(target_arch = "x86_64")]
    return Arch::X86_64;
    #[cfg(target_arch = "x86")]
    return Arch::X86;
    #[cfg(target_arch = "aarch64")]
    return Arch::Aarch64;
    #[cfg(target_arch = "arm")]
    return Arch::Arm;
    #[cfg(target_arch = "riscv64")]
    return Arch::Riscv64;
    #[cfg(target_arch = "mips64")]
    return Arch::Mips64;
    #[cfg(target_arch = "wasm32")]
    return Arch::Wasm32;
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64",
        target_arch = "mips64",
        target_arch = "wasm32",
    )))]
    return Arch::Other;
}
