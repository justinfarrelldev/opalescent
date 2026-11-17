//! Canonical declaration layout queries shared by ordinary and native lowering.
use super::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
extern crate alloc;
use alloc::collections::BTreeMap;

impl CodegenEnv<'_> {
    /// Resolve a local ADT owner or variant name to its canonical layout key.
    #[must_use]
    pub fn adt_layout_key<'name>(&'name self, owner: &'name str) -> &'name str {
        self.adt_layout_aliases
            .get(owner)
            .map_or(owner, String::as_str)
    }

    /// Look up ordered field layout metadata for a local or canonical ADT owner.
    #[must_use]
    pub fn adt_field_layout(&self, owner: &str) -> Option<&Vec<(String, CoreType)>> {
        let key = self.adt_layout_key(owner);
        self.adt_field_layouts
            .get(key)
            .or_else(|| self.adt_field_layouts.get(owner))
    }

    /// Look up field indices for a local or canonical ADT owner.
    #[must_use]
    pub fn adt_field_indices_for(&self, owner: &str) -> Option<&BTreeMap<String, u32>> {
        let key = self.adt_layout_key(owner);
        self.adt_field_indices
            .get(key)
            .or_else(|| self.adt_field_indices.get(owner))
    }

    /// Look up a sum variant discriminant for a local or canonical variant owner.
    #[must_use]
    pub fn adt_variant_discriminant(&self, owner: &str) -> Option<i64> {
        let key = self.adt_layout_key(owner);
        self.adt_variant_discriminants
            .get(key)
            .copied()
            .or_else(|| self.adt_variant_discriminants.get(owner).copied())
    }
}
