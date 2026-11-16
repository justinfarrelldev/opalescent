//! Static World dependency validation and deterministic per-attempt ordering.
use super::identity::WorldIdentity;
extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// A resolved World node, supplied in deterministic source/declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldNode {
    /// Canonical nominal provider identity.
    pub identity: WorldIdentity,
    /// Explicit, ordered canonical dependencies; aliases have already resolved.
    pub dependencies: Vec<WorldIdentity>,
}

/// Compilation-time dependency graph error, never an attempt assertion failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorldGraphError {
    /// The same canonical declaration appears more than once.
    #[error("duplicate World declaration: {0:?}")]
    DuplicateWorld(WorldIdentity),
    /// Dependency aliases resolve to the same provider twice.
    #[error("World {world:?} subscribes to {dependency:?} more than once")]
    DuplicateDependency {
        /// Provider with a duplicate subscription.
        world: WorldIdentity,
        /// Duplicated canonical dependency.
        dependency: WorldIdentity,
    },
    /// A requested or declared dependency cannot be resolved.
    #[error("unknown World: {0:?}")]
    UnknownWorld(WorldIdentity),
    /// Complete closed cycle path, including its repeated first node.
    #[error("World dependency cycle: {0:?}")]
    Cycle(Vec<WorldIdentity>),
}

impl WorldGraphError {
    /// Inspect the exact diagnostic cycle path when this is a cycle error.
    #[must_use]
    pub fn cycle_path(&self) -> Option<&[WorldIdentity]> {
        match *self {
            Self::Cycle(ref path) => Some(path.as_slice()),
            _ => None,
        }
    }
}

/// Validated graph shared by compiled descriptors, never mutable World state.
#[derive(Debug, Clone)]
pub struct WorldGraph {
    /// Canonical nodes in deterministic source/declaration order.
    nodes: Vec<WorldNode>,
    /// Canonical identity to source-order index.
    indices: BTreeMap<WorldIdentity, usize>,
}

impl WorldGraph {
    /// Resolve graph invariants once, before generating any executable cases.
    ///
    /// # Errors
    /// Rejects duplicate declarations/dependencies, missing providers and cycles.
    pub fn new(nodes: Vec<WorldNode>) -> Result<Self, WorldGraphError> {
        let mut indices = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            if indices.insert(node.identity.clone(), index).is_some() {
                return Err(WorldGraphError::DuplicateWorld(node.identity.clone()));
            }
        }
        for node in &nodes {
            let mut dependencies = BTreeSet::new();
            for dependency in &node.dependencies {
                if !indices.contains_key(dependency) {
                    return Err(WorldGraphError::UnknownWorld(dependency.clone()));
                }
                if !dependencies.insert(dependency) {
                    return Err(WorldGraphError::DuplicateDependency {
                        world: node.identity.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        let graph = Self { nodes, indices };
        graph.validate_acyclic()?;
        Ok(graph)
    }

    /// Compute the reachable union and initialize each provider at most once.
    /// Independent ready nodes follow deterministic source/declaration order.
    /// Cleanup uses the reverse of this returned order.
    ///
    /// # Errors
    /// Returns a diagnostic when a root is not a resolved provider in this graph.
    pub fn initialization_order(
        &self,
        roots: &[WorldIdentity],
    ) -> Result<Vec<WorldIdentity>, WorldGraphError> {
        let mut reachable = BTreeSet::new();
        let mut pending = roots.to_vec();
        while let Some(identity) = pending.pop() {
            let Some(&index) = self.indices.get(&identity) else {
                return Err(WorldGraphError::UnknownWorld(identity));
            };
            if reachable.insert(index) {
                pending.extend(self.nodes[index].dependencies.iter().cloned());
            }
        }
        let mut remaining = BTreeMap::new();
        let mut dependents: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut ready = BTreeSet::new();
        for &index in &reachable {
            let node = &self.nodes[index];
            remaining.insert(index, node.dependencies.len());
            if node.dependencies.is_empty() {
                ready.insert(index);
            }
            for dependency in &node.dependencies {
                let dependency_index = self.indices[dependency];
                dependents.entry(dependency_index).or_default().push(index);
            }
        }
        let mut order = Vec::with_capacity(reachable.len());
        while let Some(index) = ready.pop_first() {
            order.push(self.nodes[index].identity.clone());
            for dependent in dependents.get(&index).into_iter().flatten() {
                if let Some(count) = remaining.get_mut(dependent) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(*dependent);
                    }
                }
            }
        }
        Ok(order)
    }

    /// Iterative DFS reports cycles without risking native-stack exhaustion.
    fn validate_acyclic(&self) -> Result<(), WorldGraphError> {
        let mut completed = BTreeSet::new();
        let mut active = Vec::new();
        let mut active_positions = BTreeMap::new();
        for node in &self.nodes {
            let mut pending = vec![(node.identity.clone(), false)];
            while let Some((identity, exiting)) = pending.pop() {
                if exiting {
                    active.pop();
                    active_positions.remove(&identity);
                    completed.insert(identity);
                    continue;
                }
                if completed.contains(&identity) {
                    continue;
                }
                if let Some(&position) = active_positions.get(&identity) {
                    let mut cycle = active.get(position..).unwrap_or_default().to_vec();
                    cycle.push(identity);
                    return Err(WorldGraphError::Cycle(cycle));
                }
                active_positions.insert(identity.clone(), active.len());
                active.push(identity.clone());
                pending.push((identity.clone(), true));
                let index = self.indices[&identity];
                pending.extend(
                    self.nodes[index]
                        .dependencies
                        .iter()
                        .rev()
                        .cloned()
                        .map(|dependency| (dependency, false)),
                );
            }
        }
        Ok(())
    }
}
