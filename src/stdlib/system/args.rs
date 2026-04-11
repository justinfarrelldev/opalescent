//! Command-line argument access — trait-based for mockability.
//!
//! The [`ArgsProvider`] trait abstracts over the process argument list so that
//! tests can inject deterministic argument sequences.  [`StdArgs`] delegates
//! to [`std::env::args`]; [`MockArgs`] owns a fixed `Vec<String>`.

extern crate alloc;
extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

/// Read-only access to command-line arguments.
pub trait ArgsProvider {
    /// Returns all arguments as an owned list of strings.
    ///
    /// The first element is conventionally the program name (argv\[0\]).
    fn args(&self) -> Vec<String>;

    /// Returns the number of arguments (including the program name).
    fn len(&self) -> usize {
        self.args().len()
    }

    /// Returns `true` when there are no arguments at all.
    ///
    /// This is unusual in practice because argv\[0\] is always the program
    /// name, but an implementation may choose to omit it.
    fn is_empty(&self) -> bool {
        self.args().is_empty()
    }

    /// Returns the argument at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<String> {
        self.args().into_iter().nth(index)
    }
}

/// Production [`ArgsProvider`] backed by the real process argument list.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdArgs;

impl ArgsProvider for StdArgs {
    fn args(&self) -> Vec<String> {
        std::env::args().collect()
    }
}

/// In-memory [`ArgsProvider`] for use in tests.
#[derive(Debug, Clone, Default)]
pub struct MockArgs {
    /// The fixed argument list served by this mock.
    pub inner: Vec<String>,
}

impl MockArgs {
    /// Creates a new [`MockArgs`] from a slice of string slices.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let args = MockArgs::new(&["opal", "--release", "build"]);
    /// assert_eq!(args.get(1), Some(String::from("--release")));
    /// ```
    #[must_use]
    pub fn new(items: &[&str]) -> Self {
        Self {
            inner: items.iter().map(|s| String::from(*s)).collect(),
        }
    }
}

impl ArgsProvider for MockArgs {
    fn args(&self) -> Vec<String> {
        self.inner.clone()
    }
}
