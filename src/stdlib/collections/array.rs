//! Dynamic array (`Vec`-backed) for the Opalescent standard library.
//!
//! `OpalVec<T>` wraps `alloc::vec::Vec<T>` and provides a curated API matching
//! the Opalescent language specification: push, pop, insert, remove, slice, map,
//! filter, reduce, find, sort, reverse, contains, and length.
//!
//! # Design Rationale
//!
//! Rather than duplicating `src/runtime/arrays.rs` (which uses `Arc<[T]>` immutable
//! storage for the LLVM code-generation layer), `OpalVec` is a mutable, language-level
//! collection backed by `Vec<T>`. The runtime layer is for generated code execution;
//! this layer is for the language's user-facing standard library.
//!
//! # `no_std` compatibility
//!
//! Only `alloc` is used. This module is safe to link into embedded or LLVM targets.

extern crate alloc;

use alloc::vec::Vec;

/// Error type for `OpalVec` operations that can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VecError {
    /// The requested index is out of bounds.
    IndexOutOfBounds {
        /// The index that was requested.
        index: usize,
        /// The current length of the collection.
        length: usize,
    },
    /// The provided range is invalid (start > end, or end > length).
    InvalidRange {
        /// The start of the invalid range.
        start: usize,
        /// The end of the invalid range.
        end: usize,
        /// The current length of the collection.
        length: usize,
    },
}

/// A mutable, dynamically-sized array of elements of type `T`.
///
/// This is the language-level array type for Opalescent. It provides a curated,
/// ergonomic API built on top of `alloc::vec::Vec<T>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalVec<T> {
    /// The inner `Vec` holding the elements.
    inner: Vec<T>,
}

impl<T> OpalVec<T> {
    /// Create a new, empty `OpalVec`.
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Return the number of elements in this collection.
    #[must_use]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Return a reference to the element at `index`, or `None` if out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    /// Append `value` to the end of the collection.
    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    /// Remove and return the last element, or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    /// Insert `value` at `index`, shifting subsequent elements right.
    ///
    /// # Errors
    ///
    /// Returns [`VecError::IndexOutOfBounds`] when `index > self.length()`.
    pub fn insert(&mut self, index: usize, value: T) -> Result<(), VecError> {
        if index > self.inner.len() {
            return Err(VecError::IndexOutOfBounds {
                index,
                length: self.inner.len(),
            });
        }
        self.inner.insert(index, value);
        Ok(())
    }

    /// Remove and return the element at `index`, shifting subsequent elements left.
    ///
    /// # Errors
    ///
    /// Returns [`VecError::IndexOutOfBounds`] when `index >= self.length()`.
    pub fn remove(&mut self, index: usize) -> Result<T, VecError> {
        if index >= self.inner.len() {
            return Err(VecError::IndexOutOfBounds {
                index,
                length: self.inner.len(),
            });
        }
        Ok(self.inner.remove(index))
    }

    /// Return a new `OpalVec` containing elements in `[start, end)`.
    ///
    /// # Errors
    ///
    /// Returns [`VecError::InvalidRange`] when `start > end` or `end > self.length()`.
    pub fn slice(&self, start: usize, end: usize) -> Result<Self, VecError>
    where
        T: Clone,
    {
        if start > end || end > self.inner.len() {
            return Err(VecError::InvalidRange {
                start,
                end,
                length: self.inner.len(),
            });
        }
        Ok(Self {
            inner: self.inner[start..end].to_vec(),
        })
    }

    /// Apply `f` to each element and return a new `OpalVec` with the results.
    #[must_use]
    pub fn map<U, F>(&self, f: F) -> OpalVec<U>
    where
        F: Fn(T) -> U,
        T: Clone,
    {
        OpalVec {
            inner: self.inner.iter().map(|x| f(x.clone())).collect(),
        }
    }

    /// Return a new `OpalVec` containing only elements for which `predicate` returns `true`.
    #[must_use]
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        F: Fn(T) -> bool,
        T: Clone,
    {
        Self {
            inner: self
                .inner
                .iter()
                .filter(|x| predicate((*x).clone()))
                .cloned()
                .collect(),
        }
    }

    /// Fold all elements into an accumulator using `f`, starting from `initial`.
    #[must_use]
    pub fn reduce<Acc, F>(&self, initial: Acc, f: F) -> Acc
    where
        F: Fn(Acc, T) -> Acc,
        T: Clone,
    {
        self.inner.iter().fold(initial, |acc, x| f(acc, x.clone()))
    }

    /// Return the index of the first element for which `predicate` returns `true`, or `None`.
    #[must_use]
    pub fn find<F>(&self, predicate: F) -> Option<usize>
    where
        F: Fn(&T) -> bool,
    {
        self.inner.iter().position(predicate)
    }

    /// Return `true` if any element equals `value`.
    #[must_use]
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.inner.contains(value)
    }

    /// Reverse the order of elements in-place.
    pub fn reverse(&mut self) {
        self.inner.reverse();
    }
}

impl<T: Ord> OpalVec<T> {
    /// Sort elements in ascending order in-place.
    pub fn sort(&mut self) {
        self.inner.sort();
    }
}

impl<T> Default for OpalVec<T> {
    /// Create a new empty `OpalVec` (same as [`OpalVec::new`]).
    fn default() -> Self {
        Self::new()
    }
}
