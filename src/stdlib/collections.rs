//! Collections sub-modules for the Opalescent standard library.
//!
//! This module aggregates the following collection types:
//!
//! - [`array::OpalVec`] — dynamic array (Vec-backed)
//! - [`map::OpalMap`] — ordered key-value map (BTreeMap-backed)
//! - [`set::OpalSet`] — ordered unique-element set (BTreeSet-backed)
//! - [`list::OpalList`] — double-ended list (VecDeque-backed)
//! - [`iter::OpalIter`] — owning iterator adapter with higher-order operations

#[path = "collections/array.rs"]
pub mod array;
#[path = "collections/iter.rs"]
pub mod iter;
#[path = "collections/list.rs"]
pub mod list;
#[path = "collections/map.rs"]
pub mod map;
#[path = "collections/set.rs"]
pub mod set;

#[cfg(test)]
#[path = "collections/tests.rs"]
mod tests;
