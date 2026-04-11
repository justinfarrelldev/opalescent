//! Type substitution for unification and type inference

extern crate alloc;

use super::types::CoreType;
use alloc::collections::BTreeMap;

/// Represents a substitution from type variables to types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    /// Map from type variable IDs to their substituted types
    pub(super) mappings: BTreeMap<usize, CoreType>,
}

impl Substitution {
    /// Create an empty substitution
    #[expect(clippy::missing_const_for_fn, reason = "BTreeMap::new() is not const")]
    pub fn empty() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Create a substitution with a single mapping
    pub fn single(var_id: usize, type_value: CoreType) -> Self {
        let mut mappings = BTreeMap::new();
        mappings.insert(var_id, type_value);
        Self { mappings }
    }

    /// Apply this substitution to a type
    pub fn apply(&self, core_type: &CoreType) -> CoreType {
        match *core_type {
            CoreType::Variable(ref var) => self
                .mappings
                .get(&var.id)
                .map_or_else(|| core_type.clone(), |substituted| self.apply(substituted)),
            CoreType::Array(ref element_type) => {
                CoreType::Array(Box::new(self.apply(element_type)))
            }
            CoreType::Function {
                ref parameters,
                ref return_types,
                ref error_types,
                ..
            } => CoreType::Function {
                generic_params: Vec::new(),
                parameters: parameters.iter().map(|p| self.apply(p)).collect(),
                return_types: return_types.iter().map(|r| self.apply(r)).collect(),
                error_types: error_types.iter().map(|e| self.apply(e)).collect(),
            },
            CoreType::Generic {
                ref name,
                ref type_args,
            } => CoreType::Generic {
                name: name.clone(),
                type_args: type_args.iter().map(|arg| self.apply(arg)).collect(),
            },
            // Primitive types don't contain type variables
            _ => core_type.clone(),
        }
    }

    /// Compose this substitution with another (self after other)
    pub fn compose(self, other: &Self) -> Self {
        let mut result_mappings = BTreeMap::new();

        // Apply self to all mappings in other
        for (var_id, type_value) in &other.mappings {
            result_mappings.insert(*var_id, self.apply(type_value));
        }

        // Add mappings from self that are not in other
        for (var_id, type_value) in self.mappings {
            result_mappings.entry(var_id).or_insert(type_value);
        }

        Self {
            mappings: result_mappings,
        }
    }

    /// Check if this substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Get the mappings for testing
    pub const fn mappings(&self) -> &BTreeMap<usize, CoreType> {
        &self.mappings
    }
}
