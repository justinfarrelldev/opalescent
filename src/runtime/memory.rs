extern crate alloc;

use crate::runtime::errors::RuntimeResult;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

/// Runtime memory strategy for Opalescent values.
///
/// The runtime currently uses deterministic reference counting through `Arc<T>` for
/// strings and arrays (`Arc<str>`, `Arc<[T]>`). This keeps ownership simple across
/// generated code and host integration while providing thread-safe shared values.
///
/// ## Cycle detection strategy
///
/// Reference counting does not reclaim cyclic graphs. For future self-referential
/// runtime structures, cycle edges must be represented with [`Weak<T>`] to break
/// strong ownership loops. [`OpalWeakRef`] provides the weak-reference API surface
/// for that strategy.
///
/// A tracing collector may be introduced in a later runtime milestone; until then,
/// generated runtime data structures must avoid all-strong reference cycles.
///
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

/// Weak runtime reference used to break strong-reference cycles.
#[derive(Debug, Clone)]
pub struct OpalWeakRef<T>
where
    T: ?Sized,
{
    /// Weak backing storage that does not keep the allocation alive.
    value: Weak<T>,
}

impl OpalWeakRef<str> {
    /// Create a weak string reference from a strong runtime string.
    #[must_use]
    pub fn from_string(value: &OpalString) -> Self {
        Self {
            value: Arc::downgrade(value.arc()),
        }
    }

    /// Upgrade weak reference to strong runtime string when still alive.
    #[must_use]
    pub fn upgrade_string(&self) -> Option<OpalString> {
        self.value.upgrade().map(|value| OpalString { value })
    }
}

impl<T> OpalWeakRef<[T]> {
    /// Create a weak array reference from a strong runtime array.
    #[must_use]
    pub fn from_array(value: &OpalArray<T>) -> Self {
        Self {
            value: Arc::downgrade(value.arc()),
        }
    }

    /// Upgrade weak reference to strong runtime array when still alive.
    #[must_use]
    pub fn upgrade_array(&self) -> Option<OpalArray<T>> {
        self.value.upgrade().map(|elements| OpalArray { elements })
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
