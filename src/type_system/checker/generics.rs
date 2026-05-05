extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::{CoreType, GenericTypeParameter, TypeVar};
use alloc::{collections::BTreeMap, string::String, vec::Vec};

impl TypeChecker {
    /// Register the predefined generic Pair<T, U> product type metadata.
    pub(super) fn register_builtin_pair_type(&mut self) {
        let first_var = TypeVar::new(10_001, "T".to_owned());
        let second_var = TypeVar::new(10_002, "U".to_owned());
        let pair_params = vec![
            GenericTypeParameter {
                name: "T".to_owned(),
                type_var: first_var.clone(),
                constraints: Vec::new(),
            },
            GenericTypeParameter {
                name: "U".to_owned(),
                type_var: second_var.clone(),
                constraints: Vec::new(),
            },
        ];
        let pair_fields = BTreeMap::from([
            ("first".to_owned(), CoreType::Variable(first_var)),
            ("second".to_owned(), CoreType::Variable(second_var)),
        ]);

        self.register_adt_generic_params("Pair".to_owned(), pair_params);
        self.register_adt_fields("Pair".to_owned(), pair_fields);
    }

    /// Register declared generic parameters for a nominal ADT.
    pub(super) fn register_adt_generic_params(
        &mut self,
        owner: String,
        params: Vec<GenericTypeParameter>,
    ) {
        self.adt_generic_params.insert(owner, params);
    }

    /// Retrieve declared generic parameters for a nominal ADT.
    pub(super) fn adt_generic_params_for(&self, owner: &str) -> Option<&Vec<GenericTypeParameter>> {
        self.adt_generic_params.get(owner)
    }

    /// Record one concrete generic instantiation if it has not been seen before.
    pub fn record_generic_instantiation(&mut self, name: &str, type_args: &[CoreType]) {
        let entry = self
            .generic_instantiations
            .entry(name.to_owned())
            .or_default();
        if !entry.iter().any(|existing| existing == type_args) {
            entry.push(type_args.to_vec());
        }
    }

    /// Expose recorded generic instantiation metadata for downstream phases.
    pub const fn generic_instantiations(&self) -> &BTreeMap<String, Vec<Vec<CoreType>>> {
        &self.generic_instantiations
    }
}
