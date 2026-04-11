//! Double-ended list for the Opalescent standard library.
//!
//! `OpalList<T>` wraps `alloc::collections::VecDeque<T>` to provide efficient
//! O(1) push/pop at both ends. It supports `push_front`, `push_back`, `pop_front`,
//! `pop_back`, and length operations.
//!
//! # Design Rationale
//!
//! `VecDeque` is chosen over a linked list because:
//! - It provides O(1) amortized push/pop at both ends.
//! - It avoids the unsafe code and allocator overhead of node-based linked lists.
//! - It works in `no_std` / `alloc`-only environments.
//!
//! # `no_std` compatibility
//!
//! Only `alloc` is used. This module is safe to link into embedded or LLVM targets.

extern crate alloc;

use alloc::collections::VecDeque;

/// A double-ended list that supports O(1) push and pop at both ends.
///
/// Backed by `VecDeque<T>`, this provides an efficient deque interface for the
/// Opalescent language's list type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpalList<T> {
    /// The inner `VecDeque` holding the elements.
    inner: VecDeque<T>,
}

impl<T> OpalList<T> {
    /// Create a new, empty `OpalList`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    /// Return the number of elements in this list.
    #[must_use]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Append `value` to the back of the list.
    pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
    }

    /// Prepend `value` to the front of the list.
    pub fn push_front(&mut self, value: T) {
        self.inner.push_front(value);
    }

    /// Remove and return the front element, or `None` if empty.
    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    /// Remove and return the back element, or `None` if empty.
    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }
}

impl<T> Default for OpalList<T> {
    /// Create a new empty `OpalList` (same as [`OpalList::new`]).
    fn default() -> Self {
        Self::new()
    }
}
