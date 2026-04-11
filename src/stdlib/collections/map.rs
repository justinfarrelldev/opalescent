//! Ordered map for the Opalescent standard library.
//!
//! `OpalMap<K, V>` wraps `alloc::collections::BTreeMap<K, V>` for deterministic
//! iteration order and `no_std` compatibility. It provides insert, get, remove,
//! `contains_key`, keys, values, entries, and length operations.
//!
//! # Design Rationale
//!
//! `BTreeMap` is chosen over `HashMap` because:
//! - It works in `no_std` / `alloc`-only environments (no hash randomness needed).
//! - Iteration order is deterministic and sorted by key.
//! - It aligns with the rest of the codebase's `alloc::collections` usage.
//!
//! The `get`, `remove`, and `contains_key` methods accept any type `Q` where
//! `K: Borrow<Q>`, mirroring the ergonomics of the standard `BTreeMap` API.
//!
//! # `no_std` compatibility
//!
//! Only `alloc` and `core` are used. This module is safe to link into embedded
//! or LLVM targets.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::borrow::Borrow;

/// An ordered, key-value map backed by `BTreeMap`.
///
/// Keys are ordered by their natural `Ord` implementation, giving deterministic
/// iteration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalMap<K, V> {
    /// The inner `BTreeMap` holding key-value pairs.
    inner: BTreeMap<K, V>,
}

impl<K: Ord, V> OpalMap<K, V> {
    /// Create a new, empty `OpalMap`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// Return the number of key-value pairs in this map.
    #[must_use]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Insert `value` for `key`. If `key` already exists, its value is replaced.
    pub fn insert(&mut self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    /// Return a reference to the value associated with `key`, or `None` if absent.
    ///
    /// Accepts any type `Q` where `K: Borrow<Q>` — e.g., passing `"hello"` for
    /// `K = String` or `K = &str`.
    #[must_use]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.get(key)
    }

    /// Remove and return the value associated with `key`, or `None` if absent.
    ///
    /// Accepts any type `Q` where `K: Borrow<Q>`.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.remove(key)
    }

    /// Return `true` if the map contains `key`.
    ///
    /// Accepts any type `Q` where `K: Borrow<Q>`.
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner.contains_key(key)
    }

    /// Return an iterator over all keys in ascending order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    /// Return an iterator over all values in key-ascending order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values()
    }

    /// Return an iterator over all key-value pairs in key-ascending order.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }
}

impl<K: Ord, V> Default for OpalMap<K, V> {
    /// Create a new empty `OpalMap` (same as [`OpalMap::new`]).
    fn default() -> Self {
        Self::new()
    }
}
