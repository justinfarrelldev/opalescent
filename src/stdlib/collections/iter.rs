//! Iterator adapter for the Opalescent standard library.
//!
//! `OpalIter<T>` is a lightweight, owning iterator adapter that wraps a
//! `alloc::vec::IntoIter<T>`. It exposes a curated set of higher-order operations
//! (`opal_map`, `opal_filter`, `opal_reduce`, `opal_take`, `opal_skip`,
//! `opal_enumerate`, `opal_zip`) under `opal_`-prefixed method names to avoid
//! conflicts with Rust's standard `Iterator` trait methods.
//!
//! # Design Rationale
//!
//! By prefixing methods with `opal_`, we avoid shadowing the standard `Iterator`
//! trait methods on the underlying iterator while still providing a clean public API.
//! The struct itself implements `Iterator<Item = T>` so it is composable with
//! standard Rust iterator adapters and can be used in `for` loops.
//!
//! # `no_std` compatibility
//!
//! Only `alloc` is used. This module is safe to link into embedded or LLVM targets.

extern crate alloc;

use alloc::vec::Vec;

/// An owning iterator adapter over a `Vec<T>`.
///
/// Created via [`OpalIter::from_vec`], this provides the Opalescent language's
/// iterator protocol with `opal_map`, `opal_filter`, `opal_reduce`, `opal_take`,
/// `opal_skip`, `opal_enumerate`, and `opal_zip` operations.
pub struct OpalIter<T> {
    /// The underlying iterator over the owned `Vec<T>`.
    inner: alloc::vec::IntoIter<T>,
}

impl<T> OpalIter<T> {
    /// Create a new `OpalIter` from an owned `Vec<T>`.
    #[must_use]
    pub fn from_vec(items: Vec<T>) -> Self {
        Self {
            inner: items.into_iter(),
        }
    }

    /// Apply `f` to each element, yielding a new `OpalIter<U>` with the transformed values.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::map`.
    #[must_use]
    pub fn opal_map<U, F>(self, f: F) -> OpalIter<U>
    where
        F: Fn(T) -> U,
    {
        OpalIter::from_vec(self.inner.map(f).collect())
    }

    /// Keep only elements for which `predicate` returns `true`.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::filter`.
    #[must_use]
    pub fn opal_filter<F>(self, predicate: F) -> Self
    where
        F: Fn(T) -> bool,
        T: Clone,
    {
        Self::from_vec(self.inner.filter(|x| predicate(x.clone())).collect())
    }

    /// Fold all elements into an accumulator starting from `initial`.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::fold`.
    #[must_use]
    pub fn opal_reduce<Acc, F>(self, initial: Acc, f: F) -> Acc
    where
        F: Fn(Acc, T) -> Acc,
    {
        self.inner.fold(initial, f)
    }

    /// Return a new iterator that yields only the first `n` elements.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::take`.
    #[must_use]
    pub fn opal_take(self, n: usize) -> Self {
        Self::from_vec(self.inner.take(n).collect())
    }

    /// Return a new iterator that skips the first `n` elements.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::skip`.
    #[must_use]
    pub fn opal_skip(self, n: usize) -> Self {
        Self::from_vec(self.inner.skip(n).collect())
    }

    /// Return a new iterator that pairs each element with its index `(index, element)`.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::enumerate`.
    #[must_use]
    pub fn opal_enumerate(self) -> OpalIter<(usize, T)> {
        OpalIter::from_vec(self.inner.enumerate().collect())
    }

    /// Pair elements from `self` and `other` into `(T, U)` tuples, stopping at the shorter.
    ///
    /// The `opal_` prefix avoids collision with `Iterator::zip`.
    #[must_use]
    pub fn opal_zip<U>(self, other: OpalIter<U>) -> OpalIter<(T, U)> {
        OpalIter::from_vec(self.inner.zip(other.inner).collect())
    }
}

impl<T> Iterator for OpalIter<T> {
    /// The item type produced by advancing this iterator.
    type Item = T;

    /// Advance the iterator and return the next element, or `None` if exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
