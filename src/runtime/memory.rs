extern crate alloc;

use crate::runtime::errors::RuntimeResult;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Reference counting (Rc/Arc) for initial implementation; tracing GC deferred to future milestone.
/// Runtime allocator abstraction used by generated code and tests.
pub trait RuntimeAllocator {
    /// Allocate an Opalescent string value.
    ///
    /// # Errors
    ///
    /// Returns a runtime allocation error when string allocation fails.
    fn allocate_string(&self, value: &str) -> RuntimeResult<OpalString>;

    /// Allocate an Opalescent array value.
    ///
    /// # Errors
    ///
    /// Returns a runtime allocation error when array allocation fails.
    fn allocate_array<T>(&self, values: &[T]) -> RuntimeResult<OpalArray<T>>
    where
        T: Clone;
}

/// Reference-counted runtime string container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalString {
    /// Reference-counted UTF-8 storage.
    value: Arc<str>,
}

impl OpalString {
    /// Create a runtime string from owned content.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self {
            value: Arc::<str>::from(value),
        }
    }

    /// Borrow the underlying UTF-8 contents.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc deref is not const-evaluable on stable Rust"
    )]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Expose the reference-counted backing storage.
    #[must_use]
    pub const fn arc(&self) -> &Arc<str> {
        &self.value
    }
}

/// Reference-counted runtime array container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalArray<T> {
    /// Reference-counted contiguous element storage.
    elements: Arc<[T]>,
}

impl<T> OpalArray<T> {
    /// Create a runtime array from owned elements.
    #[must_use]
    pub fn new(elements: Vec<T>) -> Self {
        Self {
            elements: Arc::<[T]>::from(elements),
        }
    }

    /// Expose the reference-counted backing storage.
    #[must_use]
    pub const fn arc(&self) -> &Arc<[T]> {
        &self.elements
    }

    /// Return element count.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc slice deref is not const-evaluable on stable Rust"
    )]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Return whether the array has no elements.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc slice deref is not const-evaluable on stable Rust"
    )]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Borrow an element by index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.elements.get(index)
    }
}

/// Default host allocator implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultRuntimeAllocator;

impl RuntimeAllocator for DefaultRuntimeAllocator {
    fn allocate_string(&self, value: &str) -> RuntimeResult<OpalString> {
        Ok(OpalString::new(value.to_owned()))
    }

    fn allocate_array<T>(&self, values: &[T]) -> RuntimeResult<OpalArray<T>>
    where
        T: Clone,
    {
        Ok(OpalArray::new(values.to_vec()))
    }
}
