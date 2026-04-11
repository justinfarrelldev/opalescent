//! Build cache keyed by module/file name and source hash.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Cache storing latest known content hash for each module/file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildCache {
    /// Mapping from logical module/file name to hash value.
    pub entries: BTreeMap<String, u64>,
}

impl BuildCache {
    /// Creates an empty build cache.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Determines whether cached content hash matches the current content.
    #[must_use]
    pub fn is_cache_hit(&self, name: &str, content: &str) -> bool {
        let content_hash = hash_content(content);
        self.entries
            .get(name)
            .is_some_and(|cached_hash| *cached_hash == content_hash)
    }

    /// Updates cache entry for one module/file content.
    pub fn update_cache(&mut self, name: &str, content: &str) {
        self.entries.insert(name.to_owned(), hash_content(content));
    }
}

/// Hashes in-memory source content for build-cache comparisons.
#[must_use]
pub fn hash_content(content: &str) -> u64 {
    let mut hash = 5_381_u64;
    for byte in content.bytes() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(u64::from(byte));
    }
    hash
}
