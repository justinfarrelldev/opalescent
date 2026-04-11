//! ABI signature cache for incremental hot-reload analysis.

extern crate alloc;

use crate::hot_reload::abi::AbiSignature;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Cache keyed by module name for ABI signatures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AbiSignatureCache {
    /// Cached signatures by logical module name.
    signatures: BTreeMap<String, AbiSignature>,
}

impl AbiSignatureCache {
    /// Creates an empty ABI signature cache.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            signatures: BTreeMap::new(),
        }
    }

    /// Gets a cached signature for a module.
    #[must_use]
    pub fn get(&self, module_name: &str) -> Option<&AbiSignature> {
        self.signatures.get(module_name)
    }

    /// Inserts or replaces a module signature.
    pub fn insert(&mut self, module_name: &str, signature: AbiSignature) {
        self.signatures.insert(module_name.to_owned(), signature);
    }

    /// Removes and returns a module signature from cache.
    pub fn remove(&mut self, module_name: &str) -> Option<AbiSignature> {
        self.signatures.remove(module_name)
    }

    /// Returns whether a module signature exists in cache.
    #[must_use]
    pub fn contains_module(&self, module_name: &str) -> bool {
        self.signatures.contains_key(module_name)
    }

    /// Returns number of cached module signatures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Returns true when no signatures are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    /// Gets a cached signature or computes/inserts it when absent.
    pub fn get_or_insert_with<F>(&mut self, module_name: &str, build: F) -> AbiSignature
    where
        F: FnOnce() -> AbiSignature,
    {
        if let Some(signature) = self.signatures.get(module_name) {
            return signature.clone();
        }

        let signature = build();
        self.signatures
            .insert(module_name.to_owned(), signature.clone());
        signature
    }
}
