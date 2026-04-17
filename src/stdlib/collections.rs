//! Collections sub-modules for the Opalescent standard library.
//!
//! This module aggregates the following collection types:
//!
//! - [`array::OpalVec`] — dynamic array (Vec-backed)
//! - [`map::OpalMap`] — ordered key-value map (BTreeMap-backed)
//! - [`set::OpalSet`] — ordered unique-element set (BTreeSet-backed)
//! - [`list::OpalList`] — double-ended list (VecDeque-backed)
//! - [`iter::OpalIter`] — owning iterator adapter with higher-order operations

pub mod array;
pub mod iter;
pub mod list;
pub mod map;
pub mod set;

#[cfg(test)]
mod tests;
