//! Environment variable access — trait-based for mockability.
//!
//! The [`EnvProvider`] trait abstracts over the OS environment so that tests
//! can inject deterministic values without touching the real process
//! environment.  [`StdEnv`] is the production implementation that delegates
//! to [`std::env`]; [`MockEnv`] is the in-memory test double.

extern crate alloc;
extern crate std;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Read-only access to environment variables.
pub trait EnvProvider {
    /// Returns the value of the environment variable `name`, or `None` if it
    /// is not set or its value is not valid UTF-8.
    fn get(&self, name: &str) -> Option<String>;

    /// Returns all environment variables as a list of `(name, value)` pairs.
    fn all(&self) -> Vec<(String, String)>;

    /// Returns `true` if the environment variable `name` is set.
    fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

/// Production [`EnvProvider`] backed by the real process environment.
///
/// Uses [`std::env::var`] and [`std::env::vars`] internally.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdEnv;

impl EnvProvider for StdEnv {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn all(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
}

/// In-memory [`EnvProvider`] for use in tests.
///
/// Pre-populate via [`MockEnv::new`] with a slice of `(name, value)` pairs.
#[derive(Debug, Clone, Default)]
pub struct MockEnv {
    /// Internal storage mapping variable names to values.
    pub vars: BTreeMap<String, String>,
}

impl MockEnv {
    /// Creates a new [`MockEnv`] pre-populated with `entries`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env = MockEnv::new(&[("HOME", "/root"), ("PATH", "/usr/bin")]);
    /// assert_eq!(env.get("HOME"), Some(String::from("/root")));
    /// ```
    #[must_use]
    pub fn new(entries: &[(&str, &str)]) -> Self {
        let mut vars = BTreeMap::new();
        for &(k, v) in entries {
            drop(vars.insert(String::from(k), String::from(v)));
        }
        Self { vars }
    }

    /// Inserts or replaces an environment variable.
    pub fn set(&mut self, name: &str, value: &str) {
        drop(self.vars.insert(String::from(name), String::from(value)));
    }

    /// Removes an environment variable.
    pub fn remove(&mut self, name: &str) {
        drop(self.vars.remove(name));
    }
}

impl EnvProvider for MockEnv {
    fn get(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }

    fn all(&self) -> Vec<(String, String)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}
