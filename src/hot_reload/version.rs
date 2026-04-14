//! Version management for hot-reload module artifacts.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::fmt;

/// Version number for a compiled hot-reload module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleVersion(u32);

impl ModuleVersion {
    /// Creates a new module version value.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw numeric version.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{:04}", self.0)
    }
}

/// Creates a platform-artifact name with embedded module version.
#[must_use]
pub fn versioned_module_name(base_name: &str, version: ModuleVersion) -> String {
    format!("{base_name}_{version}{}", shared_library_extension())
}

/// Returns the platform shared-library extension for hot-reload artifacts.
#[must_use]
const fn shared_library_extension() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return ".dll";
    }
    #[cfg(target_os = "macos")]
    {
        return ".dylib";
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        ".so"
    }
}
