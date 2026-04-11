//! Incremental build planning over module dependency graphs.

extern crate alloc;

use crate::hot_reload::dependency_graph::ModuleDependencyGraph;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

/// Compute the full set of modules to rebuild for a set of changed modules.
#[must_use]
pub fn modules_to_rebuild(changed: &[String], graph: &ModuleDependencyGraph) -> Vec<String> {
    let mut rebuild = BTreeSet::new();
    for module in changed {
        rebuild.insert(module.clone());
        for dependent in graph.transitive_dependents(module) {
            rebuild.insert(dependent);
        }
    }
    rebuild.into_iter().collect()
}
