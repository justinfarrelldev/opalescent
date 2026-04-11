//! Module dependency graph for transitive invalidation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

/// Directed module dependency graph.
///
/// The edge `dependent -> dependency` means `dependent` imports `dependency`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleDependencyGraph {
    /// Adjacency list storing dependent-to-dependency edges.
    dependencies: BTreeMap<String, Vec<String>>,
}

impl ModuleDependencyGraph {
    /// Creates an empty dependency graph.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }

    /// Registers one dependency edge (`dependent` imports `dependency`).
    pub fn add_dependency(&mut self, dependent: &str, dependency: &str) {
        let entry = self.dependencies.entry(dependent.to_owned()).or_default();
        if !entry.iter().any(|existing| existing == dependency) {
            entry.push(dependency.to_owned());
            entry.sort();
        }

        self.dependencies.entry(dependency.to_owned()).or_default();
    }

    /// Returns all transitive dependents of `module` in deterministic order.
    #[must_use]
    pub fn transitive_dependents(&self, module: &str) -> Vec<String> {
        let reverse_graph = self.reverse_adjacency();
        let mut visited = BTreeSet::new();
        let mut stack = Vec::new();
        stack.push(module.to_owned());

        while let Some(current_module) = stack.pop() {
            if let Some(dependents) = reverse_graph.get(&current_module) {
                for dependent in dependents {
                    if visited.insert(dependent.clone()) {
                        stack.push(dependent.clone());
                    }
                }
            }
        }

        visited.into_iter().collect()
    }

    /// Returns a copy of the adjacency list (`dependent -> dependencies`).
    #[must_use]
    pub fn adjacency_list(&self) -> BTreeMap<String, Vec<String>> {
        self.dependencies.clone()
    }

    /// Builds reverse edges (`dependency -> dependents`) for graph traversal.
    fn reverse_adjacency(&self) -> BTreeMap<String, Vec<String>> {
        let mut reverse = BTreeMap::new();

        for (dependent, dependencies) in &self.dependencies {
            reverse.entry(dependent.clone()).or_insert_with(Vec::new);

            for dependency in dependencies {
                let dependents = reverse.entry(dependency.clone()).or_insert_with(Vec::new);
                if !dependents.iter().any(|existing| existing == dependent) {
                    dependents.push(dependent.clone());
                    dependents.sort();
                }
            }
        }

        reverse
    }
}
