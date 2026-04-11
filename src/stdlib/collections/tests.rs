//! Test suite for the Opalescent collections standard library.
//!
//! All tests are inline with no filesystem I/O. Tests follow TDD red-green-refactor
//! discipline: they were written before the corresponding implementation.
//!
//! # Test Coverage
//!
//! - [`OpalVec`] — dynamic array: push, pop, insert, remove, slice, map, filter, reduce,
//!   find, sort, reverse, contains, length
//! - [`OpalMap`] — ordered map: insert, get, remove, `contains_key`, keys, values, entries, length
//! - [`OpalSet`] — ordered set: insert, remove, contains, union, intersection, difference, length
//! - [`OpalList`] — double-ended list: `push_front`, `push_back`, `pop_front`, `pop_back`, length
//! - [`OpalIter`] — iterator: map, filter, reduce, collect, take, skip, enumerate, zip

#[cfg(test)]
#[expect(
    clippy::module_inception,
    reason = "test modules nested in test files follow this pattern throughout the codebase"
)]
mod tests {
    use crate::stdlib::collections::array::OpalVec;
    use crate::stdlib::collections::iter::OpalIter;
    use crate::stdlib::collections::list::OpalList;
    use crate::stdlib::collections::map::OpalMap;
    use crate::stdlib::collections::set::OpalSet;

    // =========================================================================
    // OpalVec — dynamic array
    // =========================================================================

    /// Verify a new `OpalVec` has length zero.
    #[test]
    fn test_vec_new_is_empty() {
        let v: OpalVec<i32> = OpalVec::new();
        assert_eq!(v.length(), 0_usize, "new OpalVec should be empty");
    }

    /// Verify `push` increments length.
    #[test]
    fn test_vec_push_increments_length() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        assert_eq!(v.length(), 1_usize, "push should increment length");
    }

    /// Verify `pop` returns the last element and decrements length.
    #[test]
    fn test_vec_pop_returns_last() {
        let mut v = OpalVec::new();
        v.push(42_i32);
        let result = v.pop();
        assert_eq!(result, Some(42_i32), "pop should return last element");
        assert_eq!(v.length(), 0_usize, "pop should decrement length");
    }

    /// Verify `pop` on an empty vec returns `None`.
    #[test]
    fn test_vec_pop_empty_returns_none() {
        let mut v: OpalVec<i32> = OpalVec::new();
        assert_eq!(v.pop(), None, "pop on empty vec should return None");
    }

    /// Verify `insert` places an element at the given index.
    #[test]
    fn test_vec_insert_at_index() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(3_i32);
        let result = v.insert(1_usize, 2_i32);
        assert!(result.is_ok(), "insert at valid index should succeed");
        assert_eq!(v.length(), 3_usize, "insert should increment length");
        assert_eq!(
            v.get(1_usize),
            Some(&2_i32),
            "inserted element should be at index 1"
        );
    }

    /// Verify `insert` at out-of-bounds index returns `Err`.
    #[test]
    fn test_vec_insert_out_of_bounds_returns_err() {
        let mut v: OpalVec<i32> = OpalVec::new();
        let result = v.insert(5_usize, 99_i32);
        assert!(result.is_err(), "insert out of bounds should return Err");
    }

    /// Verify `remove` removes element at index and returns it.
    #[test]
    fn test_vec_remove_returns_element() {
        let mut v = OpalVec::new();
        v.push(10_i32);
        v.push(20_i32);
        v.push(30_i32);
        let result = v.remove(1_usize);
        assert_eq!(
            result,
            Ok(20_i32),
            "remove should return the element at index"
        );
        assert_eq!(v.length(), 2_usize, "remove should decrement length");
    }

    /// Verify `remove` at out-of-bounds returns `Err`.
    #[test]
    fn test_vec_remove_out_of_bounds_returns_err() {
        let mut v: OpalVec<i32> = OpalVec::new();
        let result = v.remove(0_usize);
        assert!(result.is_err(), "remove out of bounds should return Err");
    }

    /// Verify `slice` returns a sub-vec for valid range.
    #[test]
    fn test_vec_slice_valid() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(2_i32);
        v.push(3_i32);
        v.push(4_i32);
        let result = v.slice(1_usize, 3_usize);
        assert!(result.is_ok(), "slice of valid range should succeed");
        let s = result.expect("already checked is_ok");
        assert_eq!(s.length(), 2_usize, "slice [1,3) should have length 2");
        assert_eq!(
            s.get(0_usize),
            Some(&2_i32),
            "first slice element should be 2"
        );
    }

    /// Verify `slice` with start > end returns `Err`.
    #[test]
    fn test_vec_slice_inverted_range_returns_err() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        let result = v.slice(1_usize, 0_usize);
        assert!(result.is_err(), "slice with start > end should return Err");
    }

    /// Verify `map` transforms each element.
    #[test]
    fn test_vec_map_doubles() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(2_i32);
        v.push(3_i32);
        let doubled = v.map(|x| x * 2_i32);
        assert_eq!(
            doubled.get(0_usize),
            Some(&2_i32),
            "map should double first element"
        );
        assert_eq!(
            doubled.get(1_usize),
            Some(&4_i32),
            "map should double second element"
        );
        assert_eq!(
            doubled.get(2_usize),
            Some(&6_i32),
            "map should double third element"
        );
    }

    /// Verify `filter` keeps only matching elements.
    #[test]
    fn test_vec_filter_evens() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(2_i32);
        v.push(3_i32);
        v.push(4_i32);
        let evens = v.filter(|x| x % 2_i32 == 0_i32);
        assert_eq!(
            evens.length(),
            2_usize,
            "filter should keep 2 even elements"
        );
    }

    /// Verify `reduce` folds elements.
    #[test]
    fn test_vec_reduce_sum() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(2_i32);
        v.push(3_i32);
        let sum = v.reduce(0_i32, |acc, x| acc + x);
        assert_eq!(sum, 6_i32, "reduce sum of [1,2,3] should be 6");
    }

    /// Verify `reduce` on empty vec returns initial accumulator.
    #[test]
    fn test_vec_reduce_empty_returns_initial() {
        let v: OpalVec<i32> = OpalVec::new();
        let result = v.reduce(99_i32, |acc, x| acc + x);
        assert_eq!(
            result, 99_i32,
            "reduce on empty should return initial accumulator"
        );
    }

    /// Verify `find` returns index of first match.
    #[test]
    fn test_vec_find_returns_index() {
        let mut v = OpalVec::new();
        v.push(10_i32);
        v.push(20_i32);
        v.push(30_i32);
        let result = v.find(|x| *x == 20_i32);
        assert_eq!(
            result,
            Some(1_usize),
            "find should return index 1 for value 20"
        );
    }

    /// Verify `find` returns `None` when no match.
    #[test]
    fn test_vec_find_no_match_returns_none() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        let result = v.find(|x| *x == 99_i32);
        assert_eq!(result, None, "find with no match should return None");
    }

    /// Verify `sort` orders elements ascending.
    #[test]
    fn test_vec_sort_ascending() {
        let mut v = OpalVec::new();
        v.push(3_i32);
        v.push(1_i32);
        v.push(2_i32);
        v.sort();
        assert_eq!(
            v.get(0_usize),
            Some(&1_i32),
            "sorted first element should be 1"
        );
        assert_eq!(
            v.get(1_usize),
            Some(&2_i32),
            "sorted second element should be 2"
        );
        assert_eq!(
            v.get(2_usize),
            Some(&3_i32),
            "sorted third element should be 3"
        );
    }

    /// Verify `reverse` inverts element order.
    #[test]
    fn test_vec_reverse() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        v.push(2_i32);
        v.push(3_i32);
        v.reverse();
        assert_eq!(
            v.get(0_usize),
            Some(&3_i32),
            "reversed first element should be 3"
        );
        assert_eq!(
            v.get(2_usize),
            Some(&1_i32),
            "reversed last element should be 1"
        );
    }

    /// Verify `contains` returns true when element is present.
    #[test]
    fn test_vec_contains_present() {
        let mut v = OpalVec::new();
        v.push(42_i32);
        assert!(
            v.contains(&42_i32),
            "contains should return true for present element"
        );
    }

    /// Verify `contains` returns false when element is absent.
    #[test]
    fn test_vec_contains_absent() {
        let mut v = OpalVec::new();
        v.push(1_i32);
        assert!(
            !v.contains(&99_i32),
            "contains should return false for absent element"
        );
    }

    // =========================================================================
    // OpalMap — ordered map backed by BTreeMap
    // =========================================================================

    /// Verify new `OpalMap` has length zero.
    #[test]
    fn test_map_new_is_empty() {
        let m: OpalMap<&str, i32> = OpalMap::new();
        assert_eq!(m.length(), 0_usize, "new OpalMap should be empty");
    }

    /// Verify `insert` adds a key-value pair.
    #[test]
    fn test_map_insert_and_get() {
        let mut m = OpalMap::new();
        m.insert("key", 42_i32);
        assert_eq!(
            m.get("key"),
            Some(&42_i32),
            "get should return inserted value"
        );
    }

    /// Verify `insert` with existing key updates value.
    #[test]
    fn test_map_insert_updates_value() {
        let mut m = OpalMap::new();
        m.insert("k", 1_i32);
        m.insert("k", 2_i32);
        assert_eq!(
            m.get("k"),
            Some(&2_i32),
            "second insert should update value"
        );
        assert_eq!(m.length(), 1_usize, "duplicate key should not grow length");
    }

    /// Verify `remove` removes a key.
    #[test]
    fn test_map_remove() {
        let mut m = OpalMap::new();
        m.insert("k", 99_i32);
        let removed = m.remove("k");
        assert_eq!(removed, Some(99_i32), "remove should return removed value");
        assert_eq!(m.length(), 0_usize, "remove should decrement length");
    }

    /// Verify `remove` on missing key returns `None`.
    #[test]
    fn test_map_remove_missing_returns_none() {
        let mut m: OpalMap<&str, i32> = OpalMap::new();
        let result = m.remove("missing");
        assert_eq!(result, None, "remove of missing key should return None");
    }

    /// Verify `contains_key` returns true for present key.
    #[test]
    fn test_map_contains_key_present() {
        let mut m = OpalMap::new();
        m.insert("present", 1_i32);
        assert!(
            m.contains_key("present"),
            "contains_key should return true for present key"
        );
    }

    /// Verify `contains_key` returns false for absent key.
    #[test]
    fn test_map_contains_key_absent() {
        let m: OpalMap<&str, i32> = OpalMap::new();
        assert!(
            !m.contains_key("absent"),
            "contains_key should return false for absent key"
        );
    }

    /// Verify `keys` returns all keys in sorted order.
    #[test]
    fn test_map_keys_sorted() {
        let mut m = OpalMap::new();
        m.insert("b", 2_i32);
        m.insert("a", 1_i32);
        m.insert("c", 3_i32);
        let keys: Vec<&&str> = m.keys().collect();
        assert_eq!(
            keys,
            vec![&"a", &"b", &"c"],
            "keys should be in sorted order"
        );
    }

    /// Verify `values` returns all values.
    #[test]
    fn test_map_values_count() {
        let mut m = OpalMap::new();
        m.insert("x", 10_i32);
        m.insert("y", 20_i32);
        let count = m.values().count();
        assert_eq!(count, 2_usize, "values should yield 2 entries");
    }

    /// Verify `entries` returns key-value pairs.
    #[test]
    fn test_map_entries_count() {
        let mut m = OpalMap::new();
        m.insert("a", 1_i32);
        m.insert("b", 2_i32);
        let count = m.entries().count();
        assert_eq!(count, 2_usize, "entries should yield 2 pairs");
    }

    /// Verify `length` reflects current count.
    #[test]
    fn test_map_length() {
        let mut m = OpalMap::new();
        m.insert("x", 1_i32);
        m.insert("y", 2_i32);
        assert_eq!(m.length(), 2_usize, "length should be 2 after two inserts");
    }

    // =========================================================================
    // OpalSet — ordered set backed by BTreeSet
    // =========================================================================

    /// Verify new `OpalSet` has length zero.
    #[test]
    fn test_set_new_is_empty() {
        let s: OpalSet<i32> = OpalSet::new();
        assert_eq!(s.length(), 0_usize, "new OpalSet should be empty");
    }

    /// Verify `insert` adds an element.
    #[test]
    fn test_set_insert() {
        let mut s = OpalSet::new();
        s.insert(42_i32);
        assert_eq!(s.length(), 1_usize, "insert should increment length");
    }

    /// Verify `insert` of duplicate does not grow set.
    #[test]
    fn test_set_insert_duplicate() {
        let mut s = OpalSet::new();
        s.insert(1_i32);
        s.insert(1_i32);
        assert_eq!(
            s.length(),
            1_usize,
            "duplicate insert should not grow length"
        );
    }

    /// Verify `remove` removes element.
    #[test]
    fn test_set_remove() {
        let mut s = OpalSet::new();
        s.insert(5_i32);
        let removed = s.remove(&5_i32);
        assert!(removed, "remove should return true for present element");
        assert_eq!(s.length(), 0_usize, "remove should decrement length");
    }

    /// Verify `remove` on absent element returns false.
    #[test]
    fn test_set_remove_absent() {
        let mut s: OpalSet<i32> = OpalSet::new();
        let result = s.remove(&99_i32);
        assert!(!result, "remove on absent element should return false");
    }

    /// Verify `contains` returns true for present element.
    #[test]
    fn test_set_contains_present() {
        let mut s = OpalSet::new();
        s.insert(7_i32);
        assert!(
            s.contains(&7_i32),
            "contains should return true for present element"
        );
    }

    /// Verify `contains` returns false for absent element.
    #[test]
    fn test_set_contains_absent() {
        let s: OpalSet<i32> = OpalSet::new();
        assert!(
            !s.contains(&7_i32),
            "contains should return false for absent element"
        );
    }

    /// Verify `union` produces a set containing elements from both.
    #[test]
    fn test_set_union() {
        let mut a = OpalSet::new();
        a.insert(1_i32);
        a.insert(2_i32);
        let mut b = OpalSet::new();
        b.insert(2_i32);
        b.insert(3_i32);
        let u = a.union(&b);
        assert_eq!(u.length(), 3_usize, "union should have 3 unique elements");
        assert!(u.contains(&1_i32), "union should contain 1");
        assert!(u.contains(&3_i32), "union should contain 3");
    }

    /// Verify `intersection` produces only shared elements.
    #[test]
    fn test_set_intersection() {
        let mut a = OpalSet::new();
        a.insert(1_i32);
        a.insert(2_i32);
        let mut b = OpalSet::new();
        b.insert(2_i32);
        b.insert(3_i32);
        let i = a.intersection(&b);
        assert_eq!(
            i.length(),
            1_usize,
            "intersection should have 1 shared element"
        );
        assert!(i.contains(&2_i32), "intersection should contain 2");
    }

    /// Verify `difference` produces elements in `self` not in `other`.
    #[test]
    fn test_set_difference() {
        let mut a = OpalSet::new();
        a.insert(1_i32);
        a.insert(2_i32);
        a.insert(3_i32);
        let mut b = OpalSet::new();
        b.insert(2_i32);
        let d = a.difference(&b);
        assert_eq!(d.length(), 2_usize, "difference should have 2 elements");
        assert!(d.contains(&1_i32), "difference should contain 1");
        assert!(d.contains(&3_i32), "difference should contain 3");
    }

    // =========================================================================
    // OpalList — double-ended list backed by VecDeque
    // =========================================================================

    /// Verify new `OpalList` has length zero.
    #[test]
    fn test_list_new_is_empty() {
        let l: OpalList<i32> = OpalList::new();
        assert_eq!(l.length(), 0_usize, "new OpalList should be empty");
    }

    /// Verify `push_back` appends element.
    #[test]
    fn test_list_push_back() {
        let mut l = OpalList::new();
        l.push_back(1_i32);
        l.push_back(2_i32);
        assert_eq!(l.length(), 2_usize, "push_back should increment length");
    }

    /// Verify `push_front` prepends element.
    #[test]
    fn test_list_push_front() {
        let mut l = OpalList::new();
        l.push_back(2_i32);
        l.push_front(1_i32);
        assert_eq!(
            l.pop_front(),
            Some(1_i32),
            "pop_front should return front element"
        );
    }

    /// Verify `pop_front` removes and returns front element.
    #[test]
    fn test_list_pop_front() {
        let mut l = OpalList::new();
        l.push_back(10_i32);
        l.push_back(20_i32);
        let front = l.pop_front();
        assert_eq!(front, Some(10_i32), "pop_front should return first element");
        assert_eq!(l.length(), 1_usize, "pop_front should decrement length");
    }

    /// Verify `pop_back` removes and returns back element.
    #[test]
    fn test_list_pop_back() {
        let mut l = OpalList::new();
        l.push_back(10_i32);
        l.push_back(20_i32);
        let back = l.pop_back();
        assert_eq!(back, Some(20_i32), "pop_back should return last element");
        assert_eq!(l.length(), 1_usize, "pop_back should decrement length");
    }

    /// Verify `pop_front` on empty list returns `None`.
    #[test]
    fn test_list_pop_front_empty_returns_none() {
        let mut l: OpalList<i32> = OpalList::new();
        assert_eq!(
            l.pop_front(),
            None,
            "pop_front on empty list should return None"
        );
    }

    /// Verify `pop_back` on empty list returns `None`.
    #[test]
    fn test_list_pop_back_empty_returns_none() {
        let mut l: OpalList<i32> = OpalList::new();
        assert_eq!(
            l.pop_back(),
            None,
            "pop_back on empty list should return None"
        );
    }

    // =========================================================================
    // OpalIter — iterator adapter
    // =========================================================================

    /// Verify `OpalIter::from_vec` creates iterator over vec elements.
    #[test]
    fn test_iter_from_vec_collect() {
        let items = vec![1_i32, 2_i32, 3_i32];
        let iter = OpalIter::from_vec(items);
        let collected: Vec<i32> = iter.collect();
        assert_eq!(
            collected,
            vec![1_i32, 2_i32, 3_i32],
            "collect should return all items"
        );
    }

    /// Verify `map` on `OpalIter` transforms each element.
    #[test]
    fn test_iter_map_doubles() {
        let iter = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32]);
        let doubled: Vec<i32> = iter.opal_map(|x| x * 2_i32).collect();
        assert_eq!(
            doubled,
            vec![2_i32, 4_i32, 6_i32],
            "map should double all elements"
        );
    }

    /// Verify `filter` keeps only matching elements.
    #[test]
    fn test_iter_filter_evens() {
        let iter = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32, 4_i32]);
        let evens: Vec<i32> = iter.opal_filter(|x| x % 2_i32 == 0_i32).collect();
        assert_eq!(
            evens,
            vec![2_i32, 4_i32],
            "filter should keep only even elements"
        );
    }

    /// Verify `reduce` folds elements to a value.
    #[test]
    fn test_iter_reduce_sum() {
        let iter = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32, 4_i32]);
        let sum = iter.opal_reduce(0_i32, |acc, x| acc + x);
        assert_eq!(sum, 10_i32, "reduce sum should be 10");
    }

    /// Verify `take` returns only the first N elements.
    #[test]
    fn test_iter_take() {
        let iter = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32]);
        let taken: Vec<i32> = iter.opal_take(3_usize).collect();
        assert_eq!(
            taken,
            vec![1_i32, 2_i32, 3_i32],
            "take(3) should return first 3 elements"
        );
    }

    /// Verify `skip` skips the first N elements.
    #[test]
    fn test_iter_skip() {
        let iter = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32]);
        let skipped: Vec<i32> = iter.opal_skip(2_usize).collect();
        assert_eq!(
            skipped,
            vec![3_i32, 4_i32, 5_i32],
            "skip(2) should skip first 2 elements"
        );
    }

    /// Verify `enumerate` pairs elements with their index.
    #[test]
    fn test_iter_enumerate() {
        let iter = OpalIter::from_vec(vec![10_i32, 20_i32, 30_i32]);
        let enumerated: Vec<(usize, i32)> = iter.opal_enumerate().collect();
        assert_eq!(
            enumerated,
            vec![(0_usize, 10_i32), (1_usize, 20_i32), (2_usize, 30_i32)],
            "enumerate should pair indices with values"
        );
    }

    /// Verify `zip` pairs elements from two iterators.
    #[test]
    fn test_iter_zip() {
        let a = OpalIter::from_vec(vec![1_i32, 2_i32, 3_i32]);
        let b = OpalIter::from_vec(vec![4_i32, 5_i32, 6_i32]);
        let zipped: Vec<(i32, i32)> = a.opal_zip(b).collect();
        assert_eq!(
            zipped,
            vec![(1_i32, 4_i32), (2_i32, 5_i32), (3_i32, 6_i32)],
            "zip should pair corresponding elements"
        );
    }

    /// Verify `zip` stops at shorter iterator.
    #[test]
    fn test_iter_zip_unequal_lengths() {
        let a = OpalIter::from_vec(vec![1_i32, 2_i32]);
        let b = OpalIter::from_vec(vec![10_i32, 20_i32, 30_i32]);
        let zipped_len = a.opal_zip(b).count();
        assert_eq!(
            zipped_len, 2_usize,
            "zip should stop at length of shorter iterator"
        );
    }
}
