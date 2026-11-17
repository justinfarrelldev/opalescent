//! Deterministic canonical module order for the ordinary compiler interfaces.
extern crate alloc;
use super::{NativeCheckError, NativeTestProject, module_span, type_error};
use crate::testing::native::project::module_dependencies;
use crate::type_system::errors::TypeError;
use alloc::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Iterative dependency traversal, preserving original import order and cycles.
pub(super) fn resolve(project: &NativeTestProject) -> Result<Vec<PathBuf>, NativeCheckError> {
    let mut completed = BTreeSet::new();
    let mut active = Vec::new();
    let mut active_positions = BTreeMap::new();
    let mut order = Vec::new();
    for root in project.modules.keys() {
        let mut pending = vec![(root.clone(), false)];
        while let Some((path, exiting)) = pending.pop() {
            if exiting {
                active.pop();
                active_positions.remove(&path);
                completed.insert(path.clone());
                order.push(path);
                continue;
            }
            if completed.contains(&path) {
                continue;
            }
            if let Some(&position) = active_positions.get(&path) {
                let mut cycle: Vec<_> = active
                    .get(position..)
                    .unwrap_or_default()
                    .iter()
                    .map(|cycle_module| project.modules[cycle_module].identity.clone())
                    .collect();
                cycle.push(project.modules[&path].identity.clone());
                return Err(type_error(
                    &path,
                    TypeError::CircularDependency {
                        cycle,
                        span: TypeError::span_from_span(module_span(&project.modules[&path])),
                    },
                ));
            }
            active_positions.insert(path.clone(), active.len());
            active.push(path.clone());
            let dependencies = module_dependencies(&path, &project.modules[&path].ast)?;
            pending.push((path, true));
            pending.extend(
                dependencies
                    .into_iter()
                    .rev()
                    .map(|(dependency, _)| (dependency, false)),
            );
        }
    }
    Ok(order)
}
