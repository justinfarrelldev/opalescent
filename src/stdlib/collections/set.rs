//! Ordered set for the Opalescent standard library.
//!
//! `OpalSet<T>` wraps `alloc::collections::BTreeSet<T>` for deterministic
//! iteration order and `no_std` compatibility. It provides insert, remove, contains,
//! union, intersection, difference, and length operations.
//!
//! # Design Rationale
//!
//! `BTreeSet` is chosen over `HashSet` because:
//! - It works in `no_std` / `alloc`-only environments.
//! - Iteration order is deterministic and sorted.
//! - Set operations (union, intersection, difference) are standard on `BTreeSet`.
//!
//! # `no_std` compatibility
//!
//! Only `alloc` is used. This module is safe to link into embedded or LLVM targets.

extern crate alloc;

use alloc::collections::BTreeSet;

/// An ordered set of unique elements of type `T`.
///
/// Elements are ordered by their natural `Ord` implementation, giving deterministic
/// iteration order and enabling efficient set operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalSet<T> {
    /// The inner `BTreeSet` holding unique elements.
    inner: BTreeSet<T>,
}

impl<T: Ord + Clone> OpalSet<T> {
    /// Create a new, empty `OpalSet`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: BTreeSet::new(),
        }
    }

    /// Return the number of elements in this set.
    #[must_use]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Insert `value` into the set. If the value already exists, the set is unchanged.
    pub fn insert(&mut self, value: T) {
        self.inner.insert(value);
    }

    /// Remove `value` from the set. Returns `true` if the element was present.
    pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
    }

    /// Return `true` if the set contains `value`.
    #[must_use]
    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }

    /// Return a new set containing all elements in `self` or `other` (or both).
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.union(&other.inner).cloned().collect(),
        }
    }

    /// Return a new set containing only elements present in both `self` and `other`.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.intersection(&other.inner).cloned().collect(),
        }
    }

    /// Return a new set containing elements in `self` that are not in `other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.difference(&other.inner).cloned().collect(),
        }
    }
}

impl<T: Ord + Clone> Default for OpalSet<T> {
    /// Create a new empty `OpalSet` (same as [`OpalSet::new`]).
    fn default() -> Self {
        Self::new()
    }
}
